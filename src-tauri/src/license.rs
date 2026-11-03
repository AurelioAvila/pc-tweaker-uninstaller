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
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// The ecosystem product this binary is. Licenses naming any other product
/// are rejected even when their signature is valid.
pub const PRODUCT_ID: &str = "uninstaller";

/// Public half of the ecosystem's license-signing key pair — the same key
/// PC Tweaker embeds, because both products verify licenses signed by the
/// same backend. Safe to embed: it can only verify, never sign. Key 2 since the
/// 23 Oct 2026 rotation, requested with `?sig=2`; key 1 is no longer trusted.
const PUBLIC_KEY_B64: &str = "0Cy9wfSUsS/47MQrkgWjlaphga0Rqoho2RPCWMNUSro=";

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

/// How far this PC's clock may be off the server's: a licence signed up to this
/// far in the future is accepted, and the clock offset measured when a licence
/// arrives (`clock_offset`) is bounded by it.
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
/// process has run, so moving the clock back while the app is open cannot
/// stretch the offline window. Still this PC's time, not the server's.
fn wall_now() -> u64 {
    static ANCHOR: OnceLock<(u64, Instant)> = OnceLock::new();
    let wall = now_secs();
    let (start, tick) = ANCHOR.get_or_init(|| (wall, Instant::now()));
    wall.max(start.saturating_add(tick.elapsed().as_secs()))
}

/// The server's clock as far as this PC can tell: its own clock moved by the
/// licence's `offset` (`clock_offset`), never earlier than the server time it
/// has proof of (`observe_server_time`), which a restart cannot undo either.
/// Every freshness check reads this. Same anchors as PC Tweaker's license.rs.
fn monotonic_now(offset: i64) -> u64 {
    server_now(wall_now(), offset, trusted_now())
}

/// Only this PC's clock takes the offset; the floor is server time already. A
/// floor raised from this PC's own clock would put a PC a day ahead back a day
/// ahead, so nothing but signed server times ever raises it.
fn server_now(wall: u64, offset: i64, floor: u64) -> u64 {
    wall.saturating_add_signed(offset).max(floor)
}

/// Seconds to add to this PC's clock to read the server's: a licence's signed
/// `issued_at` minus this PC's clock when it arrived. A PC whose clock runs a
/// day ahead (a time zone set by hand) otherwise read the last day of a plan as
/// over. Bounded by the skew allowance: `receivedAt` is not signed, so an
/// edited one, or an old licence saved again, moves the clock by a day at most,
/// never a licence by a month.
fn clock_offset(issued_at: u64, received_at: u64) -> i64 {
    let skew = i128::from(MAX_CLOCK_SKEW_SECS);
    (i128::from(issued_at) - i128::from(received_at)).clamp(-skew, skew) as i64
}

/// Server time this process has proof of: the newest signed `issued_at` it
/// verified, or the floor saved with the cached licence (`seenAt`), plus the
/// time the app has run since (monotonic). Never ahead of real time, so a PC
/// whose clock runs behind is never locked out by it; it only grows.
static TRUSTED: Mutex<Option<(u64, Instant)>> = Mutex::new(None);

fn trusted_now() -> u64 {
    TRUSTED
        .lock()
        .ok()
        .and_then(|guard| *guard)
        .map_or(0, |(time, tick)| {
            time.saturating_add(tick.elapsed().as_secs())
        })
}

fn observe_server_time(time: u64) {
    if let Ok(mut guard) = TRUSTED.lock() {
        let current = guard.map_or(0, |(base, tick)| {
            base.saturating_add(tick.elapsed().as_secs())
        });
        if time > current {
            *guard = Some((time, Instant::now()));
        }
    }
}

/// The last moment a licence can be fresh offline. A floor read back from disk
/// is capped here: an edited or damaged `seenAt` can end this licence early,
/// never the next one the server signs.
fn offline_window_end(payload: &LicensePayload) -> u64 {
    payload
        .issued_at
        .saturating_add(if payload.expires_at.is_some() {
            PAID_PERIOD_GRACE_SECS
        } else {
            GRACE_PERIOD_SECS
        })
}

