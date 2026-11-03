//! The elevated side of leftover cleaning: one child per cleanup for the
//! items of all users, and one per Undo of such a cleanup.
//!
//! `--elevated-clean-leftovers <request> <fingerprint> <pipe>`: the request
//! names the program and the item ids, never a path or a key. The child
//! treats it as hostile: it scans again from places Windows records (never
//! the user's environment), re-gates the install folder against the program
//! list it reads itself, requires the fingerprint of the confirmed plan,
//! refuses items of a user, and acts only on ids its own scan reports. Then
//! restore point, backup into the HKLM store, the change, and the report
//! over the parent's pipe. Nothing is read from or written to user folders.
//!
//! `--elevated-undo-leftovers <stamp> <pipe>`: a number, and the HKLM store
//! only, whose access list lets nobody but SYSTEM and Administrators change
//! it. A file the user can edit is never imported as administrator.
//!
//! Pro is a business gate in the parent, not a security boundary: the child
//! checks no licence.

use crate::residue::{self, Actor, CleanResult, Host, Identity, Mode};
use crate::residue_backup::{self as rb, Backup, Bin, Hive, Outcome, Registry, STORE};
use crate::restore_point::RestorePointOutcome;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const STORE_UNTRUSTED: &str =
    "The backup store of this app can be changed by other accounts, so it was not used.";
pub const CHILD_CHANGED: &str =
    "The leftovers changed while Windows asked for permission, so nothing was changed. Preview the cleanup again.";
pub const UNDO_GONE: &str = "This cleanup's backup is no longer kept, so it cannot be undone.";
pub const BACKUP_DAMAGED: &str = "This cleanup's backup is damaged, so it was not used.";
pub const ADMIN_NOT_RUN: &str =
    "The administrator step did not run, so nothing was changed for all users";
pub const REPORT_UNREADABLE: &str =
    "The administrator step finished, but its report could not be read.";

