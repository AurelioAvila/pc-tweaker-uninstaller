//! Removal Confidence Score — the evidence-based heart of "Removal
//! Intelligence".
//!
//! Every installed program is placed in one of three bands from VISIBLE
//! evidence only (registry values this app already reads). The score is a
//! guide, never a certainty, and the UI is required to say so: the output
//! carries the reason codes so every verdict is explainable in one line.
//!
//! Banding rules, in strict priority order (first match wins the band; all
//! matching reasons are still collected so the explanation is complete):
//!
//! 1. `Keep` — components other software depends on:
//!    - entries Add/Remove Programs itself hides (SystemComponent, child
//!      updates): the OS explicitly says "don't casually remove me";
//!    - shared runtimes (Visual C++ Redistributable, .NET runtimes/SDKs,
//!      Edge WebView2, Windows SDK/App Runtime, Java runtimes): removing one
//!      breaks every app built on it;
//!    - drivers and chipset packages: removing them can take hardware down.
//! 2. `Review` — removable, but a human should look first:
//!    - game/store launchers (Steam, Epic, Battle.net, ...): games installed
//!      through them stop working;
//!    - no publisher recorded: unverifiable origin;
//!    - broken, manual-only or missing uninstall command: automatic removal
//!      is impossible or unreliable anyway.
//! 3. `Safe` — a named publisher plus a standard MSI/EXE uninstaller.
//!
//! Pure functions over `RawEntry` — unit-tested on any platform.

use crate::programs::{RawEntry, UninstallSummary};
use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConfidenceLevel {
    Safe,
    Review,
    Keep,
}

/// Reason codes, not sentences: the frontend maps each to a localized line.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Confidence {
    pub level: ConfidenceLevel,
    pub reasons: Vec<&'static str>,
}

