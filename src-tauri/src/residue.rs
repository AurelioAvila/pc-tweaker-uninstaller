//! Residue Intelligence: what a removed program left behind.
//!
//! A scan starts from an identity (a program's name and the install folder
//! the registry recorded) and reports items, each with the evidence tying it
//! to the program, a tier (Safe is pre-selected, Review is shown, Keep is
//! never selectable) and, for Keep, the reason. See `residue_rules`.
//!
//! Safety posture, in order of importance:
//! 1. Nothing is changed without a way back saved first: files and folders
//!    go to the Recycle Bin, registry keys and values are backed up byte for
//!    byte (`residue_backup`), and one Undo in the Removal Ledger restores
//!    them without overwriting anything that came back since.
//! 2. The webview never sends a path. A scan returns a token and item ids;
//!    cleaning looks the identity up by token, scans again, and acts only on
//!    requested ids that the fresh scan also reports, so a stale or forged id
//!    does nothing.
//! 3. Only an item proven to come from the program (inside the install folder
//!    its own registry entry named) is ever pre-selected. A name match, a
//!    manually typed folder and a dangling startup entry are shown but never
//!    pre-selected.
//! 4. Free cleans files and folders; registry keys and startup entries need
//!    Pro. That is enforced here, per item kind, not only in the interface.
//! 5. Items of all users (HKLM keys, folders in Program Files and
//!    ProgramData, the all-users Start menu) are cleaned by one elevated
//!    child per cleanup, which scans again on its own and acts only on ids
//!    its own scan reports, under the fingerprint of the confirmed plan
//!    (`residue_elevated`). Items of this user never run elevated.

use crate::residue_backup::{
    capture_tree, Backup, Hive, KeyBackup, RawValue, Recycled, Registry, ValueBackup,
};
use crate::residue_rules::{
    anchor_gate, effective_blocker, item_id, registry_candidate_ok, tier, Blocker, Evidence, Facts,
    GateEnv, Kind, Refusal, Tier, Trust,
};
use crate::residue_startup::{self as startup, Disk, WinDisk};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

// ---- identity ---------------------------------------------------------------

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// Identity read from the registry by this app.
    Registry,
    /// Typed by the user.
    Manual,
    /// No identity: dead startup entries of programs already gone.
    Sweep,
}

#[derive(Clone, Debug)]
pub struct Identity {
    pub name: String,
    /// The folder that anchors the scan, as recorded (not yet gated).
    pub anchor: Option<String>,
    pub mode: Mode,
}

impl Identity {
    fn trust(&self) -> Trust {
        match self.mode {
            Mode::Registry => Trust::Registry,
            Mode::Manual => Trust::Manual,
            Mode::Sweep => Trust::None,
        }
    }
}

/// The install folder to anchor on: `InstallLocation`; failing that, a folder
/// that DisplayIcon and UninstallString both point to.
pub fn anchor_of(
    install_location: Option<&str>,
    icon_dir: Option<&str>,
    uninstall_dir: Option<&str>,
) -> Option<String> {
    let location = install_location.map(str::trim).filter(|l| !l.is_empty());
    if let Some(location) = location {
        return Some(location.to_string());
    }
    match (icon_dir, uninstall_dir) {
        (Some(a), Some(b)) if !a.trim().is_empty() && a.eq_ignore_ascii_case(b) => {
            Some(a.to_string())
        }
        _ => None,
    }
}

/// Successful removals of this session, by (source, key).
static REMOVED: Mutex<Vec<((String, String), Identity)>> = Mutex::new(Vec::new());

pub const NOT_REMOVED: &str =
    "Leftovers can be looked for only right after this app removed the program.";
pub const SCAN_GONE: &str = "This scan is no longer available. Scan again.";

fn remember_session(source: &str, id: &str, identity: Identity) {
    let key = (source.to_string(), id.to_string());
    let mut removed = REMOVED.lock().unwrap_or_else(|e| e.into_inner());
    removed.retain(|(k, _)| k != &key);
    removed.push((key, identity));
}

/// Records an entry, as read before its removal, once the removal succeeded:
/// for this session's scan, and in `removed-programs.json` so the program can
/// be scanned again after a restart.
pub fn remember_removed(source: &str, id: &str, entry: &crate::programs::RawEntry) {
    let snap = crate::removed_programs::entry_from_raw(source, entry);
    remember_session(
        source,
        id,
        Identity {
            name: snap.name.clone(),
            anchor: anchor_of(
                snap.install_location.as_deref(),
                snap.icon_dir.as_deref(),
                snap.uninstall_dir.as_deref(),
            ),
            mode: Mode::Registry,
        },
    );
    if let Ok(dir) = crate::uninstall_exec::fixed_data_dir() {
        crate::removed_programs::record_app_removal(
            &dir,
            source,
            entry,
            crate::removed_programs::now_secs(),
        );
    }
}

fn removed_identity(source: &str, id: &str) -> Result<Identity, String> {
    REMOVED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .find(|((s, i), _)| s == source && i == id)
        .map(|(_, identity)| identity.clone())
        .ok_or_else(|| NOT_REMOVED.to_string())
}

/// Scans the webview can come back to, by token. The webview holds a token,
/// never a path.
static SCANS: Mutex<Vec<(String, Identity)>> = Mutex::new(Vec::new());
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn open_scan(identity: Identity) -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(COUNTER.fetch_add(1, Ordering::Relaxed).to_le_bytes());
    hash.update(std::process::id().to_le_bytes());
    hash.update(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
            .to_le_bytes(),
    );
    let token: String = hash.finalize()[..12]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let mut scans = SCANS.lock().unwrap_or_else(|e| e.into_inner());
    scans.push((token.clone(), identity));
    if scans.len() > 16 {
        scans.remove(0);
    }
    token
}

fn scan_for(token: &str) -> Result<Identity, String> {
    SCANS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .find(|(t, _)| t == token)
        .map(|(_, identity)| identity.clone())
        .ok_or_else(|| SCAN_GONE.to_string())
}

// ---- items ------------------------------------------------------------------

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResidueItem {
    /// `SHA-256(kind, scope, location, value)`, first 8 bytes.
    pub id: String,
    pub kind: Kind,
    /// "user" or "machine".
    pub scope: &'static str,
    /// Where it is, for display only.
    pub path: String,
    /// A short name: the folder, key or value.
    pub label: String,
    pub size_kb: Option<u64>,
    /// Whether cleaning can act on it (never for Keep).
    pub deletable: bool,
    pub tier: Tier,
    pub evidence: Evidence,
    pub blocker: Option<Blocker>,
    /// The command a startup entry runs.
    pub command: Option<String>,
    /// The file that command needs, which is no longer there.
    pub missing: Option<String>,
    /// "recycle", "backup" (saved byte for byte first) or "none".
    pub restore: &'static str,
}

#[derive(Clone, Debug)]
enum Target {
    Path(PathBuf),
    /// A whole key, below its hive: `Software\Foo`, `Software\WOW6432Node\Foo`.
    Key {
        hive: Hive,
        path: String,
    },
    /// A startup value of this user; `key` is below HKCU.
    RunValue {
        key: String,
        name: String,
    },
}

#[derive(Clone, Debug)]
pub struct Found {
    pub item: ResidueItem,
    target: Target,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResidueReport {
    /// Come back with this (and item ids) to clean.
    pub token: String,
    pub mode: Mode,
    pub items: Vec<ResidueItem>,
    pub total_kb: u64,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub removed: Vec<String>,
    pub failed: Vec<String>,
    /// Pro-only items that were not touched because Pro is not active.
    pub locked: Vec<String>,
    pub freed_kb: u64,
    /// The backup id, which Undo in the Removal Ledger takes; set when
    /// anything was cleaned.
    pub stamp: Option<String>,
    /// Requested before the first change to items of all users.
    pub restore_point: Option<crate::restore_point::RestorePointOutcome>,
    /// Why the items of all users were not cleaned, when they were not.
    pub admin_refused: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub id: String,
    /// "recycle" or "backup" (backed up byte for byte, then deleted).
    pub step: &'static str,
    pub pro_only: bool,
    /// Pro-only and Pro is not active: this would be skipped.
    pub locked: bool,
    /// Not selectable, or not in a fresh scan: this would be skipped.
    pub refused: bool,
    /// An item of all users: done in the one administrator step.
    pub admin: bool,
}

/// The dry run: the steps, and what `clean_residue` must be given back.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CleanPlan {
    pub steps: Vec<PlanStep>,
    pub fingerprint: String,
    /// One administrator prompt is needed.
    pub needs_admin: bool,
    /// What System Restore will likely do first, when admin is needed.
    pub restore_point: Option<crate::restore_point::Forecast>,
}

// ---- names ------------------------------------------------------------------

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
    // Shared registry roots and generic words (the BCU failures).
    "classes",
    "policies",
    "wow6432node",
    "clients",
    "registeredapplications",
    "helper",
    "service",
    "services",
    "launcher",
    "tools",
    "driver",
    "drivers",
    "runtime",
    // Shared runtimes and Windows parts that also have `Software` keys.
    "python",
    "java",
    "javasoft",
    "odbc",
    "khronos",
    "vulkan",
    "opencl",
    "directx",
    "description",
    "partner",
    "defaultuserenvironment",
    "windowsapps",
    "windowsdefender",
    "internetexplorer",
    "microsoftnet",
    "dotnet",
    "msbuild",
    "referenceassemblies",
    "uninstallinformation",
    "packagecache",
    // This app, whose backup store is `HKLM\SOFTWARE\PC Tweaker Uninstaller`.
    "pctweakeruninstaller",
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
pub fn strip_version(name: &str) -> String {
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

/// True when `file_name` (a bare folder/file name, extension already
/// stripped for files) matches one of the candidates exactly.
pub fn matches_candidates(file_name: &str, candidates: &[String]) -> bool {
    let token = normalize(file_name);
    !token.is_empty() && candidates.contains(&token)
}

// ---- host -------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    /// `HKCU\Software`
    User,
    /// `HKLM\Software` (64-bit view)
    Machine,
    /// `HKLM\Software\WOW6432Node`
    MachineWow,
}

