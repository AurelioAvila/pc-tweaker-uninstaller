//! Small quality-of-life commands: open a program's install folder, open
//! Windows' own System Restore UI, open ecosystem websites.
//!
//! Same trust rules as the executor: the webview never supplies a path or a
//! URL. Folder paths are re-read fresh from the registry and gated (must be
//! absolute and actually a directory); URLs come from a compile-time
//! allowlist keyed by a validated name. Programs are launched by absolute
//! path with an argv array — no shell.

use crate::programs;
use crate::uninstall_exec::{validate_exe_path_shape, validate_key_name, validate_source};
use tauri_plugin_opener::OpenerExt;

fn system_root() -> String {
    std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string())
}

/// Opens the program's install folder in Explorer. The location is re-read
/// from the registry at call time and must be an absolute path that exists
/// as a directory — otherwise the click honestly fails instead of opening
/// something else.
#[tauri::command]
pub fn open_install_folder(source: String, id: String) -> Result<(), String> {
    validate_source(&source)?;
    validate_key_name(&id)?;
    let entry = programs::read_raw_entry(&source, &id)?;
    let location = entry
        .install_location
        .ok_or_else(|| "This program does not record an install folder.".to_string())?;
    validate_exe_path_shape(&location)
        .map_err(|_| "This program's install folder path is not usable.".to_string())?;
    let meta = std::fs::metadata(&location)
        .map_err(|_| "The install folder no longer exists on disk.".to_string())?;
    if !meta.is_dir() {
        return Err("The recorded install location is not a folder.".to_string());
    }
    std::process::Command::new(format!(r"{}\explorer.exe", system_root()))
        .arg(&location)
        .spawn()
        .map_err(|e| format!("Explorer could not be opened: {e}"))?;
    Ok(())
}

/// Opens Windows' System Protection panel — where restore points live and
/// are managed. Fixed executable, zero arguments; it elevates itself if the
/// user proceeds.
#[tauri::command]
pub fn open_system_restore() -> Result<(), String> {
    std::process::Command::new(format!(
        r"{}\System32\SystemPropertiesProtection.exe",
        system_root()
    ))
    .spawn()
    .map_err(|e| format!("The System Protection panel could not be opened: {e}"))?;
    Ok(())
}

/// Compile-time allowlist of ecosystem destinations. The webview picks a
/// name; the URL never crosses the IPC boundary.
fn ecosystem_url(target: &str) -> Option<&'static str> {
    match target {
        "pctweaker" => Some("https://pctweaker.app"),
        "privacy" => Some("https://pctweaker.app/privacy"),
        "promptshield" => Some("https://promptshield-beta.vercel.app"),
        _ => None,
    }
}

#[tauri::command]
pub fn open_ecosystem_link(app: tauri::AppHandle, target: String) -> Result<(), String> {
    let url = ecosystem_url(&target).ok_or_else(|| "Unknown ecosystem destination.".to_string())?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| format!("The link could not be opened: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_allowlisted_ecosystem_targets_resolve() {
        assert_eq!(ecosystem_url("pctweaker"), Some("https://pctweaker.app"));
        assert!(ecosystem_url("promptshield").is_some());
        assert!(ecosystem_url("privacy").is_some());
        assert_eq!(ecosystem_url(""), None);
        assert_eq!(ecosystem_url("https://evil.example"), None);
        assert_eq!(ecosystem_url("PCTWEAKER"), None);
    }
}
