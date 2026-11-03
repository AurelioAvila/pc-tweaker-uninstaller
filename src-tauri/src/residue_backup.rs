//! Byte-exact backups of what leftover cleaning deletes, and the way back.
//!
//! Ported from PC Tweaker's `regclean.rs` (the raw value format, the `.reg`
//! text, the restore that never overwrites, the allowed roots) and extended
//! from single keys to whole key trees, which is what a program's
//! `Software\<name>` key is.
//!
//! Where backups live, and why:
//! - Items of this user (HKCU keys and Run values, files in the profile):
//!   JSON plus a `.reg` copy in the app's data folder. Undo restores them as
//!   the user, so a file the user can edit grants nothing.
//! - Machine items (HKLM keys, folders in Program Files and ProgramData, the
//!   all-users Start menu): `HKLM\SOFTWARE\PC Tweaker Uninstaller\
//!   LeftoverBackups\<stamp>`, written by the elevated child under an access
//!   list that lets only SYSTEM and Administrators change it. The elevated
//!   undo reads only there: an administrator importing a file the user can
//!   edit would let any program running as the user choose what the
//!   administrator writes, then wait for the Undo click. The parent keeps a
//!   read-only `.reg` and JSON copy beside the user backups, for people to
//!   read. Nothing ever imports that copy.
//!
//! Restore never overwrites: a key or value that exists again stays as it
//! is, and only places leftover cleaning may touch are written
//! (`allowed_key`, `allowed_value`, and for files the Recycle Bin's own
//! record of where each one was).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum Hive {
    #[serde(rename = "HKCU")]
    Hkcu,
    #[serde(rename = "HKLM")]
    Hklm,
}

impl Hive {
    pub fn short(self) -> &'static str {
        match self {
            Hive::Hkcu => "HKCU",
            Hive::Hklm => "HKLM",
        }
    }
    fn long_name(self) -> &'static str {
        match self {
            Hive::Hkcu => "HKEY_CURRENT_USER",
            Hive::Hklm => "HKEY_LOCAL_MACHINE",
        }
    }
}

/// One raw registry value, kept byte for byte so a restore is exact whatever
/// its type.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawValue {
    pub name: String,
    pub kind: u32,
    #[serde(with = "b64")]
    pub bytes: Vec<u8>,
}

const REG_SZ: u32 = 1;
const REG_EXPAND_SZ: u32 = 2;
const REG_BINARY: u32 = 3;
const REG_DWORD: u32 = 4;
const REG_MULTI_SZ: u32 = 7;
/// `REG_QWORD`, the highest type Windows defines.
const REG_LAST: u32 = 11;

impl RawValue {
    /// The strings a text value holds (one for SZ and EXPAND_SZ, each item of
    /// a MULTI_SZ); none for other types.
    pub fn strings(&self) -> Vec<String> {
        if !matches!(self.kind, REG_SZ | REG_EXPAND_SZ | REG_MULTI_SZ) {
            return Vec::new();
        }
        let units: Vec<u16> = self
            .bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        let parts = units.split(|u| *u == 0).map(String::from_utf16_lossy);
        if self.kind == REG_MULTI_SZ {
            parts.filter(|s| !s.is_empty()).collect()
        } else {
            parts.take(1).filter(|s| !s.is_empty()).collect()
        }
    }
}

mod b64 {
    use base64::Engine as _;
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(bytes))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let text = String::deserialize(d)?;
        base64::engine::general_purpose::STANDARD
            .decode(text)
            .map_err(serde::de::Error::custom)
    }
}

/// One key of a backed-up tree. `sub` is relative to the tree's root, empty
/// for the root itself; parents always come before their children.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KeyNode {
    pub sub: String,
    pub values: Vec<RawValue>,
}

/// A deleted key and everything under it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KeyBackup {
    pub id: String,
    pub hive: Hive,
    /// Below the hive, e.g. `SOFTWARE\WOW6432Node\Foo`.
    pub path: String,
    pub nodes: Vec<KeyNode>,
}

/// A deleted value (a startup entry).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ValueBackup {
    pub id: String,
    pub hive: Hive,
    pub key: String,
    pub value: RawValue,
}

/// A file or folder sent to the Recycle Bin, and when (Unix seconds): the
/// bin's own entry is found again by its original path and that window.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Recycled {
    pub id: String,
    pub path: String,
    pub from: u64,
    pub to: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Backup {
    pub version: u32,
    pub stamp: String,
    /// The program the leftovers came from, for people reading the copy.
    pub program: String,
    pub keys: Vec<KeyBackup>,
    pub values: Vec<ValueBackup>,
    /// In the order they were recycled.
    pub recycled: Vec<Recycled>,
}

impl Backup {
    pub fn new(stamp: &str, program: &str) -> Backup {
        Backup {
            version: 1,
            stamp: stamp.to_string(),
            program: program.chars().take(256).collect(),
            ..Backup::default()
        }
    }
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty() && self.values.is_empty() && self.recycled.is_empty()
    }
}

/// A backup id: a millisecond time stamp, nothing that could name a path.
pub fn valid_stamp(stamp: &str) -> bool {
    (10..=16).contains(&stamp.len()) && stamp.bytes().all(|b| b.is_ascii_digit())
}

pub fn new_stamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
        .to_string()
}

// ---- registry access ----------------------------------------------------------

/// Registry access, so the rules run against fixtures in the tests.
pub trait Registry {
    /// `None` when the key does not exist or cannot be read.
    fn subkeys(&self, hive: Hive, path: &str) -> Option<Vec<String>>;
    fn values(&self, hive: Hive, path: &str) -> Option<Vec<RawValue>>;
    /// True also when the key exists but cannot be read.
    fn key_exists(&self, hive: Hive, path: &str) -> bool;
    /// Creates a key that must not exist yet: an existing one is an
    /// `AlreadyExists` error, decided by Windows in the same call.
    fn create_new_key(&self, hive: Hive, path: &str) -> std::io::Result<()>;
    fn set_value(&self, hive: Hive, path: &str, value: &RawValue) -> std::io::Result<()>;
    fn delete_tree(&self, hive: Hive, path: &str) -> std::io::Result<()>;
    fn delete_value(&self, hive: Hive, path: &str, name: &str) -> std::io::Result<()>;
}

/// Caps on one key tree. A program's own key is small; anything beyond these
/// is not backed up, so it is not cleaned either.
const MAX_NODES: usize = 2_000;
const MAX_VALUES: usize = 20_000;
const MAX_BYTES: usize = 4 << 20;
const MAX_DEPTH: usize = 32;

fn join(base: &str, sub: &str) -> String {
    match (base.is_empty(), sub.is_empty()) {
        (_, true) => base.to_string(),
        (true, false) => sub.to_string(),
        (false, false) => format!("{base}\\{sub}"),
    }
}

