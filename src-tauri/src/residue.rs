//! Residue Intelligence: what a removed program left behind.
//!
//! A scan starts from an identity (a program's name and the install folder
//! the registry recorded) and reports items, each with the evidence tying it
//! to the program, a tier (Safe is pre-selected, Review is shown, Keep is
//! never selectable) and, for Keep, the reason. See `residue_rules`.
//!
//! Safety posture, in order of importance:
//! 1. Cleaning files and folders moves them to the Recycle Bin, so every
//!    action is reversible from Windows itself. Registry keys and values are
//!    exported to a `.reg` file first.
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
//! 5. HKLM keys (both views) are reported and never touched.

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
    /// Whether cleaning can act on it (never for Keep, never for HKLM).
    pub deletable: bool,
    pub tier: Tier,
    pub evidence: Evidence,
    pub blocker: Option<Blocker>,
    /// The command a startup entry runs.
    pub command: Option<String>,
    /// The file that command needs, which is no longer there.
    pub missing: Option<String>,
    /// "recycle", "regFile" (saved as a .reg file first) or "none".
    pub restore: &'static str,
}

#[derive(Clone, Debug)]
enum Target {
    Path(PathBuf),
    UserKey(String),
    RunValue { key: String, name: String },
    ReportOnly,
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
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub id: String,
    /// "recycle" or "regFile".
    pub step: &'static str,
    pub pro_only: bool,
    /// Pro-only and Pro is not active: this would be skipped.
    pub locked: bool,
    /// Not selectable, or not in a fresh scan: this would be skipped.
    pub refused: bool,
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
}

pub struct RunValue {
    /// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` or `...\RunOnce`.
    pub key: String,
    pub name: String,
    pub data: String,
}

/// Everything a scan reads from the machine, so tests can use a fake.
pub trait Host {
    fn data_roots(&self) -> Vec<PathBuf>;
    fn start_menu_roots(&self) -> Vec<PathBuf>;
    fn software_keys(&self, view: View) -> Vec<String>;
    fn gate_env(&self) -> GateEnv;
    fn canonical(&self, path: &Path) -> Option<PathBuf>;
    fn disk(&self) -> &dyn Disk;
    fn run_values(&self) -> Vec<RunValue>;
    fn startup_folder(&self) -> Option<PathBuf>;
    fn shortcut_target(&self, path: &Path) -> Option<String>;
    /// The Recycle Bin will really take it (and will not delete it outright).
    fn can_recycle(&self, path: &Path, size_kb: Option<u64>) -> bool;
}

struct Real;

fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var(var).ok().map(PathBuf::from)
}

impl Host for Real {
    fn data_roots(&self) -> Vec<PathBuf> {
        let mut roots: Vec<PathBuf> = ["APPDATA", "LOCALAPPDATA", "PROGRAMDATA"]
            .iter()
            .filter_map(|v| env_path(v))
            .collect();
        if let Some(local) = env_path("LOCALAPPDATA") {
            roots.push(local.join("Programs"));
        }
        roots
    }

