//! Verifies the server-signed entitlement before any Pro feature runs.
//!
//! Ported from PC Tweaker's `license.rs` with one ecosystem-critical
//! addition: the payload now carries a `product` field, and this client
//! accepts a license only when that field equals [`PRODUCT_ID`]. Without the
//! check, a license fetched for PC Tweaker (same signing key, same account)
//! would unlock the Uninstaller's Pro features too — the signature alone
//! cannot tell the products apart because the whole ecosystem shares one
//! signing key on purpose (one backend, one key to protect).
//!
//! Everything else keeps PC Tweaker's proven semantics: the server signs the
//! exact JSON string it returns, the client verifies those exact bytes with
//! the embedded Ed25519 public key BEFORE parsing, a short freshness window
//! bounds how long a cached license works offline, and every failure mode
//! collapses to "not Pro" — there is no partial-credit path.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// The ecosystem product this binary is. Licenses naming any other product
/// are rejected even when their signature is valid.
pub const PRODUCT_ID: &str = "uninstaller";

/// Public half of the ecosystem's license-signing key pair — the same key
/// PC Tweaker embeds, because both products verify licenses signed by the
/// same backend. Safe to embed: it can only verify, never sign.
const PUBLIC_KEY_B64: &str = "QisYr46g3mqEeiz1BDyEcPbRO1xO4z0lR3d5/ODppIU=";

/// How long a signed license with no stated expiry is trusted without a fresh
/// fetch. Long enough for a weekend offline, short enough that a cancelled
/// subscription does not keep working indefinitely.
const GRACE_PERIOD_SECS: u64 = 3 * 24 * 60 * 60;

/// How long a signed license is trusted when its payload names a paid period
/// that has not ended yet.
///
/// Three days was the only window, and it was applied whether or not the
/// signature said the customer had paid through next March. That made every
/// outage a refund conversation: a backend that cannot be reached for four
/// days - Railway down, DNS, a customer's company firewall, a laptop taken
/// somewhere without signal - takes Pro away from people who are paid up,
/// with no way for them to prove it and nothing they can do about it.
///
/// When `expires_at` is in the future the signature itself is the proof: the
/// backend stated a paid-through date and signed it, and the private half of
/// that key is not on this machine. Honouring it is not a concession, it is
/// reading what was signed. The bound stays so that a licence can never run
/// forever without the product hearing from the server again - it just stops
/// being three days for somebody who is demonstrably paid up.
const PAID_PERIOD_GRACE_SECS: u64 = 30 * 24 * 60 * 60;

/// How far this PC's clock may run behind the server's.
const MAX_CLOCK_SKEW_SECS: u64 = 24 * 60 * 60;

/// Field names must match the backend's `LicensePayload` type exactly,
/// camelCase included — parsed directly from the JSON string the server
/// signed, never adapted at any boundary in between.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LicensePayload {
    pub user_id: String,
    pub is_pro: bool,
    pub plan: Option<String>,
    /// Which ecosystem product this license speaks for. Checked against
    /// [`PRODUCT_ID`] in [`LicenseStore::is_pro_and_fresh`].
    pub product: String,
    /// Unix seconds, set by the server at signing time. Freshness is
    /// measured from this, not from local receipt — a cached-and-replayed
    /// response can't be made to look newer than it actually is.
    pub issued_at: u64,
    #[serde(default)]
    pub expires_at: Option<u64>,
}

/// What the server sends: the exact bytes it signed, plus the signature over
/// those exact bytes. `payload_json` is verified as opaque bytes first and
/// parsed only afterward — never re-serialized to check a signature against.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignedLicenseResponse {
    pub payload_json: String,
    pub signature: String,
}

#[derive(Debug, PartialEq)]
pub enum VerifyError {
    BadPublicKey,
    BadSignatureEncoding,
    SignatureInvalid,
    UnparsablePayload,
}