/// Every key and value under `path`, parents first. `None` when the key is
/// missing, any part of it cannot be read, or it is beyond the caps: then it
/// cannot be put back exactly.
pub fn capture_tree(reg: &dyn Registry, hive: Hive, path: &str) -> Option<Vec<KeyNode>> {
    let mut nodes: Vec<KeyNode> = Vec::new();
    let mut queue = std::collections::VecDeque::from([String::new()]);
    let (mut count, mut bytes) = (0usize, 0usize);
    while let Some(sub) = queue.pop_front() {
        if nodes.len() >= MAX_NODES {
            return None;
        }
        let full = join(path, &sub);
        let values = reg.values(hive, &full)?;
        count += values.len();
        bytes += values
            .iter()
            .map(|v| v.bytes.len() + 2 * v.name.len())
            .sum::<usize>();
        if count > MAX_VALUES || bytes > MAX_BYTES {
            return None;
        }
        let children = reg.subkeys(hive, &full)?;
        let depth = if sub.is_empty() {
            0
        } else {
            sub.matches('\\').count() + 1
        };
        if !children.is_empty() && depth + 1 > MAX_DEPTH {
            return None;
        }
        for child in children {
            if child.is_empty() || child.contains(['\\', '\0']) {
                return None;
            }
            queue.push_back(join(&sub, &child));
        }
        nodes.push(KeyNode { sub, values });
    }
    Some(nodes)
}

/// The nodes as a restore may write them: the root first, every other node
/// below one that came before it, no empty or NUL parts, known value types.
fn valid_nodes(nodes: &[KeyNode]) -> bool {
    if nodes.is_empty() || !nodes[0].sub.is_empty() || nodes.len() > MAX_NODES {
        return false;
    }
    let mut seen: Vec<String> = vec![String::new()];
    for node in &nodes[1..] {
        let lower = node.sub.to_lowercase();
        let parent = lower.rsplit_once('\\').map_or("", |(p, _)| p);
        if node.sub.split('\\').any(str::is_empty)
            || node.sub.contains('\0')
            || seen.contains(&lower)
            || !seen.iter().any(|s| s == parent)
        {
            return false;
        }
        seen.push(lower);
    }
    nodes
        .iter()
        .flat_map(|n| &n.values)
        .all(|v| v.kind <= REG_LAST && !v.name.contains('\0'))
}

const RUN_KEYS: &[&str] = &[
    r"Software\Microsoft\Windows\CurrentVersion\Run",
    r"Software\Microsoft\Windows\CurrentVersion\RunOnce",
];

