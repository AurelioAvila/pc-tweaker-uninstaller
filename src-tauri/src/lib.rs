//! PC Tweaker Uninstaller — application core.
//!
//! Phase 1 scaffold: the modules below are the security-critical pieces
//! ported from PC Tweaker with their tests (see each module's header for
//! what changed in the port). No uninstall features exist yet, by design:
//! the baseline (type checks, lints, tests, CI) must be green before any
//! feature lands.

pub mod elevation;
pub mod license;
pub mod rollback;

use std::path::PathBuf;
use tauri::Manager;

/// Resolves the per-user app data directory every store in this app writes
/// under. Centralized so no module ever invents its own path.
pub fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

/// The app's own version, for the About panel and support reports.
#[tauri::command]
fn app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            app_version,
            license::save_license,
            license::license_status,
            license::clear_license,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
