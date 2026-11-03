//! The rules behind the leftover scan, as pure functions: what a candidate
//! item proves, which tier that earns it, which folders may anchor a scan at
//! all, and the stable id the webview uses instead of a path.
//!
//! Tiers are rule gates, deliberately not additive points: name evidence can
//! never add up to a pre-selection (the failure behind the BCU bugs #699,
//! #727 and #520, where a name match was enough to delete).

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// What a leftover is. Free cleans the first three (files and folders, sent
/// to the Recycle Bin); everything else is Pro.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    InstallDir,
    AppData,
    Shortcut,
    RegistryUser,
    RegistryMachine,
    StartupRun,
    StartupShortcut,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::InstallDir => "install-dir",
            Kind::AppData => "app-data",
            Kind::Shortcut => "shortcut",
            Kind::RegistryUser => "registry-user",
            Kind::RegistryMachine => "registry-machine",
            Kind::StartupRun => "startup-run",
            Kind::StartupShortcut => "startup-shortcut",
        }
    }

    /// Owner decision 3/11/2026: files and folders are free, registry keys
    /// and startup entries are Pro. Enforced in `clean_residue`, not only in
    /// the interface.
    pub fn pro_only(self) -> bool {
        !matches!(self, Kind::InstallDir | Kind::AppData | Kind::Shortcut)
    }
}

/// How the item is tied to the removed program, strongest first. Proof from a
/// pre-uninstall snapshot (phase 3) will slot in above `Path`.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Evidence {
    /// Inside the install folder the registry recorded.
    Path,
    /// Runs a file that is provably gone, and nothing says whose it was.
    Dangling,
    /// Only its name looks like the program's.
    Name,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    /// Pre-selected.
    Safe,
    /// Shown, not selected.
    Review,
    /// Never selectable; the reason is shown.
    Keep,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Blocker {
    WindowsFolder,
    ProtectedFolder,
    OtherProgram,
    Linked,
    NotRestorable,
    WellKnownKey,
}

/// Where the program's identity came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trust {
    /// Read from the registry by this app (this session, or recorded earlier).
    Registry,
    /// Typed by the user.
    Manual,
    /// No identity at all (the first-launch sweep).
    None,
}

pub struct Facts {
    pub evidence: Evidence,
    pub trust: Trust,
    pub blocker: Option<Blocker>,
    /// The item starts code (service, task, startup entry, firewall rule).
    pub starts_code: bool,
    /// The item can be put back exactly (Recycle Bin, byte-exact backup).
    pub restorable: bool,
    /// Nothing it points at can still run: its target is missing, or it is
    /// the registry-sourced install folder itself.
    pub inert: bool,
}

/// The blocker that decides the item, if any: a real one, or the lack of an
/// exact way back.
pub fn effective_blocker(f: &Facts) -> Option<Blocker> {
    f.blocker
        .or((!f.restorable).then_some(Blocker::NotRestorable))
}

pub fn tier(f: &Facts) -> Tier {
    if effective_blocker(f).is_some() {
        return Tier::Keep;
    }
    match f.evidence {
        Evidence::Name if f.starts_code => Tier::Keep,
        Evidence::Name | Evidence::Dangling => Tier::Review,
        Evidence::Path if f.trust == Trust::Registry && f.inert => Tier::Safe,
        Evidence::Path => Tier::Review,
    }
}

