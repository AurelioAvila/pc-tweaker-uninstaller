//! Programs that were removed earlier.
//!
//! On every launch and every list refresh the listable entries are compared
//! with the previous snapshot (`inventory-snapshot.json`). What disappeared,
//! and was not simply upgraded in place, goes into `removed-programs.json`
//! (the last 50, with the time and how: through this app or outside it), so
//! its leftovers can be looked for later. This app's own removals are written
//! there too, which is what lets them be scanned again after a restart.
//!
//! Everything stays on this PC, like the ledger. The files hold only what the
//! uninstall registry key already held; a damaged file is ignored, never
//! trusted and never a reason to fail.

use crate::programs::RawEntry;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const SNAPSHOT_FILE: &str = "inventory-snapshot.json";
const STORE_FILE: &str = "removed-programs.json";
pub const CAP: usize = 50;

static WRITE: Mutex<()> = Mutex::new(());

/// One listable program as the snapshot keeps it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapEntry {
    pub source: String,
    pub key: String,
    pub name: String,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
    /// Folder of the executable DisplayIcon points at.
    pub icon_dir: Option<String>,
    /// Folder of the executable UninstallString runs.
    pub uninstall_dir: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    taken_at: u64,
    entries: Vec<SnapEntry>,
}

/// A removed program: what it was, and when and how it went.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub rid: String,
    pub source: String,
    pub key: String,
    pub name: String,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
    pub icon_dir: Option<String>,
    pub uninstall_dir: Option<String>,
    pub removed_at: u64,
    /// "app" or "outside".
    pub how: String,
    /// The outside-removal prompt was answered or dismissed.
    pub seen: bool,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Store {
    records: Vec<Record>,
    /// First-launch sweep: absent = never looked, "new" = card to show,
    /// "done" = shown or reviewed.
    #[serde(default)]
    sweep: Option<String>,
}

/// What the webview gets: no paths.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemovedView {
    pub rid: String,
    pub name: String,
    pub publisher: Option<String>,
    pub removed_at: u64,
    pub how: String,
    pub seen: bool,
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn folder_of(exe: Option<PathBuf>) -> Option<String> {
    exe?.parent().map(|p| p.to_string_lossy().into_owned())
}

pub fn entry_from_raw(source: &str, raw: &RawEntry) -> SnapEntry {
    let icon = raw.display_icon.as_deref().map(|icon| {
        // "path,index", sometimes quoted.
        let icon = icon.trim().trim_matches('"');
        let icon = icon.rsplit_once(',').map_or(icon, |(path, _)| path);
        icon.trim().trim_matches('"').to_string()
    });
    SnapEntry {
        source: source.to_string(),
        key: raw.key_name.clone(),
        name: raw.display_name.clone().unwrap_or_default(),
        publisher: raw.publisher.clone(),
        install_location: raw.install_location.clone(),
        icon_dir: folder_of(icon.and_then(|i| crate::residue_startup::extract_exe_path(&i))),
        uninstall_dir: folder_of(
            raw.uninstall_string
                .as_deref()
                .and_then(crate::residue_startup::extract_exe_path),
        ),
    }
}

fn name_key(name: &str) -> String {
    crate::residue::normalize(&crate::residue::strip_version(name))
}

fn publisher_key(publisher: Option<&str>) -> String {
    crate::residue::normalize(publisher.unwrap_or(""))
}

fn same_key(a: &SnapEntry, b: &SnapEntry) -> bool {
    a.source == b.source && a.key == b.key
}

/// Entries of `previous` that are gone from `current` and not merely
/// replaced by another entry of the same program (an MSI upgrade changes the
/// product code, so the key changes with it).
///
/// A read that came back nearly empty is a failed read, not an uninstall
/// spree: nothing is reported then.
pub fn diff_removed(previous: &[SnapEntry], current: &[SnapEntry]) -> Vec<SnapEntry> {
    // ponytail: a fixed "half the list vanished" cut-off; a smarter guess
    // would need the registry error itself.
    if current.is_empty() || (previous.len() >= 10 && current.len() * 2 < previous.len()) {
        return Vec::new();
    }
    let appeared: Vec<&SnapEntry> = current
        .iter()
        .filter(|c| !previous.iter().any(|p| same_key(p, c)))
        .collect();
    previous
        .iter()
        .filter(|p| !current.iter().any(|c| same_key(p, c)))
        .filter(|gone| {
            let name = name_key(&gone.name);
            let publisher = publisher_key(gone.publisher.as_deref());
            let location = crate::relations::normalized_root(gone.install_location.as_deref());
            !appeared.iter().any(|new| {
                (!name.is_empty()
                    && name_key(&new.name) == name
                    && publisher_key(new.publisher.as_deref()) == publisher)
                    || (location.is_some()
                        && crate::relations::normalized_root(new.install_location.as_deref())
                            == location)
            })
        })
        .cloned()
        .collect()
}