/// A key leftover cleaning may delete and undo may write: exactly one
/// program-named key below `Software` (HKCU) or below `SOFTWARE` or
/// `SOFTWARE\WOW6432Node` (HKLM), never a well-known shared one. This is
/// what keeps a tampered backup from naming anywhere else.
pub fn allowed_key(hive: Hive, path: &str) -> bool {
    let wow = path
        .get(..21)
        .is_some_and(|p| p.eq_ignore_ascii_case(r"Software\WOW6432Node\"));
    !path.contains('\0')
        && (hive == Hive::Hklm || !wow)
        && crate::residue_rules::registry_candidate_ok(&format!("{}\\{path}", hive.short()))
}

/// A value leftover cleaning may delete and undo may write: a startup entry
/// of this user (HKCU `Run` or `RunOnce`). Machine-wide startup entries are
/// never restored from a backup a user could have written.
pub fn allowed_value(hive: Hive, key: &str, name: &str) -> bool {
    hive == Hive::Hkcu
        && !name.contains('\0')
        && RUN_KEYS.iter().any(|k| k.eq_ignore_ascii_case(key))
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Restored,
    /// Something exists there again; it was left as it is.
    Conflict,
    /// Outside the places a backup may write.
    Refused,
    /// No longer in the Recycle Bin.
    Missing,
    Failed,
}

/// Puts a deleted key tree back, unless the key exists again.
pub fn restore_key(reg: &dyn Registry, backup: &KeyBackup) -> Outcome {
    if !allowed_key(backup.hive, &backup.path) || !valid_nodes(&backup.nodes) {
        return Outcome::Refused;
    }
    if reg.key_exists(backup.hive, &backup.path) {
        return Outcome::Conflict;
    }
    for node in &backup.nodes {
        let full = join(&backup.path, &node.sub);
        if let Err(e) = reg.create_new_key(backup.hive, &full) {
            // Created by someone else between the check and now: theirs.
            return if node.sub.is_empty() && e.kind() == std::io::ErrorKind::AlreadyExists {
                Outcome::Conflict
            } else {
                Outcome::Failed
            };
        }
        if node
            .values
            .iter()
            .any(|v| reg.set_value(backup.hive, &full, v).is_err())
        {
            return Outcome::Failed;
        }
    }
    Outcome::Restored
}

/// Puts a deleted value back, unless a value of that name exists again.
pub fn restore_value(reg: &dyn Registry, backup: &ValueBackup) -> Outcome {
    let value = &backup.value;
    if !allowed_value(backup.hive, &backup.key, &value.name) || value.kind > REG_LAST {
        return Outcome::Refused;
    }
    let Some(existing) = reg.values(backup.hive, &backup.key) else {
        return Outcome::Failed;
    };
    if existing
        .iter()
        .any(|v| v.name.eq_ignore_ascii_case(&value.name))
    {
        return Outcome::Conflict;
    }
    match reg.set_value(backup.hive, &backup.key, value) {
        Ok(()) => Outcome::Restored,
        Err(_) => Outcome::Failed,
    }
}

// ---- Recycle Bin --------------------------------------------------------------

/// One item in the Recycle Bin: where it came from and when it was deleted.
#[derive(Clone, Debug)]
pub struct BinItem {
    pub index: usize,
    pub path: PathBuf,
    pub deleted: i64,
}

pub enum BinError {
    /// Something is at the original path again.
    Collision,
    Other,
}

/// The Recycle Bin of whoever runs this, so the matching is testable.
pub trait Bin {
    fn items(&self) -> Option<Vec<BinItem>>;
    fn restore(&self, item: &BinItem) -> Result<(), BinError>;
}

/// The real bin. Elevated, this is the administrator's bin, which is where
/// the elevated child recycled to.
pub struct WinBin {
    list: std::cell::RefCell<Vec<trash::TrashItem>>,
}

impl WinBin {
    pub fn new() -> WinBin {
        WinBin {
            list: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl Default for WinBin {
    fn default() -> Self {
        WinBin::new()
    }
}

impl Bin for WinBin {
    fn items(&self) -> Option<Vec<BinItem>> {
        let list = trash::os_limited::list().ok()?;
        let items = list
            .iter()
            .enumerate()
            .map(|(index, item)| BinItem {
                index,
                path: item.original_path(),
                deleted: item.time_deleted,
            })
            .collect();
        *self.list.borrow_mut() = list;
        Some(items)
    }
    fn restore(&self, item: &BinItem) -> Result<(), BinError> {
        let Some(entry) = self.list.borrow().get(item.index).cloned() else {
            return Err(BinError::Other);
        };
        match trash::os_limited::restore_all([entry]) {
            Ok(()) => Ok(()),
            Err(trash::Error::RestoreCollision { .. }) => Err(BinError::Collision),
            Err(_) => Err(BinError::Other),
        }
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        p.to_string_lossy()
            .trim_end_matches('\\')
            .to_lowercase()
            .replace('/', "\\")
    };
    norm(a) == norm(b)
}

/// Puts recycled items back, newest deletion first, one at a time so one
/// collision does not stop the rest. Only an item at the recorded path,
/// deleted inside the recorded window, is taken, and only where `allowed`
/// says this side may write.
pub fn restore_recycled(
    bin: &dyn Bin,
    items: &[Recycled],
    allowed: &dyn Fn(&Path) -> bool,
) -> Vec<(String, Outcome)> {
    let listed = bin.items();
    items
        .iter()
        .rev()
        .map(|rec| {
            let path = Path::new(&rec.path);
            let outcome = if !allowed(path) {
                Outcome::Refused
            } else if std::fs::symlink_metadata(path).is_ok() {
                Outcome::Conflict
            } else if let Some(listed) = &listed {
                let (from, to) = (rec.from as i64 - 2, rec.to as i64 + 2);
                listed
                    .iter()
                    .filter(|i| same_path(&i.path, path) && (from..=to).contains(&i.deleted))
                    .max_by_key(|i| i.deleted)
                    .map_or(Outcome::Missing, |item| match bin.restore(item) {
                        Ok(()) => Outcome::Restored,
                        Err(BinError::Collision) => Outcome::Conflict,
                        Err(BinError::Other) => Outcome::Failed,
                    })
            } else {
                Outcome::Failed
            };
            (rec.id.clone(), outcome)
        })
        .collect()
}

// ---- readable copy ------------------------------------------------------------

/// The backup in regedit's own format, so it can be read or merged by hand.
/// For reading: undo never imports it.
pub fn reg_text(backup: &Backup) -> String {
    fn quoted(s: &str) -> String {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }
    fn line(value: &RawValue) -> String {
        let name = if value.name.is_empty() {
            "@".to_string()
        } else {
            quoted(&value.name)
        };
        let text = value.strings();
        let data = match value.kind {
            // An exact round trip only: one string, no line breaks, nothing
            // after its terminating NUL.
            REG_SZ
                if text.len() <= 1
                    && !text.concat().contains(['\r', '\n'])
                    && value.bytes
                        == text
                            .concat()
                            .encode_utf16()
                            .chain([0])
                            .flat_map(u16::to_le_bytes)
                            .collect::<Vec<u8>>() =>
            {
                quoted(&text.concat())
            }
            REG_DWORD if value.bytes.len() == 4 => format!(
                "dword:{:08x}",
                u32::from_le_bytes([
                    value.bytes[0],
                    value.bytes[1],
                    value.bytes[2],
                    value.bytes[3]
                ])
            ),
            kind => {
                let hex = value
                    .bytes
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(",");
                if kind == REG_BINARY {
                    format!("hex:{hex}")
                } else {
                    format!("hex({kind:x}):{hex}")
                }
            }
        };
        format!("{name}={data}\r\n")
    }
    let mut out = String::from("Windows Registry Editor Version 5.00\r\n");
    for key in &backup.keys {
        for node in &key.nodes {
            out.push_str(&format!(
                "\r\n[{}\\{}]\r\n",
                key.hive.long_name(),
                join(&key.path, &node.sub)
            ));
            for value in &node.values {
                out.push_str(&line(value));
            }
        }
    }
    for value in &backup.values {
        out.push_str(&format!(
            "\r\n[{}\\{}]\r\n",
            value.hive.long_name(),
            value.key
        ));
        out.push_str(&line(&value.value));
    }
    out
}

// ---- this user's store (app data) -----------------------------------------------

/// Backups and readable copies kept, per side.
pub const KEEP: usize = 10;

pub fn user_dir() -> Result<PathBuf, String> {
    Ok(crate::uninstall_exec::fixed_data_dir()?.join("leftover-backups"))
}

fn file(dir: &Path, stamp: &str, tag: &str, ext: &str) -> PathBuf {
    dir.join(format!("{stamp}-{tag}.{ext}"))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, bytes).map_err(|e| format!("could not write a backup: {e}"))?;
    std::fs::rename(&temp, path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        format!("could not save a backup: {e}")
    })
}

fn reg_bytes(backup: &Backup) -> Vec<u8> {
    let mut reg: Vec<u8> = vec![0xFF, 0xFE];
    reg.extend(reg_text(backup).encode_utf16().flat_map(u16::to_le_bytes));
    reg
}

/// Saves this user's half of a cleanup: the JSON undo reads and the `.reg`
/// copy. Nothing of this user's registry is deleted unless this succeeded.
pub fn save_user(dir: &Path, backup: &Backup) -> Result<(), String> {
    crate::elevation::ensure_plain_app_data_dir(dir)?;
    std::fs::create_dir_all(dir).map_err(|e| format!("could not create the backup folder: {e}"))?;
    let json = serde_json::to_vec_pretty(backup).map_err(|e| e.to_string())?;
    write_atomic(&file(dir, &backup.stamp, "user", "json"), &json)?;
    write_atomic(&file(dir, &backup.stamp, "user", "reg"), &reg_bytes(backup))
}

pub fn load_user(dir: &Path, stamp: &str) -> Result<Option<Backup>, String> {
    if !valid_stamp(stamp) {
        return Err(crate::residue_elevated::UNDO_GONE.into());
    }
    match std::fs::read(file(dir, stamp, "user", "json")) {
        Ok(raw) => serde_json::from_slice::<Backup>(&raw)
            .ok()
            .filter(|b| b.stamp == stamp)
            .map(Some)
            .ok_or_else(|| crate::residue_elevated::BACKUP_DAMAGED.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("could not read the backup: {e}")),
    }
}

/// The administrator's half, copied for people to read, read-only. Undo
/// never reads it.
pub fn save_readable_copy(dir: &Path, backup: &Backup) -> Result<(), String> {
    crate::elevation::ensure_plain_app_data_dir(dir)?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_vec_pretty(backup).map_err(|e| e.to_string())?;
    for (ext, bytes) in [("json", json), ("reg", reg_bytes(backup))] {
        let path = file(dir, &backup.stamp, "system", ext);
        write_atomic(&path, &bytes)?;
        if let Ok(meta) = std::fs::metadata(&path) {
            let mut perms = meta.permissions();
            perms.set_readonly(true);
            let _ = std::fs::set_permissions(&path, perms);
        }
    }
    Ok(())
}

/// The stamps that have a backup here, newest first.
pub fn user_stamps(dir: &Path) -> Vec<String> {
    let mut stamps: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let (stamp, _) = name.split_once('-')?;
            valid_stamp(stamp).then(|| stamp.to_string())
        })
        .collect();
    stamps.sort_by(|a, b| b.len().cmp(&a.len()).then(b.cmp(a)));
    stamps.dedup();
    stamps
}

/// Keeps the newest `keep` cleanups' files.
pub fn prune_user(dir: &Path, keep: usize) {
    let old: Vec<String> = user_stamps(dir).into_iter().skip(keep).collect();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if old.iter().any(|s| name.starts_with(&format!("{s}-"))) {
            if let Ok(meta) = entry.metadata() {
                let mut perms = meta.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                let _ = std::fs::set_permissions(entry.path(), perms);
            }
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

// ---- the machine store (HKLM) -----------------------------------------------------

/// Where the elevated child keeps machine backups. A parameter only so the
/// tests can use their own root.
#[derive(Clone, Copy)]
pub struct MachineStore {
    pub root: &'static str,
}

pub const STORE: MachineStore = MachineStore {
    root: r"SOFTWARE\PC Tweaker Uninstaller",
};

/// Owner Administrators; SYSTEM and Administrators may do anything, Users
/// may read; nothing inherited from `SOFTWARE`, whose access list may have
/// been loosened by some installer.
#[cfg(windows)]
const STORE_SDDL: &str = "O:BAD:P(A;OICI;KA;;;SY)(A;OICI;KA;;;BA)(A;OICI;KR;;;BU)";

impl MachineStore {
    fn backups(&self) -> String {
        format!(r"{}\LeftoverBackups", self.root)
    }

    /// Only SYSTEM and Administrators can change the store: its owner is one
    /// of them, and no entry lets anyone else write, create, delete, or
    /// change permissions.
    pub fn trusted(&self) -> bool {
        [self.root.to_string(), self.backups()].iter().all(|key| {
            crate::uninstall_exec::read_key_access(&format!(r"MACHINE\{key}")).is_some_and(
                |(owner, dacl)| {
                    crate::uninstall_exec::only_admins_can_change_with(
                        &owner,
                        dacl.as_deref(),
                        crate::uninstall_exec::KEY_CHANGE_RIGHTS,
                    )
                },
            )
        })
    }

    /// Creates the store with its own access list the first time, then
    /// checks it. Elevated only.
    #[cfg(windows)]
    fn ensure(&self) -> Result<(), String> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::{
            Foundation::{LocalFree, ERROR_SUCCESS},
            Security::{
                Authorization::{
                    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
                },
                SECURITY_ATTRIBUTES,
            },
            System::Registry::{
                RegCloseKey, RegCreateKeyExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
                REG_OPTION_NON_VOLATILE,
            },
        };
        let wide = |s: &str| {
            std::ffi::OsStr::new(s)
                .encode_wide()
                .chain([0])
                .collect::<Vec<u16>>()
        };
        let sddl = wide(STORE_SDDL);
        let mut descriptor = std::ptr::null_mut();
        // SAFETY: a NUL-terminated SDDL string and valid out pointers; the
        // descriptor is freed with LocalFree once both keys exist.
        unsafe {
            if ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err("the backup store's permissions could not be prepared".into());
            }
            let attributes = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor,
                bInheritHandle: 0,
            };
            let mut status = ERROR_SUCCESS;
            for key in [self.root.to_string(), self.backups()] {
                let name = wide(&key);
                let mut handle: HKEY = std::ptr::null_mut();
                let mut disposition = 0u32;
                status = RegCreateKeyExW(
                    HKEY_LOCAL_MACHINE,
                    name.as_ptr(),
                    0,
                    std::ptr::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_READ,
                    &attributes,
                    &mut handle,
                    &mut disposition,
                );
                if status != ERROR_SUCCESS {
                    break;
                }
                RegCloseKey(handle);
            }
            LocalFree(descriptor as _);
            if status != ERROR_SUCCESS {
                return Err(format!(
                    "the backup store could not be created (error {status})"
                ));
            }
        }
        if self.trusted() {
            Ok(())
        } else {
            Err(crate::residue_elevated::STORE_UNTRUSTED.into())
        }
    }

    /// Saves (or, for the same cleanup, updates) a machine backup. `fresh`:
    /// the first save of a cleanup, which must not find a backup under the
    /// same stamp. Elevated only.
    #[cfg(windows)]
    pub fn save(&self, backup: &Backup, fresh: bool) -> Result<(), String> {
        use winreg::enums::{RegDisposition, RegType, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS};
        if !valid_stamp(&backup.stamp) {
            return Err(crate::residue_elevated::UNDO_GONE.into());
        }
        self.ensure()?;
        let (key, disposition) = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
            .create_subkey_with_flags(
                format!(r"{}\{}", self.backups(), backup.stamp),
                KEY_ALL_ACCESS,
            )
            .map_err(|e| format!("the backup could not be saved: {e}"))?;
        if fresh && matches!(disposition, RegDisposition::REG_OPENED_EXISTING_KEY) {
            return Err("a backup with this id already exists".into());
        }
        let json = serde_json::to_vec(backup).map_err(|e| e.to_string())?;
        key.set_raw_value(
            "backup",
            &winreg::RegValue {
                bytes: json,
                vtype: RegType::REG_BINARY,
            },
        )
        .map_err(|e| format!("the backup could not be saved: {e}"))
    }

    /// A machine backup, read only from a store only administrators can
    /// change. `Err` when the store is not trusted.
    #[cfg(windows)]
    pub fn load(&self, stamp: &str) -> Result<Option<Backup>, String> {
        use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
        if !valid_stamp(stamp) {
            return Err(crate::residue_elevated::UNDO_GONE.into());
        }
        let Ok(key) = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(format!(r"{}\{stamp}", self.backups()), KEY_READ)
        else {
            return Ok(None);
        };
        if !self.trusted() {
            return Err(crate::residue_elevated::STORE_UNTRUSTED.into());
        }
        let raw = key
            .get_raw_value("backup")
            .map_err(|_| crate::residue_elevated::BACKUP_DAMAGED.to_string())?;
        serde_json::from_slice::<Backup>(&raw.bytes)
            .ok()
            .filter(|b| b.stamp == stamp)
            .map(Some)
            .ok_or_else(|| crate::residue_elevated::BACKUP_DAMAGED.to_string())
    }

    /// The stamps with a machine backup, newest first.
    #[cfg(windows)]
    pub fn stamps(&self) -> Vec<String> {
        use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
        let mut stamps: Vec<String> = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(self.backups(), KEY_READ)
            .map(|k| k.enum_keys().flatten().filter(|s| valid_stamp(s)).collect())
            .unwrap_or_default();
        stamps.sort_by(|a, b| b.len().cmp(&a.len()).then(b.cmp(a)));
        stamps
    }

    /// Keeps the newest `keep` machine backups. Elevated only.
    #[cfg(windows)]
    pub fn prune(&self, keep: usize) {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        for stamp in self.stamps().into_iter().skip(keep) {
            let _ = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
                .delete_subkey_all(format!(r"{}\{stamp}", self.backups()));
        }
    }

    #[cfg(not(windows))]
    pub fn save(&self, _backup: &Backup, _fresh: bool) -> Result<(), String> {
        Err("machine backups exist only on Windows".into())
    }
    #[cfg(not(windows))]
    pub fn load(&self, _stamp: &str) -> Result<Option<Backup>, String> {
        Ok(None)
    }
    #[cfg(not(windows))]
    pub fn stamps(&self) -> Vec<String> {
        Vec::new()
    }
    #[cfg(not(windows))]
    pub fn prune(&self, _keep: usize) {}
}