/// `SHA-256(kind, scope, location, value)`, first 8 bytes. The webview only
/// ever sends these back, never a path.
pub fn item_id(kind: Kind, machine_scope: bool, location: &str, value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(kind.as_str());
    hash.update([0]);
    hash.update(if machine_scope { "machine" } else { "user" });
    hash.update([0]);
    hash.update(location.to_lowercase());
    hash.update([0]);
    hash.update(value.to_lowercase());
    hash.finalize()[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Registry keys that are never leftovers of anything: the shared roots of
/// `Software`. Matched on the first component under `Software` (and under
/// `WOW6432Node`).
const WELL_KNOWN_KEYS: &[&str] = &[
    "classes",
    "microsoft",
    "policies",
    "wow6432node",
    "clients",
    "registeredapplications",
    "python",
    "javasoft",
    "khronos",
    "odbc",
    "partner",
    "description",
    "setup",
    "defaultuserenvironment",
    "intel",
    "nvidia corporation",
    "amd",
    // This app's own backup store.
    "pc tweaker uninstaller",
];

/// Folders shared by many programs or by Windows: a folder with one of these
/// anywhere in its path never anchors a scan, so neither it nor anything in
/// it is ever cleaned as a whole.
const SHARED_FOLDERS: &[&str] = &[
    "common files",
    "package cache",
    "microsoft",
    "windows",
    "windowsapps",
    "modifiablewindowsapps",
    "microsoft shared",
    "microsoft.net",
    "windows nt",
    "windows defender",
    "windows defender advanced threat protection",
    "windows mail",
    "windows media player",
    "windows photo viewer",
    "windows sidebar",
    "windows security",
    "windowspowershell",
    "reference assemblies",
    "msbuild",
    "dotnet",
    "uninstall information",
    "installshield installation information",
    "usoshared",
    "usoprivate",
    "regid.1991-06.com.microsoft",
    "ssh",
];

/// False for a key path that is a well-known shared root or not a single
/// program-named key below `Software` (or `Software\WOW6432Node`).
pub fn registry_candidate_ok(path: &str) -> bool {
    let Some((_, rest)) = path.split_once('\\') else {
        return false;
    };
    let Some(rest) = rest
        .get(..9)
        .filter(|s| s.eq_ignore_ascii_case(r"Software\"))
        .map(|_| &rest[9..])
    else {
        return false;
    };
    let rest = if rest.len() > 12 && rest[..12].eq_ignore_ascii_case(r"WOW6432Node\") {
        &rest[12..]
    } else {
        rest
    };
    !rest.is_empty()
        && !rest.contains('\\')
        && !WELL_KNOWN_KEYS.contains(&rest.to_ascii_lowercase().as_str())
}

/// A folder name shared by many programs or by Windows, never one program's.
pub fn shared_folder(name: &str) -> bool {
    SHARED_FOLDERS.contains(&name.to_lowercase().as_str())
}

/// What the anchor gate looks at, gathered once.
#[derive(Clone)]
pub struct GateEnv {
    pub windows: Vec<String>,
    pub protected: Vec<String>,
    pub shared_roots: Vec<String>,
    pub own: Option<String>,
    /// Install locations still registered with Windows.
    pub others: Vec<Option<String>>,
    /// The program list could not be read: every folder is treated as shared.
    pub programs_unknown: bool,
}

pub enum Refusal {
    /// Not an absolute local path at all.
    Shape,
    Blocked(Blocker),
}

fn clean_path(raw: &str) -> String {
    raw.trim()
        .trim_matches('"')
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_string()
}

/// True when any of `others` (install locations still registered with
/// Windows) is `location` itself or lies inside it. A shared vendor folder,
/// or the folder of a program whose uninstall did not finish, must stay.
/// An unreadable location is treated as shared.
pub fn location_hosts_program(location: &str, others: &[Option<String>]) -> bool {
    let Some(root) = crate::relations::normalized_root(Some(location)) else {
        return true;
    };
    others.iter().any(|other| {
        crate::relations::normalized_root(other.as_deref())
            .is_some_and(|other| other == root || crate::relations::is_inside(&other, &root))
    })
}

fn strip_verbatim(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_lowercase()
}

/// Whether `raw` may anchor a scan or be cleaned as a whole: an absolute
/// local path at least two folders below a drive, with no `.` or `..` parts,
/// outside the Windows folder and every well-known or shared location, not
/// this app's own folder, not holding another registered program, and not
/// reached through a link. A folder that is already gone is judged by its
/// nearest existing parent.
pub fn anchor_gate(
    raw: &str,
    env: &GateEnv,
    canonical: &dyn Fn(&Path) -> Option<PathBuf>,
) -> Result<PathBuf, Refusal> {
    let clean = clean_path(raw);
    let bytes = clean.as_bytes();
    let absolute = bytes.len() > 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && bytes[2] == b'\\'
        && !clean.contains(['*', '?', '<', '>', '|', '"']);
    if !absolute || clean.split('\\').any(|p| p == "." || p == "..") {
        return Err(Refusal::Shape);
    }
    let parts: Vec<String> = clean[3..]
        .split('\\')
        .filter(|p| !p.is_empty())
        .map(str::to_lowercase)
        .collect();
    let lower = clean.to_lowercase();
    let inside = |root: &str| lower == root || lower.starts_with(&format!("{root}\\"));
    if env.windows.iter().any(|w| inside(w)) {
        return Err(Refusal::Blocked(Blocker::WindowsFolder));
    }
    let own_ancestor = env
        .own
        .as_deref()
        .is_some_and(|own| inside(own) || own.starts_with(&format!("{lower}\\")));
    if parts.len() < 2
        || env.protected.iter().any(|p| &lower == p)
        || env.shared_roots.iter().any(|r| inside(r))
        || parts.iter().any(|p| SHARED_FOLDERS.contains(&p.as_str()))
        || own_ancestor
    {
        return Err(Refusal::Blocked(Blocker::ProtectedFolder));
    }
    let path = PathBuf::from(&clean);
    // A protected folder reached by another spelling (a junction, 8.3 names).
    if let Some(real) = canonical(&path).map(|p| strip_verbatim(&p)) {
        if env
            .windows
            .iter()
            .any(|w| real == *w || real.starts_with(&format!("{w}\\")))
        {
            return Err(Refusal::Blocked(Blocker::WindowsFolder));
        }
        if env.protected.contains(&real) {
            return Err(Refusal::Blocked(Blocker::ProtectedFolder));
        }
    }
    if env.programs_unknown || location_hosts_program(&clean, &env.others) {
        return Err(Refusal::Blocked(Blocker::OtherProgram));
    }
    let mut probe = path.clone();
    loop {
        if let Some(real) = canonical(&probe) {
            let want = probe.to_string_lossy().to_lowercase();
            if strip_verbatim(&real) != want {
                return Err(Refusal::Blocked(Blocker::Linked));
            }
            break;
        }
        if !probe.pop() || probe.parent().is_none() {
            break;
        }
    }
    Ok(path)
}

fn env_dir(var: &str) -> Option<String> {
    std::env::var(var)
        .ok()
        .map(|v| clean_path(&v).to_lowercase())
        .filter(|v| !v.is_empty())
}

impl GateEnv {
    pub fn from_process(others: Option<Vec<Option<String>>>) -> GateEnv {
        #[allow(unused_mut)]
        let mut windows: Vec<String> = ["SystemRoot", "windir"]
            .iter()
            .filter_map(|v| env_dir(v))
            .collect();
        let mut protected: Vec<String> = [
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramW6432",
            "ProgramData",
            "PUBLIC",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
            "SystemDrive",
        ]
        .iter()
        .filter_map(|v| env_dir(v))
        .collect();
        if let Some(local) = env_dir("LOCALAPPDATA") {
            protected.push(format!(r"{local}\programs"));
        }
        if let Some(profile) = env_dir("USERPROFILE") {
            for folder in [
                "desktop",
                "documents",
                "downloads",
                "pictures",
                "music",
                "videos",
            ] {
                protected.push(format!(r"{profile}\{folder}"));
            }
        }
        // The same folders as Windows records them: the user's own variables
        // reach the elevated child too, so they alone are not trusted.
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Shell::{
                FOLDERID_LocalAppData, FOLDERID_Profile, FOLDERID_ProgramData,
                FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86, FOLDERID_Public,
                FOLDERID_RoamingAppData, FOLDERID_Windows,
            };
            let known = |id: &windows_sys::core::GUID| {
                crate::elevation::known_folder(id)
                    .map(|v| clean_path(&v).to_lowercase())
                    .filter(|v| !v.is_empty())
            };
            windows.extend(known(&FOLDERID_Windows));
            for id in [
                FOLDERID_ProgramFiles,
                FOLDERID_ProgramFilesX86,
                FOLDERID_ProgramData,
                FOLDERID_Public,
                FOLDERID_Profile,
                FOLDERID_RoamingAppData,
                FOLDERID_LocalAppData,
            ] {
                protected.extend(known(&id));
            }
            if let Some(local) = known(&FOLDERID_LocalAppData) {
                protected.push(format!(r"{local}\programs"));
            }
        }
        let shared_roots = ["TEMP", "TMP"].iter().filter_map(|v| env_dir(v)).collect();
        let own = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.to_string_lossy().to_lowercase()));
        GateEnv {
            windows,
            protected,
            shared_roots,
            own,
            programs_unknown: others.is_none(),
            others: others.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> GateEnv {
        GateEnv {
            windows: vec![r"c:\windows".into()],
            protected: vec![
                r"c:\program files".into(),
                r"c:\programdata".into(),
                r"c:\users\me".into(),
                r"c:\users\me\appdata\local".into(),
                r"c:\users\me\appdata\local\programs".into(),
                r"c:\users\me\documents".into(),
            ],
            shared_roots: vec![r"c:\users\me\appdata\local\temp".into()],
            own: Some(r"c:\program files\pc tweaker uninstaller".into()),
            others: vec![
                Some(r"C:\Program Files\JetBrains\PyCharm 2026.2".into()),
                None,
            ],
            programs_unknown: false,
        }
    }
    // Everything exists and resolves to itself.
    fn same(p: &Path) -> Option<PathBuf> {
        Some(p.to_path_buf())
    }
    fn gate(raw: &str) -> Result<PathBuf, Refusal> {
        anchor_gate(raw, &env(), &same)
    }
    fn blocker(raw: &str) -> Option<Blocker> {
        match gate(raw) {
            Err(Refusal::Blocked(b)) => Some(b),
            _ => None,
        }
    }

    fn facts(evidence: Evidence, trust: Trust) -> Facts {
        Facts {
            evidence,
            trust,
            blocker: None,
            starts_code: false,
            restorable: true,
            inert: true,
        }
    }

    /// Every combination of the inputs, so a new rule cannot quietly open a
    /// path to Safe for name evidence, a manual identity, or no identity.
    #[test]
    fn name_evidence_and_user_assertions_are_never_safe() {
        for evidence in [Evidence::Path, Evidence::Dangling, Evidence::Name] {
            for trust in [Trust::Registry, Trust::Manual, Trust::None] {
                for bits in 0..8u8 {
                    let (starts_code, restorable, inert) =
                        (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
                    for blocker in [None, Some(Blocker::OtherProgram)] {
                        let f = Facts {
                            evidence,
                            trust,
                            blocker,
                            starts_code,
                            restorable,
                            inert,
                        };
                        if tier(&f) == Tier::Safe {
                            assert_eq!(evidence, Evidence::Path);
                            assert_eq!(trust, Trust::Registry);
                            assert!(blocker.is_none() && restorable && inert);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_tier_table() {
        assert_eq!(tier(&facts(Evidence::Path, Trust::Registry)), Tier::Safe);
        assert_eq!(tier(&facts(Evidence::Path, Trust::Manual)), Tier::Review);
        assert_eq!(tier(&facts(Evidence::Name, Trust::Registry)), Tier::Review);
        assert_eq!(tier(&facts(Evidence::Dangling, Trust::None)), Tier::Review);
        let mut live = facts(Evidence::Path, Trust::Registry);
        live.inert = false;
        assert_eq!(tier(&live), Tier::Review);
        let mut code = facts(Evidence::Name, Trust::Registry);
        code.starts_code = true;
        assert_eq!(tier(&code), Tier::Keep);
        let mut blocked = facts(Evidence::Path, Trust::Registry);
        blocked.blocker = Some(Blocker::WindowsFolder);
        assert_eq!(tier(&blocked), Tier::Keep);
        let mut loose = facts(Evidence::Path, Trust::Registry);
        loose.restorable = false;
        assert_eq!(tier(&loose), Tier::Keep);
        assert_eq!(effective_blocker(&loose), Some(Blocker::NotRestorable));
    }

    #[test]
    fn only_files_and_folders_are_free() {
        for kind in [Kind::InstallDir, Kind::AppData, Kind::Shortcut] {
            assert!(!kind.pro_only(), "{kind:?}");
        }
        for kind in [
            Kind::RegistryUser,
            Kind::RegistryMachine,
            Kind::StartupRun,
            Kind::StartupShortcut,
        ] {
            assert!(kind.pro_only(), "{kind:?}");
        }
    }

    #[test]
    fn ids_are_stable_short_and_tell_items_apart() {
        let a = item_id(Kind::AppData, false, r"C:\Users\me\AppData\Roaming\Foo", "");
        assert_eq!(a.len(), 16);
        assert_eq!(
            a,
            item_id(Kind::AppData, false, r"c:\users\me\appdata\roaming\FOO", "")
        );
        assert_ne!(
            a,
            item_id(
                Kind::Shortcut,
                false,
                r"C:\Users\me\AppData\Roaming\Foo",
                ""
            )
        );
        assert_ne!(
            a,
            item_id(Kind::AppData, true, r"C:\Users\me\AppData\Roaming\Foo", "")
        );
        assert_ne!(
            item_id(Kind::StartupRun, false, "HKCU\\Run", "One"),
            item_id(Kind::StartupRun, false, "HKCU\\Run", "Two")
        );
    }

    #[test]
    fn well_known_registry_keys_are_never_candidates() {
        for bad in [
            r"HKCU\Software\Classes",
            r"HKLM\Software\Microsoft",
            r"HKLM\SOFTWARE\Policies",
            r"HKLM\Software\WOW6432Node",
            r"HKLM\Software\WOW6432Node\Classes",
            r"HKLM\Software\WOW6432Node\Microsoft",
            r"HKCU\Software\Clients",
            r"HKLM\Software\RegisteredApplications",
            r"HKLM\SOFTWARE\Python",
            r"HKLM\Software\WOW6432Node\JavaSoft",
            r"HKLM\SOFTWARE\ODBC",
            r"HKLM\SOFTWARE\PC Tweaker Uninstaller",
            r"HKCU\Software\Foo\Bar",
            r"HKCU\Software",
            r"HKCU\Software\",
            r"HKCU\Other\Foo",
            "nonsense",
        ] {
            assert!(!registry_candidate_ok(bad), "{bad}");
        }
        for good in [
            r"HKCU\Software\SuperTool",
            r"HKLM\Software\SuperTool",
            r"HKLM\Software\WOW6432Node\SuperTool",
        ] {
            assert!(registry_candidate_ok(good), "{good}");
        }
    }

    #[test]
    fn the_anchor_gate_refuses_what_a_scan_must_never_start_from() {
        assert!(gate(r"C:\Program Files\Foo").is_ok());
        assert!(gate(r"C:\Program Files\Foo\").is_ok());
        assert!(gate(r#""C:\Games\Foo""#).is_ok());
        assert!(gate(r"C:/Games/Foo").is_ok());
        // Shapes.
        for bad in [
            "",
            "Foo",
            r"\\server\share\Foo",
            r"C:\Games\Foo\..",
            r"C:\Games\.\Foo",
            r"C:\Games\Fo*",
            r"C:\",
        ] {
            assert!(matches!(gate(bad), Err(Refusal::Shape)), "{bad}");
        }
        // Too shallow, well-known, the Windows folder and everything in it.
        assert_eq!(blocker(r"C:\Foo"), Some(Blocker::ProtectedFolder));
        assert_eq!(blocker(r"C:\Program Files"), Some(Blocker::ProtectedFolder));
        assert_eq!(
            blocker(r"C:\USERS\ME\APPDATA\LOCAL\PROGRAMS"),
            Some(Blocker::ProtectedFolder)
        );
        assert_eq!(
            blocker(r"C:\Users\me\Documents"),
            Some(Blocker::ProtectedFolder)
        );
        assert_eq!(
            blocker(r"C:\Windows\System32"),
            Some(Blocker::WindowsFolder)
        );
        assert_eq!(
            blocker(r"c:\WINDOWS\Installer\Foo"),
            Some(Blocker::WindowsFolder)
        );
        // Shared places.
        assert_eq!(
            blocker(r"C:\Program Files\Common Files\Foo"),
            Some(Blocker::ProtectedFolder)
        );
        assert_eq!(
            blocker(r"C:\ProgramData\Package Cache\Foo"),
            Some(Blocker::ProtectedFolder)
        );
        assert_eq!(
            blocker(r"C:\Users\me\AppData\Local\Temp\Foo"),
            Some(Blocker::ProtectedFolder)
        );
        for shared in [
            r"C:\Program Files (x86)\Microsoft\EdgeUpdate",
            r"C:\Program Files\WindowsApps\Foo_1.0_x64",
            r"C:\Program Files\Windows Defender",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Foo",
            r"C:\Program Files (x86)\Reference Assemblies\Foo",
            r"D:\Tools\dotnet",
        ] {
            assert_eq!(blocker(shared), Some(Blocker::ProtectedFolder), "{shared}");
        }
        assert!(gate(r"C:\Program Files\Microsoft VS Code").is_ok());
        // This app's own folder, and any folder that contains it.
        assert_eq!(
            blocker(r"C:\Program Files\PC Tweaker Uninstaller"),
            Some(Blocker::ProtectedFolder)
        );
        assert_eq!(
            blocker(r"C:\Program Files\PC Tweaker Uninstaller\bin"),
            Some(Blocker::ProtectedFolder)
        );
        // Another program still registered there, above or at it.
        assert_eq!(
            blocker(r"C:\Program Files\JetBrains"),
            Some(Blocker::OtherProgram)
        );
        assert_eq!(
            blocker(r"C:\Program Files\JetBrains\PyCharm 2026.2"),
            Some(Blocker::OtherProgram)
        );
        assert!(gate(r"C:\Program Files\JetBrains\IntelliJ IDEA 2026.2").is_ok());
    }

    /// `C:\Foo` against `C:\Foobar`: a folder is inside another only at a
    /// separator.
    #[test]
    fn folder_boundaries_are_whole_components() {
        let mut e = env();
        e.others = vec![Some(r"D:\Apps\Foo".into())];
        assert!(location_hosts_program(r"D:\Apps\Foo", &e.others));
        assert!(!location_hosts_program(r"D:\Apps\Foobar", &e.others));
        assert!(!location_hosts_program(r"D:\Apps\Fo", &e.others));
        assert!(location_hosts_program(r"D:\Apps", &e.others));
        assert!(anchor_gate(r"D:\Apps\Foobar", &e, &same).is_ok());
    }

    #[test]
    fn a_link_anywhere_on_the_way_is_refused_even_when_the_folder_is_gone() {
        // `C:\Games` is a junction to `D:\Elsewhere`: the parent resolves to
        // somewhere else, so a not-yet-existing child is refused too.
        let resolve = |p: &Path| -> Option<PathBuf> {
            let s = p.to_string_lossy().to_lowercase();
            if s == r"c:\games" {
                Some(PathBuf::from(r"\\?\D:\Elsewhere"))
            } else if s.starts_with(r"c:\games\") {
                None
            } else {
                Some(p.to_path_buf())
            }
        };
        assert!(matches!(
            anchor_gate(r"C:\Games\Gone", &env(), &resolve),
            Err(Refusal::Blocked(Blocker::Linked))
        ));
        assert!(anchor_gate(r"C:\Other\Gone", &env(), &resolve).is_ok());
        // The verbatim prefix alone is not a link.
        let verbatim = |p: &Path| Some(PathBuf::from(format!(r"\\?\{}", p.display())));
        assert!(anchor_gate(r"C:\Games\Foo", &env(), &verbatim).is_ok());
    }

    #[test]
    fn a_protected_folder_reached_through_another_spelling_is_refused() {
        let resolve = |p: &Path| -> Option<PathBuf> {
            if p.to_string_lossy().to_lowercase() == r"c:\short\pf" {
                Some(PathBuf::from(r"\\?\C:\Program Files"))
            } else {
                Some(p.to_path_buf())
            }
        };
        assert!(matches!(
            anchor_gate(r"C:\Short\PF", &env(), &resolve),
            Err(Refusal::Blocked(Blocker::ProtectedFolder))
        ));
    }
}