impl View {
    fn prefix(self) -> &'static str {
        match self {
            View::User => r"HKCU\Software\",
            View::Machine => r"HKLM\Software\",
            View::MachineWow => r"HKLM\Software\WOW6432Node\",
        }
    }
    fn key(self) -> (Hive, &'static str) {
        match self {
            View::User => (Hive::Hkcu, "Software"),
            View::Machine => (Hive::Hklm, "Software"),
            View::MachineWow => (Hive::Hklm, r"Software\WOW6432Node"),
        }
    }
}

pub struct RunValue {
    /// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` or `...\RunOnce`.
    pub key: String,
    pub name: String,
    pub data: String,
}

/// Everything a scan reads from the machine, so tests can use a fake.
pub trait Host {
    /// Folders whose program-named children are leftovers, and their kind.
    fn data_roots(&self) -> Vec<(PathBuf, Kind)>;
    fn start_menu_roots(&self) -> Vec<PathBuf>;
    /// Folders of all users: what is in them is cleaned as administrator.
    fn machine_roots(&self) -> Vec<PathBuf>;
    fn software_keys(&self, view: View) -> Vec<String>;
    fn registry(&self) -> &dyn Registry;
    fn gate_env(&self) -> GateEnv;
    fn canonical(&self, path: &Path) -> Option<PathBuf>;
    fn disk(&self) -> &dyn Disk;
    fn run_values(&self) -> Vec<RunValue>;
    fn startup_folder(&self) -> Option<PathBuf>;
    fn shortcut_target(&self, path: &Path) -> Option<String>;
    /// The Recycle Bin will really take it (and will not delete it outright).
    fn can_recycle(&self, path: &Path, size_kb: Option<u64>) -> bool;
}

/// The real machine. The elevated child sees only places of all users:
/// under over-the-shoulder UAC its own profile and HKCU are the
/// administrator's, never the user's.
pub(crate) struct Real {
    pub machine_only: bool,
}

pub(crate) const USER_SIDE: Real = Real {
    machine_only: false,
};
pub(crate) const MACHINE_SIDE: Real = Real { machine_only: true };

fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var(var).ok().map(PathBuf::from)
}

/// Folders as Windows records them (`elevation::known_folder`), never from
/// the user's environment, which reaches the elevated child too.
#[cfg(windows)]
fn known(id: &windows_sys::core::GUID) -> Option<PathBuf> {
    crate::elevation::known_folder(id)
        .filter(|p| !p.trim().is_empty())
        .map(PathBuf::from)
}

impl Host for Real {
    fn data_roots(&self) -> Vec<(PathBuf, Kind)> {
        let mut roots = Vec::new();
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Shell::{
                FOLDERID_ProgramData, FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86,
            };
            for (id, kind) in [
                (FOLDERID_ProgramFiles, Kind::InstallDir),
                (FOLDERID_ProgramFilesX86, Kind::InstallDir),
                (FOLDERID_ProgramData, Kind::AppData),
            ] {
                roots.extend(known(&id).map(|p| (p, kind)));
            }
        }
        if !self.machine_only {
            for var in ["APPDATA", "LOCALAPPDATA"] {
                roots.extend(env_path(var).map(|p| (p, Kind::AppData)));
            }
            if let Some(local) = env_path("LOCALAPPDATA") {
                roots.push((local.join("Programs"), Kind::InstallDir));
            }
        }
        roots
    }

    fn start_menu_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Shell::{FOLDERID_CommonPrograms, FOLDERID_PublicDesktop};
            roots.extend(
                [FOLDERID_CommonPrograms, FOLDERID_PublicDesktop]
                    .iter()
                    .filter_map(known),
            );
        }
        if !self.machine_only {
            if let Some(appdata) = env_path("APPDATA") {
                roots.push(appdata.join(r"Microsoft\Windows\Start Menu\Programs"));
            }
            if let Some(profile) = env_path("USERPROFILE") {
                roots.push(profile.join("Desktop"));
            }
        }
        roots
    }

    fn machine_roots(&self) -> Vec<PathBuf> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Shell::{
                FOLDERID_ProgramData, FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86,
                FOLDERID_Public,
            };
            [
                FOLDERID_ProgramFiles,
                FOLDERID_ProgramFilesX86,
                FOLDERID_ProgramData,
                FOLDERID_Public,
            ]
            .iter()
            .filter_map(known)
            .collect()
        }
        #[cfg(not(windows))]
        Vec::new()
    }

    fn software_keys(&self, view: View) -> Vec<String> {
        if self.machine_only && view == View::User {
            return Vec::new();
        }
        let (hive, path) = view.key();
        self.registry().subkeys(hive, path).unwrap_or_default()
    }

    fn registry(&self) -> &dyn Registry {
        &crate::residue_backup::WinRegistry
    }

    fn gate_env(&self) -> GateEnv {
        let others = crate::programs::list_programs()
            .ok()
            .map(|programs| programs.into_iter().map(|p| p.install_location).collect());
        GateEnv::from_process(others)
    }

    fn canonical(&self, path: &Path) -> Option<PathBuf> {
        path.canonicalize().ok()
    }

    fn disk(&self) -> &dyn Disk {
        &WinDisk
    }

    #[cfg(windows)]
    fn run_values(&self) -> Vec<RunValue> {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
        use winreg::types::FromRegValue;
        use winreg::RegKey;
        let mut out = Vec::new();
        if self.machine_only {
            return out;
        }
        for key in ["Run", "RunOnce"] {
            let sub = format!(r"Software\Microsoft\Windows\CurrentVersion\{key}");
            let Ok(reg) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(&sub, KEY_READ)
            else {
                continue;
            };
            for (name, value) in reg.enum_values().flatten() {
                if let Ok(data) = String::from_reg_value(&value) {
                    out.push(RunValue {
                        key: format!(r"HKCU\{sub}"),
                        name,
                        data,
                    });
                }
            }
        }
        out
    }

    #[cfg(not(windows))]
    fn run_values(&self) -> Vec<RunValue> {
        Vec::new()
    }

    fn startup_folder(&self) -> Option<PathBuf> {
        if self.machine_only {
            return None;
        }
        env_path("APPDATA").map(|a| a.join(r"Microsoft\Windows\Start Menu\Programs\Startup"))
    }

    fn shortcut_target(&self, path: &Path) -> Option<String> {
        startup::shortcut_target(path)
    }

    fn can_recycle(&self, path: &Path, size_kb: Option<u64>) -> bool {
        crate::recycle_bin::refusal(crate::recycle_bin::limits_for(path), size_kb).is_none()
    }
}

// ---- scanning ---------------------------------------------------------------

fn dir_size_kb(path: &Path) -> Option<u64> {
    let mut cap = 20_000u32;
    crate::uninstall_exec::dir_size_capped(path, &mut cap).map(|b| b / 1024)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

struct Draft {
    kind: Kind,
    path: PathBuf,
    size_kb: Option<u64>,
    evidence: Evidence,
    trust: Trust,
    blocker: Option<Blocker>,
    /// The install folder itself (registry-sourced) is inert by definition.
    inert: bool,
}

fn file_found(host: &dyn Host, d: Draft) -> Found {
    let facts = Facts {
        evidence: d.evidence,
        trust: d.trust,
        blocker: d.blocker,
        starts_code: false,
        restorable: host.can_recycle(&d.path, d.size_kb),
        inert: d.inert,
    };
    let tier = tier(&facts);
    let location = d.path.to_string_lossy().into_owned();
    let machine = host.machine_roots().iter().any(|root| under(&d.path, root));
    Found {
        item: ResidueItem {
            id: item_id(d.kind, machine, &location, ""),
            kind: d.kind,
            scope: if machine { "machine" } else { "user" },
            label: file_name(&d.path),
            path: location,
            size_kb: d.size_kb,
            deletable: tier != Tier::Keep,
            tier,
            evidence: d.evidence,
            blocker: effective_blocker(&facts),
            command: None,
            missing: None,
            restore: "recycle",
        },
        target: Target::Path(d.path),
    }
}

/// Strictly inside `root`, compared as Windows does (case-insensitively).
pub(crate) fn under(path: &Path, root: &Path) -> bool {
    let root = root.to_string_lossy().to_lowercase();
    let root = root.trim_end_matches('\\');
    !root.is_empty()
        && path
            .to_string_lossy()
            .to_lowercase()
            .starts_with(&format!("{root}\\"))
}

/// True when `text` names `dir` or something inside it, as a whole path
/// (`C:\Foo` is not in `C:\Foobar`).
fn mentions(text: &str, dir: &str) -> bool {
    let text = text.to_lowercase().replace('/', "\\");
    let dir = dir.to_lowercase();
    let dir = dir.trim_end_matches('\\');
    dir.len() > 3
        && text.match_indices(dir).any(|(at, _)| {
            let before = at.checked_sub(1).map(|i| text.as_bytes()[i]);
            let after = text.as_bytes().get(at + dir.len()).copied();
            matches!(before, None | Some(b'"' | b' ' | b'=' | b',' | b';' | b'@'))
                && matches!(after, None | Some(b'\\' | b'"' | b' ' | b',' | b';'))
        })
}

/// A program-named key below `Software`. A value that names the install
/// folder is proof (Path); a value that names a program still registered
/// keeps the key; a tree that cannot be read whole cannot be backed up, so
/// it is kept too.
fn registry_found(
    host: &dyn Host,
    view: View,
    name: &str,
    trust: Trust,
    anchor: Option<&Path>,
    env: &GateEnv,
) -> Found {
    let (hive, root) = view.key();
    let key = format!(r"{root}\{name}");
    let shown = format!("{}{name}", view.prefix());
    let nodes = capture_tree(host.registry(), hive, &key);
    let texts: Vec<String> = nodes
        .iter()
        .flatten()
        .flat_map(|n| &n.values)
        .flat_map(RawValue::strings)
        .collect();
    let names = |dir: &str| texts.iter().any(|t| mentions(t, dir));
    let user = view == View::User;
    let kind = if user {
        Kind::RegistryUser
    } else {
        Kind::RegistryMachine
    };
    let evidence = if anchor.is_some_and(|d| names(d.to_string_lossy().as_ref())) {
        Evidence::Path
    } else {
        Evidence::Name
    };
    let facts = Facts {
        evidence,
        trust,
        blocker: (env.programs_unknown || env.others.iter().flatten().any(|o| names(o.as_str())))
            .then_some(Blocker::OtherProgram),
        starts_code: false,
        restorable: nodes.is_some(),
        inert: true,
    };
    let tier = tier(&facts);
    Found {
        item: ResidueItem {
            id: item_id(kind, !user, &shown, ""),
            kind,
            scope: if user { "user" } else { "machine" },
            label: name.to_string(),
            path: shown,
            size_kb: None,
            deletable: tier != Tier::Keep,
            tier,
            evidence,
            blocker: effective_blocker(&facts),
            command: None,
            missing: None,
            restore: if nodes.is_some() { "backup" } else { "none" },
        },
        target: Target::Key { hive, path: key },
    }
}

fn scan_registry(
    host: &dyn Host,
    candidates: &[String],
    trust: Trust,
    anchor: Option<&Path>,
    env: &GateEnv,
    out: &mut Vec<Found>,
) {
    for view in [View::User, View::Machine, View::MachineWow] {
        for name in host.software_keys(view) {
            if matches_candidates(&name, candidates)
                && registry_candidate_ok(&format!("{}{name}", view.prefix()))
            {
                out.push(registry_found(host, view, &name, trust, anchor, env));
            }
        }
    }
}

/// Dead startup entries of programs already gone: user Run and RunOnce
/// values, and user Startup shortcuts, whose program is provably missing.
fn sweep_items(host: &dyn Host) -> Vec<Found> {
    let mut out = Vec::new();
    for value in host.run_values() {
        let Some(missing) = startup::missing_target(&value.data, host.disk()) else {
            continue;
        };
        out.push(Found {
            item: ResidueItem {
                id: item_id(Kind::StartupRun, false, &value.key, &value.name),
                kind: Kind::StartupRun,
                scope: "user",
                path: value.key.clone(),
                label: value.name.clone(),
                size_kb: None,
                deletable: true,
                tier: Tier::Review,
                evidence: Evidence::Dangling,
                blocker: None,
                command: Some(value.data.clone()),
                missing: Some(missing.to_string_lossy().into_owned()),
                restore: "backup",
            },
            target: Target::RunValue {
                key: value
                    .key
                    .strip_prefix(r"HKCU\")
                    .unwrap_or(&value.key)
                    .to_string(),
                name: value.name,
            },
        });
    }
    if let Some(folder) = host.startup_folder() {
        for entry in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
            let path = entry.path();
            let is_lnk = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("lnk"));
            if !is_lnk || !path.is_file() {
                continue;
            }
            let Some(target) = host.shortcut_target(&path) else {
                continue;
            };
            let Some(missing) = startup::missing_target(&target, host.disk()) else {
                continue;
            };
            let mut found = file_found(
                host,
                Draft {
                    kind: Kind::StartupShortcut,
                    path: path.clone(),
                    size_kb: Some(1),
                    evidence: Evidence::Dangling,
                    trust: Trust::None,
                    blocker: None,
                    inert: true,
                },
            );
            found.item.label = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            found.item.command = Some(target);
            found.item.missing = Some(missing.to_string_lossy().into_owned());
            out.push(found);
        }
    }
    out
}