// ---- the real registry -------------------------------------------------------------

pub struct WinRegistry;

#[cfg(not(windows))]
impl Registry for WinRegistry {
    fn subkeys(&self, _hive: Hive, _path: &str) -> Option<Vec<String>> {
        None
    }
    fn values(&self, _hive: Hive, _path: &str) -> Option<Vec<RawValue>> {
        None
    }
    fn key_exists(&self, _hive: Hive, _path: &str) -> bool {
        true
    }
    fn create_new_key(&self, _hive: Hive, _path: &str) -> std::io::Result<()> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    fn set_value(&self, _hive: Hive, _path: &str, _value: &RawValue) -> std::io::Result<()> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    fn delete_tree(&self, _hive: Hive, _path: &str) -> std::io::Result<()> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    fn delete_value(&self, _hive: Hive, _path: &str, _name: &str) -> std::io::Result<()> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
}

#[cfg(windows)]
mod live {
    use super::*;
    use winreg::enums::{
        RegDisposition, RegType, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, KEY_READ,
        KEY_SET_VALUE,
    };
    use winreg::RegKey;

    fn hkey(hive: Hive) -> RegKey {
        RegKey::predef(match hive {
            Hive::Hkcu => HKEY_CURRENT_USER,
            Hive::Hklm => HKEY_LOCAL_MACHINE,
        })
    }