fn name_lower(entry: &RawEntry) -> String {
    entry
        .display_name
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Shared runtimes: the packages many OTHER programs silently depend on.
fn is_shared_runtime(name: &str) -> bool {
    const MARKERS: [&str; 8] = [
        "visual c++",
        "vc++ redistributable",
        ".net runtime",
        ".net sdk",
        "windows desktop runtime",
        "windows software development kit",
        "windows app runtime",
        "edge webview2",
    ];
    if MARKERS.iter().any(|m| name.contains(m)) {
        return true;
    }
    // Java runtimes ship under many vendors; the name is the stable signal.
    name.contains("java") && (name.contains("runtime") || name.contains("jre") || name.contains("jdk"))
}

/// Driver/chipset packages: hardware support, not applications.
fn is_driver_package(name: &str) -> bool {
    name.contains("driver") || name.contains("chipset") || name.contains(" firmware")
}

/// Store/game launchers: removing the launcher strands everything installed
/// through it. Matched on names users actually see in Add/Remove Programs.
fn is_shared_launcher(name: &str) -> bool {
    const LAUNCHERS: [&str; 9] = [
        "steam",
        "epic games launcher",
        "battle.net",
        "riot client",
        "ea app",
        "ubisoft connect",
        "gog galaxy",
        "rockstar games launcher",
        "xbox app",
    ];
    LAUNCHERS.iter().any(|l| name == *l || name.starts_with(l))
}

/// Assesses one entry. `hidden` is the ARP-hides-this flag the enumeration
/// already computes; `summary` is the parsed uninstall-command class.
pub fn assess(entry: &RawEntry, hidden: bool, summary: &UninstallSummary) -> Confidence {
    let name = name_lower(entry);
    let mut keep_reasons: Vec<&'static str> = Vec::new();
    let mut review_reasons: Vec<&'static str> = Vec::new();

    if hidden {
        keep_reasons.push("hiddenSystem");
    }
    if is_shared_runtime(&name) {
        keep_reasons.push("sharedRuntime");
    }
    if is_driver_package(&name) {
        keep_reasons.push("driverComponent");
    }

    if is_shared_launcher(&name) {
        review_reasons.push("sharedLauncher");
    }
    if entry.publisher.as_deref().map_or(true, |p| p.trim().is_empty()) {
        review_reasons.push("noPublisher");
    }
    match summary {
        UninstallSummary::Invalid => review_reasons.push("brokenUninstaller"),
        UninstallSummary::ManualOnly => review_reasons.push("manualUninstaller"),
        UninstallSummary::None => review_reasons.push("noUninstaller"),
        UninstallSummary::Msi | UninstallSummary::Executable => {}
    }

    if !keep_reasons.is_empty() {
        // A Keep verdict still carries any Review evidence: "system component
        // AND its uninstall entry is broken" is worth knowing whole.
        keep_reasons.extend(review_reasons);
        return Confidence {
            level: ConfidenceLevel::Keep,
            reasons: keep_reasons,
        };
    }
    if !review_reasons.is_empty() {
        return Confidence {
            level: ConfidenceLevel::Review,
            reasons: review_reasons,
        };
    }
    Confidence {
        level: ConfidenceLevel::Safe,
        reasons: vec!["namedPublisher", "standardUninstaller"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, publisher: Option<&str>, uninstall: Option<&str>) -> RawEntry {
        RawEntry {
            key_name: "K".into(),
            display_name: Some(name.into()),
            publisher: publisher.map(str::to_string),
            uninstall_string: uninstall.map(str::to_string),
            ..Default::default()
        }
    }

    fn summary_of(e: &RawEntry) -> UninstallSummary {
        crate::programs::summarize_uninstall(e)
    }

    #[test]
    fn a_normal_app_with_publisher_and_exe_uninstaller_is_safe() {
        let e = entry("7-Zip", Some("Igor Pavlov"), Some(r#""C:\7z\Uninstall.exe""#));
        let c = assess(&e, false, &summary_of(&e));
        assert_eq!(c.level, ConfidenceLevel::Safe);
        assert!(c.reasons.contains(&"namedPublisher"));
    }

    #[test]
    fn shared_runtimes_are_keep_regardless_of_a_valid_uninstaller() {
        for name in [
            "Microsoft Visual C++ 2015-2022 Redistributable (x64)",
            "Microsoft .NET Runtime - 8.0.11 (x64)",
            "Microsoft Edge WebView2 Runtime",
            "Java(TM) SE Runtime Environment 8",
        ] {
            let e = entry(name, Some("Microsoft Corporation"), Some(r#""C:\x\u.exe""#));
            let c = assess(&e, false, &summary_of(&e));
            assert_eq!(c.level, ConfidenceLevel::Keep, "{name}");
            assert!(c.reasons.contains(&"sharedRuntime"), "{name}");
        }
    }

    #[test]
    fn drivers_and_chipset_packages_are_keep() {
        let e = entry(
            "Intel(R) Chipset Device Software",
            Some("Intel Corporation"),
            Some(r#""C:\Intel\u.exe""#),
        );
        assert_eq!(assess(&e, false, &summary_of(&e)).level, ConfidenceLevel::Keep);
    }

    #[test]
    fn hidden_entries_are_keep_and_the_reason_says_why() {
        let e = entry("Microsoft Update Health Tools", Some("Microsoft"), None);
        let c = assess(&e, true, &summary_of(&e));
        assert_eq!(c.level, ConfidenceLevel::Keep);
        assert!(c.reasons.contains(&"hiddenSystem"));
        // Review evidence is still carried through for a complete explanation.
        assert!(c.reasons.contains(&"noUninstaller"));
    }

    #[test]
    fn launchers_are_review_not_keep_and_not_safe() {
        let e = entry("Steam", Some("Valve Corporation"), Some(r#""C:\Steam\u.exe""#));
        let c = assess(&e, false, &summary_of(&e));
        assert_eq!(c.level, ConfidenceLevel::Review);
        assert!(c.reasons.contains(&"sharedLauncher"));
    }

    #[test]
    fn missing_publisher_and_broken_commands_demand_review() {
        // No publisher at all (also: no size, no version — must not panic).
        let e = entry("Mystery Tool", None, Some(r#""C:\m\u.exe""#));
        let c = assess(&e, false, &summary_of(&e));
        assert_eq!(c.level, ConfidenceLevel::Review);
        assert!(c.reasons.contains(&"noPublisher"));

        // Malformed msiexec GUID → Invalid → brokenUninstaller.
        let b = entry("Broken", Some("Someone"), Some("msiexec.exe /x {not-a-guid}"));
        let c = assess(&b, false, &summary_of(&b));
        assert_eq!(c.level, ConfidenceLevel::Review);
        assert!(c.reasons.contains(&"brokenUninstaller"));

        // Interpreter-based command → manual only.
        let m = entry("Batchy", Some("Someone"), Some("cmd.exe /c clean.bat"));
        let c = assess(&m, false, &summary_of(&m));
        assert!(c.reasons.contains(&"manualUninstaller"));
    }

    #[test]
    fn per_user_scope_alone_never_downgrades_a_clean_entry() {
        // Scope (HKCU vs HKLM) is presented as a badge, not as risk: a clean
        // per-user app with publisher + EXE uninstaller stays Safe.
        let e = entry("Notable", Some("Notable Team", ), Some(r#""C:\n\u.exe" /S"#));
        assert_eq!(assess(&e, false, &summary_of(&e)).level, ConfidenceLevel::Safe);
    }
}
