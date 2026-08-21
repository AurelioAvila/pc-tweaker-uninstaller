//! Residue Intelligence — slice 2 of Removal Intelligence.
//!
//! After a program is uninstalled, its identity (name, publisher, install
//! location — captured from the registry BEFORE the uninstall erased it) is
//! used to find what the uninstaller left behind: data folders, Start Menu
//! shortcuts, per-user registry keys, and the install folder itself.
//!
//! Safety posture, in order of importance:
//! 1. NOTHING is deleted permanently — cleanup moves items to the Recycle
//!    Bin, so every action the user takes here is reversible from Windows
//!    itself, matching the app-wide "always have a rollback" rule.
//! 2. Matching is deliberately conservative: a folder is a candidate only if
//!    its name equals a normalized form of the program's name or publisher,
//!    the token is at least four characters, and it is not on the stoplist
//!    of shared/vendor names (a "Microsoft" folder is never residue).
//! 3. Cleanup revalidates every path against the same rules that proposed
//!    it — the frontend cannot ask this module to remove an arbitrary path.
//! 4. HKLM registry leftovers are reported but never touched; only HKCU
//!    keys (per-user, recreatable) are deletable, and those cannot go to a
//!    recycle bin, so they are the one destructive step — flagged as such.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResidueItem {
    /// "install-dir" | "app-data" | "shortcut" | "registry-user" | "registry-machine"
    pub kind: String,
    /// Filesystem path, or a `HKCU\...` / `HKLM\...` registry path.
    pub path: String,
    pub size_kb: Option<u64>,
    /// Whether clean_residue will act on it (HKLM keys are report-only).
    pub deletable: bool,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResidueReport {
    pub items: Vec<ResidueItem>,
    pub total_kb: u64,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub removed: Vec<String>,
    pub failed: Vec<String>,
    pub freed_kb: u64,
}

/// Folder names that are never residue no matter how a program is called.
/// Lowercase, normalized (alphanumeric only).
const STOPLIST: &[&str] = &[
    "microsoft", "windows", "google", "mozilla", "apple", "adobe", "intel",
    "nvidia", "amd", "common", "commonfiles", "programs", "programfiles",
    "temp", "system", "system32", "data", "cache", "local", "roaming",
    "packages", "default", "public", "update", "updates", "setup", "install",
    "app", "apps", "application", "applications", "software", "games",
];

/// Lowercases and strips everything but ASCII alphanumerics, so
/// "PC Tweaker 1.0.0", "pc-tweaker-app" and "PCTweaker" all compare equal
/// after version-number stripping.
pub fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Drops trailing version-ish tokens ("MyApp 2.1.0" -> "MyApp") so the
/// registry display name matches folder names that never carry the version.
fn strip_version(name: &str) -> String {
    let mut words: Vec<&str> = name.split_whitespace().collect();
    while let Some(last) = words.last() {
        let looks_like_version = last
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == 'v' || c == 'V' || c == '(' || c == ')');
        if looks_like_version && words.len() > 1 {
            words.pop();
        } else {
            break;
        }
    }
    words.join(" ")
}

/// The normalized tokens a leftover's file/folder name may equal.
pub fn name_candidates(display_name: &str, publisher: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    for raw in [Some(strip_version(display_name)), publisher.map(str::to_string)]
        .into_iter()
        .flatten()
    {
        let token = normalize(&raw);
        if token.len() >= 4 && !STOPLIST.contains(&token.as_str()) && !out.contains(&token) {
            out.push(token);
        }
    }
    out
}

/// True when `file_name` (a bare folder/file name, extension already
/// stripped for files) matches one of the candidates exactly.
pub fn matches_candidates(file_name: &str, candidates: &[String]) -> bool {
    let token = normalize(file_name);
    !token.is_empty() && candidates.contains(&token)
}

fn dir_size_kb(path: &Path) -> Option<u64> {
    let mut cap = 20_000u32;
    crate::uninstall_exec::dir_size_capped(path, &mut cap).map(|b| b / 1024)
}

fn push_if_dir(items: &mut Vec<ResidueItem>, kind: &str, path: PathBuf) {
    if path.is_dir() {
        let size_kb = dir_size_kb(&path);
        items.push(ResidueItem {
            kind: kind.into(),
            path: path.to_string_lossy().into_owned(),
            size_kb,
            deletable: true,
        });
    }
}

