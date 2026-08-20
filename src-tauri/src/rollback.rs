//! Snapshot-before-change store, ported from PC Tweaker's battle-tested
//! `rollback.rs` (same locking, same atomic-write pattern, same tests) with
//! the snapshot vocabulary trimmed to what an uninstaller actually does:
//! registry values it changes, registry keys it creates, and composites.
//! Tweak-specific variants (power schemes, DNS, services) were deliberately
//! not carried over — dead variants in a serde enum are attack/maintenance
//! surface with no payoff. File removals never appear here at all: leftover
//! files go to a Recycle-Bin-backed quarantine, which is its own reversal
//! mechanism (Phase 4).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// A registry value of a supported type. Uninstall entries store both DWORDs
/// and strings, so snapshots must preserve whichever type they found.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RegValue {
    Dword(u32),
    Str(String),
}

/// A snapshot of a single registry value, taken right before a change.
/// `None` means the value did not exist beforehand, so rollback deletes it.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegistrySnapshot {
    pub hive: String,
    pub path: String,
    pub name: String,
    pub original_value: Option<RegValue>,
}

/// Any reversible action this app can take.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind")]
pub enum SnapshotEntry {
    Registry(RegistrySnapshot),
    /// A registry key that did not exist until this app created it, recorded
    /// so rollback can delete the whole subtree instead of only clearing a
    /// value inside it — otherwise an empty key would be left behind.
    RegistryKeyCreated {
        hive: String,
        path: String,
    },
    Composite {
        entries: Vec<SnapshotEntry>,
    },
}

#[derive(Serialize, Deserialize, Default)]
struct Store {
    snapshots: HashMap<String, SnapshotEntry>,
}

pub struct RollbackStore {
    file_path: PathBuf,
}

/// Serializes every read-modify-write of the snapshot file. Each entry is
/// written by loading the whole file, changing one key and writing it back —
/// two overlapping callers would otherwise silently drop each other's
/// snapshot, leaving a change applied with no way back. Cross-process safety
/// comes from the elevated helper running to completion while the parent
/// blocks on it, same as PC Tweaker.
fn store_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

impl RollbackStore {
    pub fn new(app_data_dir: PathBuf) -> Self {
        RollbackStore {
            file_path: app_data_dir.join("rollback_store.json"),
        }
    }

    fn load(&self) -> Store {
        fs::read_to_string(&self.file_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Temp file + rename, so an interrupted write can never leave a
    /// half-written (unparseable) snapshot file behind — losing this file
    /// means losing the ability to undo every applied change.
    fn save(&self, store: &Store) -> std::io::Result<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(store).unwrap();
        // Unique temp name per write: a fixed one would let a writer not
        // covered by the in-process lock (the elevated helper) clobber
        // another's temp file and fail the rename.
        let tmp = self.file_path.with_extension(format!(
            "json.{}.{}.tmp",
            std::process::id(),
            TMP_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&tmp, json)?;
        let result = fs::rename(&tmp, &self.file_path);
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }

    pub fn is_applied(&self, action_id: &str) -> bool {
        let _guard = store_lock().lock().unwrap_or_else(|e| e.into_inner());
        self.load().snapshots.contains_key(action_id)
    }

    pub fn save_entry(&self, action_id: &str, entry: SnapshotEntry) -> std::io::Result<()> {
        let _guard = store_lock().lock().unwrap_or_else(|e| e.into_inner());
        let mut store = self.load();
        store.snapshots.insert(action_id.to_string(), entry);
        self.save(&store)
    }

    pub fn take_entry(&self, action_id: &str) -> Option<SnapshotEntry> {
        let _guard = store_lock().lock().unwrap_or_else(|e| e.into_inner());
        let mut store = self.load();
        let entry = store.snapshots.remove(action_id);
        if entry.is_some() {
            let _ = self.save(&store);
        }
        entry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ptu-rollback-test-{}-{}",
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn dummy(name: &str) -> SnapshotEntry {
        SnapshotEntry::RegistryKeyCreated {
            hive: "HKCU".to_string(),
            path: name.to_string(),
        }
    }

    #[test]
    fn saves_and_takes_a_single_entry() {
        let dir = temp_dir("single");
        let store = RollbackStore::new(dir.clone());

        assert!(!store.is_applied("a"));
        store.save_entry("a", dummy("Software\\A")).unwrap();
        assert!(store.is_applied("a"));

        match store.take_entry("a") {
            Some(SnapshotEntry::RegistryKeyCreated { path, .. }) => assert_eq!(path, "Software\\A"),
            other => panic!("unexpected entry: {:?}", other.is_some()),
        }
        assert!(!store.is_applied("a"));
        assert!(store.take_entry("a").is_none());

        let _ = fs::remove_dir_all(dir);
    }

    /// The regression PC Tweaker learned the hard way: concurrent writers
    /// with no lock silently overwrite each other's snapshots, leaving a
    /// change applied with nothing to roll back to.
    #[test]
    fn concurrent_writers_never_lose_a_snapshot() {
        let dir = temp_dir("concurrent");
        const THREADS: usize = 8;
        const PER_THREAD: usize = 40;

        let handles: Vec<_> = (0..THREADS)
            .map(|t| {
                let dir = dir.clone();
                std::thread::spawn(move || {
                    let store = RollbackStore::new(dir);
                    for i in 0..PER_THREAD {
                        let id = format!("action-{}-{}", t, i);
                        store.save_entry(&id, dummy(&id)).unwrap();
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("writer thread panicked");
        }

        let store = RollbackStore::new(dir.clone());
        for t in 0..THREADS {
            for i in 0..PER_THREAD {
                let id = format!("action-{}-{}", t, i);
                assert!(store.is_applied(&id), "lost snapshot {}", id);
            }
        }

        let _ = fs::remove_dir_all(dir);
    }

    /// A crash between "write" and "replace" must not leave an unparseable
    /// snapshot file, which would strand every applied change.
    #[test]
    fn writes_are_committed_atomically_and_leave_no_temp_file() {
        let dir = temp_dir("atomic");
        let store = RollbackStore::new(dir.clone());
        store.save_entry("a", dummy("Software\\A")).unwrap();

        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temp files left behind: {:?}",
            leftovers
        );

        let raw = fs::read_to_string(dir.join("rollback_store.json")).unwrap();
        serde_json::from_str::<Store>(&raw).expect("snapshot file must always parse");

        let _ = fs::remove_dir_all(dir);
    }
}
