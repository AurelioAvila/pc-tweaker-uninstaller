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
    "microsoft",
    "windows",
    "google",
    "mozilla",
    "apple",
    "adobe",
    "intel",
    "nvidia",
    "amd",
    "common",
    "commonfiles",
    "programs",
    "programfiles",
    "temp",
    "system",
    "system32",
    "data",
    "cache",
    "local",
    "roaming",
    "packages",
    "default",
    "public",
    "update",
    "updates",
    "setup",
    "install",
    "app",
    "apps",
    "application",
    "applications",
    "software",
    "games",
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
        let looks_like_version = last.chars().all(|c| {
            c.is_ascii_digit() || c == '.' || c == 'v' || c == 'V' || c == '(' || c == ')'
        });
        if looks_like_version && words.len() > 1 {
            words.pop();
        } else {
            break;
        }
    }
    words.join(" ")
}

/// The normalized tokens a leftover's file/folder name may equal. Only the
/// program's own name counts: a publisher folder (`%APPDATA%\JetBrains`,
/// `HKCU\Software\Logitech`) holds every product from that vendor, so
/// matching it would offer to delete programs that are still installed.
pub fn name_candidates(display_name: &str) -> Vec<String> {
    let token = normalize(&strip_version(display_name));
    if token.len() >= 4 && !STOPLIST.contains(&token.as_str()) {
        vec![token]
    } else {
        Vec::new()
    }
}

/// True when any of `others` (install locations still registered with
/// Windows) is `location` itself or lies inside it. A shared vendor folder,
/// or the folder of a program whose uninstall did not finish, must stay.
/// An unreadable location is treated as shared.
fn location_hosts_program(location: &str, others: &[Option<String>]) -> bool {
    let Some(root) = crate::relations::normalized_root(Some(location)) else {
        return true;
    };
    others.iter().any(|other| {
        crate::relations::normalized_root(other.as_deref())
            .is_some_and(|other| other == root || crate::relations::is_inside(&other, &root))
    })
}