fn scan_root_for_candidates(items: &mut Vec<ResidueItem>, kind: &str, root: &Path, candidates: &[String]) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if matches_candidates(&name, candidates) {
            push_if_dir(items, kind, entry.path());
        }
    }
}

fn data_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for var in ["APPDATA", "LOCALAPPDATA", "PROGRAMDATA"] {
        if let Ok(value) = std::env::var(var) {
            roots.push(PathBuf::from(value));
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        roots.push(Path::new(&local).join("Programs"));
    }
    roots
}

fn start_menu_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        roots.push(Path::new(&appdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        roots.push(Path::new(&profile).join("Desktop"));
    }
    roots
}

fn scan_shortcuts(items: &mut Vec<ResidueItem>, candidates: &[String]) {
    for root in start_menu_roots() {
        let Ok(entries) = std::fs::read_dir(&root) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let raw = entry.file_name().to_string_lossy().into_owned();
            let stem = raw.strip_suffix(".lnk").unwrap_or(&raw);
            if !matches_candidates(stem, candidates) {
                continue;
            }
            if path.is_dir() {
                push_if_dir(items, "shortcut", path);
            } else if raw.to_ascii_lowercase().ends_with(".lnk") {
                items.push(ResidueItem {
                    kind: "shortcut".into(),
                    path: path.to_string_lossy().into_owned(),
                    size_kb: Some(1),
                    deletable: true,
                });
            }
        }
    }
}

#[cfg(windows)]
fn scan_registry(items: &mut Vec<ResidueItem>, candidates: &[String]) {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;
    for (root, label, deletable) in [
        (HKEY_CURRENT_USER, "HKCU", true),
        (HKEY_LOCAL_MACHINE, "HKLM", false),
    ] {
        let Ok(software) = RegKey::predef(root).open_subkey_with_flags("Software", KEY_READ) else {
            continue;
        };
        for name in software.enum_keys().flatten() {
            if matches_candidates(&name, candidates) {
                items.push(ResidueItem {
                    kind: if deletable { "registry-user" } else { "registry-machine" }.into(),
                    path: format!(r"{label}\Software\{name}"),
                    size_kb: None,
                    deletable,
                });
            }
        }
    }
}

#[cfg(not(windows))]
fn scan_registry(_items: &mut Vec<ResidueItem>, _candidates: &[String]) {}

/// Scans for leftovers of an uninstalled program. `install_location` is the
/// path the registry reported before the uninstall; if the folder still
/// exists it is the highest-confidence residue there is.
#[tauri::command]
pub fn scan_residue(
    name: String,
    publisher: Option<String>,
    install_location: Option<String>,
) -> Result<ResidueReport, String> {
    let candidates = name_candidates(&name, publisher.as_deref());
    let mut items = Vec::new();

    if let Some(location) = install_location.as_deref().filter(|l| !l.trim().is_empty()) {
        let path = PathBuf::from(location.trim().trim_matches('"'));
        // The install dir bypasses name matching (the registry itself vouched
        // for it) but never a bare drive/root path.
        if path.components().count() > 2 {
            push_if_dir(&mut items, "install-dir", path);
        }
    }
    if !candidates.is_empty() {
        for root in data_roots() {
            scan_root_for_candidates(&mut items, "app-data", &root, &candidates);
        }
        scan_shortcuts(&mut items, &candidates);
        scan_registry(&mut items, &candidates);
    }

    items.dedup_by(|a, b| a.path == b.path);
    let total_kb = items.iter().filter_map(|i| i.size_kb).sum();
    Ok(ResidueReport { items, total_kb })
}

/// Validates that a path the frontend asked to clean is one this module
/// would itself have proposed: inside a known root (or the recorded install
/// location), with a final component that matches the candidates.
fn path_is_cleanable(path: &Path, candidates: &[String], install_location: Option<&str>) -> bool {
    if let Some(location) = install_location {
        let loc = PathBuf::from(location.trim().trim_matches('"'));
        if loc.components().count() > 2 && path == loc {
            return true;
        }
    }
    let Some(file_name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
        return false;
    };
    let stem = file_name.strip_suffix(".lnk").unwrap_or(&file_name);
    if !matches_candidates(stem, candidates) {
        return false;
    }
    let mut roots = data_roots();
    roots.extend(start_menu_roots());
    roots.iter().any(|root| path.parent() == Some(root.as_path()))
}

