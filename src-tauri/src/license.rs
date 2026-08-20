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
use std::time::{SystemTime, UNIX_EPOCH};

/// The ecosystem product this binary is. Licenses naming any other product
/// are rejected even when their signature is valid.
pub const PRODUCT_ID: &str = "uninstaller";

/// Public half of the ecosystem's license-signing key pair — the same key
/// PC Tweaker embeds, because both products verify licenses signed by the
/// same backend. Safe to embed: it can only verify, never sign.
const PUBLIC_KEY_B64: &str = "QisYr46g3mqEeiz1BDyEcPbRO1xO4z0lR3d5/ODppIU=";

/// How long a signed license is trusted without a fresh fetch. Mirrors
/// PC Tweaker: long enough for a weekend offline, short enough that a
/// cancelled subscription doesn't keep working indefinitely.
const GRACE_PERIOD_SECS: u64 = 3 * 24 * 60 * 60;

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

/// A payload that passed signature verification AND is still inside its grace
/// period: a validly signed license from three weeks ago proves the server
/// said something true three weeks ago, not that it is still true now.
fn is_fresh(payload: &LicensePayload) -> bool {
    now_secs().saturating_sub(payload.issued_at) <= GRACE_PERIOD_SECS
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
        payload.product == PRODUCT_ID && payload.is_pro && is_fresh(&payload)
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
        };
        assert!(!is_fresh(&stale));
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
}