/// Freshness of a cached licence after a restart: the time this PC had
/// already proved when the licence was last checked (`seen_at`) counts even if
/// the clock has been moved back since, so rolling Windows time back to before
/// a subscription's end stops working once the app has seen that end.
fn fresh_after_restart(payload: &LicensePayload, seen_at: Option<u64>, now: u64) -> bool {
    let floor = seen_at.map_or(0, |seen| seen.min(offline_window_end(payload)));
    is_fresh_at(payload, now.max(floor))
}

/// How often a still-running app writes its proven time back to `license.json`.
const FLOOR_WRITE_SECS: u64 = 60;

/// A payload that passed signature verification AND is still inside its grace
/// period: a validly signed license from three weeks ago proves the server
/// said something true three weeks ago, not that it is still true now.
/// `offset` is the one measured when this licence arrived.
fn is_fresh(payload: &LicensePayload, offset: i64) -> bool {
    // A signed issue time is the server's clock: real time is at least that.
    // Only within the skew a slow PC is allowed.
    if payload.issued_at <= monotonic_now(offset).saturating_add(MAX_CLOCK_SKEW_SECS) {
        observe_server_time(payload.issued_at);
    }
    is_fresh_at(payload, monotonic_now(offset))
}

fn is_fresh_at(payload: &LicensePayload, now: u64) -> bool {
    // A licence issued in the future is a clock problem or a forgery. A day of
    // slack covers PCs whose clock runs behind the server (a hand-set time
    // zone, a drifting clock), as the account service allows; a minute locked
    // Pro on them. Freshness is still measured from the signed time.
    if payload.issued_at > now.saturating_add(MAX_CLOCK_SKEW_SECS) {
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

/// A licence carrying `deviceHash` must name this PC. One without it is
/// accepted only on a PC whose identity cannot be read: the server issues such
/// a licence to any request without a valid X-Device-Hash, so on a PC that has
/// an identity it would be a licence fetched for no PC in particular, which a
/// copied `license.json` must not be able to use. Read from the exact signed
/// bytes, after the signature check.
fn bound_to(payload_json: &str, this_pc: Option<&str>) -> bool {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Binding {
        #[serde(default)]
        device_hash: Option<String>,
    }
    match serde_json::from_str::<Binding>(payload_json) {
        Ok(Binding { device_hash: None }) => this_pc.is_none(),
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
/// Written via temp-file-then-rename (same pattern as the ledger), so an
/// interrupted write can never leave a half-written — and therefore
/// Pro-denying — file behind.
pub struct LicenseStore {
    file_path: PathBuf,
}

/// `license.json`: the signed response, verbatim, plus `seenAt`, the server
/// time this PC had proof of when the licence was last checked (see
/// `fresh_after_restart`), and `receivedAt`, this PC's clock when the licence
/// arrived (see `clock_offset`). Older releases read the same file and ignore
/// both; files they wrote lack them and read as they always did.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CachedLicense {
    #[serde(flatten)]
    response: SignedLicenseResponse,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    seen_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    received_at: Option<u64>,
}

impl CachedLicense {
    /// The offset measured when this licence arrived; none (0) for a file
    /// written before `receivedAt` existed.
    fn offset(&self, payload: &LicensePayload) -> i64 {
        self.received_at
            .map_or(0, |received| clock_offset(payload.issued_at, received))
    }
}

/// One writer at a time: the floor is written back from read paths too.
static STORE_WRITE: Mutex<()> = Mutex::new(());

impl LicenseStore {
    pub fn new(app_data_dir: PathBuf) -> Self {
        LicenseStore {
            file_path: app_data_dir.join("license.json"),
        }
    }

    pub fn save(&self, response: &SignedLicenseResponse) -> std::io::Result<()> {
        let seen_at = trusted_now();
        let cached = CachedLicense {
            response: response.clone(),
            seen_at: (seen_at > 0).then_some(seen_at),
            received_at: Some(now_secs()),
        };
        self.write(&cached, false)
    }

    /// `only_if_present`: the floor rewrite from a status check, which must
    /// never bring back a licence that sign-out deleted after it was read.
    fn write(&self, cached: &CachedLicense, only_if_present: bool) -> std::io::Result<()> {
        let _guard = STORE_WRITE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if only_if_present && !self.file_path.is_file() {
            return Ok(());
        }
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(cached).unwrap();
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

    #[cfg(test)]
    fn load(&self) -> Option<SignedLicenseResponse> {
        self.load_cached().map(|cached| cached.response)
    }

    fn load_cached(&self) -> Option<CachedLicense> {
        let text = std::fs::read_to_string(&self.file_path).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Removes the cached license on logout, so a still-fresh Pro cache from
    /// one account can't keep unlocking features for whoever signs in next.
    pub fn delete(&self) -> std::io::Result<()> {
        let _guard = STORE_WRITE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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
        let Some(cached) = self.load_cached() else {
            return false;
        };
        let Ok(payload) = verify(&cached.response.payload_json, &cached.response.signature) else {
            return false;
        };
        if payload.product != PRODUCT_ID
            || !bound_to(&cached.response.payload_json, device_hash().as_deref())
        {
            return false;
        }
        if let Some(seen) = cached.seen_at {
            observe_server_time(seen.min(offline_window_end(&payload)));
        }
        let offset = cached.offset(&payload);
        let fresh = is_fresh(&payload, offset)
            && fresh_after_restart(&payload, cached.seen_at, monotonic_now(offset));
        // Keep the proven time on disk while the app runs, so a restart with
        // the clock moved back starts from it. Server time only: `trusted_now`.
        let proven = trusted_now();
        if proven >= cached.seen_at.unwrap_or(0).saturating_add(FLOOR_WRITE_SECS) {
            let floor = CachedLicense {
                seen_at: Some(proven),
                ..cached
            };
            let _ = self.write(&floor, true);
        }
        payload.is_pro && fresh
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

    /// Real responses signed by the backend's key 2 (re-signed 2026-10-23 for
    /// the rotation) — not values invented on the Rust side. These prove the Node and Rust implementations agree on
    /// wire format including the new `product` field.
    const REAL_UNINSTALLER_RESPONSE: &str = r#"{"payloadJson":"{\"userId\":\"17\",\"isPro\":false,\"plan\":null,\"product\":\"uninstaller\",\"issuedAt\":1787184494}","signature":"4sJEDtbhVgMWTyGhGXY0OaSzHcXap6dqEnCK7LaAji11fBCeBZ2XxnAai7ePtxzOPDzAHzRBEDqjzIDWdK4QDw=="}"#;
    const REAL_PCTWEAKER_RESPONSE: &str = r#"{"payloadJson":"{\"userId\":\"17\",\"isPro\":false,\"plan\":null,\"product\":\"pctweaker\",\"issuedAt\":1787184494}","signature":"oIa8M1x7OD2sg/ZroNJkA/l6L9UQIC7v9WtsWjfYl+IzWk0c1ZtDMdYNZHxSM0Daswi4CYlAYsXJmD9XqMMNDw=="}"#;

    /// The same uninstaller payload signed by the retired key 1.
    const KEY1_UNINSTALLER_RESPONSE: &str = r#"{"payloadJson":"{\"userId\":\"17\",\"isPro\":false,\"plan\":null,\"product\":\"uninstaller\",\"issuedAt\":1787184494}","signature":"tkIXmjlBNm4ZcQ5P+Zn6gbKeVM3FN5Wkmmt1qgn6m9giYzI6NsyiU1rV+ZGYv8zS+6RoU+J52/HT4OMt9WlaCA=="}"#;

    #[test]
    fn a_licence_signed_by_the_retired_key_1_is_rejected() {
        let resp: SignedLicenseResponse = serde_json::from_str(KEY1_UNINSTALLER_RESPONSE).unwrap();
        assert_eq!(
            verify(&resp.payload_json, &resp.signature),
            Err(VerifyError::SignatureInvalid)
        );
    }

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
        assert!(is_fresh(&payload, 0));
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
        assert!(!is_fresh(&stale, 0));
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
            is_fresh(&payload, 0),
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
        assert!(!is_fresh(&payload, 0));
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
        assert!(!is_fresh(&payload, 0));
    }

    #[test]
    fn expiry_and_future_issue_dates_deny_cached_access() {
        // A fixed clock: is_fresh() also learns server time from what it verifies,
        // shared by every test in this process.
        let now = 1_800_000_000;
        let mut payload = LicensePayload {
            user_id: "1".into(),
            is_pro: true,
            plan: Some("lifetime_bonus".into()),
            product: PRODUCT_ID.into(),
            issued_at: now,
            expires_at: Some(now),
        };
        assert!(!is_fresh_at(&payload, now));
        payload.expires_at = Some(now + 3600);
        assert!(is_fresh_at(&payload, now));
        // Regression: a PC two hours behind the server must accept a licence
        // signed a moment ago; one more than a day ahead is still refused.
        payload.issued_at = now + 2 * 60 * 60;
        assert!(is_fresh_at(&payload, now));
        payload.issued_at = now + MAX_CLOCK_SKEW_SECS + 600;
        assert!(!is_fresh_at(&payload, now));
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
        // Codex 1 / Argon 3: the server signs a licence without deviceHash for any
        // request that omits X-Device-Hash; on a PC with an identity it unlocks nothing.
        assert!(!bound_to(&payload(None), Some(&here)));
        assert!(
            bound_to(&payload(None), None),
            "a PC whose identity cannot be read keeps the unbound licence it was issued"
        );
        assert!(!bound_to("not json", Some(&here)));
    }

    /// Codex 5: rolling Windows time back across a restart must not revive a
    /// licence the app has already seen expire.
    #[test]
    fn a_clock_moved_back_across_a_restart_does_not_extend_a_cached_licence() {
        let issued = 1_800_000_000;
        let payload = LicensePayload {
            user_id: "rollback".into(),
            is_pro: true,
            plan: Some("annual".into()),
            product: PRODUCT_ID.into(),
            issued_at: issued,
            expires_at: Some(issued + 3600),
        };
        let seen = issued + 7200;
        let rolled_back = issued + 60;
        assert!(
            is_fresh_at(&payload, rolled_back),
            "the freshness check alone believes the moved-back clock"
        );
        assert!(!fresh_after_restart(&payload, Some(seen), rolled_back));
        // Not locked out: a PC two hours behind the server, before the end.
        assert!(fresh_after_restart(
            &payload,
            Some(issued),
            issued - 2 * 3600
        ));
        assert!(fresh_after_restart(&payload, None, issued + 59));
        // A damaged or edited seenAt is capped at this licence's own window.
        let open = LicensePayload {
            expires_at: Some(issued + 90 * 24 * 3600),
            ..payload.clone()
        };
        assert!(fresh_after_restart(&open, Some(u64::MAX), issued));
    }

    #[test]
    fn the_cache_keeps_its_proven_time_and_when_the_licence_arrived() {
        let dir = temp_dir("seen-at");
        let store = LicenseStore::new(dir.clone());
        let resp: SignedLicenseResponse = serde_json::from_str(REAL_UNINSTALLER_RESPONSE).unwrap();
        let cached = |seen_at| CachedLicense {
            response: resp.clone(),
            seen_at: Some(seen_at),
            received_at: Some(1_800_000_000),
        };
        // The floor rewrite of a status check never recreates a deleted file.
        store.write(&cached(1_800_000_000), true).unwrap();
        assert!(!dir.join("license.json").exists());
        store.write(&cached(1_800_000_123), false).unwrap();
        let text = std::fs::read_to_string(dir.join("license.json")).unwrap();
        assert!(text.contains("\"seenAt\": 1800000123"), "{text}");
        assert!(text.contains("\"receivedAt\": 1800000000"), "{text}");
        let loaded = store.load_cached().unwrap();
        assert_eq!(loaded.seen_at, Some(1_800_000_123));
        assert_eq!(loaded.received_at, Some(1_800_000_000));
        // A licence saved now records this PC's clock as it arrived.
        store.save(&resp).unwrap();
        let saved = store.load_cached().unwrap();
        assert!(saved
            .received_at
            .is_some_and(|at| at.abs_diff(now_secs()) < 60));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// An upgrade neither takes Pro away nor gives an old cache more time: a
    /// file without `receivedAt` gets no offset, which is this PC's clock and
    /// the same floor, exactly as before. Its next licence brings the offset.
    #[test]
    fn upgrading_from_each_older_cache_shape() {
        let dir = temp_dir("shapes");
        let store = LicenseStore::new(dir.clone());
        let resp: SignedLicenseResponse = serde_json::from_str(REAL_UNINSTALLER_RESPONSE).unwrap();
        let payload = verify(&resp.payload_json, &resp.signature).unwrap();
        let file = |extra: serde_json::Value| {
            let mut value: serde_json::Value =
                serde_json::from_str(REAL_UNINSTALLER_RESPONSE).unwrap();
            value
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            std::fs::write(dir.join("license.json"), value.to_string()).unwrap();
            let cached = store.load_cached().unwrap();
            assert_eq!(cached.response.payload_json, resp.payload_json);
            (cached.seen_at, cached.received_at, cached.offset(&payload))
        };
        // 0.13.6 and earlier: the signed response alone.
        assert_eq!(file(serde_json::json!({})), (None, None, 0));
        // 0.13.7 to 0.14.0: plus seenAt.
        assert_eq!(
            file(serde_json::json!({ "seenAt": 1_787_200_000_u64 })),
            (Some(1_787_200_000), None, 0)
        );
        // The build of #56, never released: receivedAt alone.
        let received = payload.issued_at + 3600;
        assert_eq!(
            file(serde_json::json!({ "receivedAt": received })),
            (None, Some(received), -3600)
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_clock_used_for_freshness_never_runs_backwards_in_a_process() {
        let first = monotonic_now(0);
        assert!(monotonic_now(0) >= first);
        assert!(first + 5 >= now_secs());
    }

    /// The floor holds server time only: reading a clock corrected a day
    /// forward, and checking a licence on it, must not lift the floor with it.
    #[test]
    fn reading_the_corrected_clock_never_raises_the_floor() {
        let payload = LicensePayload {
            issued_at: now_secs(),
            expires_at: None,
            ..last_two_days()
        };
        let ahead = MAX_CLOCK_SKEW_SECS as i64;
        assert!(monotonic_now(ahead) + 5 >= now_secs() + MAX_CLOCK_SKEW_SECS);
        assert!(is_fresh(&payload, ahead));
        // No test shows this process a server time past the real one.
        assert!(trusted_now() <= now_secs() + 60);
    }

    const DAY: u64 = 24 * 60 * 60;
    /// The server's clock when the licence below was signed.
    const SIGNED: u64 = 1_800_000_000;

    /// A plan in its last two days, signed at `SIGNED`.
    fn last_two_days() -> LicensePayload {
        LicensePayload {
            user_id: "clock".into(),
            is_pro: true,
            plan: Some("annual".into()),
            product: PRODUCT_ID.into(),
            issued_at: SIGNED,
            expires_at: Some(SIGNED + 2 * DAY),
        }
    }

    /// One check on fixed clocks: this PC reads `wall` now and read `received`
    /// when the licence arrived; `floor` is the server time already proven (the
    /// signed issue time plus the time the app has run, or `seenAt`).
    fn pro_on(payload: &LicensePayload, wall: u64, received: u64, floor: u64) -> bool {
        let offset = clock_offset(payload.issued_at, received);
        is_fresh_at(payload, server_now(wall, offset, floor))
    }

    /// QA 15 Oct 2026: with the PC clock a day ahead, the last day of a plan
    /// read as Free while the server said Pro.
    #[test]
    fn a_pc_a_day_ahead_keeps_pro_for_the_whole_licence() {
        let payload = last_two_days();
        let received = SIGNED + DAY;
        // The floor (server time) stays under the corrected clock all along.
        for run in [0, DAY, 2 * DAY - 1] {
            assert!(
                pro_on(&payload, received + run, received, SIGNED + run),
                "{run}"
            );
        }
        assert!(!pro_on(
            &payload,
            received + 2 * DAY,
            received,
            SIGNED + 2 * DAY
        ));
        // Before: this PC's own clock ended the plan a day early.
        assert!(!is_fresh_at(&payload, received + DAY));
    }

    #[test]
    fn a_pc_a_day_behind_keeps_pro_for_the_whole_licence_and_no_longer() {
        let payload = last_two_days();
        let received = SIGNED - DAY;
        for run in [0, DAY, 2 * DAY - 1] {
            assert!(
                pro_on(&payload, received + run, received, SIGNED + run),
                "{run}"
            );
        }
        // Offline after a restart, the floor from an earlier run: the plan
        // still ends on time, where this PC's own clock gave it a day more.
        assert!(!pro_on(&payload, received + 2 * DAY, received, SIGNED));
        assert!(is_fresh_at(&payload, received + 2 * DAY));
    }

    #[test]
    fn a_clock_set_back_after_receipt_still_expires_on_schedule() {
        let payload = last_two_days();
        let end = SIGNED + 2 * DAY;
        for received in [SIGNED - DAY, SIGNED, SIGNED + DAY] {
            let offset = clock_offset(SIGNED, received);
            // At the end of the plan, this PC's clock moved back a day and a half.
            let set_back = received + DAY / 2;
            assert!(
                is_fresh_at(&payload, server_now(set_back, offset, 0)),
                "the moved-back clock alone would keep Pro"
            );
            // While the app runs, the floor has reached the end...
            assert!(!pro_on(&payload, set_back, received, end));
            // ...and after a restart, seenAt has.
            assert!(!fresh_after_restart(
                &payload,
                Some(end),
                server_now(set_back, offset, end)
            ));
        }
    }

    #[test]
    fn a_clock_pushed_forward_after_receipt_gains_nothing() {
        let payload = last_two_days();
        // Five days forward a day in: the plan ends early on this PC, never late...
        assert!(!pro_on(&payload, SIGNED + 6 * DAY, SIGNED, SIGNED + DAY));
        // ...and the floor did not follow the clock, so setting it right gives
        // Pro back until the signed end, and not past it.
        assert!(pro_on(&payload, SIGNED + DAY, SIGNED, SIGNED + DAY));
        assert!(!pro_on(
            &payload,
            SIGNED + 2 * DAY,
            SIGNED,
            SIGNED + 2 * DAY
        ));
    }

    #[test]
    fn a_restart_with_seen_at_and_an_offset_gives_the_right_verdict() {
        let payload = last_two_days();
        let received = SIGNED + DAY;
        let offset = clock_offset(SIGNED, received);
        let after_restart = |wall: u64, seen: u64| {
            let floor = seen.min(offline_window_end(&payload));
            fresh_after_restart(&payload, Some(seen), server_now(wall, offset, floor))
        };
        // A day in, last seen an hour earlier: still Pro.
        assert!(after_restart(received + DAY, SIGNED + DAY - 3600));
        // The app saw the end, then the clock went back a day: over.
        assert!(!after_restart(received + DAY, SIGNED + 2 * DAY));
        // Past the end on the corrected clock, whatever seenAt says.
        assert!(!after_restart(received + 2 * DAY, SIGNED));
    }

    /// `receivedAt` is not signed: an offset out of range is bounded to the
    /// skew allowance, so an edited file buys a day at most, never a month.
    #[test]
    fn an_out_of_range_offset_is_bounded_to_a_day() {
        let skew = MAX_CLOCK_SKEW_SECS as i64;
        assert_eq!(clock_offset(SIGNED, SIGNED + 30 * DAY), -skew);
        assert_eq!(clock_offset(SIGNED, 0), skew);
        assert_eq!(clock_offset(SIGNED, u64::MAX), -skew);
        assert_eq!(clock_offset(u64::MAX, 0), skew);
        // A receivedAt edited to a month ahead, offline, seenAt removed: the
        // plan runs at most a day past its signed end.
        let payload = last_two_days();
        let edited = SIGNED + 30 * DAY;
        assert!(pro_on(&payload, SIGNED + 3 * DAY - 1, edited, SIGNED));
        assert!(!pro_on(&payload, SIGNED + 3 * DAY, edited, SIGNED));
        assert!(!pro_on(&payload, SIGNED + 30 * DAY, edited, SIGNED));
    }
}