/// Moves the selected leftovers to the Recycle Bin (filesystem items) or
/// deletes them (HKCU registry keys — flagged in the UI as the one
/// non-recoverable step). Every path is revalidated; unknown paths fail.
#[tauri::command]
pub fn clean_residue(
    name: String,
    publisher: Option<String>,
    install_location: Option<String>,
    paths: Vec<String>,
) -> Result<CleanResult, String> {
    if paths.len() > 64 {
        return Err("Too many items in one cleanup.".into());
    }
    let candidates = name_candidates(&name, publisher.as_deref());
    let mut result = CleanResult::default();

    for raw in paths {
        if let Some(key) = raw.strip_prefix(r"HKCU\Software\") {
            #[cfg(windows)]
            {
                use winreg::enums::HKEY_CURRENT_USER;
                use winreg::RegKey;
                let valid = !key.contains('\\') && matches_candidates(key, &candidates);
                let deleted = valid
                    && RegKey::predef(HKEY_CURRENT_USER)
                        .open_subkey("Software")
                        .and_then(|s| s.delete_subkey_all(key))
                        .is_ok();
                if deleted {
                    result.removed.push(raw);
                } else {
                    result.failed.push(raw);
                }
            }
            #[cfg(not(windows))]
            result.failed.push(raw);
            continue;
        }
        let path = PathBuf::from(&raw);
        if !path_is_cleanable(&path, &candidates, install_location.as_deref()) {
            result.failed.push(raw);
            continue;
        }
        let size = dir_size_kb(&path).unwrap_or(0);
        match trash::delete(&path) {
            Ok(()) => {
                result.freed_kb += size;
                result.removed.push(raw);
            }
            Err(_) => result.failed.push(raw),
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_unifies_spellings() {
        assert_eq!(normalize("PC Tweaker"), "pctweaker");
        assert_eq!(normalize("pc-tweaker-app"), "pctweakerapp");
        assert_eq!(normalize("Éxample!"), "xample");
    }

    #[test]
    fn version_suffixes_are_stripped_from_names() {
        assert_eq!(strip_version("MyApp 2.1.0"), "MyApp");
        assert_eq!(strip_version("MyApp v3 (64)"), "MyApp");
        assert_eq!(strip_version("7.1.2"), "7.1.2"); // never empty the name
    }

    #[test]
    fn candidates_exclude_short_and_stoplisted_tokens() {
        assert_eq!(name_candidates("VLC", None), Vec::<String>::new()); // too short
        assert_eq!(name_candidates("Microsoft", None), Vec::<String>::new());
        let c = name_candidates("SuperTool 1.2", Some("Acme Corp"));
        assert!(c.contains(&"supertool".to_string()));
        assert!(c.contains(&"acmecorp".to_string()));
    }

    #[test]
    fn matching_is_exact_not_substring() {
        let c = name_candidates("SuperTool", None);
        assert!(matches_candidates("SuperTool", &c));
        assert!(matches_candidates("super-tool", &c));
        assert!(!matches_candidates("SuperTools", &c));
        assert!(!matches_candidates("MySuperTool", &c));
        assert!(!matches_candidates("Microsoft", &c));
    }

    #[test]
    fn cleanup_rejects_paths_outside_known_roots() {
        let c = name_candidates("SuperTool", None);
        assert!(!path_is_cleanable(Path::new(r"C:\Windows\System32"), &c, None));
        assert!(!path_is_cleanable(Path::new(r"C:\random\supertool"), &c, None));
        // Install location is honored exactly, nothing near it.
        assert!(path_is_cleanable(
            Path::new(r"C:\Program Files\SuperTool"),
            &c,
            Some(r"C:\Program Files\SuperTool")
        ));
        assert!(!path_is_cleanable(
            Path::new(r"C:\Program Files\Other"),
            &c,
            Some(r"C:\Program Files\SuperTool")
        ));
    }

    #[test]
    fn bare_drive_install_locations_are_never_cleanable() {
        let c = name_candidates("SuperTool", None);
        assert!(!path_is_cleanable(Path::new(r"C:\"), &c, Some(r"C:\")));
    }
}