pub(crate) fn scan_items(identity: &Identity, host: &dyn Host) -> Vec<Found> {
    if identity.mode == Mode::Sweep {
        return sweep_items(host);
    }
    let trust = identity.trust();
    let env = host.gate_env();
    let mut out: Vec<Found> = Vec::new();
    let mut anchor: Option<PathBuf> = None;

    if let Some(raw) = identity.anchor.as_deref() {
        let canonical = |p: &Path| host.canonical(p);
        match anchor_gate(raw, &env, &canonical) {
            Ok(dir) => {
                if dir.is_dir() {
                    let size_kb = dir_size_kb(&dir);
                    out.push(file_found(
                        host,
                        Draft {
                            kind: Kind::InstallDir,
                            path: dir.clone(),
                            size_kb,
                            evidence: Evidence::Path,
                            trust,
                            blocker: None,
                            inert: true,
                        },
                    ));
                }
                anchor = Some(dir);
            }
            // The folder exists but is not ours to clean: say why.
            Err(Refusal::Blocked(blocker)) => {
                let shown = PathBuf::from(
                    raw.trim()
                        .trim_matches('"')
                        .replace('/', "\\")
                        .trim_end_matches('\\'),
                );
                if shown.is_dir() {
                    out.push(file_found(
                        host,
                        Draft {
                            kind: Kind::InstallDir,
                            path: shown,
                            size_kb: None,
                            evidence: Evidence::Path,
                            trust,
                            blocker: Some(blocker),
                            inert: true,
                        },
                    ));
                }
            }
            Err(Refusal::Shape) => {}
        }
    }

    let candidates = name_candidates(&identity.name);
    if !candidates.is_empty() {
        let anchor_lower = anchor.as_ref().map(|a| a.to_string_lossy().to_lowercase());
        let push_dir = |kind: Kind, path: PathBuf, out: &mut Vec<Found>| {
            if !path.is_dir()
                || crate::residue_rules::shared_folder(&file_name(&path))
                || anchor_lower.as_deref() == Some(path.to_string_lossy().to_lowercase().as_str())
            {
                return;
            }
            // A folder that is, or holds, a program still registered stays.
            let blocker = (env.programs_unknown
                || crate::residue_rules::location_hosts_program(
                    &path.to_string_lossy(),
                    &env.others,
                ))
            .then_some(Blocker::OtherProgram);
            let size_kb = dir_size_kb(&path);
            out.push(file_found(
                host,
                Draft {
                    kind,
                    path,
                    size_kb,
                    evidence: Evidence::Name,
                    trust,
                    blocker,
                    inert: true,
                },
            ));
        };
        for (root, kind) in host.data_roots() {
            for entry in std::fs::read_dir(&root).into_iter().flatten().flatten() {
                if matches_candidates(&entry.file_name().to_string_lossy(), &candidates) {
                    push_dir(kind, entry.path(), &mut out);
                }
            }
        }
        for root in host.start_menu_roots() {
            for entry in std::fs::read_dir(&root).into_iter().flatten().flatten() {
                let path = entry.path();
                let raw = entry.file_name().to_string_lossy().into_owned();
                let stem = raw.strip_suffix(".lnk").unwrap_or(&raw).to_string();
                if !matches_candidates(&stem, &candidates) {
                    continue;
                }
                if path.is_dir() {
                    push_dir(Kind::Shortcut, path, &mut out);
                } else if raw.to_ascii_lowercase().ends_with(".lnk") {
                    out.push(file_found(
                        host,
                        Draft {
                            kind: Kind::Shortcut,
                            path,
                            size_kb: Some(1),
                            evidence: Evidence::Name,
                            trust,
                            blocker: None,
                            inert: true,
                        },
                    ));
                }
            }
        }
        scan_registry(host, &candidates, trust, anchor.as_deref(), &env, &mut out);
    }

    let mut seen = std::collections::HashSet::new();
    out.retain(|f| seen.insert(f.item.id.clone()));
    out
}

fn report(identity: &Identity, found: &[Found]) -> ResidueReport {
    let items: Vec<ResidueItem> = found.iter().map(|f| f.item.clone()).collect();
    let total_kb = items
        .iter()
        .filter(|i| i.tier != Tier::Keep)
        .filter_map(|i| i.size_kb)
        .sum();
    ResidueReport {
        token: open_scan(identity.clone()),
        mode: identity.mode,
        items,
        total_kb,
    }
}

// ---- manual scan --------------------------------------------------------------

pub const MANUAL_EMPTY: &str = "Type a program name, a folder, or both.";
pub const MANUAL_NAME: &str =
    "That name is too short or too common to search for. Use the full name of the program.";
pub const MANUAL_SHAPE: &str =
    "Type the folder as a full path that starts with a drive letter, such as C:.";
pub const MANUAL_GONE: &str = "That folder does not exist, so there is nothing to scan in it.";
pub const MANUAL_PROTECTED: &str =
    "That folder is a protected or shared location, so it is never offered for cleanup.";
pub const MANUAL_OTHER: &str =
    "Another installed program is in that folder, so it is never offered for cleanup.";
pub const MANUAL_LINKED: &str =
    "That folder is reached through a link, so it is never offered for cleanup.";

/// The identity a typed name and/or folder stands for, or why not.
fn manual_identity(name: &str, folder: &str, host: &dyn Host) -> Result<Identity, String> {
    let (name, folder) = (name.trim(), folder.trim());
    if name.is_empty() && folder.is_empty() {
        return Err(MANUAL_EMPTY.into());
    }
    if name.chars().count() > 120
        || folder.chars().count() > 260
        || name.chars().chain(folder.chars()).any(char::is_control)
    {
        return Err(MANUAL_SHAPE.into());
    }
    if !name.is_empty() && name_candidates(name).is_empty() {
        return Err(MANUAL_NAME.into());
    }
    if !folder.is_empty() {
        let env = host.gate_env();
        match anchor_gate(folder, &env, &|p| host.canonical(p)) {
            Ok(dir) if dir.is_dir() => {}
            Ok(_) => return Err(MANUAL_GONE.into()),
            Err(Refusal::Shape) => return Err(MANUAL_SHAPE.into()),
            Err(Refusal::Blocked(Blocker::OtherProgram)) => return Err(MANUAL_OTHER.into()),
            Err(Refusal::Blocked(Blocker::Linked)) => return Err(MANUAL_LINKED.into()),
            Err(Refusal::Blocked(_)) => return Err(MANUAL_PROTECTED.into()),
        }
    }
    Ok(Identity {
        name: name.to_string(),
        anchor: (!folder.is_empty()).then(|| folder.to_string()),
        mode: Mode::Manual,
    })
}

// ---- cleaning -----------------------------------------------------------------

