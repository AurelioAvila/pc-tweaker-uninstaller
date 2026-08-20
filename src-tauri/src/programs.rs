//! Read-only enumeration of installed programs from the Windows registry.
//!
//! Sources are the three views Add/Remove Programs itself reads:
//! - HKLM\...\Uninstall            (64-bit machine installs)
//! - HKLM\...\WOW6432Node\...      (32-bit machine installs)
//! - HKCU\...\Uninstall            (per-user installs)
//!
//! Everything is opened with KEY_READ — this module cannot write, by
//! construction. Filtering (which raw entries count as "a program" versus an
//! update or OS component) and value normalization are pure functions so
//! they are unit-tested on any platform; only the registry walk itself is
//! Windows-specific.

use crate::uninstall_command::{self, Classification};
use serde::Serialize;

/// One installed program, shaped for the UI. Every string field originates
/// in the registry and is untrusted display data — the frontend renders it
/// as text (React escaping), never as markup.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProgramInfo {
    /// Registry key name — stable identifier for later phases.
    pub id: String,
    /// Which view the entry came from: "machine64", "machine32", "user".
    pub source: &'static str,
    pub name: String,
    pub version: Option<String>,
    pub publisher: Option<String>,
    /// ISO date (YYYY-MM-DD) when the registry recorded one.
    pub install_date: Option<String>,
    pub estimated_size_kb: Option<u32>,
    pub install_location: Option<String>,
    /// How the uninstall command classified; drives the UI badge and, in
    /// Phase 3, what the executor is allowed to do.
    pub uninstall: UninstallSummary,
    /// True for entries Add/Remove Programs hides (system components, child
    /// updates). Listed behind an explicit UI toggle, clearly marked — power
    /// users get the full picture, nobody trips over an OS component by
    /// accident.
    pub hidden: bool,
    /// Removal Confidence Score: Safe / Review / Keep plus reason codes.
    /// Evidence-based, never presented as certainty (see confidence.rs).
    pub confidence: crate::confidence::Confidence,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum UninstallSummary {
    Msi,
    Executable,
    ManualOnly,
    None,
    Invalid,
}

/// The raw values Phase 2 reads from one Uninstall subkey, decoupled from
/// winreg so the filtering rules below are testable everywhere.
#[derive(Default, Clone, Debug)]
pub struct RawEntry {
    pub key_name: String,
    pub display_name: Option<String>,
    pub display_version: Option<String>,
    pub publisher: Option<String>,
    pub install_date: Option<String>,
    pub estimated_size_kb: Option<u32>,
    pub install_location: Option<String>,
    pub uninstall_string: Option<String>,
    pub quiet_uninstall_string: Option<String>,
    pub system_component: Option<u32>,
    pub parent_key_name: Option<String>,
    pub release_type: Option<String>,
    /// Conventionally the app's main executable (possibly with ",iconIndex").
    /// Used only for the "Open PC Tweaker" suite launch, behind full gating.
    pub display_icon: Option<String>,
}

/// Add/Remove Programs' own rules, distilled: an entry is a listable program
/// only when it has a display name, is not flagged as an OS/system
/// component, and is not a child update/hotfix of another product. Erring
/// toward exclusion is deliberate — showing an OS component in an
/// *uninstaller* invites exactly the destructive click this app exists to
/// prevent.
pub fn is_listable(entry: &RawEntry) -> bool {
    let has_name = entry
        .display_name
        .as_deref()
        .is_some_and(|n| !n.trim().is_empty());
    if !has_name {
        return false;
    }
    if entry.system_component == Some(1) {
        return false;
    }
    if entry
        .parent_key_name
        .as_deref()
        .is_some_and(|p| !p.trim().is_empty())
    {
        return false;
    }
    if entry.release_type.as_deref().is_some_and(|r| {
        let r = r.to_ascii_lowercase();
        r.contains("update") || r.contains("hotfix") || r.contains("service pack")
    }) {
        return false;
    }
    true
}

