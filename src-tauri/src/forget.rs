//! Broken entries: programs whose own uninstaller no longer exists on disk.
//!
//! Such an entry can never be uninstalled, so it would sit in the list
//! forever. This module removes the entry itself from "Installed programs"
//! and nothing else; leftover files are handled afterwards by the normal
//! leftover scan, with its own checks and the Recycle Bin.
//!
//! The trust model matches `uninstall_exec`: the webview supplies only a
//! (source, key) pair, everything else is re-read from the registry at the
//! moment of removal, and machine-wide entries go through the same elevated
//! child, which re-checks everything itself before touching the registry.
//! This is the only module that writes to the uninstall registry views.

use crate::ledger::{RemovalReceipt, RemovedEntry};
use crate::programs::{self, RawEntry};
use crate::restore_point::RestorePointOutcome;
use crate::uninstall_exec::{self, UninstallReport};
use std::time::Instant;

/// Why an entry may not be removed, or `Ok` with the entry as re-read now.
pub fn check_removable(source: &str, id: &str) -> Result<RawEntry, String> {
    uninstall_exec::validate_source(source)?;
    uninstall_exec::validate_key_name(id)?;
    let entry = programs::read_raw_entry(source, id)?;
    check_entry(&entry, |path| std::path::Path::new(path).is_file())?;
    Ok(entry)
}

/// The pure half of [`check_removable`], so the rule is testable anywhere.
pub fn check_entry(entry: &RawEntry, exists: impl Fn(&str) -> bool) -> Result<(), String> {
    if !programs::is_listable(entry) {
        return Err(
            "Windows marks this entry as a system component or an update, so it is not removed."
                .to_string(),
        );
    }
    if !programs::uninstaller_missing(entry, exists) {
        return Err(
            "This program's uninstaller is still present. Uninstall it normally instead."
                .to_string(),
        );
    }
    Ok(())
}

/// Removes the entry and describes what happened. Runs in the current
/// process for per-user entries and inside the elevated child otherwise.
pub fn remove_and_report(
    source: &str,
    id: &str,
    restore_point: RestorePointOutcome,
) -> Result<UninstallReport, String> {
    let started = Instant::now();
    let entry = check_removable(source, id)?;
    let program_name = entry.display_name.unwrap_or_else(|| id.to_string());
    let (success, message) = match delete_entry(source, id) {
        Ok(()) => (
            true,
            "The broken entry was removed from Installed programs. Its leftover files can be \
             scanned next."
                .to_string(),
        ),
        Err(e) => (false, e),
    };
    crate::applog::line(&format!("broken entry {source}:{id} removed: {success}"));
    Ok(UninstallReport {
        program_name,
        command: Vec::new(),
        restore_point,
        exit_code: None,
        success,
        reboot_required: false,
        message,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

#[cfg(windows)]
fn delete_entry(source: &str, id: &str) -> Result<(), String> {
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::RegKey;
    const UNINSTALL_PATH: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

    let (hive, flags) = match source {
        "machine64" => (HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY),
        "machine32" => (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY),
        "user" => (HKEY_CURRENT_USER, 0),
        _ => return Err("Unknown program source.".to_string()),
    };
    RegKey::predef(hive)
        .open_subkey_with_flags(UNINSTALL_PATH, KEY_ALL_ACCESS | flags)
        .map_err(|_| "The uninstall registry view could not be opened for writing.".to_string())?
        .delete_subkey_all(id)
        .map_err(|e| format!("The registry entry could not be removed: {e}"))
}

#[cfg(not(windows))]
fn delete_entry(_source: &str, _id: &str) -> Result<(), String> {
    Err("Removing registry entries is only supported on Windows.".to_string())
}

/// Entry point for the headless elevated child (`--elevated-forget`).
/// Re-derives everything from the two arguments and writes the shared report.
pub fn run_elevated_child(source: &str, id: &str) -> i32 {
    let report = (|| -> Result<UninstallReport, String> {
        check_removable(source, id)?;
        let restore = crate::restore_point::create_restore_point();
        remove_and_report(source, id, restore)
    })();
    let report = report.unwrap_or_else(|reason| UninstallReport {
        program_name: id.to_string(),
        command: Vec::new(),
        restore_point: RestorePointOutcome::Skipped {
            reason: "The removal was refused before it started.".to_string(),
        },
        exit_code: None,
        success: false,
        reboot_required: false,
        message: reason,
        duration_ms: 0,
    });
    match uninstall_exec::write_report(&report) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn record_receipt(source: &str, report: &UninstallReport, entry: &RawEntry) {
    let restore_point = match &report.restore_point {
        RestorePointOutcome::Created => "created".to_string(),
        RestorePointOutcome::Skipped { reason } => format!("skipped: {reason}"),
        RestorePointOutcome::Failed { reason } => format!("failed: {reason}"),
    };
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // The removed values are kept in the receipt, so the entry can be
    // recreated by hand if it was ever needed again.
    let _ = crate::ledger::append(&RemovalReceipt {
        ts,
        program_name: report.program_name.clone(),
        source: source.to_string(),
        method: "entry".to_string(),
        success: report.success,
        exit_code: None,
        reboot_required: false,
        restore_point,
        estimated_size_kb: entry.estimated_size_kb,
        verified_freed_kb: None,
        message: report.message.clone(),
        removed_entry: Some(RemovedEntry {
            key: entry.key_name.clone(),
            publisher: entry.publisher.clone(),
            version: entry.display_version.clone(),
            install_location: entry.install_location.clone(),
            uninstall_string: entry.uninstall_string.clone(),
        }),
    });
}

/// Removes a broken entry (Pro). Per-user entries are removed directly;
/// machine-wide entries ask for one UAC consent and get a restore point.
#[tauri::command(async)]
pub async fn forget_broken_entry(
    app: tauri::AppHandle,
    source: String,
    id: String,
) -> Result<UninstallReport, String> {
    if !crate::license::license_status(app)? {
        return Err(
            "An active Uninstaller Pro license is required to remove broken entries.".into(),
        );
    }
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = uninstall_exec::lock_exec()?;
        // Checked here too, so an ineligible entry never costs a UAC prompt.
        let entry = check_removable(&source, &id)?;
        let report = if source == "user" {
            remove_and_report(
                &source,
                &id,
                RestorePointOutcome::Skipped {
                    reason: "Restore points require administrator rights; this per-user entry \
                             is removed without one."
                        .to_string(),
                },
            )?
        } else {
            uninstall_exec::run_elevated_flag("--elevated-forget", &source, &id)?
        };
        record_receipt(&source, &report, &entry);
        Ok(report)
    })
    .await
    .map_err(|e| format!("The removal task failed to run: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(uninstall: &str) -> RawEntry {
        RawEntry {
            key_name: "Key".into(),
            display_name: Some("Old Game".into()),
            uninstall_string: Some(uninstall.into()),
            ..Default::default()
        }
    }

    #[test]
    fn a_listed_entry_with_a_missing_uninstaller_is_removable() {
        assert!(check_entry(&entry(r#""C:\Games\Old\unins000.exe""#), |_| false).is_ok());
    }

    #[test]
    fn a_working_uninstaller_is_never_bypassed() {
        assert!(check_entry(&entry(r#""C:\Games\Old\unins000.exe""#), |_| true).is_err());
    }

    #[test]
    fn system_components_are_never_removed() {
        let mut component = entry(r#""C:\Gone\unins.exe""#);
        component.system_component = Some(1);
        assert!(check_entry(&component, |_| false).is_err());
    }
}