    fn start_menu_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Some(appdata) = env_path("APPDATA") {
            roots.push(appdata.join(r"Microsoft\Windows\Start Menu\Programs"));
        }
        if let Some(profile) = env_path("USERPROFILE") {
            roots.push(profile.join("Desktop"));
        }
        roots
    }

    #[cfg(windows)]
    fn software_keys(&self, view: View) -> Vec<String> {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
        use winreg::RegKey;
        let (hive, path) = match view {
            View::User => (HKEY_CURRENT_USER, "Software"),
            View::Machine => (HKEY_LOCAL_MACHINE, "Software"),
            View::MachineWow => (HKEY_LOCAL_MACHINE, r"Software\WOW6432Node"),
        };
        RegKey::predef(hive)
            .open_subkey_with_flags(path, KEY_READ)
            .map(|k| k.enum_keys().flatten().collect())
            .unwrap_or_default()
    }

    #[cfg(not(windows))]
    fn software_keys(&self, _view: View) -> Vec<String> {
        Vec::new()
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
    Found {
        item: ResidueItem {
            id: item_id(d.kind, false, &location, ""),
            kind: d.kind,
            scope: "user",
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

fn registry_found(view: View, name: &str, trust: Trust) -> Found {
    let path = format!("{}{name}", view.prefix());
    let user = view == View::User;
    let kind = if user {
        Kind::RegistryUser
    } else {
        Kind::RegistryMachine
    };
    let facts = Facts {
        evidence: Evidence::Name,
        trust,
        blocker: None,
        starts_code: false,
        restorable: true,
        inert: true,
    };
    let tier = tier(&facts);
    Found {
        item: ResidueItem {
            id: item_id(kind, !user, &path, ""),
            kind,
            scope: if user { "user" } else { "machine" },
            label: name.to_string(),
            path,
            size_kb: None,
            // HKLM keys are reported, never touched.
            deletable: user && tier != Tier::Keep,
            tier,
            evidence: Evidence::Name,
            blocker: None,
            command: None,
            missing: None,
            restore: if user { "regFile" } else { "none" },
        },
        target: if user {
            Target::UserKey(name.to_string())
        } else {
            Target::ReportOnly
        },
    }
}

fn scan_registry(host: &dyn Host, candidates: &[String], trust: Trust, out: &mut Vec<Found>) {
    for view in [View::User, View::Machine, View::MachineWow] {
        for name in host.software_keys(view) {
            if matches_candidates(&name, candidates)
                && registry_candidate_ok(&format!("{}{name}", view.prefix()))
            {
                out.push(registry_found(view, &name, trust));
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
                restore: "regFile",
            },
            target: Target::RunValue {
                key: value.key,
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

fn scan_items(identity: &Identity, host: &dyn Host) -> Vec<Found> {
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
        for root in host.data_roots() {
            for entry in std::fs::read_dir(&root).into_iter().flatten().flatten() {
                if matches_candidates(&entry.file_name().to_string_lossy(), &candidates) {
                    push_dir(Kind::AppData, entry.path(), &mut out);
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
        scan_registry(host, &candidates, trust, &mut out);
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

/// Decides what happens to each requested id, given a fresh scan, and hands
/// the ones that may proceed to `act`. An id the fresh scan does not report,
/// a Keep item and a report-only item fail; a Pro-only kind without Pro is
/// left alone and reported as locked.
fn clean_with(
    found: &[Found],
    ids: &[String],
    pro: bool,
    act: &mut dyn FnMut(&Found) -> Option<u64>,
) -> CleanResult {
    let mut result = CleanResult::default();
    for id in ids {
        let Some(f) = found.iter().find(|f| &f.item.id == id) else {
            result.failed.push(id.clone());
            continue;
        };
        if f.item.tier == Tier::Keep || !f.item.deletable {
            result.failed.push(id.clone());
        } else if f.item.kind.pro_only() && !pro {
            result.locked.push(id.clone());
        } else if let Some(freed) = act(f) {
            result.freed_kb += freed;
            result.removed.push(id.clone());
        } else {
            result.failed.push(id.clone());
        }
    }
    result
}

/// Saves `key` (a `HKCU\...` path) as a `.reg` file before anything under it
/// is deleted, so the one step that skips the Recycle Bin can still be undone
/// by double-clicking the file. False, and the key is kept, if the export does
/// not complete.
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

/// Does the change for one item the fresh scan vouched for. Returns the KB
/// freed, or `None` if it was not done.
fn act_real(f: &Found, backup_dir: &Path) -> Option<u64> {
    match &f.target {
        Target::Path(path) => {
            let size = crate::recycle_bin::item_size_kb(path);
            if let Some(reason) =
                crate::recycle_bin::refusal(crate::recycle_bin::limits_for(path), size)
            {
                // Windows would delete it permanently instead of recycling
                // it, so it stays where it is.
                crate::applog::line(&format!(
                    "residue not recycled: {} ({reason})",
                    path.to_string_lossy()
                ));
                return None;
            }
            match trash::delete(path) {
                Ok(()) => Some(size.unwrap_or(0)),
                Err(e) => {
                    // Cleanup is the destructive step, so a refusal is the
                    // one thing a support request most needs to show.
                    crate::applog::line(&format!(
                        "residue not recycled: {} ({e})",
                        path.to_string_lossy()
                    ));
                    None
                }
            }
        }
        #[cfg(windows)]
        Target::UserKey(name) => {
            use winreg::enums::HKEY_CURRENT_USER;
            use winreg::RegKey;
            (export_registry_key(&format!(r"HKCU\Software\{name}"), backup_dir)
                && RegKey::predef(HKEY_CURRENT_USER)
                    .open_subkey("Software")
                    .and_then(|s| s.delete_subkey_all(name))
                    .is_ok())
            .then_some(0)
        }
        #[cfg(windows)]
        Target::RunValue { key, name } => {
            use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
            use winreg::RegKey;
            let sub = key.strip_prefix(r"HKCU\")?;
            (export_registry_key(key, backup_dir)
                && RegKey::predef(HKEY_CURRENT_USER)
                    .open_subkey_with_flags(sub, KEY_SET_VALUE)
                    .and_then(|k| k.delete_value(name))
                    .is_ok())
            .then_some(0)
        }
        #[cfg(not(windows))]
        Target::UserKey(_) | Target::RunValue { .. } => {
            let _ = backup_dir;
            None
        }
        Target::ReportOnly => None,
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
    Ok(finish(removed_identity(&source, &id)?, &Real))
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
        &Real,
    ))
}

/// Scans for leftovers of a program the user names: a name, a folder, or both.
/// Validated here; nothing in the result is ever pre-selected.
#[tauri::command(async)]
pub fn scan_manual(name: String, folder: String) -> Result<ResidueReport, String> {
    let identity = manual_identity(&name, &folder, &Real)?;
    Ok(finish(identity, &Real))
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
        &Real,
    ))
}

/// How many dead startup entries there are right now.
pub fn sweep_count() -> usize {
    sweep_items(&Real).len()
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
                    Some(Target::UserKey(_) | Target::RunValue { .. }) => "regFile",
                    _ => "recycle",
                },
                pro_only: kind.is_some_and(Kind::pro_only),
                locked: kind.is_some_and(|k| k.pro_only() && !pro),
                refused: f.is_none_or(|f| f.item.tier == Tier::Keep || !f.item.deletable),
            }
        })
        .collect()
}

/// Dry run: what cleaning these items would do, changing nothing.
#[tauri::command(async)]
pub fn plan_residue_clean(
    app: tauri::AppHandle,
    token: String,
    item_ids: Vec<String>,
) -> Result<Vec<PlanStep>, String> {
    if item_ids.len() > 64 {
        return Err("Too many items in one cleanup.".into());
    }
    let identity = scan_for(&token)?;
    let found = scan_items(&identity, &Real);
    Ok(plan_with(&found, &item_ids, is_pro(&app)))
}

/// Cleans the selected items of a scan: files and folders go to the Recycle
/// Bin (free); registry keys and startup entries are saved to a `.reg` file
/// and deleted (Pro). The webview names the scan by token and the items by
/// id; every id is checked against a fresh scan first.
#[tauri::command(async)]
pub fn clean_residue(
    app: tauri::AppHandle,
    token: String,
    item_ids: Vec<String>,
) -> Result<CleanResult, String> {
    use tauri::Manager;
    if item_ids.len() > 64 {
        return Err("Too many items in one cleanup.".into());
    }
    let backup_dir = app
        .path()
        .app_data_dir()
        .map(|dir| dir.join("registry-backups"))
        .map_err(|e| e.to_string())?;
    let identity = scan_for(&token)?;
    let found = scan_items(&identity, &Real);
    let result = clean_with(&found, &item_ids, is_pro(&app), &mut |f| {
        act_real(f, &backup_dir)
    });
    crate::applog::line(&format!(
        "residue cleanup for {}: {} removed, {} failed, {} locked, {} KB freed",
        if identity.name.is_empty() {
            "startup entries"
        } else {
            &identity.name
        },
        result.removed.len(),
        result.failed.len(),
        result.locked.len(),
        result.freed_kb
    ));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    struct FakeDisk {
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

    struct Fake {
        data: Vec<PathBuf>,
        menu: Vec<PathBuf>,
        keys: Vec<(View, Vec<String>)>,
        env: GateEnv,
        runs: Vec<(String, String)>,
        startup: Option<PathBuf>,
        targets: Vec<(String, String)>,
        disk: FakeDisk,
        recycle: bool,
    }
    impl Host for Fake {
        fn data_roots(&self) -> Vec<PathBuf> {
            self.data.clone()
        }
        fn start_menu_roots(&self) -> Vec<PathBuf> {
            self.menu.clone()
        }
        fn software_keys(&self, view: View) -> Vec<String> {
            self.keys
                .iter()
                .filter(|(v, _)| *v == view)
                .flat_map(|(_, k)| k.clone())
                .collect()
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
    struct Sandbox {
        root: PathBuf,
    }
    impl Drop for Sandbox {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).ok();
        }
    }
    fn sandbox(tag: &str) -> (Sandbox, Fake) {
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
            data: vec![root.join("roaming")],
            menu: vec![root.join("menu")],
            keys: Vec::new(),
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
        host.keys = vec![
            (View::User, vec!["SuperTool".into(), "Classes".into()]),
            (View::Machine, vec!["SuperTool".into()]),
            (View::MachineWow, vec!["SuperTool".into()]),
        ];
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
        assert!(by_kind(&found, Kind::RegistryMachine)
            .iter()
            .all(|i| !i.deletable));
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
        host.keys = vec![
            (View::User, names.iter().map(|n| n.to_string()).collect()),
            (View::Machine, names.iter().map(|n| n.to_string()).collect()),
            (
                View::MachineWow,
                names.iter().map(|n| n.to_string()).collect(),
            ),
        ];
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
        host.keys = vec![(View::User, vec!["WOW6432Node".into(), "Clients".into()])];
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

    #[test]
    fn cleaning_acts_only_on_what_a_fresh_scan_vouches_for_and_free_stays_off_pro_kinds() {
        let (sb, mut host) = sandbox("clean");
        let install = mk(sb.root.join("apps").join("SuperTool"));
        host.keys = vec![(View::User, vec!["SuperTool".into()])];
        host.env.others = vec![Some(r"D:\Elsewhere\Other".into())];
        let found = scan_items(
            &identity("SuperTool", Some(&install), Mode::Registry),
            &host,
        );
        let id_of = |kind: Kind| by_kind(&found, kind)[0].id.clone();
        let (dir, key) = (id_of(Kind::InstallDir), id_of(Kind::RegistryUser));
        let ids = vec![dir.clone(), key.clone(), "feedfacefeedface".to_string()];

        let mut acted: Vec<String> = Vec::new();
        let free = clean_with(&found, &ids, false, &mut |f| {
            acted.push(f.item.id.clone());
            Some(5)
        });
        assert_eq!(free.removed, vec![dir.clone()]);
        assert_eq!(free.locked, vec![key.clone()]);
        assert_eq!(free.failed, vec!["feedfacefeedface".to_string()]);
        assert_eq!(free.freed_kb, 5);
        assert_eq!(acted, vec![dir.clone()]);

        let pro = clean_with(&found, &ids, true, &mut |_| Some(0));
        assert_eq!(pro.removed, vec![dir.clone(), key.clone()]);
        assert!(pro.locked.is_empty());

        // A step that did not happen is a failure, not a removal.
        let refused = clean_with(&found, std::slice::from_ref(&dir), true, &mut |_| None);
        assert_eq!(refused.failed, vec![dir.clone()]);

        // Report-only keys and Keep items are never acted on, Pro or not.
        host.keys = vec![(View::Machine, vec!["SuperTool".into()])];
        let machine = scan_items(&identity("SuperTool", None, Mode::Registry), &host);
        let id = machine[0].item.id.clone();
        let r = clean_with(&machine, std::slice::from_ref(&id), true, &mut |_| {
            panic!("must not act")
        });
        assert_eq!(r.failed, vec![id]);
    }

    #[test]
    fn the_dry_run_matches_what_cleaning_would_do() {
        let (sb, mut host) = sandbox("plan");
        let install = mk(sb.root.join("apps").join("SuperTool"));
        host.keys = vec![(View::User, vec!["SuperTool".into()])];
        let found = scan_items(
            &identity("SuperTool", Some(&install), Mode::Registry),
            &host,
        );
        let ids: Vec<String> = found.iter().map(|f| f.item.id.clone()).collect();
        let free = plan_with(&found, &ids, false);
        assert_eq!(free.len(), 2);
        let dir = free.iter().find(|s| s.step == "recycle").unwrap();
        assert!(!dir.pro_only && !dir.locked && !dir.refused);
        let key = free.iter().find(|s| s.step == "regFile").unwrap();
        assert!(key.pro_only && key.locked && !key.refused);
        assert!(plan_with(&found, &ids, true).iter().all(|s| !s.locked));
        assert!(plan_with(&found, &["nope".to_string()], true)[0].refused);
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