fn record_from(entry: &SnapEntry, removed_at: u64, how: &str) -> Record {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    for part in [
        &entry.source,
        &entry.key,
        &entry.name,
        &removed_at.to_string(),
    ] {
        hash.update(part.as_bytes());
        hash.update([0]);
    }
    Record {
        rid: hash.finalize()[..8]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        source: entry.source.clone(),
        key: entry.key.clone(),
        name: entry.name.clone(),
        publisher: entry.publisher.clone(),
        install_location: entry.install_location.clone(),
        icon_dir: entry.icon_dir.clone(),
        uninstall_dir: entry.uninstall_dir.clone(),
        removed_at,
        how: how.to_string(),
        // A removal through this app needs no "removed outside" prompt.
        seen: how == "app",
    }
}

/// Newest first, capped.
fn push_record(store: &mut Store, record: Record) {
    store.records.retain(|r| r.rid != record.rid);
    store.records.insert(0, record);
    store.records.truncate(CAP);
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    use std::io::Write;
    std::fs::create_dir_all(path.parent().ok_or("no folder")?).map_err(|e| e.to_string())?;
    let body = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    // Temp-then-rename, like the ledger: a crash never leaves half a file.
    let tmp = path.with_extension("json.tmp");
    std::fs::File::create(&tmp)
        .and_then(|mut f| f.write_all(&body))
        .map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn load_store(dir: &Path) -> Store {
    read_json(&dir.join(STORE_FILE)).unwrap_or_default()
}

fn save_store(dir: &Path, store: &Store) {
    if let Err(e) = write_json(&dir.join(STORE_FILE), store) {
        crate::applog::line(&format!("removed programs not saved: {e}"));
    }
}

/// Compares `current` with the previous snapshot, saves it as the new one and
/// records what vanished. Returns the records added.
pub fn observe(dir: &Path, current: Vec<SnapEntry>, now: u64) -> Vec<Record> {
    let _guard = WRITE.lock().unwrap_or_else(|e| e.into_inner());
    let snapshot_path = dir.join(SNAPSHOT_FILE);
    // A damaged snapshot is no snapshot: the first run after it just
    // re-learns the list.
    let previous: Option<Snapshot> = read_json(&snapshot_path);
    let first_run = previous.is_none() && !dir.join(STORE_FILE).exists();
    let mut store = load_store(dir);
    let mut added = Vec::new();
    if let Some(previous) = &previous {
        for gone in diff_removed(&previous.entries, &current) {
            // Removed through this app a moment ago: already recorded.
            let known = store.records.iter().any(|r| {
                r.how == "app"
                    && r.source == gone.source
                    && r.key == gone.key
                    && r.removed_at >= previous.taken_at
            });
            if known {
                continue;
            }
            let record = record_from(&gone, now, "outside");
            push_record(&mut store, record.clone());
            added.push(record);
        }
    }
    if first_run {
        store.sweep = Some("new".into());
    }
    // A read that came back empty must not replace a good snapshot.
    if !current.is_empty() {
        if let Err(e) = write_json(
            &snapshot_path,
            &Snapshot {
                taken_at: now,
                entries: current,
            },
        ) {
            crate::applog::line(&format!("inventory snapshot not saved: {e}"));
        }
    }
    if !added.is_empty() || first_run {
        save_store(dir, &store);
    }
    added
}

/// A removal this app just ran.
pub fn record_app_removal(dir: &Path, source: &str, raw: &RawEntry, now: u64) {
    let _guard = WRITE.lock().unwrap_or_else(|e| e.into_inner());
    let mut store = load_store(dir);
    push_record(
        &mut store,
        record_from(&entry_from_raw(source, raw), now, "app"),
    );
    save_store(dir, &store);
}

pub fn list(dir: &Path) -> Vec<RemovedView> {
    load_store(dir)
        .records
        .into_iter()
        .map(|r| RemovedView {
            rid: r.rid,
            name: r.name,
            publisher: r.publisher,
            removed_at: r.removed_at,
            how: r.how,
            seen: r.seen,
        })
        .collect()
}

pub fn find(dir: &Path, rid: &str) -> Option<Record> {
    load_store(dir).records.into_iter().find(|r| r.rid == rid)
}

pub fn mark_seen(dir: &Path, rid: &str) {
    let _guard = WRITE.lock().unwrap_or_else(|e| e.into_inner());
    let mut store = load_store(dir);
    if let Some(r) = store.records.iter_mut().find(|r| r.rid == rid) {
        r.seen = true;
        save_store(dir, &store);
    }
}

pub fn sweep_is_new(dir: &Path) -> bool {
    load_store(dir).sweep.as_deref() == Some("new")
}

pub fn finish_sweep(dir: &Path) {
    let _guard = WRITE.lock().unwrap_or_else(|e| e.into_inner());
    let mut store = load_store(dir);
    if store.sweep.as_deref() != Some("done") {
        store.sweep = Some("done".into());
        save_store(dir, &store);
    }
}

/// The removed-programs list, and whether the first-launch card is due.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub records: Vec<RemovedView>,
    /// Dead startup entries to offer, on the first launch only.
    pub startup_card: Option<usize>,
}

