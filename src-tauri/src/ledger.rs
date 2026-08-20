//! Clean Removal Ledger — a local, append-only receipt for every uninstall
//! this app ran.
//!
//! One JSON line per removal in the app's own data directory. Never uploaded
//! anywhere; exportable on explicit request as a pretty-printed JSON file in
//! the user's Downloads folder. The ledger records what actually happened —
//! including failures — not a sanitized success story.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;

const LEDGER_FILE: &str = "removal-ledger.jsonl";
/// Receipts are small; 500 is years of normal use. Trimmed oldest-first.
const MAX_ENTRIES: usize = 500;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemovalReceipt {
    /// Unix seconds.
    pub ts: u64,
    pub program_name: String,
    pub source: String,
    /// "msi" or "executable".
    pub method: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub reboot_required: bool,
    /// One human line about the restore point outcome.
    pub restore_point: String,
    /// What the registry claimed the program occupied.
    pub estimated_size_kb: Option<u32>,
    /// Measured install-folder shrinkage, when the folder was measurable
    /// before and after. Absent is honest; a guess would not be.
    #[serde(default)]
    pub verified_freed_kb: Option<u64>,
    pub message: String,
}

fn ledger_path() -> Result<PathBuf, String> {
    Ok(crate::uninstall_exec::fixed_data_dir()?.join(LEDGER_FILE))
}

/// Parses the JSONL body, dropping unreadable lines instead of failing the
/// whole ledger: one corrupt line must not eat the user's history.
pub fn parse_ledger(body: &str) -> Vec<RemovalReceipt> {
    body.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Oldest entries fall off first once past the cap.
pub fn trim_to_cap(mut entries: Vec<RemovalReceipt>, cap: usize) -> Vec<RemovalReceipt> {
    if entries.len() > cap {
        entries.drain(0..entries.len() - cap);
    }
    entries
}

pub fn append(receipt: &RemovalReceipt) -> Result<(), String> {
    let path = ledger_path()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let body = std::fs::read_to_string(&path).unwrap_or_default();
    let mut entries = parse_ledger(&body);
    entries.push(receipt.clone());
    let entries = trim_to_cap(entries, MAX_ENTRIES);

    let mut out = Vec::new();
    for e in &entries {
        serde_json::to_writer(&mut out, e).map_err(|e| e.to_string())?;
        out.push(b'\n');
    }
    // Temp-then-rename: a crash mid-write must never corrupt the ledger.
    let tmp = path.with_extension("jsonl.tmp");
    std::fs::File::create(&tmp)
        .and_then(|mut f| f.write_all(&out))
        .map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Newest first, for display.
#[tauri::command]
pub fn list_removal_ledger() -> Result<Vec<RemovalReceipt>, String> {
    let body = std::fs::read_to_string(ledger_path()?).unwrap_or_default();
    let mut entries = parse_ledger(&body);
    entries.reverse();
    Ok(entries)
}

/// Writes the full ledger as pretty JSON into Downloads and returns the path.
/// Explicit user action only — nothing is exported automatically.
#[tauri::command]
pub fn export_removal_ledger() -> Result<String, String> {
    let body = std::fs::read_to_string(ledger_path()?).unwrap_or_default();
    let entries = parse_ledger(&body);
    let home = std::env::var_os("USERPROFILE")
        .ok_or_else(|| "USERPROFILE is not set; cannot locate Downloads.".to_string())?;
    let out_path = PathBuf::from(home)
        .join("Downloads")
        .join("pc-tweaker-uninstaller-ledger.json");
    let json = serde_json::to_vec_pretty(&entries).map_err(|e| e.to_string())?;
    std::fs::write(&out_path, json).map_err(|e| e.to_string())?;
    Ok(out_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(name: &str, ts: u64) -> RemovalReceipt {
        RemovalReceipt {
            ts,
            program_name: name.into(),
            source: "machine64".into(),
            method: "msi".into(),
            success: true,
            exit_code: Some(0),
            reboot_required: false,
            restore_point: "created".into(),
            estimated_size_kb: Some(1024),
            verified_freed_kb: None,
            message: "Uninstalled successfully.".into(),
        }
    }

    #[test]
    fn receipts_roundtrip_through_jsonl() {
        let a = receipt("7-Zip", 1);
        let b = receipt("VLC", 2);
        let body = format!(
            "{}\n{}\n",
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        assert_eq!(parse_ledger(&body), vec![a, b]);
    }

    #[test]
    fn corrupt_lines_are_skipped_not_fatal() {
        let good = receipt("Good", 1);
        let body = format!("not json at all\n{}\n{{half", serde_json::to_string(&good).unwrap());
        assert_eq!(parse_ledger(&body), vec![good]);
    }

    #[test]
    fn the_ledger_trims_oldest_first() {
        let entries: Vec<_> = (0..10).map(|i| receipt("App", i)).collect();
        let trimmed = trim_to_cap(entries, 3);
        assert_eq!(trimmed.len(), 3);
        assert_eq!(trimmed[0].ts, 7);
        assert_eq!(trimmed[2].ts, 9);
    }

    #[test]
    fn older_receipts_without_verified_freed_still_parse() {
        // Forward-compat: the field was added later and is defaulted.
        let line = r#"{"ts":1,"programName":"X","source":"user","method":"executable","success":false,"exitCode":1,"rebootRequired":false,"restorePoint":"skipped","estimatedSizeKb":null,"message":"failed"}"#;
        let parsed = parse_ledger(line);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].verified_freed_kb, None);
    }
}