/// The largest request the child reads, and the most ids in it.
const REQUEST_LIMIT: usize = 16 * 1024;
const MAX_IDS: usize = 64;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CleanRequest {
    pub name: String,
    pub anchor: Option<String>,
    /// "registry" or "manual".
    pub mode: String,
    pub item_ids: Vec<String>,
    pub stamp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChildReport {
    pub removed: Vec<String>,
    pub failed: Vec<String>,
    pub freed_kb: u64,
    pub restore_point: Option<RestorePointOutcome>,
    /// Why nothing was done, when the child refused.
    pub refused: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemOutcome {
    pub id: String,
    pub outcome: Outcome,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UndoReport {
    pub outcomes: Vec<ItemOutcome>,
    /// Why (part of) the undo did not run.
    pub refused: Option<String>,
}

impl UndoReport {
    pub fn add(&mut self, items: impl IntoIterator<Item = (String, Outcome)>) {
        self.outcomes.extend(
            items
                .into_iter()
                .map(|(id, outcome)| ItemOutcome { id, outcome }),
        );
    }
}

fn valid_id(id: &str) -> bool {
    id.len() == 16 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn plain(text: &str, max: usize) -> bool {
    text.chars().count() <= max && !text.chars().any(char::is_control)
}

/// The request as the child accepts it, or `None`: then nothing runs.
pub fn parse_request(raw: &str) -> Option<(Identity, CleanRequest)> {
    if raw.len() > REQUEST_LIMIT {
        return None;
    }
    let request: CleanRequest = serde_json::from_str(raw).ok()?;
    let mode = match request.mode.as_str() {
        "registry" => Mode::Registry,
        "manual" => Mode::Manual,
        _ => return None,
    };
    let ok = !request.item_ids.is_empty()
        && request.item_ids.len() <= MAX_IDS
        && request.item_ids.iter().all(|id| valid_id(id))
        && rb::valid_stamp(&request.stamp)
        && plain(&request.name, 256)
        && request.anchor.as_deref().is_none_or(|a| plain(a, 260));
    ok.then(|| {
        (
            Identity {
                name: request.name.clone(),
                anchor: request.anchor.clone(),
                mode,
            },
            request,
        )
    })
}

/// What the child decides and does, given where it reads from and how it
/// acts (fakes in the tests).
pub(crate) fn clean_child_with(
    identity: &Identity,
    request: &CleanRequest,
    confirmed: &str,
    host: &dyn Host,
    actor: &dyn Actor,
    restore_point: &dyn Fn() -> RestorePointOutcome,
) -> ChildReport {
    let found = residue::scan_items(identity, host);
    if residue::machine_fingerprint(identity, &found, &request.item_ids) != confirmed {
        return ChildReport {
            failed: request.item_ids.clone(),
            refused: Some(CHILD_CHANGED.to_string()),
            ..ChildReport::default()
        };
    }
    let mut result = CleanResult::default();
    let selected = residue::machine_select(&found, &request.item_ids, &mut result);
    let mut report = ChildReport::default();
    if !selected.is_empty() {
        report.restore_point = Some(restore_point());
        let mut backup = Backup::new(&request.stamp, &identity.name);
        residue::apply(&selected, &mut backup, actor, &mut result);
    }
    report.removed = result.removed;
    report.failed = result.failed;
    report.freed_kb = result.freed_kb;
    report
}

/// Puts back what an elevated cleanup removed: keys of all users only, and
/// files only into folders of all users.
pub(crate) fn undo_machine_with(
    backup: &Backup,
    reg: &dyn Registry,
    bin: &dyn Bin,
    roots: &[PathBuf],
) -> UndoReport {
    let mut report = UndoReport::default();
    report.add(rb::restore_recycled(bin, &backup.recycled, &|p: &Path| {
        roots.iter().any(|root| residue::under(p, root))
    }));
    report.add(
        backup
            .values
            .iter()
            .map(|v| (v.id.clone(), Outcome::Refused)),
    );
    report.add(backup.keys.iter().rev().map(|k| {
        let outcome = if k.hive == Hive::Hklm {
            rb::restore_key(reg, k)
        } else {
            Outcome::Refused
        };
        (k.id.clone(), outcome)
    }));
    report
}

/// The child's actor: the live registry, the Recycle Bin, the HKLM store.
struct MachineActor;

impl Actor for MachineActor {
    fn registry(&self) -> &dyn Registry {
        &rb::WinRegistry
    }
    fn recycle(&self, path: &Path) -> Option<u64> {
        residue::recycle_real(path)
    }
    fn save(&self, backup: &Backup, fresh: bool) -> Result<(), String> {
        STORE.save(backup, fresh)
    }
}

fn send<T: Serialize>(pipe: &str, report: &T) -> i32 {
    let bytes = serde_json::to_vec(report).unwrap_or_default();
    match crate::elevation::send_report_bytes(pipe, &bytes) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// `--elevated-clean-leftovers`. Exit 2: the request was refused and
/// nothing ran.
pub fn run_clean_child(request: &str, confirmed: &str, pipe: &str) -> i32 {
    let Some((identity, request)) = parse_request(request) else {
        return 2;
    };
    let report = clean_child_with(
        &identity,
        &request,
        confirmed,
        &residue::MACHINE_SIDE,
        &MachineActor,
        &crate::restore_point::create_restore_point,
    );
    STORE.prune(rb::KEEP);
    send(pipe, &report)
}

/// `--elevated-undo-leftovers`. Exit 2: not a backup id, nothing ran.
pub fn run_undo_child(stamp: &str, pipe: &str) -> i32 {
    if !rb::valid_stamp(stamp) {
        return 2;
    }
    let report = match STORE.load(stamp) {
        Ok(Some(backup)) => undo_machine_with(
            &backup,
            &rb::WinRegistry,
            &rb::WinBin::new(),
            &residue::MACHINE_SIDE.machine_roots(),
        ),
        Ok(None) => UndoReport {
            refused: Some(UNDO_GONE.to_string()),
            ..UndoReport::default()
        },
        Err(e) => UndoReport {
            refused: Some(e),
            ..UndoReport::default()
        },
    };
    send(pipe, &report)
}

fn elevated(flag: &str, values: &[&str]) -> Result<Vec<u8>, String> {
    crate::elevation::run_elevated_args(flag, values).map_err(|e| {
        crate::applog::line(&format!("{flag} did not run: {e}"));
        format!("{ADMIN_NOT_RUN} ({e}).")
    })
}

fn report_from<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_slice(bytes).map_err(|_| {
        crate::applog::line(&format!(
            "leftover report unreadable, {} bytes",
            bytes.len()
        ));
        REPORT_UNREADABLE.to_string()
    })
}

/// The parent's half of a cleanup of items of all users: one UAC prompt.
pub fn clean_machine(
    identity: &Identity,
    ids: &[String],
    confirmed: &str,
    stamp: &str,
) -> Result<ChildReport, String> {
    let mode = match identity.mode {
        Mode::Registry => "registry",
        Mode::Manual => "manual",
        // The sweep finds only this user's startup entries.
        Mode::Sweep => return Err(CHILD_CHANGED.to_string()),
    };
    let request = serde_json::to_string(&CleanRequest {
        name: identity.name.clone(),
        anchor: identity.anchor.clone(),
        mode: mode.to_string(),
        item_ids: ids.to_vec(),
        stamp: stamp.to_string(),
    })
    .map_err(|e| e.to_string())?;
    let report: ChildReport = report_from(&elevated(
        "--elevated-clean-leftovers",
        &[&request, confirmed],
    )?)?;
    crate::applog::line(&format!(
        "elevated leftover cleanup: {} removed, {} failed, refused: {}",
        report.removed.len(),
        report.failed.len(),
        report.refused.is_some()
    ));
    Ok(report)
}

/// The parent's half of an Undo of items of all users: one UAC prompt.
pub fn undo_machine(stamp: &str) -> Result<UndoReport, String> {
    report_from(&elevated("--elevated-undo-leftovers", &[stamp])?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::residue::tests::{sandbox, Fake, Sandbox};
    use crate::residue_backup::tests::{sz, FakeRegistry};
    use crate::residue_backup::BinItem;
    use std::cell::RefCell;

    struct FakeActor<'a> {
        reg: &'a FakeRegistry,
        saved: RefCell<Vec<Backup>>,
        recycled: RefCell<Vec<PathBuf>>,
    }
    impl Actor for FakeActor<'_> {
        fn registry(&self) -> &dyn Registry {
            self.reg
        }
        fn recycle(&self, path: &Path) -> Option<u64> {
            self.recycled.borrow_mut().push(path.to_path_buf());
            Some(1)
        }
        fn save(&self, backup: &Backup, _fresh: bool) -> Result<(), String> {
            self.saved.borrow_mut().push(backup.clone());
            Ok(())
        }
    }
    fn actor(reg: &FakeRegistry) -> FakeActor<'_> {
        FakeActor {
            reg,
            saved: RefCell::new(Vec::new()),
            recycled: RefCell::new(Vec::new()),
        }
    }
    fn no_point() -> RestorePointOutcome {
        RestorePointOutcome::Skipped {
            reason: "test".into(),
        }
    }

    fn request(identity: &Identity, ids: &[String]) -> CleanRequest {
        CleanRequest {
            name: identity.name.clone(),
            anchor: identity.anchor.clone(),
            mode: "registry".into(),
            item_ids: ids.to_vec(),
            stamp: "1767225600000".into(),
        }
    }

    /// A machine with SuperTool's leftovers in both scopes.
    fn machine(tag: &str) -> (Sandbox, Fake, Identity) {
        let (sb, mut host) = sandbox(tag);
        host.machine = vec![sb.root.join("programdata")];
        std::fs::create_dir_all(sb.root.join("programdata").join("SuperTool")).unwrap();
        host.data.push((
            sb.root.join("programdata"),
            crate::residue_rules::Kind::AppData,
        ));
        std::fs::create_dir_all(sb.root.join("roaming").join("SuperTool")).unwrap();
        host.reg
            .key(Hive::Hklm, r"Software\SuperTool", vec![sz("v", "1")]);
        host.reg
            .key(Hive::Hkcu, r"Software\SuperTool", vec![sz("v", "2")]);
        let identity = Identity {
            name: "SuperTool".into(),
            anchor: None,
            mode: Mode::Registry,
        };
        (sb, host, identity)
    }

    #[test]
    fn the_child_acts_only_on_machine_items_its_own_scan_reports() {
        let (_sb, host, identity) = machine("child-ok");
        let found = residue::scan_items(&identity, &host);
        let machine_ids: Vec<String> = found
            .iter()
            .filter(|f| f.item.scope == "machine")
            .map(|f| f.item.id.clone())
            .chain(["feedfacefeedface".to_string()])
            .collect();
        assert_eq!(machine_ids.len(), 3);
        let confirmed = residue::machine_fingerprint(&identity, &found, &machine_ids);
        let act = actor(&host.reg);
        let report = clean_child_with(
            &identity,
            &request(&identity, &machine_ids),
            &confirmed,
            &host,
            &act,
            &no_point,
        );
        assert!(report.refused.is_none());
        assert_eq!(report.removed.len(), 2);
        // An id its own scan does not report is never acted on.
        assert_eq!(report.failed, vec!["feedfacefeedface".to_string()]);
        assert!(!host.reg.has(Hive::Hklm, r"Software\SuperTool"));
        assert!(host.reg.has(Hive::Hkcu, r"Software\SuperTool"));
        assert_eq!(act.recycled.borrow().len(), 1);
        assert!(report.restore_point.is_some());
        // Backed up before the change: the first save already held the key.
        assert_eq!(act.saved.borrow()[0].keys.len(), 1);

        // Items of a user are refused even when the fingerprint matches.
        let (_sb, host, identity) = machine("child-user");
        let found = residue::scan_items(&identity, &host);
        let ids: Vec<String> = found.iter().map(|f| f.item.id.clone()).collect();
        let confirmed = residue::machine_fingerprint(&identity, &found, &ids);
        let act = actor(&host.reg);
        let report = clean_child_with(
            &identity,
            &request(&identity, &ids),
            &confirmed,
            &host,
            &act,
            &no_point,
        );
        assert!(host.reg.has(Hive::Hkcu, r"Software\SuperTool"));
        let user_ids: Vec<&String> = found
            .iter()
            .filter(|f| f.item.scope == "user")
            .map(|f| &f.item.id)
            .collect();
        assert_eq!(user_ids.len(), 2);
        assert!(user_ids.iter().all(|id| report.failed.contains(id)));
        assert!(user_ids.iter().all(|id| !report.removed.contains(id)));
    }

    #[test]
    fn a_tampered_fingerprint_or_a_changed_machine_changes_nothing() {
        let (_sb, mut host, identity) = machine("child-tamper");
        let found = residue::scan_items(&identity, &host);
        let ids: Vec<String> = found
            .iter()
            .filter(|f| f.item.scope == "machine")
            .map(|f| f.item.id.clone())
            .collect();
        let confirmed = residue::machine_fingerprint(&identity, &found, &ids);
        let act = actor(&host.reg);
        for (fingerprint, wanted) in [
            ("0".repeat(64), ids.clone()),
            (
                confirmed.clone(),
                vec![ids[0].clone(), "0123456789abcdef".into()],
            ),
        ] {
            let report = clean_child_with(
                &identity,
                &request(&identity, &wanted),
                &fingerprint,
                &host,
                &act,
                &no_point,
            );
            assert_eq!(report.refused.as_deref(), Some(CHILD_CHANGED));
            assert!(report.removed.is_empty());
        }
        // The machine changed between the preview and the prompt.
        host.reg.key(Hive::Hklm, r"Software\SuperTool\New", vec![]);
        let changed = residue::scan_items(&identity, &host);
        assert_eq!(changed.len(), found.len());
        // The key is still found and still cleanable, so the plan holds; but
        // a key a program put back (now blocked) would change it:
        host.reg.key(
            Hive::Hklm,
            r"Software\SuperTool",
            vec![sz("Path", r"C:\Apps\Other\x.exe")],
        );
        host.env.others = vec![Some(r"C:\Apps\Other".into())];
        let report = clean_child_with(
            &identity,
            &request(&identity, &ids),
            &confirmed,
            &host,
            &act,
            &no_point,
        );
        assert_eq!(report.refused.as_deref(), Some(CHILD_CHANGED));
        assert!(host.reg.has(Hive::Hklm, r"Software\SuperTool"));
        assert!(act.saved.borrow().is_empty());
        assert!(act.recycled.borrow().is_empty());
    }

    #[test]
    fn the_request_is_refused_unless_it_is_exactly_what_the_parent_sends() {
        let good = r#"{"name":"SuperTool","anchor":null,"mode":"registry","itemIds":["0123456789abcdef"],"stamp":"1767225600000"}"#;
        assert!(parse_request(good).is_some());
        for bad in [
            // Unknown fields, such as a path to act on.
            r#"{"name":"S","anchor":null,"mode":"registry","itemIds":["0123456789abcdef"],"stamp":"1767225600000","path":"C:\\Windows"}"#,
            r#"{"name":"S","anchor":null,"mode":"sweep","itemIds":["0123456789abcdef"],"stamp":"1767225600000"}"#,
            r#"{"name":"S","anchor":null,"mode":"registry","itemIds":["..\\..\\x"],"stamp":"1767225600000"}"#,
            r#"{"name":"S","anchor":null,"mode":"registry","itemIds":[],"stamp":"1767225600000"}"#,
            r#"{"name":"S","anchor":null,"mode":"registry","itemIds":["0123456789abcdef"],"stamp":"..\\x"}"#,
            r#"{"name":"S\u0007","anchor":null,"mode":"registry","itemIds":["0123456789abcdef"],"stamp":"1767225600000"}"#,
            "not json",
            "",
        ] {
            assert!(parse_request(bad).is_none(), "{bad}");
        }
        let many: Vec<String> = (0..65).map(|i| format!("{i:016x}")).collect();
        let request = CleanRequest {
            name: "S".into(),
            anchor: None,
            mode: "registry".into(),
            item_ids: many,
            stamp: "1767225600000".into(),
        };
        assert!(parse_request(&serde_json::to_string(&request).unwrap()).is_none());
        let huge = good.replace("SuperTool", &"x".repeat(REQUEST_LIMIT));
        assert!(parse_request(&huge).is_none());
    }

    struct NoBin;
    impl Bin for NoBin {
        fn items(&self) -> Option<Vec<BinItem>> {
            Some(Vec::new())
        }
        fn restore(&self, _item: &BinItem) -> Result<(), rb::BinError> {
            Err(rb::BinError::Other)
        }
    }

    #[test]
    fn the_elevated_undo_writes_only_machine_keys_and_machine_folders() {
        let reg = FakeRegistry::default();
        let key = |hive: Hive, path: &str| rb::KeyBackup {
            id: format!("{}:{path}", hive.short()),
            hive,
            path: path.into(),
            nodes: vec![rb::KeyNode {
                sub: String::new(),
                values: vec![sz("v", "1")],
            }],
        };
        let mut backup = Backup::new("1767225600000", "SuperTool");
        backup.keys = vec![
            key(Hive::Hklm, r"Software\SuperTool"),
            key(Hive::Hkcu, r"Software\SuperTool"),
            key(Hive::Hklm, r"Software\Microsoft"),
        ];
        backup.recycled = vec![rb::Recycled {
            id: "user-file".into(),
            path: r"C:\Users\me\AppData\Roaming\SuperTool".into(),
            from: 1,
            to: 2,
        }];
        let report = undo_machine_with(&backup, &reg, &NoBin, &[PathBuf::from(r"C:\ProgramData")]);
        let outcome = |id: &str| report.outcomes.iter().find(|o| o.id == id).unwrap().outcome;
        assert_eq!(outcome(r"HKLM:Software\SuperTool"), Outcome::Restored);
        assert_eq!(outcome(r"HKCU:Software\SuperTool"), Outcome::Refused);
        assert_eq!(outcome(r"HKLM:Software\Microsoft"), Outcome::Refused);
        assert_eq!(outcome("user-file"), Outcome::Refused);
        assert!(reg.has(Hive::Hklm, r"Software\SuperTool"));
        assert!(!reg.has(Hive::Hkcu, r"Software\SuperTool"));
        assert!(!reg.has(Hive::Hklm, r"Software\Microsoft"));
    }
}