/// Registry `InstallDate` is conventionally `YYYYMMDD`. Anything else is
/// dropped rather than displayed wrong — a missing date is honest, a
/// misparsed one is misinformation.
pub fn normalize_install_date(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.len() != 8 || !raw.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let (year, rest) = raw.split_at(4);
    let (month, day) = rest.split_at(2);
    let (m, d): (u32, u32) = (month.parse().ok()?, day.parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(format!("{year}-{month}-{day}"))
}

/// Classifies the entry's uninstall command for the UI. Prefers the quiet
/// string when both exist and parse — it is the one built for unattended
/// runs. A string that fails to parse is surfaced as Invalid, not hidden:
/// the user deserves to know an entry is broken.
pub fn summarize_uninstall(entry: &RawEntry) -> UninstallSummary {
    let candidates = [
        entry.quiet_uninstall_string.as_deref(),
        entry.uninstall_string.as_deref(),
    ];
    let mut saw_any = false;
    let mut saw_manual = false;
    for candidate in candidates.into_iter().flatten() {
        saw_any = true;
        match uninstall_command::parse(candidate) {
            Ok(Classification::Msi { .. }) => return UninstallSummary::Msi,
            Ok(Classification::Executable { .. }) => return UninstallSummary::Executable,
            Ok(Classification::ManualOnly { .. }) => saw_manual = true,
            Err(_) => {}
        }
    }
    if saw_manual {
        UninstallSummary::ManualOnly
    } else if saw_any {
        UninstallSummary::Invalid
    } else {
        UninstallSummary::None
    }
}

/// An entry can be shown at all only if it has something to call itself.
pub fn has_display_name(entry: &RawEntry) -> bool {
    entry
        .display_name
        .as_deref()
        .is_some_and(|n| !n.trim().is_empty())
}

/// Converts a raw entry into UI shape.
pub fn to_program_info(entry: RawEntry, source: &'static str, hidden: bool) -> ProgramInfo {
    let uninstall = summarize_uninstall(&entry);
    let confidence = crate::confidence::assess(&entry, hidden, &uninstall);
    ProgramInfo {
        confidence,
        id: entry.key_name,
        source,
        name: entry.display_name.unwrap_or_default(),
        version: entry.display_version,
        publisher: entry.publisher,
        install_date: entry
            .install_date
            .as_deref()
            .and_then(normalize_install_date),
        estimated_size_kb: entry.estimated_size_kb,
        install_location: entry.install_location,
        uninstall,
        hidden,
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::RegKey;

    const UNINSTALL_PATH: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

    fn read_entry(key: &RegKey, key_name: &str) -> RawEntry {
        // Missing values are the norm, not an error: every field is optional.
        let s = |name: &str| {
            key.get_value::<String, _>(name)
                .ok()
                .filter(|v| !v.is_empty())
        };
        let d = |name: &str| key.get_value::<u32, _>(name).ok();
        RawEntry {
            key_name: key_name.to_string(),
            display_name: s("DisplayName"),
            display_version: s("DisplayVersion"),
            publisher: s("Publisher"),
            install_date: s("InstallDate"),
            estimated_size_kb: d("EstimatedSize"),
            install_location: s("InstallLocation"),
            uninstall_string: s("UninstallString"),
            quiet_uninstall_string: s("QuietUninstallString"),
            system_component: d("SystemComponent"),
            parent_key_name: s("ParentKeyName"),
            release_type: s("ReleaseType"),
            display_icon: s("DisplayIcon"),
        }
    }

    fn read_view(
        hive: winreg::HKEY,
        access_flags: u32,
        source: &'static str,
        out: &mut Vec<ProgramInfo>,
    ) {
        let root = RegKey::predef(hive);
        // A missing/unreadable view is skipped, not fatal: HKCU always
        // exists, but WOW6432Node doesn't on 32-bit-only systems.
        let Ok(uninstall) = root.open_subkey_with_flags(UNINSTALL_PATH, KEY_READ | access_flags)
        else {
            return;
        };
        for key_name in uninstall.enum_keys().flatten() {
            let Ok(sub) = uninstall.open_subkey_with_flags(&key_name, KEY_READ | access_flags)
            else {
                continue;
            };
            let entry = read_entry(&sub, &key_name);
            // Everything with a name is returned; entries ARP would hide are
            // flagged instead of dropped, so the UI's "show hidden" toggle
            // can reveal the complete picture.
            if has_display_name(&entry) {
                let hidden = !is_listable(&entry);
                out.push(to_program_info(entry, source, hidden));
            }
        }
    }

    /// Re-reads ONE entry fresh from the registry, by the same (source, key)
    /// pair `list()` reported. Phase 3's executor calls this instead of
    /// trusting anything cached or webview-supplied: the registry at execution
    /// time is the only source of truth.
    pub fn read_one(source: &str, key_name: &str) -> Result<RawEntry, String> {
        let (hive, flags) = match source {
            "machine64" => (HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY),
            "machine32" => (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY),
            "user" => (HKEY_CURRENT_USER, 0),
            _ => return Err("Unknown program source.".to_string()),
        };
        let root = RegKey::predef(hive);
        let uninstall = root
            .open_subkey_with_flags(UNINSTALL_PATH, KEY_READ | flags)
            .map_err(|_| "The uninstall registry view could not be opened.".to_string())?;
        let sub = uninstall
            .open_subkey_with_flags(key_name, KEY_READ | flags)
            .map_err(|_| {
                "This program is no longer registered — it may already be uninstalled.".to_string()
            })?;
        Ok(read_entry(&sub, key_name))
    }

    pub fn list() -> Vec<ProgramInfo> {
        let mut programs = Vec::new();
        read_view(
            HKEY_LOCAL_MACHINE,
            KEY_WOW64_64KEY,
            "machine64",
            &mut programs,
        );
        read_view(
            HKEY_LOCAL_MACHINE,
            KEY_WOW64_32KEY,
            "machine32",
            &mut programs,
        );
        // HKCU has a single view; extra WOW flags are unnecessary there.
        read_view(HKEY_CURRENT_USER, 0, "user", &mut programs);
        programs.sort_by_key(|p| p.name.to_lowercase());
        programs
    }
}

/// Re-reads one entry fresh from the registry. See `imp::read_one`.
#[cfg(windows)]
pub fn read_raw_entry(source: &str, key_name: &str) -> Result<RawEntry, String> {
    imp::read_one(source, key_name)
}

/// Finds a suite member's executable: scans the three uninstall views for an
/// entry whose display name satisfies `matches`, then extracts an .exe path
/// from its DisplayIcon via `extract`. Read-only; the caller gates the path.
#[cfg(windows)]
pub fn find_suite_exe(
    matches: fn(&str) -> bool,
    extract: fn(&str) -> Option<String>,
) -> Option<String> {
    for info in imp::list() {
        if !matches(&info.name) {
            continue;
        }
        let entry = read_raw_entry(info.source, &info.id).ok()?;
        if let Some(exe) = entry.display_icon.as_deref().and_then(extract) {
            return Some(exe);
        }
    }
    None
}

#[cfg(not(windows))]
pub fn find_suite_exe(
    _matches: fn(&str) -> bool,
    _extract: fn(&str) -> Option<String>,
) -> Option<String> {
    None
}

#[cfg(not(windows))]
pub fn read_raw_entry(_source: &str, _key_name: &str) -> Result<RawEntry, String> {
    Err("Reading installed programs is only supported on Windows.".to_string())
}

/// Lists installed programs. Takes no input from the webview at all — the
/// safest IPC surface is the one with nothing to validate.
#[tauri::command]
pub fn list_programs() -> Result<Vec<ProgramInfo>, String> {
    #[cfg(windows)]
    {
        Ok(imp::list())
    }
    #[cfg(not(windows))]
    {
        Err("Listing installed programs is only supported on Windows.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> RawEntry {
        RawEntry {
            key_name: "TestKey".into(),
            display_name: Some(name.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn entries_without_a_display_name_are_never_listed() {
        assert!(!is_listable(&RawEntry::default()));
        let mut blank = named("   ");
        blank.display_name = Some("   ".into());
        assert!(!is_listable(&blank));
        assert!(is_listable(&named("7-Zip")));
    }

    #[test]
    fn system_components_and_child_updates_are_excluded() {
        let mut sys = named("OS Component");
        sys.system_component = Some(1);
        assert!(!is_listable(&sys));

        let mut sys0 = named("Normal with flag 0");
        sys0.system_component = Some(0);
        assert!(is_listable(&sys0));

        let mut child = named("Security Update for Foo");
        child.parent_key_name = Some("Foo".into());
        assert!(!is_listable(&child));

        let mut hotfix = named("Hotfix XYZ");
        hotfix.release_type = Some("Hotfix".into());
        assert!(!is_listable(&hotfix));
    }

    #[test]
    fn install_dates_normalize_or_vanish_never_garble() {
        assert_eq!(
            normalize_install_date("20260812").as_deref(),
            Some("2026-08-12")
        );
        assert_eq!(normalize_install_date("2026-08-12"), None);
        assert_eq!(normalize_install_date("20261301"), None); // month 13
        assert_eq!(normalize_install_date("20260800"), None); // day 0
        assert_eq!(normalize_install_date("garbage"), None);
        assert_eq!(normalize_install_date(""), None);
    }

    #[test]
    fn uninstall_summary_prefers_the_quiet_string_and_reports_invalid_honestly() {
        let mut entry = named("App");
        entry.uninstall_string = Some(r#""C:\App\unins.exe""#.into());
        entry.quiet_uninstall_string =
            Some("MsiExec.exe /X{6F340107-F9AA-47C6-B54C-C3A19F11553C}".into());
        assert_eq!(summarize_uninstall(&entry), UninstallSummary::Msi);

        let mut exe_only = named("App");
        exe_only.uninstall_string = Some(r#""C:\App\unins.exe" /SILENT"#.into());
        assert_eq!(summarize_uninstall(&exe_only), UninstallSummary::Executable);

        let mut manual = named("App");
        manual.uninstall_string = Some("cmd.exe /c cleanup.bat".into());
        assert_eq!(summarize_uninstall(&manual), UninstallSummary::ManualOnly);

        let mut broken = named("App");
        broken.uninstall_string = Some("msiexec.exe /x {not-a-guid}".into());
        assert_eq!(summarize_uninstall(&broken), UninstallSummary::Invalid);

        assert_eq!(summarize_uninstall(&named("App")), UninstallSummary::None);
    }

    #[test]
    fn to_program_info_carries_fields_through_and_normalizes_the_date() {
        let mut entry = named("7-Zip");
        entry.display_version = Some("24.08".into());
        entry.publisher = Some("Igor Pavlov".into());
        entry.install_date = Some("20260101".into());
        entry.estimated_size_kb = Some(5312);
        entry.uninstall_string = Some(r#""C:\Program Files\7-Zip\Uninstall.exe""#.into());

        let info = to_program_info(entry, "machine64", false);
        assert_eq!(info.name, "7-Zip");
        assert_eq!(info.source, "machine64");
        assert_eq!(info.install_date.as_deref(), Some("2026-01-01"));
        assert_eq!(info.uninstall, UninstallSummary::Executable);
        assert!(!info.hidden);
    }

    #[test]
    fn hidden_entries_keep_their_name_and_carry_the_flag() {
        let mut sys = named("Microsoft Update Health Tools");
        sys.system_component = Some(1);
        assert!(has_display_name(&sys));
        assert!(!is_listable(&sys));
        let info = to_program_info(sys, "machine64", true);
        assert!(info.hidden);
    }
}