/// Decides what happens to each requested id, given a fresh scan: an id the
/// scan does not report and a Keep item fail; a Pro-only kind without Pro is
/// left alone and reported as locked. Returns the items that may proceed,
/// each once.
fn select<'a>(
    found: &'a [Found],
    ids: &[String],
    pro: bool,
    result: &mut CleanResult,
) -> Vec<&'a Found> {
    let mut out: Vec<&Found> = Vec::new();
    for id in ids {
        match found.iter().find(|f| &f.item.id == id) {
            Some(f) if f.item.tier == Tier::Keep || !f.item.deletable => {
                result.failed.push(id.clone())
            }
            Some(f) if f.item.kind.pro_only() && !pro => result.locked.push(id.clone()),
            Some(f) => {
                if !out.iter().any(|o| o.item.id == f.item.id) {
                    out.push(f);
                }
            }
            None => result.failed.push(id.clone()),
        }
    }
    out
}

/// What acting needs from the machine, so the order and the backups are
/// testable. The parent acts for this user; the elevated child for all
/// users, with its backup in the HKLM store.
pub(crate) trait Actor {
    fn registry(&self) -> &dyn Registry;
    /// Sends a file or folder to the Recycle Bin: the KB freed, or `None`.
    fn recycle(&self, path: &Path) -> Option<u64>;
    /// `fresh`: the first save of this cleanup.
    fn save(&self, backup: &Backup, fresh: bool) -> Result<(), String>;
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn run_value(reg: &dyn Registry, key: &str, name: &str) -> Option<RawValue> {
    reg.values(Hive::Hkcu, key)?
        .into_iter()
        .find(|v| v.name.eq_ignore_ascii_case(name))
}

/// Registry first, then shortcuts, then folders: a folder a shortcut points
/// into goes last.
fn order(f: &Found) -> u8 {
    match (&f.target, f.item.kind) {
        (Target::Path(_), Kind::Shortcut | Kind::StartupShortcut) => 1,
        (Target::Path(_), _) => 2,
        _ => 0,
    }
}

/// Backs up, then acts. Nothing changes unless its backup was saved first,
/// and a key or value that changed after its backup is left alone. The
/// backup ends up holding exactly what was removed.
pub(crate) fn apply(
    items: &[&Found],
    backup: &mut Backup,
    actor: &dyn Actor,
    result: &mut CleanResult,
) {
    let reg = actor.registry();
    let start = now_secs();
    let mut ready: Vec<&Found> = Vec::new();
    for &f in items {
        let id = f.item.id.clone();
        let saved = match &f.target {
            Target::Key { hive, path } => capture_tree(reg, *hive, path).map(|nodes| {
                backup.keys.push(KeyBackup {
                    id: id.clone(),
                    hive: *hive,
                    path: path.clone(),
                    nodes,
                })
            }),
            Target::RunValue { key, name } => run_value(reg, key, name).map(|value| {
                backup.values.push(ValueBackup {
                    id: id.clone(),
                    hive: Hive::Hkcu,
                    key: key.clone(),
                    value,
                })
            }),
            // Until the final save: a window wide enough for the whole run.
            Target::Path(path) => {
                backup.recycled.push(Recycled {
                    id: id.clone(),
                    path: path.to_string_lossy().into_owned(),
                    from: start,
                    to: start + 3_600,
                });
                Some(())
            }
        };
        match saved {
            Some(()) => ready.push(f),
            None => result.failed.push(id),
        }
    }
    if ready.is_empty() {
        return;
    }
    if let Err(e) = actor.save(backup, true) {
        crate::applog::line(&format!("leftover backup not saved, nothing changed: {e}"));
        result
            .failed
            .extend(ready.iter().map(|f| f.item.id.clone()));
        *backup = Backup::new(&backup.stamp, &backup.program);
        return;
    }
    ready.sort_by_key(|f| order(f));
    let mut done: Vec<String> = Vec::new();
    for f in ready {
        let id = &f.item.id;
        let freed = match &f.target {
            Target::Key { hive, path } => {
                let saved = backup.keys.iter().find(|k| &k.id == id).map(|k| &k.nodes);
                (capture_tree(reg, *hive, path).as_ref() == saved
                    && reg.delete_tree(*hive, path).is_ok())
                .then_some(0)
            }
            Target::RunValue { key, name } => {
                let saved = backup.values.iter().find(|v| &v.id == id).map(|v| &v.value);
                (run_value(reg, key, name).as_ref() == saved
                    && reg.delete_value(Hive::Hkcu, key, name).is_ok())
                .then_some(0)
            }
            Target::Path(path) => {
                let from = now_secs();
                let freed = actor.recycle(path);
                if let Some(rec) = backup.recycled.iter_mut().find(|r| &r.id == id) {
                    (rec.from, rec.to) = (from, now_secs());
                }
                freed
            }
        };
        match freed {
            Some(kb) => {
                result.freed_kb += kb;
                result.removed.push(id.clone());
                done.push(id.clone());
            }
            None => result.failed.push(id.clone()),
        }
    }
    backup.keys.retain(|k| done.contains(&k.id));
    backup.values.retain(|v| done.contains(&v.id));
    backup.recycled.retain(|r| done.contains(&r.id));
    if let Err(e) = actor.save(backup, false) {
        // The first save still covers everything removed.
        crate::applog::line(&format!("leftover backup not updated: {e}"));
    }
}

/// Moves a file or folder to the Recycle Bin, unless Windows would delete it
/// outright instead. Returns the KB freed.
pub(crate) fn recycle_real(path: &Path) -> Option<u64> {
    let size = crate::recycle_bin::item_size_kb(path);
    if let Some(reason) = crate::recycle_bin::refusal(crate::recycle_bin::limits_for(path), size) {
        crate::applog::line(&format!(
            "residue not recycled: {} ({reason})",
            path.to_string_lossy()
        ));
        return None;
    }
    match trash::delete(path) {
        Ok(()) => Some(size.unwrap_or(0)),
        Err(e) => {
            // Cleanup is the destructive step, so a refusal is the one thing
            // a support request most needs to show.
            crate::applog::line(&format!(
                "residue not recycled: {} ({e})",
                path.to_string_lossy()
            ));
            None
        }
    }
}

/// This user's side: backups in the app's data folder.
struct UserActor {
    dir: PathBuf,
}

impl Actor for UserActor {
    fn registry(&self) -> &dyn Registry {
        &crate::residue_backup::WinRegistry
    }
    fn recycle(&self, path: &Path) -> Option<u64> {
        recycle_real(path)
    }
    fn save(&self, backup: &Backup, _fresh: bool) -> Result<(), String> {
        crate::residue_backup::save_user(&self.dir, backup)
    }
}

// ---- commands -----------------------------------------------------------------

fn finish(identity: Identity, host: &dyn Host) -> ResidueReport {
    let found = scan_items(&identity, host);
    report(&identity, &found)
}

/// Scans for leftovers of a program this app just removed, named by its
/// entry (source and key). The install folder the registry reported before
/// the removal, if it still exists, is the highest-confidence residue there
/// is.
#[tauri::command(async)]
pub fn scan_residue(source: String, id: String) -> Result<ResidueReport, String> {
    Ok(finish(removed_identity(&source, &id)?, &USER_SIDE))
}

/// Scans for leftovers of an entry in the removed-programs list.
#[tauri::command(async)]
pub fn scan_removed_program(rid: String) -> Result<ResidueReport, String> {
    let dir = crate::uninstall_exec::fixed_data_dir()?;
    let record = crate::removed_programs::find(&dir, &rid).ok_or(SCAN_GONE)?;
    crate::removed_programs::mark_seen(&dir, &rid);
    Ok(finish(
        Identity {
            name: record.name.clone(),
            anchor: anchor_of(
                record.install_location.as_deref(),
                record.icon_dir.as_deref(),
                record.uninstall_dir.as_deref(),
            ),
            mode: Mode::Registry,
        },
        &USER_SIDE,
    ))
}

/// Scans for leftovers of a program the user names: a name, a folder, or both.
/// Validated here; nothing in the result is ever pre-selected.
#[tauri::command(async)]
pub fn scan_manual(name: String, folder: String) -> Result<ResidueReport, String> {
    let identity = manual_identity(&name, &folder, &USER_SIDE)?;
    Ok(finish(identity, &USER_SIDE))
}

/// Dead startup entries of programs already gone (the first-launch sweep).
#[tauri::command(async)]
pub fn scan_startup_sweep() -> Result<ResidueReport, String> {
    if let Ok(dir) = crate::uninstall_exec::fixed_data_dir() {
        crate::removed_programs::finish_sweep(&dir);
    }
    Ok(finish(
        Identity {
            name: String::new(),
            anchor: None,
            mode: Mode::Sweep,
        },
        &USER_SIDE,
    ))
}

/// How many dead startup entries there are right now.
pub fn sweep_count() -> usize {
    sweep_items(&USER_SIDE).len()
}

fn is_pro(app: &tauri::AppHandle) -> bool {
    crate::license::license_status(app.clone()).unwrap_or(false)
}

fn plan_with(found: &[Found], ids: &[String], pro: bool) -> Vec<PlanStep> {
    ids.iter()
        .map(|id| {
            let f = found.iter().find(|f| &f.item.id == id);
            let kind = f.map(|f| f.item.kind);
            PlanStep {
                id: id.clone(),
                step: match f.map(|f| &f.target) {
                    Some(Target::Key { .. } | Target::RunValue { .. }) => "backup",
                    _ => "recycle",
                },
                pro_only: kind.is_some_and(Kind::pro_only),
                locked: kind.is_some_and(|k| k.pro_only() && !pro),
                refused: f.is_none_or(|f| f.item.tier == Tier::Keep || !f.item.deletable),
                admin: f.is_some_and(|f| f.item.scope == "machine"),
            }
        })
        .collect()
}

/// SHA-256 over the identity and what would happen to each id. The parent
/// compares it with the confirmed plan, and the elevated child with the
/// machine part of it, each after scanning again.
pub(crate) fn plan_fingerprint(identity: &Identity, steps: &[PlanStep]) -> String {
    use sha2::{Digest, Sha256};
    let mut pairs: Vec<(&str, &str)> = steps
        .iter()
        .map(|s| {
            let what = if s.refused {
                "refused"
            } else if s.locked {
                "locked"
            } else {
                s.step
            };
            (s.id.as_str(), what)
        })
        .collect();
    pairs.sort_unstable();
    pairs.dedup();
    let material = serde_json::json!([identity.name, identity.anchor, identity.mode, pairs]);
    format!("{:x}", Sha256::digest(material.to_string()))
}

pub const PLAN_CHANGED: &str =
    "The leftovers changed after the preview. Preview the cleanup again.";
pub const TOO_MANY: &str = "Too many items in one cleanup.";

/// The machine part of a plan: what the elevated child must find too.
pub(crate) fn machine_fingerprint(identity: &Identity, found: &[Found], ids: &[String]) -> String {
    plan_fingerprint(identity, &plan_with(found, ids, true))
}

/// The items the elevated child may act on: only its own fresh scan's, and
/// only those of all users.
pub(crate) fn machine_select<'a>(
    found: &'a [Found],
    ids: &[String],
    result: &mut CleanResult,
) -> Vec<&'a Found> {
    let (machine, user): (Vec<&Found>, Vec<&Found>) = select(found, ids, true, result)
        .into_iter()
        .partition(|f| f.item.scope == "machine");
    result
        .failed
        .extend(user.into_iter().map(|f| f.item.id.clone()));
    machine
}