/// Verifies `signature` (base64) against `payload_json`'s raw UTF-8 bytes and
/// returns the parsed payload only if it checks out. This is the only path
/// by which a `LicensePayload` should ever come into existence in this
/// process — there is no constructor that skips verification.
pub fn verify(payload_json: &str, signature_b64: &str) -> Result<LicensePayload, VerifyError> {
    let key_bytes: [u8; 32] = STANDARD
        .decode(PUBLIC_KEY_B64)
        .ok()
        .and_then(|v| v.try_into().ok())
        .ok_or(VerifyError::BadPublicKey)?;
    let verifying_key =
        VerifyingKey::from_bytes(&key_bytes).map_err(|_| VerifyError::BadPublicKey)?;

    let sig_bytes: [u8; 64] = STANDARD
        .decode(signature_b64)
        .ok()
        .and_then(|v| v.try_into().ok())
        .ok_or(VerifyError::BadSignatureEncoding)?;
    let signature = Signature::from_bytes(&sig_bytes);

    verifying_key
        .verify(payload_json.as_bytes(), &signature)
        .map_err(|_| VerifyError::SignatureInvalid)?;

    serde_json::from_str(payload_json).map_err(|_| VerifyError::UnparsablePayload)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// This PC's clock, never earlier than the clock at start plus the time the
/// process has run: moving the clock back while the app is open cannot
/// stretch the offline window (same anchor as PC Tweaker's license.rs).
fn monotonic_now() -> u64 {
    static ANCHOR: OnceLock<(u64, Instant)> = OnceLock::new();
    let wall = now_secs();
    let (start, tick) = ANCHOR.get_or_init(|| (wall, Instant::now()));
    wall.max(start.saturating_add(tick.elapsed().as_secs()))
}

/// A payload that passed signature verification AND is still inside its grace
/// period: a validly signed license from three weeks ago proves the server
/// said something true three weeks ago, not that it is still true now.
fn is_fresh(payload: &LicensePayload) -> bool {
    let now = monotonic_now();
    // A licence issued in the future is a clock problem or a forgery. A day of
    // slack covers PCs whose clock runs behind the server (a hand-set time
    // zone, a drifting clock), as the account service allows; a minute locked
    // Pro on them. Freshness is still measured from the signed time.
    if payload.issued_at > now + MAX_CLOCK_SKEW_SECS {
        return false;
    }
    let age = now.saturating_sub(payload.issued_at);
    match payload.expires_at {
        // Paid through a date still ahead of us: trust it for the longer
        // window, never past the date the backend actually signed.
        Some(expiry) if now < expiry => age <= PAID_PERIOD_GRACE_SECS,
        // The paid period has ended. No window applies; this is not a
        // staleness question any more.
        Some(_) => false,
        // Older licences carry no expiry, so age is all there is to go on.
        None => age <= GRACE_PERIOD_SECS,
    }
}

/// SHA-256 of `pc-tweaker-licence:` and this Windows installation's
/// MachineGuid (lower case). Sent with every licence request; the backend
/// signs it into the licence, so a `license.json` copied to another PC
/// unlocks nothing there. The same value in PC Tweaker and the Uninstaller:
/// it names the PC, not the product. None where the registry cannot be read.
pub(crate) fn device_hash() -> Option<String> {
    static HASH: OnceLock<Option<String>> = OnceLock::new();
    HASH.get_or_init(|| machine_guid().map(|guid| hash_guid(&guid)))
        .clone()
}

fn hash_guid(guid: &str) -> String {
    use sha2::{Digest, Sha256};
    format!(
        "{:x}",
        Sha256::digest(format!(
            "pc-tweaker-licence:{}",
            guid.trim().to_ascii_lowercase()
        ))
    )
}

#[cfg(windows)]
fn machine_guid() -> Option<String> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
    let key = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(
            r"SOFTWARE\Microsoft\Cryptography",
            KEY_READ | KEY_WOW64_64KEY,
        )
        .ok()?;
    let guid: String = key.get_value("MachineGuid").ok()?;
    (!guid.trim().is_empty()).then_some(guid)
}
#[cfg(not(windows))]
fn machine_guid() -> Option<String> {
    None
}