fn install_location_in_use(location: &str) -> bool {
    match crate::programs::list_programs() {
        Ok(programs) => {
            let others: Vec<Option<String>> =
                programs.into_iter().map(|p| p.install_location).collect();
            location_hosts_program(location, &others)
        }
        Err(_) => true,
    }
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

fn scan_root_for_candidates(
    items: &mut Vec<ResidueItem>,
    kind: &str,
    root: &Path,
    candidates: &[String],
) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
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
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
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
                    kind: if deletable {
                        "registry-user"
                    } else {
                        "registry-machine"
                    }
                    .into(),
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
#[tauri::command(async)]
pub fn scan_residue(
    name: String,
    publisher: Option<String>,
    install_location: Option<String>,
) -> Result<ResidueReport, String> {
    let _ = &publisher; // kept in the IPC signature; vendor names are not matched
    let candidates = name_candidates(&name);
    let mut items = Vec::new();

    if let Some(location) = install_location.as_deref().filter(|l| !l.trim().is_empty()) {
        let path = PathBuf::from(location.trim().trim_matches('"'));
        // The install dir bypasses name matching (the registry itself vouched
        // for it) but must pass the same protected-directory gate as cleanup.
        if path_is_cleanable(&path, &candidates, Some(location)) {
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
/// Directories that must never be handed to the Recycle Bin whole, however a
/// program's `InstallLocation` happens to be written.
///
/// The depth check alone cannot express this. `C:\Program Files` is three
/// components and `C:\Windows\System32` is four — the same shapes as a real
/// per-application install directory — so a vendor that writes its
/// InstallLocation as the parent folder rather than its own would have had
/// the whole of Program Files accepted as a leftover. Installers get this
/// wrong often enough that the guard has to assume it.
fn is_protected_directory(path: &Path) -> bool {
    let canonical = path.canonicalize().ok();
    for var in ["SystemRoot", "windir"] {
        if let Ok(value) = std::env::var(var) {
            let root = PathBuf::from(value);
            if path.starts_with(&root)
                || canonical
                    .as_ref()
                    .zip(root.canonicalize().ok().as_ref())
                    .is_some_and(|(path, root)| path.starts_with(root))
            {
                return true;
            }
        }
    }
    let mut protected: Vec<PathBuf> = Vec::new();
    for var in [
        "SystemRoot",
        "windir",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramW6432",
        "ProgramData",
        "PUBLIC",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "SystemDrive",
    ] {
        if let Ok(value) = std::env::var(var) {
            let root = PathBuf::from(value);
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                if var == "LOCALAPPDATA" {
                    protected.push(Path::new(&local).join("Programs"));
                }
            }
            protected.push(root);
        }
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        for folder in [
            "Desktop",
            "Documents",
            "Downloads",
            "Pictures",
            "Music",
            "Videos",
        ] {
            protected.push(Path::new(&profile).join(folder));
        }
    }
    // A bare drive root ("C:\", "D:\") has no file name of its own.
    if path.file_name().is_none() {
        return true;
    }
    protected.iter().any(|root| {
        path == root
            || canonical
                .as_ref()
                .zip(root.canonicalize().ok().as_ref())
                .is_some_and(|(path, root)| path == root)
    })
}

fn path_is_cleanable(path: &Path, candidates: &[String], install_location: Option<&str>) -> bool {
    if is_protected_directory(path) {
        return false;
    }
    if let Some(location) = install_location {
        let loc = PathBuf::from(location.trim().trim_matches('"'));
        if loc.components().count() > 2 && path == loc && !install_location_in_use(location) {
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
    roots
        .iter()
        .any(|root| path.parent() == Some(root.as_path()))
}

/// Saves `key` (an `HKCU\Software\…` path) as a `.reg` file before it is
/// deleted, so the one step that skips the Recycle Bin can still be undone by
/// double-clicking the file. Returns false, and the key is kept, if the
/// export does not complete.
#[cfg(windows)]
fn export_registry_key(key: &str, backup_dir: &Path) -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    if crate::elevation::ensure_plain_app_data_dir(backup_dir).is_err()
        || std::fs::create_dir_all(backup_dir).is_err()
    {
        return false;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let safe: String = key
        .rsplit('\\')
        .next()
        .unwrap_or("key")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let file = backup_dir.join(format!("{safe}-{stamp}.reg"));
    let Ok(system_root) = std::env::var("SystemRoot") else {
        return false;
    };
    std::process::Command::new(Path::new(&system_root).join(r"System32\reg.exe"))
        .args(["export", key])
        .arg(&file)
        .arg("/y")
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .is_ok_and(|status| status.success())
        && file.is_file()
}

/// Moves the selected leftovers to the Recycle Bin (filesystem items) or
/// deletes them (HKCU registry keys — flagged in the UI as the one
/// non-recoverable step). Every path is revalidated; unknown paths fail.
#[tauri::command(async)]
pub fn clean_residue(
    app: tauri::AppHandle,
    name: String,
    publisher: Option<String>,
    install_location: Option<String>,
    paths: Vec<String>,
) -> Result<CleanResult, String> {
    use tauri::Manager;
    let backup_dir = app
        .path()
        .app_data_dir()
        .map(|dir| dir.join("registry-backups"))
        .map_err(|e| e.to_string())?;
    if !crate::license::license_status(app)? {
        return Err("An active Uninstaller Pro license is required for cleanup.".into());
    }
    if paths.len() > 64 {
        return Err("Too many items in one cleanup.".into());
    }
    let _ = &publisher; // kept in the IPC signature; vendor names are not matched
    let candidates = name_candidates(&name);
    let mut result = CleanResult::default();

    for raw in paths {
        if let Some(key) = raw.strip_prefix(r"HKCU\Software\") {
            #[cfg(windows)]
            {
                use winreg::enums::HKEY_CURRENT_USER;
                use winreg::RegKey;
                let valid = !key.contains('\\') && matches_candidates(key, &candidates);
                let deleted = valid
                    && export_registry_key(&raw, &backup_dir)
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
            {
                let _ = key;
                result.failed.push(raw);
            }
            continue;
        }
        let path = PathBuf::from(&raw);
        if !path_is_cleanable(&path, &candidates, install_location.as_deref()) {
            result.failed.push(raw);
            continue;
        }
        let size = crate::recycle_bin::item_size_kb(&path);
        if let Some(reason) =
            crate::recycle_bin::refusal(crate::recycle_bin::limits_for(&path), size)
        {
            // Windows would delete it permanently instead of recycling it,
            // so it stays where it is.
            crate::applog::line(&format!(
                "residue not recycled: {} ({reason})",
                path.to_string_lossy()
            ));
            result.failed.push(raw);
            continue;
        }
        match trash::delete(&path) {
            Ok(()) => {
                result.freed_kb += size.unwrap_or(0);
                result.removed.push(raw);
            }
            Err(e) => {
                // Cleanup is the destructive step, so a refusal is the one
                // thing a support request most needs to be able to show.
                crate::applog::line(&format!(
                    "residue not recycled: {} ({e})",
                    path.to_string_lossy()
                ));
                result.failed.push(raw);
            }
        }
    }
    crate::applog::line(&format!(
        "residue cleanup for {}: {} removed, {} failed, {} KB freed",
        name,
        result.removed.len(),
        result.failed.len(),
        result.freed_kb
    ));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A vendor that writes its InstallLocation as the containing folder
    /// instead of its own used to hand the whole folder to the Recycle Bin:
    /// the only guard was a component count, and "C:\Program Files" clears
    /// it exactly as "C:\Program Files\Something" does. Cleanup is the one
    /// destructive step in the product, so this is the test that has to fail
    /// if the guard is ever loosened again.
    #[test]
    fn a_well_known_folder_is_never_cleanable_as_an_install_location() {
        for var in [
            "ProgramFiles",
            "ProgramData",
            "SystemRoot",
            "USERPROFILE",
            "LOCALAPPDATA",
        ] {
            let Ok(value) = std::env::var(var) else {
                continue;
            };
            let root = PathBuf::from(&value);
            assert!(
                !path_is_cleanable(&root, &[], Some(&value)),
                "{} ({}) was accepted as a removable install location",
                var,
                value
            );
            let alternate_case = value.to_ascii_uppercase();
            assert!(
                !path_is_cleanable(Path::new(&alternate_case), &[], Some(&alternate_case)),
                "{} ({}) was accepted with alternate casing",
                var,
                alternate_case
            );
            assert!(
                scan_residue("x".into(), None, Some(value.clone()))
                    .unwrap()
                    .items
                    .is_empty(),
                "{} ({}) was shown as a removable leftover",
                var,
                value
            );
            // Ordinary app directories remain eligible; Windows descendants do not.
            let nested = root.join("ExampleVendorApp");
            assert_eq!(
                path_is_cleanable(&nested, &[], Some(&nested.to_string_lossy())),
                var != "SystemRoot",
                "unexpected eligibility under {}",
                var
            );
            if var == "SystemRoot" {
                let system32 = root.join("System32").to_string_lossy().to_ascii_uppercase();
                assert!(!path_is_cleanable(
                    Path::new(&system32),
                    &["system32".into()],
                    Some(&system32)
                ));
                assert!(scan_residue("System32".into(), None, Some(system32))
                    .unwrap()
                    .items
                    .is_empty());
            }
        }
    }

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
        assert_eq!(name_candidates("VLC"), Vec::<String>::new()); // too short
        assert_eq!(name_candidates("Microsoft"), Vec::<String>::new());
        let c = name_candidates("SuperTool 1.2");
        assert_eq!(c, vec!["supertool".to_string()]);
    }

    #[test]
    fn a_folder_still_hosting_an_installed_program_is_never_cleanable() {
        let others = vec![
            Some(r"C:\Program Files\JetBrains\PyCharm 2026.2".to_string()),
            Some(r"D:\Games\Other".to_string()),
            None,
        ];
        // A vendor parent folder, a drive-level games folder, and the exact
        // location of a program that is still registered all stay.
        assert!(location_hosts_program(
            r"C:\Program Files\JetBrains",
            &others
        ));
        assert!(location_hosts_program(r"D:\Games", &others));
        assert!(location_hosts_program(
            r"C:\Program Files\JetBrains\PyCharm 2026.2\",
            &others
        ));
        // The removed product's own folder, with nothing else inside it, may go.
        assert!(!location_hosts_program(
            r"C:\Program Files\JetBrains\IntelliJ IDEA 2026.2",
            &others
        ));
        // A bare drive or unreadable value is never treated as cleanable.
        assert!(location_hosts_program(r"C:\", &others));
    }

    #[test]
    fn matching_is_exact_not_substring() {
        let c = name_candidates("SuperTool");
        assert!(matches_candidates("SuperTool", &c));
        assert!(matches_candidates("super-tool", &c));
        assert!(!matches_candidates("SuperTools", &c));
        assert!(!matches_candidates("MySuperTool", &c));
        assert!(!matches_candidates("Microsoft", &c));
    }

    #[test]
    fn cleanup_rejects_paths_outside_known_roots() {
        let c = name_candidates("SuperTool");
        assert!(!path_is_cleanable(
            Path::new(r"C:/Windows/System32"),
            &c,
            None
        ));
        assert!(!path_is_cleanable(
            Path::new(r"C:/random/supertool"),
            &c,
            None
        ));
        // Install location is honored exactly, nothing near it.
        assert!(path_is_cleanable(
            Path::new(r"C:/Program Files/SuperTool"),
            &c,
            Some(r"C:/Program Files/SuperTool")
        ));
        assert!(!path_is_cleanable(
            Path::new(r"C:/Program Files/Other"),
            &c,
            Some(r"C:/Program Files/SuperTool")
        ));
    }

    #[test]
    fn bare_drive_install_locations_are_never_cleanable() {
        let c = name_candidates("SuperTool");
        assert!(!path_is_cleanable(Path::new(r"C:/"), &c, Some(r"C:/")));
    }
}
