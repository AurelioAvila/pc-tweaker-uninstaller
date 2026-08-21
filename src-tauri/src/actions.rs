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
        "support" => Some("https://pctweaker.app/support"),
        // Account and plans both live on the site: registration made on
        // pctweaker.app is the suite account, valid in this app too.
        "account" => Some("https://pctweaker.app"),
        "pricing" => Some("https://pctweaker.app"),
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

/// Normalizes a display name for suite matching: "pc-tweaker-app" and
/// "PC Tweaker" must both count as the flagship.
pub fn is_pc_tweaker_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase().replace(['-', '_'], " ");
    n.starts_with("pc tweaker") && !n.contains("uninstaller")
}

/// Extracts the executable path from a registry `DisplayIcon` value, which
/// conventionally looks like `"C:\path\app.exe"` or `C:\path\app.exe,0`.
pub fn exe_from_display_icon(display_icon: &str) -> Option<String> {
    let trimmed = display_icon.trim().trim_matches('"');
    // Strip a trailing ",<icon index>" if present.
    let path = match trimmed.rsplit_once(',') {
        Some((p, idx)) if idx.trim().chars().all(|c| c.is_ascii_digit()) => p,
        _ => trimmed,
    };
    let path = path.trim().trim_matches('"');
    if path.to_ascii_lowercase().ends_with(".exe") {
        Some(path.to_string())
    } else {
        None
    }
}

/// Launches the installed PC Tweaker app, if present. The path comes from
/// PC Tweaker's own uninstall entry (DisplayIcon), gated exactly like every
/// other launch: absolute, exists, is a file, ends in .exe. No arguments,
/// no shell.
#[tauri::command]
pub fn open_pc_tweaker() -> Result<(), String> {
    let exe = programs::find_suite_exe(is_pc_tweaker_name, exe_from_display_icon)
        .ok_or_else(|| "PC Tweaker was not found on this PC.".to_string())?;
    validate_exe_path_shape(&exe)
        .map_err(|_| "PC Tweaker's recorded path is not usable.".to_string())?;
    let meta = std::fs::metadata(&exe)
        .map_err(|_| "PC Tweaker's executable no longer exists on disk.".to_string())?;
    if !meta.is_file() {
        return Err("PC Tweaker's recorded path is not a file.".to_string());
    }
    std::process::Command::new(&exe)
        .spawn()
        .map_err(|e| format!("PC Tweaker could not be started: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_allowlisted_ecosystem_targets_resolve() {
        assert_eq!(ecosystem_url("pctweaker"), Some("https://pctweaker.app"));
        assert!(ecosystem_url("promptshield").is_some());
        assert!(ecosystem_url("privacy").is_some());
        assert!(ecosystem_url("support").is_some());
        assert!(ecosystem_url("account").is_some());
        assert!(ecosystem_url("pricing").is_some());
        assert_eq!(ecosystem_url(""), None);
        assert_eq!(ecosystem_url("https://evil.example"), None);
        assert_eq!(ecosystem_url("PCTWEAKER"), None);
    }

    #[test]
    fn pc_tweaker_matching_survives_hyphenated_product_names() {
        assert!(is_pc_tweaker_name("pc-tweaker-app"));
        assert!(is_pc_tweaker_name("PC Tweaker"));
        assert!(!is_pc_tweaker_name("PC Tweaker Uninstaller"));
        assert!(!is_pc_tweaker_name("pc-tweaker-uninstaller"));
        assert!(!is_pc_tweaker_name("Some Other App"));
    }

    #[test]
    fn display_icon_paths_are_extracted_and_gated_by_extension() {
        assert_eq!(
            exe_from_display_icon(r#""C:\Apps\pc-tweaker\tauri-app.exe""#).as_deref(),
            Some(r"C:\Apps\pc-tweaker\tauri-app.exe")
        );
        assert_eq!(
            exe_from_display_icon(r"C:\Apps\app.exe,0").as_deref(),
            Some(r"C:\Apps\app.exe")
        );
        // A comma inside the path (rare but legal) is not an icon index.
        assert_eq!(
            exe_from_display_icon(r"C:\Ap,ps\app.exe").as_deref(),
            Some(r"C:\Ap,ps\app.exe")
        );
        assert_eq!(exe_from_display_icon(r"C:\Apps\app.ico"), None);
        assert_eq!(exe_from_display_icon(""), None);
    }
}

/// Opens a Stripe Checkout URL in the system browser. The webview supplies
/// the URL (it came from OUR backend's /api/checkout response), but trust
/// still isn't extended to arbitrary destinations: only Stripe's checkout
/// origin passes. Anything else fails loudly.
#[tauri::command]
pub fn open_checkout_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with("https://checkout.stripe.com/") {
        return Err("Refusing to open a non-Stripe checkout URL.".to_string());
    }
    app.opener()
        .open_url(&url, None::<String>)
        .map_err(|e| format!("The browser could not be opened: {e}"))
}