/// Dry run: what cleaning these items would do, changing nothing.
#[tauri::command(async)]
pub fn plan_residue_clean(
    app: tauri::AppHandle,
    token: String,
    item_ids: Vec<String>,
) -> Result<CleanPlan, String> {
    if item_ids.len() > 64 {
        return Err(TOO_MANY.into());
    }
    let identity = scan_for(&token)?;
    let found = scan_items(&identity, &USER_SIDE);
    let steps = plan_with(&found, &item_ids, is_pro(&app));
    let needs_admin = steps.iter().any(|s| s.admin && !s.refused && !s.locked);
    Ok(CleanPlan {
        fingerprint: plan_fingerprint(&identity, &steps),
        needs_admin,
        restore_point: needs_admin.then(crate::restore_point::restore_point_forecast),
        steps,
    })
}

/// Cleans the selected items of a scan, as previewed: files and folders go
/// to the Recycle Bin (free); registry keys and startup entries are backed
/// up byte for byte and deleted (Pro). Items of all users run in one
/// elevated child, first; this user's run here. The webview names the scan
/// by token, the items by id, and the plan by its fingerprint; everything is
/// checked against a fresh scan first.
#[tauri::command(async)]
pub fn clean_residue(
    app: tauri::AppHandle,
    token: String,
    item_ids: Vec<String>,
    fingerprint: String,
) -> Result<CleanResult, String> {
    if item_ids.len() > 64 {
        return Err(TOO_MANY.into());
    }
    let _guard = crate::uninstall_exec::lock_exec()?;
    let identity = scan_for(&token)?;
    let found = scan_items(&identity, &USER_SIDE);
    let pro = is_pro(&app);
    if plan_fingerprint(&identity, &plan_with(&found, &item_ids, pro)) != fingerprint {
        return Err(PLAN_CHANGED.into());
    }
    let dir = crate::residue_backup::user_dir()?;
    let mut result = CleanResult::default();
    let (machine, user): (Vec<&Found>, Vec<&Found>) = select(&found, &item_ids, pro, &mut result)
        .into_iter()
        .partition(|f| f.item.scope == "machine");
    let stamp = crate::residue_backup::new_stamp();
    let program = if identity.name.is_empty() {
        "Startup entries".to_string()
    } else {
        identity.name.clone()
    };

    let mut machine_done = 0;
    if !machine.is_empty() {
        let ids: Vec<String> = machine.iter().map(|f| f.item.id.clone()).collect();
        let confirmed = machine_fingerprint(&identity, &found, &ids);
        match crate::residue_elevated::clean_machine(&identity, &ids, &confirmed, &stamp) {
            Ok(report) => {
                machine_done = report.removed.len();
                result.removed.extend(report.removed);
                result.failed.extend(report.failed);
                result.freed_kb += report.freed_kb;
                result.restore_point = report.restore_point;
                result.admin_refused = report.refused;
            }
            Err(e) => {
                // A child that acted but lost its report still left its
                // backup: the receipt keeps the Undo for it.
                if let Ok(Some(kept)) = crate::residue_backup::STORE.load(&stamp) {
                    machine_done = kept.keys.len() + kept.values.len() + kept.recycled.len();
                }
                result.failed.extend(ids);
                result.admin_refused = Some(e);
            }
        }
    }

    let before = result.removed.len();
    let mut backup = Backup::new(&stamp, &program);
    apply(
        &user,
        &mut backup,
        &UserActor { dir: dir.clone() },
        &mut result,
    );
    let user_done = result.removed.len() - before;
    if machine_done > 0 {
        // A copy for people to read; undo never reads it.
        if let Ok(Some(copy)) = crate::residue_backup::STORE.load(&stamp) {
            let _ = crate::residue_backup::save_readable_copy(&dir, &copy);
        }
    }
    if machine_done + user_done > 0 {
        result.stamp = Some(stamp.clone());
        crate::ledger::record_leftovers(crate::ledger::LeftoversDone {
            program: &program,
            stamp: &stamp,
            user_items: user_done,
            machine_items: machine_done,
            freed_kb: result.freed_kb,
            restore_point: result.restore_point.clone(),
        });
    }
    crate::residue_backup::prune_user(&dir, crate::residue_backup::KEEP);
    crate::applog::line(&format!(
        "residue cleanup for {program}: {} removed ({machine_done} as administrator), \
         {} failed, {} locked, {} KB freed",
        result.removed.len(),
        result.failed.len(),
        result.locked.len(),
        result.freed_kb
    ));
    Ok(result)
}