    fn kind(raw: u32) -> Option<RegType> {
        use RegType::*;
        Some(match raw {
            0 => REG_NONE,
            1 => REG_SZ,
            2 => REG_EXPAND_SZ,
            3 => REG_BINARY,
            4 => REG_DWORD,
            5 => REG_DWORD_BIG_ENDIAN,
            6 => REG_LINK,
            7 => REG_MULTI_SZ,
            8 => REG_RESOURCE_LIST,
            9 => REG_FULL_RESOURCE_DESCRIPTOR,
            10 => REG_RESOURCE_REQUIREMENTS_LIST,
            11 => REG_QWORD,
            _ => return None,
        })
    }

    impl Registry for WinRegistry {
        fn subkeys(&self, hive: Hive, path: &str) -> Option<Vec<String>> {
            let key = hkey(hive).open_subkey_with_flags(path, KEY_READ).ok()?;
            key.enum_keys().collect::<Result<Vec<_>, _>>().ok()
        }
        fn values(&self, hive: Hive, path: &str) -> Option<Vec<RawValue>> {
            let key = hkey(hive).open_subkey_with_flags(path, KEY_READ).ok()?;
            key.enum_values()
                .map(|v| {
                    v.map(|(name, value)| RawValue {
                        name,
                        kind: value.vtype as u32,
                        bytes: value.bytes,
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .ok()
        }
        fn key_exists(&self, hive: Hive, path: &str) -> bool {
            match hkey(hive).open_subkey_with_flags(path, KEY_READ) {
                Ok(_) => true,
                Err(e) => e.kind() != std::io::ErrorKind::NotFound,
            }
        }
        fn create_new_key(&self, hive: Hive, path: &str) -> std::io::Result<()> {
            match hkey(hive).create_subkey_with_flags(path, KEY_ALL_ACCESS)? {
                (_, RegDisposition::REG_CREATED_NEW_KEY) => Ok(()),
                _ => Err(std::io::ErrorKind::AlreadyExists.into()),
            }
        }
        fn set_value(&self, hive: Hive, path: &str, value: &RawValue) -> std::io::Result<()> {
            let vtype = kind(value.kind).ok_or(std::io::ErrorKind::InvalidData)?;
            hkey(hive)
                .open_subkey_with_flags(path, KEY_SET_VALUE)?
                .set_raw_value(
                    &value.name,
                    &winreg::RegValue {
                        bytes: value.bytes.clone(),
                        vtype,
                    },
                )
        }
        fn delete_tree(&self, hive: Hive, path: &str) -> std::io::Result<()> {
            hkey(hive).delete_subkey_all(path)
        }
        fn delete_value(&self, hive: Hive, path: &str, name: &str) -> std::io::Result<()> {
            hkey(hive)
                .open_subkey_with_flags(path, KEY_SET_VALUE)?
                .delete_value(name)
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    /// An in-memory registry: (hive, lowercase path) -> (real path, values).
    #[derive(Default)]
    pub(crate) struct FakeRegistry {
        pub keys: RefCell<BTreeMap<(Hive, String), (String, Vec<RawValue>)>>,
        /// Paths whose values cannot be read.
        pub unreadable: Vec<String>,
    }

    impl FakeRegistry {
        pub fn key(&self, hive: Hive, path: &str, values: Vec<RawValue>) {
            let mut at = String::new();
            for part in path.split('\\') {
                at = join(&at, part);
                self.keys
                    .borrow_mut()
                    .entry((hive, at.to_lowercase()))
                    .or_insert_with(|| (at.clone(), Vec::new()));
            }
            self.keys
                .borrow_mut()
                .get_mut(&(hive, path.to_lowercase()))
                .unwrap()
                .1 = values;
        }
        pub fn has(&self, hive: Hive, path: &str) -> bool {
            self.keys
                .borrow()
                .contains_key(&(hive, path.to_lowercase()))
        }
        /// Everything under `path`, for before/after comparisons.
        pub fn dump(&self, hive: Hive, path: &str) -> Vec<(String, Vec<RawValue>)> {
            let lower = path.to_lowercase();
            self.keys
                .borrow()
                .iter()
                .filter(|((h, p), _)| {
                    *h == hive && (*p == lower || p.starts_with(&format!("{lower}\\")))
                })
                .map(|(_, v)| v.clone())
                .collect()
        }
    }

    impl Registry for FakeRegistry {
        fn subkeys(&self, hive: Hive, path: &str) -> Option<Vec<String>> {
            if !self.has(hive, path) {
                return None;
            }
            let prefix = format!("{}\\", path.to_lowercase());
            Some(
                self.keys
                    .borrow()
                    .iter()
                    .filter(|((h, p), _)| {
                        *h == hive && p.starts_with(&prefix) && !p[prefix.len()..].contains('\\')
                    })
                    .map(|(_, (real, _))| real.rsplit('\\').next().unwrap().to_string())
                    .collect(),
            )
        }
        fn values(&self, hive: Hive, path: &str) -> Option<Vec<RawValue>> {
            if self.unreadable.iter().any(|u| u.eq_ignore_ascii_case(path)) {
                return None;
            }
            self.keys
                .borrow()
                .get(&(hive, path.to_lowercase()))
                .map(|(_, v)| v.clone())
        }
        fn key_exists(&self, hive: Hive, path: &str) -> bool {
            self.has(hive, path)
        }
        fn create_new_key(&self, hive: Hive, path: &str) -> std::io::Result<()> {
            if self.has(hive, path) {
                return Err(std::io::ErrorKind::AlreadyExists.into());
            }
            self.key(hive, path, Vec::new());
            Ok(())
        }
        fn set_value(&self, hive: Hive, path: &str, value: &RawValue) -> std::io::Result<()> {
            let mut keys = self.keys.borrow_mut();
            let (_, values) = keys
                .get_mut(&(hive, path.to_lowercase()))
                .ok_or(std::io::ErrorKind::NotFound)?;
            values.retain(|v| !v.name.eq_ignore_ascii_case(&value.name));
            values.push(value.clone());
            Ok(())
        }
        fn delete_tree(&self, hive: Hive, path: &str) -> std::io::Result<()> {
            let lower = path.to_lowercase();
            self.keys.borrow_mut().retain(|(h, p), _| {
                !(*h == hive && (*p == lower || p.starts_with(&format!("{lower}\\"))))
            });
            Ok(())
        }
        fn delete_value(&self, hive: Hive, path: &str, name: &str) -> std::io::Result<()> {
            if let Some((_, values)) = self.keys.borrow_mut().get_mut(&(hive, path.to_lowercase()))
            {
                values.retain(|v| !v.name.eq_ignore_ascii_case(name));
            }
            Ok(())
        }
    }

    pub fn sz(name: &str, text: &str) -> RawValue {
        RawValue {
            name: name.into(),
            kind: REG_SZ,
            bytes: text
                .encode_utf16()
                .chain([0])
                .flat_map(u16::to_le_bytes)
                .collect(),
        }
    }

    /// Every type Windows has, including bytes a text type would not round
    /// trip through a string.
    fn odd_values() -> Vec<RawValue> {
        vec![
            sz("", "default"),
            sz("Path", r"C:\Program Files\Foo"),
            RawValue {
                name: "Expand".into(),
                kind: REG_EXPAND_SZ,
                bytes: "%ProgramFiles%\\Foo\0"
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes)
                    .collect(),
            },
            RawValue {
                name: "Multi".into(),
                kind: REG_MULTI_SZ,
                bytes: "a\0b\0\0"
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes)
                    .collect(),
            },
            RawValue {
                name: "Dword".into(),
                kind: REG_DWORD,
                bytes: vec![1, 0, 0, 0],
            },
            RawValue {
                name: "Qword".into(),
                kind: 11,
                bytes: vec![1, 2, 3, 4, 5, 6, 7, 8],
            },
            RawValue {
                name: "Binary".into(),
                kind: REG_BINARY,
                bytes: vec![0, 255, 1],
            },
            RawValue {
                name: "None".into(),
                kind: 0,
                bytes: vec![9],
            },
            // An odd byte count and no terminator in a text type.
            RawValue {
                name: "Odd".into(),
                kind: REG_SZ,
                bytes: vec![b'x', 0, b'y'],
            },
        ]
    }

    fn tree(reg: &FakeRegistry, hive: Hive, path: &str) {
        reg.key(hive, path, odd_values());
        reg.key(hive, &format!(r"{path}\Sub\Deep"), vec![sz("Deep", "1")]);
        reg.key(hive, &format!(r"{path}\Empty"), Vec::new());
    }

    #[test]
    fn a_key_tree_comes_back_byte_for_byte() {
        let reg = FakeRegistry::default();
        for (hive, path) in [
            (Hive::Hkcu, r"Software\SuperTool"),
            (Hive::Hklm, r"SOFTWARE\SuperTool"),
            (Hive::Hklm, r"SOFTWARE\WOW6432Node\SuperTool"),
        ] {
            tree(&reg, hive, path);
            let before = reg.dump(hive, path);
            let nodes = capture_tree(&reg, hive, path).unwrap();
            assert_eq!(nodes.len(), 4);
            assert!(nodes[0].sub.is_empty());
            let backup = KeyBackup {
                id: "k".into(),
                hive,
                path: path.into(),
                nodes,
            };
            // Through JSON, as it is stored.
            let backup: KeyBackup =
                serde_json::from_str(&serde_json::to_string(&backup).unwrap()).unwrap();
            reg.delete_tree(hive, path).unwrap();
            assert!(!reg.has(hive, path));
            assert_eq!(restore_key(&reg, &backup), Outcome::Restored);
            assert_eq!(reg.dump(hive, path), before, "{path}");
            // A second undo finds it there and leaves it alone.
            assert_eq!(restore_key(&reg, &backup), Outcome::Conflict);
        }
    }

    #[test]
    fn a_key_recreated_since_is_never_overwritten() {
        let reg = FakeRegistry::default();
        tree(&reg, Hive::Hklm, r"SOFTWARE\SuperTool");
        let backup = KeyBackup {
            id: "k".into(),
            hive: Hive::Hklm,
            path: r"SOFTWARE\SuperTool".into(),
            nodes: capture_tree(&reg, Hive::Hklm, r"SOFTWARE\SuperTool").unwrap(),
        };
        reg.delete_tree(Hive::Hklm, r"SOFTWARE\SuperTool").unwrap();
        // The program was installed again and wrote its own values.
        reg.key(Hive::Hklm, r"SOFTWARE\SuperTool", vec![sz("Path", "new")]);
        let now = reg.dump(Hive::Hklm, r"SOFTWARE\SuperTool");
        assert_eq!(restore_key(&reg, &backup), Outcome::Conflict);
        assert_eq!(reg.dump(Hive::Hklm, r"SOFTWARE\SuperTool"), now);

        // The same for a startup value.
        let run = RUN_KEYS[0];
        reg.key(Hive::Hkcu, run, vec![sz("Tool", "new.exe")]);
        let value = ValueBackup {
            id: "v".into(),
            hive: Hive::Hkcu,
            key: run.into(),
            value: sz("tool", "old.exe"),
        };
        assert_eq!(restore_value(&reg, &value), Outcome::Conflict);
        assert_eq!(
            reg.values(Hive::Hkcu, run).unwrap(),
            vec![sz("Tool", "new.exe")]
        );
        reg.delete_value(Hive::Hkcu, run, "Tool").unwrap();
        assert_eq!(restore_value(&reg, &value), Outcome::Restored);
    }

    /// A backup is input a program could have edited: nothing outside the
    /// places cleaning may touch is ever written from it.
    #[test]
    fn a_tampered_backup_cannot_write_outside_its_roots() {
        let reg = FakeRegistry::default();
        let nodes = vec![KeyNode {
            sub: String::new(),
            values: vec![sz("x", "y")],
        }];
        for (hive, path) in [
            (Hive::Hklm, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run"),
            (Hive::Hklm, r"SOFTWARE\Microsoft"),
            (Hive::Hklm, r"SOFTWARE\Classes\exefile"),
            (Hive::Hklm, r"SYSTEM\CurrentControlSet\Services\Evil"),
            (Hive::Hklm, r"SOFTWARE\Policies"),
            (Hive::Hklm, r"SOFTWARE\WOW6432Node"),
            (Hive::Hklm, r"SOFTWARE\WOW6432Node\Classes"),
            (Hive::Hklm, r"SOFTWARE\PC Tweaker Uninstaller"),
            (Hive::Hklm, r"SOFTWARE\Foo\Bar"),
            (Hive::Hklm, "SOFTWARE"),
            (Hive::Hkcu, r"Software\WOW6432Node\Foo"),
            (Hive::Hkcu, r"Software\Classes"),
            (Hive::Hkcu, r"Environment"),
            (Hive::Hklm, "SOFTWARE\\Fo\0o"),
        ] {
            let backup = KeyBackup {
                id: "k".into(),
                hive,
                path: path.into(),
                nodes: nodes.clone(),
            };
            assert_eq!(restore_key(&reg, &backup), Outcome::Refused, "{path}");
            assert!(!reg.has(hive, path), "{path}");
        }
        // Malformed trees: a child before its parent, empty parts, bad types.
        for bad in [
            vec![KeyNode {
                sub: "A".into(),
                values: Vec::new(),
            }],
            vec![
                KeyNode {
                    sub: String::new(),
                    values: Vec::new(),
                },
                KeyNode {
                    sub: r"A\B".into(),
                    values: Vec::new(),
                },
            ],
            vec![
                KeyNode {
                    sub: String::new(),
                    values: Vec::new(),
                },
                KeyNode {
                    sub: r"\A".into(),
                    values: Vec::new(),
                },
            ],
            vec![KeyNode {
                sub: String::new(),
                values: vec![RawValue {
                    name: "x".into(),
                    kind: 12,
                    bytes: Vec::new(),
                }],
            }],
        ] {
            let backup = KeyBackup {
                id: "k".into(),
                hive: Hive::Hklm,
                path: r"SOFTWARE\SuperTool".into(),
                nodes: bad,
            };
            assert_eq!(restore_key(&reg, &backup), Outcome::Refused);
        }
        // Values: only this user's Run and RunOnce.
        reg.key(Hive::Hklm, RUN_KEYS[0], Vec::new());
        for (hive, key) in [
            (Hive::Hklm, RUN_KEYS[0]),
            (
                Hive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Policies\Explorer",
            ),
            (Hive::Hkcu, r"Environment"),
        ] {
            let value = ValueBackup {
                id: "v".into(),
                hive,
                key: key.into(),
                value: sz("Evil", "evil.exe"),
            };
            assert_eq!(restore_value(&reg, &value), Outcome::Refused, "{key}");
        }
        // Unknown fields are refused when a backup is read.
        assert!(serde_json::from_str::<Backup>(
            r#"{"version":1,"stamp":"1","program":"x","keys":[],"values":[],"recycled":[],"extra":1}"#
        )
        .is_err());
    }

    #[test]
    fn what_cannot_be_read_whole_is_not_backed_up() {
        let mut reg = FakeRegistry::default();
        tree(&reg, Hive::Hklm, r"SOFTWARE\SuperTool");
        assert!(capture_tree(&reg, Hive::Hklm, r"SOFTWARE\Missing").is_none());
        reg.unreadable = vec![r"SOFTWARE\SuperTool\Sub\Deep".into()];
        assert!(capture_tree(&reg, Hive::Hklm, r"SOFTWARE\SuperTool").is_none());
        // Beyond the caps.
        let reg = FakeRegistry::default();
        let big = RawValue {
            name: "big".into(),
            kind: REG_BINARY,
            bytes: vec![0; MAX_BYTES + 1],
        };
        reg.key(Hive::Hklm, r"SOFTWARE\Big", vec![big]);
        assert!(capture_tree(&reg, Hive::Hklm, r"SOFTWARE\Big").is_none());
    }

    #[test]
    fn stamps_are_numbers_only() {
        assert!(valid_stamp("1793750000123"));
        assert!(valid_stamp(&new_stamp()));
        for bad in [
            "",
            "123",
            "17937500001234567",
            "1793750000.12",
            r"..\..\x",
            "1793750000-1",
        ] {
            assert!(!valid_stamp(bad), "{bad}");
        }
    }

    #[test]
    fn the_reg_copy_reads_like_regedit() {
        let reg = FakeRegistry::default();
        tree(&reg, Hive::Hklm, r"SOFTWARE\SuperTool");
        let mut backup = Backup::new("1793750000123", "SuperTool");
        backup.keys.push(KeyBackup {
            id: "k".into(),
            hive: Hive::Hklm,
            path: r"SOFTWARE\SuperTool".into(),
            nodes: capture_tree(&reg, Hive::Hklm, r"SOFTWARE\SuperTool").unwrap(),
        });
        let text = reg_text(&backup);
        assert!(text.starts_with("Windows Registry Editor Version 5.00\r\n"));
        assert!(text.contains("[HKEY_LOCAL_MACHINE\\SOFTWARE\\SuperTool\\Sub\\Deep]\r\n"));
        assert!(text.contains("@=\"default\"\r\n"));
        assert!(text.contains("\"Path\"=\"C:\\\\Program Files\\\\Foo\"\r\n"));
        assert!(text.contains("\"Dword\"=dword:00000001\r\n"));
        assert!(text.contains("\"Qword\"=hex(b):01,02,03,04,05,06,07,08\r\n"));
        // Text that would not round trip as a string is written as hex.
        assert!(text.contains("\"Odd\"=hex(1):78,00,79\r\n"));
    }

    struct FakeBin {
        items: Vec<BinItem>,
        restored: RefCell<Vec<PathBuf>>,
    }
    impl Bin for FakeBin {
        fn items(&self) -> Option<Vec<BinItem>> {
            Some(self.items.clone())
        }
        fn restore(&self, item: &BinItem) -> Result<(), BinError> {
            self.restored.borrow_mut().push(item.path.clone());
            Ok(())
        }
    }

    #[test]
    fn recycled_items_are_matched_by_path_and_time_and_never_overwrite() {
        let temp = std::env::temp_dir().join(format!("pcu-bin-{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();
        let gone = temp.join("Gone");
        let back = temp.join("Back");
        std::fs::create_dir_all(&back).unwrap();
        let item = |index, path: &Path, deleted| BinItem {
            index,
            path: path.to_path_buf(),
            deleted,
        };
        let bin = FakeBin {
            items: vec![
                item(0, &gone, 500),   // too old: another deletion of that path
                item(1, &gone, 1_001), // ours
                item(2, &back, 1_001),
                item(3, &temp.join("Other"), 1_001),
            ],
            restored: RefCell::new(Vec::new()),
        };
        let rec = |path: &Path| Recycled {
            id: path.to_string_lossy().into_owned(),
            path: path.to_string_lossy().into_owned(),
            from: 1_000,
            to: 1_002,
        };
        let never = temp.join("Never");
        let refused = temp.join("Refused");
        let out = restore_recycled(
            &bin,
            &[rec(&gone), rec(&back), rec(&never), rec(&refused)],
            &|p| !p.ends_with("Refused"),
        );
        let outcome = |p: &Path| out.iter().find(|(path, _)| Path::new(path) == p).unwrap().1;
        assert_eq!(outcome(&gone), Outcome::Restored);
        // Something is at the path again: it stays.
        assert_eq!(outcome(&back), Outcome::Conflict);
        assert_eq!(outcome(&never), Outcome::Missing);
        assert_eq!(outcome(&refused), Outcome::Refused);
        assert_eq!(*bin.restored.borrow(), vec![gone.clone()]);
        // Reverse order of recycling: the last recycled comes back first.
        assert_eq!(Path::new(&out[0].0), refused);
        std::fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn user_backups_are_saved_loaded_and_pruned_to_the_newest() {
        let dir = std::env::temp_dir().join(format!("pcu-user-backups-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let mut stamps = Vec::new();
        for i in 0..12u64 {
            let stamp = (1_793_750_000_000 + i).to_string();
            let mut backup = Backup::new(&stamp, "SuperTool");
            backup.values.push(ValueBackup {
                id: "v".into(),
                hive: Hive::Hkcu,
                key: RUN_KEYS[0].into(),
                value: sz("Tool", "tool.exe"),
            });
            save_user(&dir, &backup).unwrap();
            assert_eq!(load_user(&dir, &stamp).unwrap(), Some(backup.clone()));
            save_readable_copy(&dir, &backup).unwrap();
            stamps.push(stamp);
        }
        assert!(dir.join(format!("{}-user.reg", stamps[0])).is_file());
        prune_user(&dir, KEEP);
        let left = user_stamps(&dir);
        assert_eq!(left.len(), KEEP);
        assert_eq!(left[0], stamps[11]);
        assert_eq!(load_user(&dir, &stamps[0]).unwrap(), None);
        assert!(load_user(&dir, r"..\..\x").is_err());
        // A damaged file is not guessed at.
        std::fs::write(dir.join(format!("{}-user.json", stamps[11])), b"{").unwrap();
        assert!(load_user(&dir, &stamps[11]).is_err());
        prune_user(&dir, 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The real registry, with fixtures made by reg.exe so the test does not
    /// grade its own writes: export, back up, delete, restore, export again,
    /// and the two exports must be identical.
    #[cfg(windows)]
    #[test]
    fn the_real_registry_round_trips_byte_for_byte() {
        let reg_exe = |args: &[&str]| {
            std::process::Command::new("reg.exe")
                .args(args)
                .output()
                .is_ok_and(|o| o.status.success())
        };
        let name = format!("PCU-Backup-Test-{}", std::process::id());
        let path = format!(r"Software\{name}");
        let full = format!(r"HKCU\{path}");
        reg_exe(&["delete", &full, "/f"]);
        let sub = format!(r"{full}\Sub\Deep");
        let made = [
            vec!["add", &full, "/ve", "/d", "default", "/f"],
            vec![
                "add",
                &full,
                "/v",
                "Sz",
                "/t",
                "REG_SZ",
                "/d",
                r"C:\Program Files\Foo",
                "/f",
            ],
            vec![
                "add",
                &full,
                "/v",
                "Expand",
                "/t",
                "REG_EXPAND_SZ",
                "/d",
                "^%ProgramFiles^%\\Foo",
                "/f",
            ],
            vec![
                "add",
                &full,
                "/v",
                "Multi",
                "/t",
                "REG_MULTI_SZ",
                "/d",
                r"a\0b",
                "/f",
            ],
            vec![
                "add",
                &full,
                "/v",
                "Dword",
                "/t",
                "REG_DWORD",
                "/d",
                "7",
                "/f",
            ],
            vec![
                "add",
                &full,
                "/v",
                "Qword",
                "/t",
                "REG_QWORD",
                "/d",
                "9",
                "/f",
            ],
            vec![
                "add",
                &full,
                "/v",
                "Binary",
                "/t",
                "REG_BINARY",
                "/d",
                "00ff10",
                "/f",
            ],
            vec!["add", &full, "/v", "None", "/t", "REG_NONE", "/f"],
            vec!["add", &sub, "/v", "Deep", "/t", "REG_SZ", "/d", "1", "/f"],
            vec!["add", &format!(r"{full}\Empty"), "/f"],
        ]
        .iter()
        .all(|args| reg_exe(args));
        assert!(made, "reg.exe could not create the fixture");
        let dir = std::env::temp_dir();
        let export = |file: &str| {
            let out = dir.join(format!("{name}-{file}.reg"));
            assert!(reg_exe(&["export", &full, &out.to_string_lossy(), "/y"]));
            let bytes = std::fs::read(&out).unwrap();
            std::fs::remove_file(&out).ok();
            bytes
        };
        let before = export("before");
        let reg = WinRegistry;
        let backup = KeyBackup {
            id: "k".into(),
            hive: Hive::Hkcu,
            path: path.clone(),
            nodes: capture_tree(&reg, Hive::Hkcu, &path).unwrap(),
        };
        reg.delete_tree(Hive::Hkcu, &path).unwrap();
        assert!(!reg_exe(&["query", &full]), "the key was not deleted");
        assert_eq!(restore_key(&reg, &backup), Outcome::Restored);
        let after = export("after");
        assert_eq!(before, after);
        // Recreated since: restore leaves it alone.
        assert_eq!(restore_key(&reg, &backup), Outcome::Conflict);
        reg_exe(&["delete", &full, "/f"]);
    }

    /// The machine store, created with its own access list, holds a backup
    /// only administrators can change. Runs where the tests are elevated (CI).
    #[cfg(windows)]
    #[test]
    fn the_machine_store_is_admin_only_and_round_trips() {
        if !crate::elevation::is_elevated() {
            return;
        }
        let root: &'static str =
            Box::leak(format!(r"SOFTWARE\PCU-Store-Test-{}", std::process::id()).into_boxed_str());
        let store = MachineStore { root };
        let mut backup = Backup::new("1793750000123", "SuperTool");
        backup.recycled.push(Recycled {
            id: "r".into(),
            path: r"C:\ProgramData\SuperTool".into(),
            from: 1,
            to: 2,
        });
        store.save(&backup, true).unwrap();
        assert!(store.trusted());
        assert_eq!(store.load("1793750000123").unwrap(), Some(backup.clone()));
        // The same stamp cannot be saved fresh twice.
        assert!(store.save(&backup, true).is_err());
        assert!(store.save(&backup, false).is_ok());
        for i in 0..12u64 {
            store
                .save(
                    &Backup::new(&(1_793_750_001_000 + i).to_string(), "x"),
                    true,
                )
                .unwrap();
        }
        store.prune(KEEP);
        assert_eq!(store.stamps().len(), KEEP);
        assert_eq!(store.load("1793750000123").unwrap(), None);
        let _ = WinRegistry.delete_tree(Hive::Hklm, root);
    }
}