/// A licence without `deviceHash` (issued before the backend bound licences
/// to a PC) stays valid until it expires as before; one with it must name
/// this PC. Read from the exact signed bytes, after the signature check.
fn bound_to(payload_json: &str, this_pc: Option<&str>) -> bool {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Binding {
        #[serde(default)]
        device_hash: Option<String>,
    }
    match serde_json::from_str::<Binding>(payload_json) {
        Ok(Binding { device_hash: None }) => true,
        Ok(Binding {
            device_hash: Some(hash),
        }) => this_pc == Some(hash.as_str()),
        Err(_) => false,
    }
}

/// For the licence request header: this PC's hash, or nothing.
#[tauri::command]
pub fn license_device_hash() -> Option<String> {
    device_hash()
}

/// Persists the raw, still-signed response so it survives a restart.
/// Written via temp-file-then-rename (same pattern as `rollback.rs`), so an
/// interrupted write can never leave a half-written — and therefore
/// Pro-denying — file behind.
pub struct LicenseStore {
    file_path: PathBuf,
}

impl LicenseStore {
    pub fn new(app_data_dir: PathBuf) -> Self {
        LicenseStore {
            file_path: app_data_dir.join("license.json"),
        }
    }

    pub fn save(&self, response: &SignedLicenseResponse) -> std::io::Result<()> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(response).unwrap();
        let tmp = self
            .file_path
            .with_extension(format!("json.{}.tmp", std::process::id()));
        std::fs::write(&tmp, json)?;
        let result = std::fs::rename(&tmp, &self.file_path);
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        result
    }

    fn load(&self) -> Option<SignedLicenseResponse> {
        let text = std::fs::read_to_string(&self.file_path).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Removes the cached license on logout, so a still-fresh Pro cache from
    /// one account can't keep unlocking features for whoever signs in next.
    pub fn delete(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.file_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// The one question this module exists to answer: is Pro currently
    /// verified, for THIS product, and still fresh? No cache, corrupt file,
    /// bad signature, wrong product, stale issue date, or `is_pro: false`
    /// all collapse to the same `false`, on purpose.
    pub fn is_pro_and_fresh(&self) -> bool {
        let Some(cached) = self.load() else {
            return false;
        };
        let Ok(payload) = verify(&cached.payload_json, &cached.signature) else {
            return false;
        };
        payload.product == PRODUCT_ID
            && payload.is_pro
            && is_fresh(&payload)
            && bound_to(&cached.payload_json, device_hash().as_deref())
    }
}

/// Called by the frontend right after fetching a fresh signed license from
/// the backend. Verifies signature AND product before saving anything, so a
/// response for the wrong product never even reaches the cache.
#[tauri::command]
pub fn save_license(app: tauri::AppHandle, response: SignedLicenseResponse) -> Result<(), String> {
    let payload = verify(&response.payload_json, &response.signature)
        .map_err(|e| format!("license did not verify: {:?}", e))?;
    if payload.product != PRODUCT_ID {
        return Err(format!(
            "license is for product '{}', this app is '{}'",
            payload.product, PRODUCT_ID
        ));
    }
    if !bound_to(&response.payload_json, device_hash().as_deref()) {
        return Err("license was issued to another PC".into());
    }
    let store = LicenseStore::new(crate::app_data_dir(&app)?);
    store.save(&response).map_err(|e| e.to_string())
}

/// Read-only status for the UI. Never the source of truth a Pro feature
/// relies on — enforcement points re-verify independently on every call.
#[tauri::command]
pub fn license_status(app: tauri::AppHandle) -> Result<bool, String> {
    let store = LicenseStore::new(crate::app_data_dir(&app)?);
    Ok(store.is_pro_and_fresh())
}

/// Called on logout.
#[tauri::command]
pub fn clear_license(app: tauri::AppHandle) -> Result<(), String> {
    LicenseStore::new(crate::app_data_dir(&app)?)
        .delete()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ptu-license-test-{}-{}-{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Real responses produced by the production backend (2026-08-20) with
    /// the private half of the embedded key — not values invented on the
    /// Rust side. These prove the Node and Rust implementations agree on
    /// wire format including the new `product` field.
    const REAL_UNINSTALLER_RESPONSE: &str = r#"{"payloadJson":"{\"userId\":\"17\",\"isPro\":false,\"plan\":null,\"product\":\"uninstaller\",\"issuedAt\":1787184494}","signature":"tkIXmjlBNm4ZcQ5P+Zn6gbKeVM3FN5Wkmmt1qgn6m9giYzI6NsyiU1rV+ZGYv8zS+6RoU+J52/HT4OMt9WlaCA=="}"#;
    const REAL_PCTWEAKER_RESPONSE: &str = r#"{"payloadJson":"{\"userId\":\"17\",\"isPro\":false,\"plan\":null,\"product\":\"pctweaker\",\"issuedAt\":1787184494}","signature":"l6azTddf0rEurxP4VqlRiTmxZvIhAH5HGH0krUXLEuqT7cNKsNiTwl2+RgC3z07UNpvt4SLpWuxIXKjwhdorAQ=="}"#;

    #[test]
    fn a_real_signature_from_the_node_backend_verifies_with_product() {
        let resp: SignedLicenseResponse = serde_json::from_str(REAL_UNINSTALLER_RESPONSE).unwrap();
        let payload = verify(&resp.payload_json, &resp.signature).expect("must verify");
        assert_eq!(payload.product, "uninstaller");
        assert_eq!(payload.user_id, "17");
        assert!(!payload.is_pro);
    }

    #[test]
    fn a_single_flipped_byte_in_the_payload_is_rejected() {
        let resp: SignedLicenseResponse = serde_json::from_str(REAL_UNINSTALLER_RESPONSE).unwrap();
        let tampered = resp
            .payload_json
            .replace("\"isPro\":false", "\"isPro\":true");
        assert_ne!(tampered, resp.payload_json);
        assert_eq!(
            verify(&tampered, &resp.signature),
            Err(VerifyError::SignatureInvalid)
        );
    }

    /// The ecosystem-critical case: a validly signed PC Tweaker license must
    /// not unlock the Uninstaller. Same key, same account — only `product`
    /// tells them apart, so this test is the product check's reason to exist.
    #[test]
    fn a_valid_license_for_another_product_never_reads_as_pro_here() {
        let resp: SignedLicenseResponse = serde_json::from_str(REAL_PCTWEAKER_RESPONSE).unwrap();
        // Signature itself is fine...
        let payload = verify(&resp.payload_json, &resp.signature).expect("must verify");
        assert_eq!(payload.product, "pctweaker");

        // ...but the store must refuse it wholesale.
        let dir = temp_dir("wrong-product");
        let store = LicenseStore::new(dir.clone());
        store.save(&resp).unwrap();
        assert!(!store.is_pro_and_fresh());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_fresh_pro_payload_for_this_product_reads_as_pro_via_the_store_logic() {
        // No private key here, so freshness/product logic is exercised
        // directly; verify() itself is proven against real Node output above.
        let payload = LicensePayload {
            user_id: "1".into(),
            is_pro: true,
            plan: Some("annual".into()),
            product: PRODUCT_ID.into(),
            issued_at: now_secs(),
            expires_at: None,
        };
        assert!(is_fresh(&payload));
        assert_eq!(payload.product, PRODUCT_ID);
    }

    #[test]
    fn a_payload_older_than_the_grace_period_is_not_fresh() {
        let stale = LicensePayload {
            user_id: "1".into(),
            is_pro: true,
            plan: None,
            product: PRODUCT_ID.into(),
            issued_at: now_secs().saturating_sub(GRACE_PERIOD_SECS + 3600),
            expires_at: None,
        };
        assert!(!is_fresh(&stale));
    }

    /// A customer who is paid up must not lose Pro because the backend could
    /// not be reached. Three days was the only window and it applied even to
    /// a licence signed as paid through next month, which turned every
    /// outage - Railway down, a firewall, a laptop with no signal - into a
    /// refund conversation the customer was right to start.
    #[test]
    fn a_paid_period_still_running_survives_more_than_the_short_grace() {
        let payload = LicensePayload {
            user_id: "17".into(),
            is_pro: true,
            plan: Some("annual".into()),
            product: PRODUCT_ID.into(),
            issued_at: now_secs().saturating_sub(GRACE_PERIOD_SECS + 4 * 24 * 60 * 60),
            expires_at: Some(now_secs() + 90 * 24 * 60 * 60),
        };
        assert!(
            is_fresh(&payload),
            "a week offline with three months paid ahead must keep Pro"
        );
    }

    /// The longer window is a window, not an exemption: a licence still has
    /// to hear from the server eventually, however far ahead it was paid.
    #[test]
    fn even_a_paid_period_stops_once_the_longer_window_runs_out() {
        let payload = LicensePayload {
            user_id: "17".into(),
            is_pro: true,
            plan: Some("annual".into()),
            product: PRODUCT_ID.into(),
            issued_at: now_secs().saturating_sub(PAID_PERIOD_GRACE_SECS + 3600),
            expires_at: Some(now_secs() + 90 * 24 * 60 * 60),
        };
        assert!(!is_fresh(&payload));
    }

    /// Past the date the backend signed, no window applies at all - the
    /// question stops being how stale the file is.
    #[test]
    fn a_finished_paid_period_is_over_however_recently_it_was_issued() {
        let payload = LicensePayload {
            user_id: "17".into(),
            is_pro: true,
            plan: Some("annual".into()),
            product: PRODUCT_ID.into(),
            issued_at: now_secs().saturating_sub(60),
            expires_at: Some(now_secs().saturating_sub(1)),
        };
        assert!(!is_fresh(&payload));
    }

    #[test]
    fn expiry_and_future_issue_dates_deny_cached_access() {
        let mut payload = LicensePayload {
            user_id: "1".into(),
            is_pro: true,
            plan: Some("lifetime_bonus".into()),
            product: PRODUCT_ID.into(),
            issued_at: now_secs(),
            expires_at: Some(now_secs()),
        };
        assert!(!is_fresh(&payload));
        payload.expires_at = Some(now_secs() + 3600);
        assert!(is_fresh(&payload));
        // Regression: a PC two hours behind the server must accept a licence
        // signed a moment ago; one more than a day ahead is still refused.
        payload.issued_at = now_secs() + 2 * 60 * 60;
        assert!(is_fresh(&payload));
        payload.issued_at = now_secs() + MAX_CLOCK_SKEW_SECS + 600;
        assert!(!is_fresh(&payload));
    }

    #[test]
    fn no_cached_file_and_corrupt_cache_both_mean_not_pro_rather_than_a_crash() {
        let dir = temp_dir("missing");
        let store = LicenseStore::new(dir.clone());
        assert!(!store.is_pro_and_fresh());
        std::fs::write(dir.join("license.json"), "not json at all").unwrap();
        assert!(!store.is_pro_and_fresh());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn deleting_the_cache_clears_it_and_is_idempotent() {
        let dir = temp_dir("delete");
        let store = LicenseStore::new(dir.clone());
        let resp: SignedLicenseResponse = serde_json::from_str(REAL_UNINSTALLER_RESPONSE).unwrap();
        store.save(&resp).unwrap();
        assert!(store.load().is_some());
        store.delete().unwrap();
        assert!(store.load().is_none());
        store.delete().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_licence_bound_to_another_pc_unlocks_nothing_here() {
        let here = hash_guid("1B4E28BA-2FA1-11D2-883F-0016D3CCA427");
        assert_eq!(here, hash_guid(" 1b4e28ba-2fa1-11d2-883f-0016d3cca427 "));
        let payload = |hash: Option<&str>| {
            let mut value = serde_json::json!({"userId":"42","isPro":true,"plan":"annual","product":"uninstaller","issuedAt":1});
            if let Some(hash) = hash {
                value["deviceHash"] = serde_json::Value::String(hash.into());
            }
            value.to_string()
        };
        assert!(bound_to(&payload(Some(&here)), Some(&here)));
        assert!(!bound_to(
            &payload(Some(&hash_guid("another-pc"))),
            Some(&here)
        ));
        assert!(!bound_to(&payload(Some(&here)), None));
        assert!(bound_to(&payload(None), Some(&here)));
        assert!(!bound_to("not json", Some(&here)));
    }

    #[test]
    fn the_clock_used_for_freshness_never_runs_backwards_in_a_process() {
        let first = monotonic_now();
        assert!(monotonic_now() >= first);
        assert!(first + 5 >= now_secs());
    }
}
