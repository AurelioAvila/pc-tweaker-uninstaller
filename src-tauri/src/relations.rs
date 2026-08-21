//! Relationship Map — slice 3 of Removal Intelligence.
//!
//! Derives relationships between installed programs from VISIBLE evidence
//! only, in the same spirit as the confidence engine: every claim the UI
//! makes must be traceable to a registry value, never to a guess.
//!
//! Two relationships are derivable with certainty from install paths:
//!
//! - **Containment**: program B's install location lives INSIDE program A's
//!   install tree (a game under `...\Steam\steamapps\common\`, a plugin under
//!   its host's folder). Removing A takes B's files with it — so A's confirm
//!   dialog must name its dependents, and B's should say "installed via A".
//! - **Publisher siblings**: other programs registered by the same publisher.
//!   Informational only — it never blocks anything, but "this is one of 6
//!   Adobe entries" is context a human wants before removing one.
//!
//! Pure functions over (name, publisher, install_location) triples so the
//! whole module is unit-tested on any platform.

use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Relations {
    /// Display names of programs whose install location is inside this
    /// program's install tree. Removing this program breaks them.
    pub dependents: Vec<String>,
    /// Display name of the program whose install tree contains this one.
    pub installed_via: Option<String>,
    /// How many OTHER listed programs share this publisher (0 = unique).
    pub publisher_siblings: u32,
}

/// The minimal projection relations are computed from.
pub struct RelEntry {
    pub name: String,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
}

/// Normalizes an install location for ancestry comparison: lowercased,
/// quotes stripped, trailing separators removed, forward slashes unified.
/// Returns None for empty or bare-drive paths (`C:\` contains everything —
/// treating it as a tree would relate the whole machine).
fn normalized_root(location: Option<&str>) -> Option<String> {
    let raw = location?.trim().trim_matches('"').replace('/', "\\");
    let trimmed = raw.trim_end_matches('\\').to_ascii_lowercase();
    if trimmed.is_empty() || trimmed.matches('\\').count() < 2 {
        return None;
    }
    Some(trimmed)
}

/// True when `child` is strictly inside `parent`'s tree.
fn is_inside(child: &str, parent: &str) -> bool {
    child.len() > parent.len() + 1
        && child.starts_with(parent)
        && child.as_bytes()[parent.len()] == b'\\'
}

fn normalized_publisher(publisher: Option<&str>) -> Option<String> {
    let token: String = publisher?
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    if token.len() < 3 { None } else { Some(token) }
}