#[tauri::command(async)]
pub fn removed_overview() -> Result<Overview, String> {
    let dir = crate::uninstall_exec::fixed_data_dir()?;
    let startup_card = if sweep_is_new(&dir) {
        match crate::residue::sweep_count() {
            0 => {
                finish_sweep(&dir);
                None
            }
            n => Some(n),
        }
    } else {
        None
    };
    Ok(Overview {
        records: list(&dir),
        startup_card,
    })
}

/// "Not now" on the outside-removal prompt.
#[tauri::command(async)]
pub fn dismiss_removed(rid: String) -> Result<(), String> {
    mark_seen(&crate::uninstall_exec::fixed_data_dir()?, &rid);
    Ok(())
}

/// The first-launch card was dismissed without reviewing.
#[tauri::command(async)]
pub fn dismiss_startup_card() -> Result<(), String> {
    finish_sweep(&crate::uninstall_exec::fixed_data_dir()?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(source: &str, key: &str, name: &str, publisher: &str, location: &str) -> SnapEntry {
        SnapEntry {
            source: source.into(),
            key: key.into(),
            name: name.into(),
            publisher: (!publisher.is_empty()).then(|| publisher.into()),
            install_location: (!location.is_empty()).then(|| location.into()),
            icon_dir: None,
            uninstall_dir: None,
        }
    }
    fn names(entries: &[SnapEntry]) -> Vec<&str> {
        entries.iter().map(|e| e.name.as_str()).collect()
    }
    fn dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pcu-removed-{tag}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        dir
    }
    /// A list long enough that losing one entry is not "the read failed".
    fn base() -> Vec<SnapEntry> {
        (0..12)
            .map(|i| {
                entry(
                    "machine64",
                    &format!("K{i}"),
                    &format!("Program Number {i}"),
                    "Acme",
                    &format!(r"C:\Program Files\Acme\P{i}"),
                )
            })
            .collect()
    }

    #[test]
    fn a_vanished_entry_is_a_removal() {
        let before = base();
        let mut after = before.clone();
        let gone = after.remove(3);
        assert_eq!(diff_removed(&before, &after), vec![gone]);
        assert!(diff_removed(&before, &before).is_empty());
    }

    #[test]
    fn an_msi_upgrade_with_a_new_product_code_is_not_a_removal() {
        let mut before = base();
        before.push(entry(
            "machine64",
            "{AAAA-1}",
            "Zoom Workplace 5.1.0",
            "Zoom",
            r"C:\Program Files\Zoom",
        ));
        let mut after = before.clone();
        after.retain(|e| e.key != "{AAAA-1}");
        after.push(entry(
            "machine64",
            "{BBBB-2}",
            "Zoom Workplace 5.2.1",
            "Zoom",
            r"C:\Program Files\Zoom",
        ));
        assert!(diff_removed(&before, &after).is_empty());
        // Same program, different folder this time: name and publisher decide.
        let mut moved = before.clone();
        moved.retain(|e| e.key != "{AAAA-1}");
        moved.push(entry(
            "machine64",
            "{CCCC-3}",
            "Zoom Workplace 5.3.0",
            "ZOOM",
            r"D:\Apps\Zoom",
        ));
        assert!(diff_removed(&before, &moved).is_empty());
        // Same folder, renamed product: the folder decides.
        let mut renamed = before.clone();
        renamed.retain(|e| e.key != "{AAAA-1}");
        renamed.push(entry(
            "machine64",
            "{DDDD-4}",
            "Zoom Meetings",
            "Zoom Video",
            r"C:\Program Files\Zoom",
        ));
        assert!(diff_removed(&before, &renamed).is_empty());
    }

    #[test]
    fn a_different_program_appearing_does_not_hide_a_removal() {
        let mut before = base();
        before.push(entry(
            "user",
            "Old",
            "Old Tool",
            "Someone",
            r"C:\Users\me\AppData\Local\OldTool",
        ));
        let mut after = before.clone();
        after.retain(|e| e.key != "Old");
        after.push(entry(
            "user",
            "New",
            "New Tool",
            "Someone Else",
            r"C:\Users\me\AppData\Local\NewTool",
        ));
        assert_eq!(names(&diff_removed(&before, &after)), vec!["Old Tool"]);
    }

    /// The key staying put while the install path moves is no removal at all.
    #[test]
    fn an_entry_whose_install_path_moved_is_not_a_removal() {
        let before = base();
        let mut after = before.clone();
        after[2].install_location = Some(r"D:\Moved\P2".into());
        assert!(diff_removed(&before, &after).is_empty());
    }

    #[test]
    fn an_unreadable_registry_is_not_an_uninstall_spree() {
        let before = base();
        assert!(diff_removed(&before, &[]).is_empty());
        assert!(diff_removed(&before, &before[..4]).is_empty());
        assert_eq!(diff_removed(&before, &before[..7]).len(), 5);
    }

    #[test]
    fn the_snapshot_diff_is_recorded_once_and_the_list_is_capped_at_50() {
        let d = dir("cap");
        let mut list: Vec<SnapEntry> = (0..80)
            .map(|i| {
                entry(
                    "user",
                    &format!("K{i}"),
                    &format!("Tool Alpha{i}x"),
                    "Pub",
                    &format!(r"C:\Apps\T{i}"),
                )
            })
            .collect();
        assert!(
            observe(&d, list.clone(), 100).is_empty(),
            "first run learns the list"
        );
        assert!(sweep_is_new(&d), "first run offers the startup sweep");
        // 70 of 80 go at once: looks like a failed read, nothing is recorded.
        assert!(observe(&d, list[..10].to_vec(), 200).is_empty());
        // The snapshot was replaced by that read: a normal list brings them back.
        assert!(observe(&d, list.clone(), 300).is_empty());
        for round in 0..6u64 {
            let before = list.clone();
            list.drain(..10);
            let added = observe(&d, list.clone(), 400 + round);
            if round < 5 {
                assert_eq!(added.len(), 10, "round {round}");
            }
            assert!(before.len() > list.len());
        }
        assert_eq!(list_len(&d), CAP);
        // Never a second record for the same removal.
        assert!(observe(&d, list.clone(), 500).is_empty());
        assert_eq!(list_len(&d), CAP);
        assert!(list_views(&d).iter().all(|r| r.how == "outside" && !r.seen));
        std::fs::remove_dir_all(&d).ok();
    }
    fn list_len(d: &Path) -> usize {
        list(d).len()
    }
    fn list_views(d: &Path) -> Vec<RemovedView> {
        list(d)
    }

    #[test]
    fn a_damaged_snapshot_or_store_is_ignored_safely() {
        let d = dir("corrupt");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(SNAPSHOT_FILE), "{ not json").unwrap();
        std::fs::write(d.join(STORE_FILE), [0xff, 0xfe, 0x00]).unwrap();
        let list = base();
        assert!(observe(&d, list.clone(), 10).is_empty());
        // The damaged snapshot was replaced; the next removal is seen.
        let mut after = list.clone();
        after.remove(0);
        assert_eq!(observe(&d, after, 20).len(), 1);
        assert_eq!(list_len(&d), 1);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn this_apps_own_removal_is_recorded_once_and_not_prompted() {
        let d = dir("app");
        let list = base();
        observe(&d, list.clone(), 100);
        let raw = RawEntry {
            key_name: "K4".into(),
            display_name: Some("Program Number 4".into()),
            publisher: Some("Acme".into()),
            install_location: Some(r"C:\Program Files\Acme\P4".into()),
            display_icon: Some(r#""C:\Program Files\Acme\P4\p4.exe",0"#.into()),
            uninstall_string: Some(r#""C:\Program Files\Acme\P4\unins000.exe" /SILENT"#.into()),
            ..Default::default()
        };
        record_app_removal(&d, "machine64", &raw, 150);
        let mut after = list.clone();
        after.retain(|e| e.key != "K4");
        // The next refresh sees the entry gone but must not add a second one.
        assert!(observe(&d, after, 160).is_empty());
        let views = list_views(&d);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].how, "app");
        assert!(views[0].seen);
        let rec = find(&d, &views[0].rid).unwrap();
        assert_eq!(rec.icon_dir.as_deref(), Some(r"C:\Program Files\Acme\P4"));
        assert_eq!(
            rec.uninstall_dir.as_deref(),
            Some(r"C:\Program Files\Acme\P4")
        );
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn the_prompt_and_the_sweep_card_are_answered_once() {
        let d = dir("seen");
        let list = base();
        observe(&d, list.clone(), 100);
        assert!(sweep_is_new(&d));
        finish_sweep(&d);
        assert!(!sweep_is_new(&d));
        let mut after = list;
        after.remove(1);
        let added = observe(&d, after, 200);
        assert!(!list_views(&d)[0].seen);
        mark_seen(&d, &added[0].rid);
        assert!(list_views(&d)[0].seen);
        assert!(!sweep_is_new(&d));
        std::fs::remove_dir_all(&d).ok();
    }
}