/// One Undo for one cleanup, always free: this user's items are put back
/// here, the items of all users by one elevated child that reads only the
/// HKLM store. Nothing that exists again is overwritten.
#[tauri::command(async)]
pub fn undo_leftovers(stamp: String) -> Result<crate::residue_elevated::UndoReport, String> {
    use crate::residue_backup::{self as rb, Outcome};
    if !rb::valid_stamp(&stamp) {
        return Err(crate::residue_elevated::UNDO_GONE.into());
    }
    let _guard = crate::uninstall_exec::lock_exec()?;
    let dir = rb::user_dir()?;
    let mut report = crate::residue_elevated::UndoReport::default();
    let user = rb::load_user(&dir, &stamp)?;
    let machine = rb::STORE.stamps().contains(&stamp);
    if user.is_none() && !machine {
        return Err(crate::residue_elevated::UNDO_GONE.into());
    }
    if let Some(backup) = user {
        report.add(rb::restore_recycled(
            &rb::WinBin::new(),
            &backup.recycled,
            &|_| true,
        ));
        for value in backup.values.iter().rev() {
            report.add([(value.id.clone(), rb::restore_value(&rb::WinRegistry, value))]);
        }
        for key in backup.keys.iter().rev() {
            let outcome = if key.hive == Hive::Hkcu {
                rb::restore_key(&rb::WinRegistry, key)
            } else {
                Outcome::Refused
            };
            report.add([(key.id.clone(), outcome)]);
        }
    }
    if machine {
        match crate::residue_elevated::undo_machine(&stamp) {
            Ok(child) => {
                report.outcomes.extend(child.outcomes);
                report.refused = child.refused;
            }
            Err(e) => report.refused = Some(e),
        }
    }
    if report.refused.is_none() {
        crate::ledger::mark_leftovers_undone(&stamp);
    }
    crate::applog::line(&format!(
        "leftover undo {stamp}: {} items, refused: {}",
        report.outcomes.len(),
        report.refused.is_some()
    ));
    Ok(report)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::residue_backup::tests::{sz, FakeRegistry};
    use std::collections::HashSet;

    pub(crate) struct FakeDisk {
        present: Vec<String>,
    }
    impl Disk for FakeDisk {
        fn exists(&self, path: &Path) -> bool {
            self.present
                .iter()
                .any(|p| p.eq_ignore_ascii_case(&path.to_string_lossy()))
        }
        fn fixed_drive(&self, letter: char) -> bool {
            letter == 'C'
        }
    }

    pub(crate) struct Fake {
        pub data: Vec<(PathBuf, Kind)>,
        pub menu: Vec<PathBuf>,
        pub machine: Vec<PathBuf>,
        pub reg: FakeRegistry,
        pub env: GateEnv,
        pub runs: Vec<(String, String)>,
        pub startup: Option<PathBuf>,
        pub targets: Vec<(String, String)>,
        pub disk: FakeDisk,
        pub recycle: bool,
    }
    impl Fake {
        /// Program keys, empty, below `Software` of each view.
        pub fn set_keys(&mut self, keys: Vec<(View, Vec<String>)>) {
            self.reg = FakeRegistry::default();
            for (view, names) in keys {
                let (hive, root) = view.key();
                for name in names {
                    self.reg.key(hive, &format!(r"{root}\{name}"), Vec::new());
                }
            }
        }
    }
    impl Host for Fake {
        fn data_roots(&self) -> Vec<(PathBuf, Kind)> {
            self.data.clone()
        }
        fn start_menu_roots(&self) -> Vec<PathBuf> {
            self.menu.clone()
        }
        fn machine_roots(&self) -> Vec<PathBuf> {
            self.machine.clone()
        }
        fn software_keys(&self, view: View) -> Vec<String> {
            let (hive, root) = view.key();
            self.reg.subkeys(hive, root).unwrap_or_default()
        }
        fn registry(&self) -> &dyn Registry {
            &self.reg
        }
        fn gate_env(&self) -> GateEnv {
            self.env.clone()
        }
        fn canonical(&self, path: &Path) -> Option<PathBuf> {
            path.canonicalize().ok()
        }
        fn disk(&self) -> &dyn Disk {
            &self.disk
        }
        fn run_values(&self) -> Vec<RunValue> {
            self.runs
                .iter()
                .map(|(name, data)| RunValue {
                    key: r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run".into(),
                    name: name.clone(),
                    data: data.clone(),
                })
                .collect()
        }
        fn startup_folder(&self) -> Option<PathBuf> {
            self.startup.clone()
        }
        fn shortcut_target(&self, path: &Path) -> Option<String> {
            self.targets
                .iter()
                .find(|(file, _)| {
                    path.file_name()
                        .is_some_and(|n| n.to_string_lossy() == *file)
                })
                .map(|(_, target)| target.clone())
        }
        fn can_recycle(&self, _path: &Path, _size_kb: Option<u64>) -> bool {
            self.recycle
        }
    }

    /// A sandbox: real folders under the temp dir, the rest faked.
    pub(crate) struct Sandbox {
        pub root: PathBuf,
    }
    impl Drop for Sandbox {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).ok();
        }
    }
    pub(crate) fn sandbox(tag: &str) -> (Sandbox, Fake) {
        // The long form of the temporary folder: %TEMP% may be an 8.3 name,
        // which does not resolve to itself and would be refused.
        let temp = std::env::temp_dir().canonicalize().unwrap();
        let temp = temp.to_string_lossy();
        let root = Path::new(temp.strip_prefix(r"\\?\").unwrap_or(&temp))
            .join(format!("pcu-residue-{tag}-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        for dir in ["apps", "roaming", "menu", "startup"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        let host = Fake {
            data: vec![(root.join("roaming"), Kind::AppData)],
            menu: vec![root.join("menu")],
            machine: Vec::new(),
            reg: FakeRegistry::default(),
            env: GateEnv {
                windows: vec![r"c:\windows".into()],
                protected: vec![r"c:\program files".into()],
                shared_roots: Vec::new(),
                own: None,
                others: Vec::new(),
                programs_unknown: false,
            },
            runs: Vec::new(),
            startup: Some(root.join("startup")),
            targets: Vec::new(),
            disk: FakeDisk {
                present: Vec::new(),
            },
            recycle: true,
        };
        (Sandbox { root }, host)
    }
    fn mk(path: PathBuf) -> PathBuf {
        std::fs::create_dir_all(path.join("data")).unwrap();
        std::fs::write(path.join("data").join("f.bin"), vec![0u8; 2048]).unwrap();
        path
    }
    fn identity(name: &str, anchor: Option<&Path>, mode: Mode) -> Identity {
        Identity {
            name: name.into(),
            anchor: anchor.map(|a| a.to_string_lossy().into_owned()),
            mode,
        }
    }
    fn by_kind(found: &[Found], kind: Kind) -> Vec<&ResidueItem> {
        found
            .iter()
            .map(|f| &f.item)
            .filter(|i| i.kind == kind)
            .collect()
    }

    #[test]
    fn the_registry_install_folder_is_safe_and_a_name_match_is_only_review() {
        let (sb, mut host) = sandbox("safe");
        let install = mk(sb.root.join("apps").join("SuperTool"));
        mk(sb.root.join("roaming").join("SuperTool"));
        std::fs::write(sb.root.join("menu").join("SuperTool.lnk"), b"x").unwrap();
        host.set_keys(vec![
            (View::User, vec!["SuperTool".into(), "Classes".into()]),
            (View::Machine, vec!["SuperTool".into()]),
            (View::MachineWow, vec!["SuperTool".into()]),
        ]);
        let found = scan_items(
            &identity("SuperTool 2.1", Some(&install), Mode::Registry),
            &host,
        );

        let dir = by_kind(&found, Kind::InstallDir);
        assert_eq!(dir.len(), 1);
        assert_eq!(dir[0].tier, Tier::Safe);
        assert_eq!(dir[0].evidence, Evidence::Path);
        assert!(dir[0].deletable && dir[0].blocker.is_none());
        assert!(dir[0].size_kb.is_some());

        // Name matches are shown, labelled "name", and never Safe.
        for item in found
            .iter()
            .map(|f| &f.item)
            .filter(|i| i.kind != Kind::InstallDir)
        {
            assert_eq!(item.evidence, Evidence::Name, "{}", item.path);
            assert_eq!(item.tier, Tier::Review, "{}", item.path);
        }
        assert_eq!(by_kind(&found, Kind::AppData).len(), 1);
        assert_eq!(by_kind(&found, Kind::Shortcut).len(), 1);
        // WOW6432Node is reported; HKLM is never deletable; `Classes` never appears.
        let paths: Vec<&str> = found.iter().map(|f| f.item.path.as_str()).collect();
        assert!(paths.contains(&r"HKLM\Software\WOW6432Node\SuperTool"));
        assert!(paths.contains(&r"HKLM\Software\SuperTool"));
        assert!(paths.contains(&r"HKCU\Software\SuperTool"));
        assert!(!paths.iter().any(|p| p.to_lowercase().contains("classes")));
        // HKLM keys are cleaned as administrator, never pre-selected by name.
        assert!(by_kind(&found, Kind::RegistryMachine)
            .iter()
            .all(|i| i.deletable && i.scope == "machine" && i.restore == "backup"));
        assert!(by_kind(&found, Kind::RegistryUser)
            .iter()
            .all(|i| i.deletable));
        // Ids are unique 16-digit hashes.
        let ids: HashSet<&str> = found.iter().map(|f| f.item.id.as_str()).collect();
        assert_eq!(ids.len(), found.len());
        assert!(ids.iter().all(|i| i.len() == 16));
    }

    #[test]
    fn manual_mode_never_has_a_safe_item() {
        let (sb, host) = sandbox("manual");
        let install = mk(sb.root.join("apps").join("SuperTool"));
        mk(sb.root.join("roaming").join("SuperTool"));
        let found = scan_items(&identity("SuperTool", Some(&install), Mode::Manual), &host);
        assert!(found.len() >= 2);
        assert!(found.iter().all(|f| f.item.tier != Tier::Safe));
        assert_eq!(by_kind(&found, Kind::InstallDir)[0].tier, Tier::Review);
    }

    /// Programs named after shared things find no name matches and never a
    /// well-known registry key (BCU #699, #727 and #520).
    #[test]
    fn generic_program_names_find_nothing() {
        let (sb, mut host) = sandbox("generic");
        let names = [
            "Classes",
            "Software",
            "Microsoft",
            "Update",
            "Helper",
            "Tools",
            "VLC",
            "Go",
            "Policies",
            "Runtime",
            "Driver",
            "Launcher",
            "Service",
        ];
        for name in names {
            mk(sb.root.join("roaming").join(name));
            std::fs::write(sb.root.join("menu").join(format!("{name}.lnk")), b"x").unwrap();
        }
        host.set_keys(vec![
            (View::User, names.iter().map(|n| n.to_string()).collect()),
            (View::Machine, names.iter().map(|n| n.to_string()).collect()),
            (
                View::MachineWow,
                names.iter().map(|n| n.to_string()).collect(),
            ),
        ]);
        for name in names {
            let found = scan_items(&identity(name, None, Mode::Registry), &host);
            assert!(
                found.is_empty(),
                "{name}: {:?}",
                found.iter().map(|f| &f.item.path).collect::<Vec<_>>()
            );
        }
        // Even a long, specific name never reaches a shared root: the
        // well-known key list is a second gate behind the stoplist.
        host.set_keys(vec![(
            View::User,
            vec!["WOW6432Node".into(), "Clients".into()],
        )]);
        let found = scan_items(&identity("WOW6432Node", None, Mode::Registry), &host);
        assert!(found.is_empty());
    }

    #[test]
    fn an_install_folder_that_may_not_be_cleaned_is_shown_as_kept_with_the_reason() {
        let (sb, mut host) = sandbox("keep");
        let shared = mk(sb.root.join("apps").join("Suite"));
        let inner = mk(shared.join("Inner"));
        host.env.others = vec![Some(inner.to_string_lossy().into_owned())];
        let found = scan_items(&identity("Suite", Some(&shared), Mode::Registry), &host);
        let dir = by_kind(&found, Kind::InstallDir);
        assert_eq!(dir.len(), 1);
        assert_eq!(dir[0].tier, Tier::Keep);
        assert_eq!(dir[0].blocker, Some(Blocker::OtherProgram));
        assert!(!dir[0].deletable);
        // A name-matched folder that holds a registered program stays too.
        let roaming = mk(sb.root.join("roaming").join("Suite"));
        host.env.others = vec![Some(roaming.join("data").to_string_lossy().into_owned())];
        let found = scan_items(&identity("Suite", None, Mode::Registry), &host);
        assert_eq!(
            by_kind(&found, Kind::AppData)[0].blocker,
            Some(Blocker::OtherProgram)
        );
        // The program list could not be read: nothing is cleanable.
        host.env.programs_unknown = true;
        let found = scan_items(&identity("Suite", Some(&shared), Mode::Registry), &host);
        assert!(found.iter().all(|f| !f.item.deletable));
    }

    #[test]
    fn a_folder_the_recycle_bin_would_delete_outright_is_kept() {
        let (sb, mut host) = sandbox("bin");
        let install = mk(sb.root.join("apps").join("Big"));
        host.recycle = false;
        let found = scan_items(
            &identity("Big Thing", Some(&install), Mode::Registry),
            &host,
        );
        assert_eq!(found[0].item.tier, Tier::Keep);
        assert_eq!(found[0].item.blocker, Some(Blocker::NotRestorable));
    }

    #[test]
    fn protected_install_locations_never_become_cleanable() {
        let (_sb, mut host) = sandbox("protected");
        host.env = GateEnv::from_process(Some(Vec::new()));
        for var in [
            "ProgramFiles",
            "ProgramData",
            "SystemRoot",
            "USERPROFILE",
            "LOCALAPPDATA",
            "APPDATA",
            "SystemDrive",
        ] {
            let Ok(value) = std::env::var(var) else {
                continue;
            };
            for spelling in [
                value.clone(),
                value.to_ascii_uppercase(),
                format!("{value}\\"),
            ] {
                let found = scan_items(
                    &identity("x", Some(Path::new(&spelling)), Mode::Registry),
                    &host,
                );
                assert!(
                    found.iter().all(|f| !f.item.deletable),
                    "{var} ({spelling}) was offered for cleanup"
                );
            }
        }
        let windows = std::env::var("SystemRoot").unwrap();
        let system32 = format!(r"{windows}\System32").to_ascii_uppercase();
        let found = scan_items(
            &identity("System32", Some(Path::new(&system32)), Mode::Registry),
            &host,
        );
        assert!(found.iter().all(|f| !f.item.deletable));
    }

    #[test]
    fn a_junction_in_place_of_the_install_folder_is_refused() {
        let (sb, host) = sandbox("junction");
        let target = mk(sb.root.join("apps").join("Real"));
        let link = sb.root.join("apps").join("Linked");
        let made = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .output()
            .is_ok_and(|o| o.status.success());
        if !made {
            return;
        }
        let found = scan_items(
            &identity("Linked Thing", Some(&link), Mode::Registry),
            &host,
        );
        assert!(found.iter().all(|f| !f.item.deletable));
        assert_eq!(found[0].item.blocker, Some(Blocker::Linked));
        std::fs::remove_dir(&link).ok();
    }

    #[test]
    fn the_sweep_reports_only_what_is_provably_dead() {
        let (sb, mut host) = sandbox("sweep");
        host.disk.present = vec![r"C:\Program Files\Live\live.exe".into()];
        host.runs = vec![
            (
                "Gone".into(),
                r#""C:\Program Files\Gone\gone.exe" /boot"#.into(),
            ),
            ("Live".into(), r#""C:\Program Files\Live\live.exe""#.into()),
            ("Removable".into(), r"E:\Portable\p.exe".into()),
            (
                "Shell".into(),
                "rundll32.exe shell32.dll,Control_RunDLL".into(),
            ),
            ("Windows".into(), r"C:\Windows\System32\gone.exe".into()),
        ];
        std::fs::write(sb.root.join("startup").join("Dead.lnk"), b"x").unwrap();
        std::fs::write(sb.root.join("startup").join("Alive.lnk"), b"x").unwrap();
        std::fs::write(sb.root.join("startup").join("Unreadable.lnk"), b"x").unwrap();
        std::fs::write(sb.root.join("startup").join("desktop.ini"), b"x").unwrap();
        host.targets = vec![
            ("Dead.lnk".into(), r"C:\Program Files\Dead\dead.exe".into()),
            ("Alive.lnk".into(), r"C:\Program Files\Live\live.exe".into()),
        ];
        let found = scan_items(&identity("", None, Mode::Sweep), &host);
        let labels: Vec<&str> = found.iter().map(|f| f.item.label.as_str()).collect();
        assert_eq!(labels, vec!["Gone", "Dead"]);
        for f in &found {
            assert_eq!(f.item.evidence, Evidence::Dangling);
            assert_eq!(f.item.tier, Tier::Review);
            assert!(f.item.deletable && f.item.missing.is_some() && f.item.command.is_some());
            assert!(f.item.kind.pro_only());
        }
        assert_eq!(found[0].item.kind, Kind::StartupRun);
        assert_eq!(found[1].item.kind, Kind::StartupShortcut);
    }

    #[test]
    fn manual_input_is_validated_in_rust() {
        let (sb, mut host) = sandbox("validate");
        let install = mk(sb.root.join("apps").join("SuperTool"));
        host.env.others = vec![Some(r"D:\Apps\Installed\Thing".into())];
        let err = |name: &str, folder: &str, host: &Fake| {
            manual_identity(name, folder, host).unwrap_err()
        };
        assert_eq!(err("", "", &host), MANUAL_EMPTY);
        assert_eq!(err("  ", "  ", &host), MANUAL_EMPTY);
        assert_eq!(err("VLC", "", &host), MANUAL_NAME);
        assert_eq!(err("Microsoft", "", &host), MANUAL_NAME);
        assert_eq!(err("Tools", "", &host), MANUAL_NAME);
        for folder in [
            r"\\server\share\Thing",
            r"C:\Games\..\Windows",
            r"Relative\Thing",
            "C:\\Games\\Th\u{7}ing",
            r"C:\Games\Th*ng",
        ] {
            assert_eq!(err("", folder, &host), MANUAL_SHAPE, "{folder}");
        }
        assert_eq!(err("", r"C:\Windows\System32", &host), MANUAL_PROTECTED);
        assert_eq!(err("", r"C:\Program Files", &host), MANUAL_PROTECTED);
        assert_eq!(err("", r"C:\Foo", &host), MANUAL_PROTECTED);
        assert_eq!(err("", r"D:\Apps\Installed\Thing", &host), MANUAL_OTHER);
        assert_eq!(err("", r"D:\Apps\Installed", &host), MANUAL_OTHER);
        assert_eq!(err("", r"C:\Games\Does Not Exist", &host), MANUAL_GONE);
        assert_eq!(err(&"x".repeat(500), "", &host), MANUAL_SHAPE);
        // Good input: name only, folder only, both.
        let both = manual_identity("SuperTool", &install.to_string_lossy(), &host).unwrap();
        assert_eq!(both.mode, Mode::Manual);
        assert!(manual_identity("SuperTool", "", &host)
            .unwrap()
            .anchor
            .is_none());
        assert!(manual_identity("", &install.to_string_lossy(), &host).is_ok());
    }

    #[test]
    fn a_link_in_the_typed_folder_is_refused() {
        let (sb, host) = sandbox("manual-link");
        let target = mk(sb.root.join("apps").join("Real"));
        let link = sb.root.join("apps").join("Linked");
        let made = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .output()
            .is_ok_and(|o| o.status.success());
        if !made {
            return;
        }
        assert_eq!(
            manual_identity("", &link.to_string_lossy(), &host).unwrap_err(),
            MANUAL_LINKED
        );
        std::fs::remove_dir(&link).ok();
    }

    /// Acts on the fake registry; recycling and saving are recorded.
    struct TestActor<'a> {
        reg: &'a FakeRegistry,
        save_fails: bool,
        recycle_fails: bool,
        log: std::cell::RefCell<Vec<String>>,
    }
    impl Actor for TestActor<'_> {
        fn registry(&self) -> &dyn Registry {
            self.reg
        }
        fn recycle(&self, path: &Path) -> Option<u64> {
            self.log
                .borrow_mut()
                .push(format!("recycle {}", file_name(path)));
            (!self.recycle_fails).then_some(5)
        }
        fn save(&self, backup: &Backup, fresh: bool) -> Result<(), String> {
            self.log.borrow_mut().push(format!(
                "save fresh={fresh} keys={} recycled={}",
                backup.keys.len(),
                backup.recycled.len()
            ));
            if self.save_fails {
                Err("disk full".into())
            } else {
                Ok(())
            }
        }
    }
    fn test_actor(reg: &FakeRegistry) -> TestActor<'_> {
        TestActor {
            reg,
            save_fails: false,
            recycle_fails: false,
            log: std::cell::RefCell::new(Vec::new()),
        }
    }
    fn run(found: &[Found], ids: &[String], pro: bool, actor: &dyn Actor) -> CleanResult {
        let mut result = CleanResult::default();
        let selected = select(found, ids, pro, &mut result);
        let mut backup = Backup::new("1767225600000", "SuperTool");
        apply(&selected, &mut backup, actor, &mut result);
        result
    }

    #[test]
    fn cleaning_acts_only_on_what_a_fresh_scan_vouches_for_and_free_stays_off_pro_kinds() {
        let (sb, mut host) = sandbox("clean");
        let install = mk(sb.root.join("apps").join("SuperTool"));
        host.set_keys(vec![(View::User, vec!["SuperTool".into()])]);
        host.env.others = vec![Some(r"D:\Elsewhere\Other".into())];
        let found = scan_items(
            &identity("SuperTool", Some(&install), Mode::Registry),
            &host,
        );
        let id_of = |kind: Kind| by_kind(&found, kind)[0].id.clone();
        let (dir, key) = (id_of(Kind::InstallDir), id_of(Kind::RegistryUser));
        let ids = vec![
            dir.clone(),
            key.clone(),
            "feedfacefeedface".to_string(),
            dir.clone(),
        ];

        // Free: the folder only, once; the key is locked, Pro enforced here.
        let actor = test_actor(&host.reg);
        let free = run(&found, &ids, false, &actor);
        assert_eq!(free.removed, vec![dir.clone()]);
        assert_eq!(free.locked, vec![key.clone()]);
        assert_eq!(free.failed, vec!["feedfacefeedface".to_string()]);
        assert_eq!(free.freed_kb, 5);
        assert!(host.reg.has(Hive::Hkcu, r"Software\SuperTool"));

        // Pro: the key too, backed up first; registry before folders.
        let actor = test_actor(&host.reg);
        let pro = run(&found, &ids, true, &actor);
        assert_eq!(pro.removed, vec![key.clone(), dir.clone()]);
        assert!(pro.locked.is_empty());
        assert!(!host.reg.has(Hive::Hkcu, r"Software\SuperTool"));
        assert_eq!(
            *actor.log.borrow(),
            vec![
                "save fresh=true keys=1 recycled=1".to_string(),
                "recycle SuperTool".to_string(),
                "save fresh=false keys=1 recycled=1".to_string(),
            ]
        );

        // A step that did not happen is a failure, and leaves the backup.
        let mut actor = test_actor(&host.reg);
        actor.recycle_fails = true;
        let refused = run(&found, std::slice::from_ref(&dir), true, &actor);
        assert_eq!(refused.failed, vec![dir.clone()]);
        assert_eq!(
            actor.log.borrow().last().unwrap(),
            "save fresh=false keys=0 recycled=0"
        );
    }

    #[test]
    fn no_backup_means_no_change() {
        let (_sb, mut host) = sandbox("nobackup");
        host.set_keys(vec![(View::User, vec!["SuperTool".into()])]);
        let found = scan_items(&identity("SuperTool", None, Mode::Registry), &host);
        let ids: Vec<String> = found.iter().map(|f| f.item.id.clone()).collect();
        let mut actor = test_actor(&host.reg);
        actor.save_fails = true;
        let result = run(&found, &ids, true, &actor);
        assert!(result.removed.is_empty());
        assert_eq!(result.failed, ids);
        assert!(host.reg.has(Hive::Hkcu, r"Software\SuperTool"));
        // A key that cannot be read whole is never offered at all.
        host.reg.unreadable = vec![r"Software\SuperTool".into()];
        let found = scan_items(&identity("SuperTool", None, Mode::Registry), &host);
        assert_eq!(found[0].item.tier, Tier::Keep);
        assert_eq!(found[0].item.blocker, Some(Blocker::NotRestorable));
        assert_eq!(found[0].item.restore, "none");
    }

    #[test]
    fn keep_items_are_never_acted_on_pro_or_not() {
        let (sb, mut host) = sandbox("keep-act");
        let install = mk(sb.root.join("apps").join("Shared"));
        host.env.others = vec![Some(install.join("data").to_string_lossy().into_owned())];
        let found = scan_items(&identity("Shared", Some(&install), Mode::Registry), &host);
        assert_eq!(found[0].item.tier, Tier::Keep);
        let actor = test_actor(&host.reg);
        let result = run(&found, &[found[0].item.id.clone()], true, &actor);
        assert_eq!(result.failed, vec![found[0].item.id.clone()]);
        assert!(actor.log.borrow().is_empty());
    }

    #[test]
    fn a_machine_key_naming_the_install_folder_is_proof_and_one_naming_another_program_stays() {
        let (sb, mut host) = sandbox("machine-keys");
        let install = mk(sb.root.join("apps").join("SuperTool"));
        let inside = install.join("bin").join("st.exe");
        host.reg.key(
            Hive::Hklm,
            r"Software\SuperTool",
            vec![sz("Path", &format!("\"{}\" --run", inside.display()))],
        );
        host.reg.key(
            Hive::Hklm,
            r"Software\WOW6432Node\SuperTool",
            vec![sz("Home", r"D:\Elsewhere\SuperToolbox")],
        );
        let found = scan_items(
            &identity("SuperTool", Some(&install), Mode::Registry),
            &host,
        );
        let item = |path: &str| {
            found
                .iter()
                .map(|f| &f.item)
                .find(|i| i.path == path)
                .unwrap()
        };
        let proven = item(r"HKLM\Software\SuperTool");
        assert_eq!(proven.evidence, Evidence::Path);
        assert_eq!(proven.tier, Tier::Safe);
        assert_eq!((proven.scope, proven.restore), ("machine", "backup"));
        assert!(proven.deletable && proven.kind.pro_only());
        // `D:\Elsewhere\SuperToolbox` is not inside the install folder.
        let named = item(r"HKLM\Software\WOW6432Node\SuperTool");
        assert_eq!(named.evidence, Evidence::Name);
        assert_eq!(named.tier, Tier::Review);

        // A value naming a program still registered keeps the key.
        host.env.others = vec![Some(r"D:\Elsewhere\SuperToolbox".into())];
        let found = scan_items(
            &identity("SuperTool", Some(&install), Mode::Registry),
            &host,
        );
        let kept = found
            .iter()
            .find(|f| f.item.path == r"HKLM\Software\WOW6432Node\SuperTool")
            .unwrap();
        assert_eq!(kept.item.tier, Tier::Keep);
        assert_eq!(kept.item.blocker, Some(Blocker::OtherProgram));
    }

    #[test]
    fn mentions_match_whole_paths_only() {
        assert!(mentions(r"C:\Apps\Foo", r"C:\Apps\Foo"));
        assert!(mentions(r#""C:\Apps\Foo\x.exe" /s"#, r"c:\apps\foo\"));
        assert!(mentions(r"path=C:/Apps/Foo/bin", r"C:\Apps\Foo"));
        assert!(!mentions(r"C:\Apps\Foobar\x.exe", r"C:\Apps\Foo"));
        assert!(!mentions(r"X:\C:\Apps\Foo", r"C:\Apps\Foo"));
        assert!(!mentions(r"C:\x", r"C:\"));
        assert!(under(
            Path::new(r"C:\ProgramData\Foo"),
            Path::new(r"c:\programdata\")
        ));
        assert!(!under(
            Path::new(r"C:\ProgramData"),
            Path::new(r"C:\ProgramData")
        ));
        assert!(!under(
            Path::new(r"C:\ProgramDataX\Foo"),
            Path::new(r"C:\ProgramData")
        ));
    }

    #[test]
    fn the_dry_run_matches_what_cleaning_would_do() {
        let (sb, mut host) = sandbox("plan");
        let install = mk(sb.root.join("apps").join("SuperTool"));
        host.set_keys(vec![(View::User, vec!["SuperTool".into()])]);
        let who = identity("SuperTool", Some(&install), Mode::Registry);
        let found = scan_items(&who, &host);
        let ids: Vec<String> = found.iter().map(|f| f.item.id.clone()).collect();
        let free = plan_with(&found, &ids, false);
        assert_eq!(free.len(), 2);
        let dir = free.iter().find(|s| s.step == "recycle").unwrap();
        assert!(!dir.pro_only && !dir.locked && !dir.refused && !dir.admin);
        let key = free.iter().find(|s| s.step == "backup").unwrap();
        assert!(key.pro_only && key.locked && !key.refused && !key.admin);
        let pro = plan_with(&found, &ids, true);
        assert!(pro.iter().all(|s| !s.locked));
        assert!(plan_with(&found, &["nope".to_string()], true)[0].refused);

        // The fingerprint binds the plan: Pro or not, other ids, other order
        // of the same ids, another identity.
        let print = |steps: &[PlanStep]| plan_fingerprint(&who, steps);
        assert_ne!(print(&free), print(&pro));
        let reversed: Vec<String> = ids.iter().rev().cloned().collect();
        assert_eq!(print(&pro), print(&plan_with(&found, &reversed, true)));
        assert_ne!(print(&pro), print(&plan_with(&found, &ids[..1], true)));
        let other = identity("SuperTool", None, Mode::Registry);
        assert_ne!(print(&pro), plan_fingerprint(&other, &pro));

        // Items in folders of all users need the administrator step.
        host.machine = vec![sb.root.join("apps")];
        let found = scan_items(&who, &host);
        let dir = found
            .iter()
            .find(|f| f.item.kind == Kind::InstallDir)
            .unwrap();
        assert_eq!(dir.item.scope, "machine");
        assert!(plan_with(&found, std::slice::from_ref(&dir.item.id), false)[0].admin);
        // A new scope is a new id: a plan made before does not carry over.
        assert!(!ids.contains(&dir.item.id));
    }

    #[test]
    fn a_scan_can_be_found_again_by_its_token_only() {
        let who = identity("Tokened", None, Mode::Registry);
        let a = open_scan(who.clone());
        let b = open_scan(who);
        assert_ne!(a, b);
        assert_eq!(scan_for(&a).unwrap().name, "Tokened");
        assert_eq!(scan_for("../../etc").unwrap_err(), SCAN_GONE);
        assert_eq!(scan_for("").unwrap_err(), SCAN_GONE);
    }

    #[test]
    fn the_anchor_is_the_install_location_or_a_folder_both_paths_share() {
        assert_eq!(
            anchor_of(Some(r"C:\Apps\Foo"), Some(r"C:\Other"), Some(r"C:\Else")).as_deref(),
            Some(r"C:\Apps\Foo")
        );
        assert_eq!(
            anchor_of(None, Some(r"C:\Apps\Foo"), Some(r"c:\apps\foo")).as_deref(),
            Some(r"C:\Apps\Foo")
        );
        assert_eq!(
            anchor_of(Some("  "), Some(r"C:\Apps\Foo"), Some(r"C:\Apps\Foo")).as_deref(),
            Some(r"C:\Apps\Foo")
        );
        assert_eq!(anchor_of(None, Some(r"C:\A"), Some(r"C:\B")), None);
        assert_eq!(anchor_of(None, Some(r"C:\A"), None), None);
        assert_eq!(anchor_of(None, None, None), None);
    }

    /// Only an entry this process removed can be scanned by (source, key),
    /// with the values it read before the removal; a later removal of the
    /// same entry replaces them.
    #[test]
    fn only_entries_removed_in_this_session_can_be_scanned() {
        assert_eq!(
            removed_identity("user", "NeverRemoved").unwrap_err(),
            NOT_REMOVED
        );
        remember_session(
            "user",
            "ResidueTestKey",
            identity(
                "SuperTool",
                Some(Path::new(r"C:\Program Files\SuperTool")),
                Mode::Registry,
            ),
        );
        assert_eq!(
            removed_identity("user", "ResidueTestKey").unwrap().name,
            "SuperTool"
        );
        assert!(removed_identity("machine64", "ResidueTestKey").is_err());
        remember_session(
            "user",
            "ResidueTestKey",
            identity(
                "SuperTool",
                Some(Path::new(r"D:\Elsewhere\SuperTool")),
                Mode::Registry,
            ),
        );
        assert_eq!(
            removed_identity("user", "ResidueTestKey")
                .unwrap()
                .anchor
                .as_deref(),
            Some(r"D:\Elsewhere\SuperTool")
        );
    }

    #[test]
    fn normalization_unifies_spellings() {
        assert_eq!(normalize("PC Tweaker"), "pctweaker");
        assert_eq!(normalize("pc-tweaker-app"), "pctweakerapp");
        assert_eq!(normalize("Éxample!"), "xample");
        assert_eq!(strip_version("MyApp 2.1.0"), "MyApp");
        assert_eq!(strip_version("MyApp v3 (64)"), "MyApp");
        assert_eq!(strip_version("7.1.2"), "7.1.2"); // never empty the name
    }

    #[test]
    fn candidates_exclude_short_and_stoplisted_tokens_and_matching_is_exact() {
        assert_eq!(name_candidates("VLC"), Vec::<String>::new());
        assert_eq!(name_candidates("Microsoft"), Vec::<String>::new());
        assert_eq!(name_candidates("Classes"), Vec::<String>::new());
        let c = name_candidates("SuperTool 1.2");
        assert_eq!(c, vec!["supertool".to_string()]);
        assert!(matches_candidates("super-tool", &c));
        assert!(!matches_candidates("SuperTools", &c));
        assert!(!matches_candidates("MySuperTool", &c));
        assert!(!matches_candidates("Microsoft", &c));
    }
}