/// Computes the relations for every entry, positionally parallel to the
/// input. O(n²) over install paths — fine for the few hundred entries a
/// real machine has, and honest beats clever here.
pub fn compute_relations(entries: &[RelEntry]) -> Vec<Relations> {
    let roots: Vec<Option<String>> = entries
        .iter()
        .map(|e| normalized_root(e.install_location.as_deref()))
        .collect();
    let publishers: Vec<Option<String>> = entries
        .iter()
        .map(|e| normalized_publisher(e.publisher.as_deref()))
        .collect();

    let mut out: Vec<Relations> = entries.iter().map(|_| Relations::default()).collect();

    for i in 0..entries.len() {
        let Some(parent) = roots[i].as_deref() else { continue };
        for j in 0..entries.len() {
            if i == j {
                continue;
            }
            let Some(child) = roots[j].as_deref() else { continue };
            if is_inside(child, parent) {
                out[i].dependents.push(entries[j].name.clone());
                // The DEEPEST containing tree wins as "installed via": a game
                // under Steam\steamapps\common belongs to Steam, not to the
                // drive-level folder some vendor also registered.
                let better = match out[j].installed_via.as_deref() {
                    None => true,
                    Some(current) => {
                        let current_root = entries
                            .iter()
                            .position(|e| e.name == current)
                            .and_then(|k| roots[k].clone());
                        current_root.is_none_or(|r| parent.len() > r.len())
                    }
                };
                if better {
                    out[j].installed_via = Some(entries[i].name.clone());
                }
            }
        }
        out[i].dependents.sort();
        out[i].dependents.dedup();
    }

    for i in 0..entries.len() {
        if let Some(publisher) = publishers[i].as_deref() {
            out[i].publisher_siblings = publishers
                .iter()
                .enumerate()
                .filter(|(j, p)| *j != i && p.as_deref() == Some(publisher))
                .count() as u32;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, publisher: Option<&str>, location: Option<&str>) -> RelEntry {
        RelEntry {
            name: name.into(),
            publisher: publisher.map(str::to_string),
            install_location: location.map(str::to_string),
        }
    }

    #[test]
    fn games_under_a_launcher_tree_are_its_dependents() {
        let entries = [
            entry("Steam", Some("Valve"), Some(r"C:\Program Files (x86)\Steam")),
            entry("Half-Life", Some("Valve"), Some(r"C:\Program Files (x86)\Steam\steamapps\common\Half-Life")),
            entry("Unrelated", Some("Acme"), Some(r"C:\Program Files\Unrelated")),
        ];
        let rel = compute_relations(&entries);
        assert_eq!(rel[0].dependents, vec!["Half-Life".to_string()]);
        assert_eq!(rel[1].installed_via.as_deref(), Some("Steam"));
        assert!(rel[2].dependents.is_empty());
        assert_eq!(rel[2].installed_via, None);
    }

    #[test]
    fn the_deepest_containing_tree_wins_installed_via() {
        let entries = [
            entry("VendorHub", None, Some(r"C:\Program Files\Vendor")),
            entry("Launcher", None, Some(r"C:\Program Files\Vendor\Launcher")),
            entry("Game", None, Some(r"C:\Program Files\Vendor\Launcher\games\Game")),
        ];
        let rel = compute_relations(&entries);
        assert_eq!(rel[2].installed_via.as_deref(), Some("Launcher"));
        // VendorHub still lists Game as a dependent — removing it does take
        // the whole tree down; both facts are true.
        assert!(rel[0].dependents.contains(&"Game".to_string()));
    }

    #[test]
    fn bare_drives_and_shallow_paths_never_form_trees() {
        let entries = [
            entry("Rooted", None, Some(r"C:\")),
            entry("Shallow", None, Some(r"C:\Tools")),
            entry("App", None, Some(r"C:\Tools\App")),
        ];
        let rel = compute_relations(&entries);
        assert!(rel[0].dependents.is_empty(), "C:\\ must not contain the world");
        assert!(rel[1].dependents.is_empty(), "single-level paths are too weak as evidence");
    }

    #[test]
    fn identical_locations_are_not_each_others_dependents() {
        let entries = [
            entry("A", None, Some(r"C:\Program Files\Shared\App")),
            entry("B", None, Some(r"C:\Program Files\Shared\App")),
        ];
        let rel = compute_relations(&entries);
        assert!(rel[0].dependents.is_empty());
        assert!(rel[1].dependents.is_empty());
    }

    #[test]
    fn publisher_siblings_are_counted_and_normalized() {
        let entries = [
            entry("A", Some("Adobe Inc."), None),
            entry("B", Some("adobe inc"), None),
            entry("C", Some("Other Corp"), None),
            entry("D", None, None),
        ];
        let rel = compute_relations(&entries);
        assert_eq!(rel[0].publisher_siblings, 1);
        assert_eq!(rel[1].publisher_siblings, 1);
        assert_eq!(rel[2].publisher_siblings, 0);
        assert_eq!(rel[3].publisher_siblings, 0);
    }

    #[test]
    fn quotes_slashes_and_case_do_not_break_containment() {
        let entries = [
            entry("Host", None, Some(r#""C:\Apps\Host\""#)),
            entry("Plugin", None, Some("c:/apps/host/plugins/plugin")),
        ];
        let rel = compute_relations(&entries);
        assert_eq!(rel[0].dependents, vec!["Plugin".to_string()]);
        assert_eq!(rel[1].installed_via.as_deref(), Some("Host"));
    }
}
