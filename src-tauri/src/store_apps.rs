//! Microsoft Store / MSIX packages — the half of "installed software" the
//! registry never sees.
//!
//! `programs.rs` reads the three classic Uninstall views, which is exactly
//! what Add/Remove Programs read for twenty years. Modern Windows installs a
//! second, parallel world: MSIX packages, listed in Settings > Installed
//! apps but absent from every registry Uninstall key. On a normal Windows 11
//! machine that is over a hundred entries an uninstaller would otherwise
//! claim do not exist.
//!
//! Enumeration and removal both go through the WinRT `PackageManager` — the
//! same API Settings itself drives. That choice is not incidental:
//! `uninstall_command.rs` refuses to execute registry uninstall strings that
//! invoke `powershell`, and shelling out to `Remove-AppxPackage` here would
//! make that rule cosmetic. The API path takes typed arguments, so there is
//! no command string for a package name to be injected into at all.
//!
//! Banding follows `confidence.rs`: the same three levels, from evidence the
//! platform itself reports, and the classifier is a pure function over
//! plain metadata so its rules are unit-tested on any platform.
//!
//! # An honest limit
//!
//! Windows gives an unelevated process no way to tell an app that shipped
//! with the OS from one the user installed. Notepad, Calculator and Spotify
//! all report `SignatureKind::Store`, the same install root and
//! `NonRemovable = false`; the list that would separate them
//! (`FindProvisionedPackages`) needs administrator rights this app does not
//! take just to draw a list. A hardcoded roster of Microsoft package names
//! would paper over that, but it would be a guess presented as evidence,
//! which is the one thing the Confidence Score is not allowed to be.
//!
//! So inbox apps can land in `Safe`, and that is defensible rather than
//! merely convenient: for an MSIX package `Safe` is a narrower claim than it
//! is for a registry entry. Removal is contained by the packaging format —
//! no residue to sweep — and anything removed here can be reinstalled from
//! the Store. The UI says exactly that instead of implying the app knows
//! more than it does.

use serde::Serialize;

use crate::confidence::{Confidence, ConfidenceLevel};

/// One MSIX package, shaped for the UI.
///
/// Every string originates in the package manifest and is untrusted display
/// data, rendered as text by React exactly like `ProgramInfo`.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StoreApp {
    /// PackageFullName — the identifier removal takes, and unique per
    /// package/version/architecture.
    pub id: String,
    pub name: String,
    pub publisher: Option<String>,
    pub version: String,
    pub install_location: Option<String>,
    /// ISO date (YYYY-MM-DD), so these rows sort and filter by age next to
    /// registry entries instead of sinking to the bottom of every list.
    pub install_date: Option<String>,
    pub confidence: Confidence,
    /// True when Windows reports this package as a framework: a shared
    /// runtime other packages link against.
    pub is_framework: bool,
    /// True when the package was signed as part of Windows itself.
    pub is_system: bool,
    /// Infrastructure rather than an application: frameworks, resource
    /// packages, and components Windows gives no display name to. Listed
    /// behind the same "show hidden" toggle registry system components use,
    /// on the same reasoning — visible to anyone who asks, never in the way
    /// of someone who did not.
    pub hidden: bool,
}

/// The subset of package metadata the banding rules actually read.
///
/// Split out from the WinRT types on purpose: the rules below are the part
/// worth testing, and they must be testable without a package manager, a
/// Store, or Windows.
#[derive(Clone, Debug, Default)]
pub struct StoreAppMeta {
    pub name: String,
    pub publisher: Option<String>,
    /// False when Windows resolved no display name for the package. The
    /// absence is itself the evidence: a component with nothing to show a
    /// user is not an application a user chose to install.
    pub has_display_name: bool,
    pub is_framework: bool,
    /// `PackageSignatureKind::System` — shipped as part of Windows.
    pub is_system: bool,
    /// `PackageSignatureKind::Store` — installed from the Microsoft Store.
    pub is_store_signed: bool,
    /// True when the package is a resource/optional satellite package rather
    /// than an application (language packs, scale assets).
    pub is_resource: bool,
}

/// Package-family prefixes that carry the operating system's own shell.
///
/// These are removable through the API and Windows will let you do it, but
/// doing so breaks parts of the desktop that have no other implementation —
/// the Start menu's own host, the shell experience pack. `Keep` is the
/// honest band: not "we refuse", but "this is not an application".
const OS_SHELL_PREFIXES: &[&str] = &[
    "microsoft.windows.shellexperiencehost",
    "microsoft.windows.startmenuexperiencehost",
    "microsoft.windows.search",
    "microsoft.ui.xaml",
    "microsoft.vclibs",
    "microsoft.net.native",
    "microsoft.windowsappruntime",
    "microsoft.services.store.engagement",
    "windows.cbs",
    "microsoft.accountscontrol",
    "microsoft.windows.cloudexperiencehost",
    "microsoft.creddialoghost",
    "microsoft.lockapp",
    "microsoft.windows.contentdeliverymanager",
    "microsoft.windows.securityhealthhost",
];

fn name_lower(meta: &StoreAppMeta) -> String {
    meta.name.to_ascii_lowercase()
}

/// Bands one package from platform-reported evidence.
///
/// Priority order mirrors `confidence.rs`: the first matching rule fixes the
/// band, but every matching reason is still collected so the UI can explain
/// the whole verdict rather than just its headline.
pub fn classify(meta: &StoreAppMeta) -> Confidence {
    let mut reasons: Vec<&'static str> = Vec::new();
    let lower = name_lower(meta);

    // --- Keep -------------------------------------------------------------
    if meta.is_framework {
        reasons.push("storeFramework");
    }
    if OS_SHELL_PREFIXES.iter().any(|p| lower.starts_with(p)) {
        reasons.push("storeOsComponent");
    }
    if meta.is_resource {
        reasons.push("storeResourcePackage");
    }
    if !meta.has_display_name {
        reasons.push("storeNoDisplayName");
    }
    if !reasons.is_empty() {
        return Confidence {
            level: ConfidenceLevel::Keep,
            reasons,
        };
    }

    // --- Review -----------------------------------------------------------
    if meta.is_system {
        reasons.push("storeSystemSigned");
    }
    if meta.publisher.as_deref().unwrap_or("").trim().is_empty() {
        reasons.push("storeNoPublisher");
    }
    if !reasons.is_empty() {
        return Confidence {
            level: ConfidenceLevel::Review,
            reasons,
        };
    }

    // --- Safe -------------------------------------------------------------
    // A named publisher and an ordinary install. Store-signed is worth
    // stating explicitly: it means Microsoft's own pipeline vouched for the
    // package, which is stronger provenance than a bare EXE installer.
    if meta.is_store_signed {
        reasons.push("storeFromStore");
    }
    reasons.push("storeCleanRemoval");
    Confidence {
        level: ConfidenceLevel::Safe,
        reasons,
    }
}

/// Days from 1601-01-01 (the WinRT/FILETIME epoch) to 1970-01-01.
const DAYS_1601_TO_1970: i64 = 134_774;

/// Formats a WinRT `DateTime.UniversalTime` as `YYYY-MM-DD`.
///
/// The value counts 100-nanosecond intervals since 1601-01-01 UTC. Only the
/// date is kept: the list shows install dates, and a timestamp would imply a
/// precision the registry side of the app cannot match.
///
/// Returns `None` for a zero or negative value — packages provisioned during
/// Windows setup sometimes carry no real date, and a fabricated 1601 is
/// worse than an empty cell.
pub fn iso_date_from_winrt(universal_time: i64) -> Option<String> {
    if universal_time <= 0 {
        return None;
    }
    let days_since_1601 = universal_time / 10_000_000 / 86_400;
    let days = days_since_1601.checked_sub(DAYS_1601_TO_1970)?;
    let (y, m, d) = civil_from_days(days);
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// Days since 1970-01-01 to a proleptic Gregorian calendar date.
///
/// Hinnant's civil-from-days: shifts the epoch to 0000-03-01 so leap days
/// land at the end of the cycle, which removes every February special case.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11], March = 0
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Package full names are used as API arguments, never as command text, but
/// they are still manifest-provided strings. Windows constrains them to a
/// documented shape (`Name_Version_Arch__PublisherId`); anything outside it
/// is not a name this app produced and is refused rather than passed on.
pub fn is_valid_package_full_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 512
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && name.contains('_')
}

#[cfg(windows)]
mod platform {
    use super::*;
    use windows::core::HSTRING;
    use windows::ApplicationModel::PackageSignatureKind;
    use windows::Management::Deployment::PackageManager;

    /// Reads every package registered for the current user.
    ///
    /// A package whose manifest cannot be read is skipped rather than
    /// failing the whole list: one damaged entry must not hide the other
    /// hundred and fifty.
    pub fn list() -> Result<Vec<StoreApp>, String> {
        let manager = PackageManager::new().map_err(|e| format!("package manager: {e}"))?;
        // An empty user SID means "the user this process runs as".
        let packages = manager
            .FindPackagesByUserSecurityId(&HSTRING::new())
            .map_err(|e| format!("enumerate packages: {e}"))?;

        let mut out = Vec::new();
        for package in packages {
            if let Some(app) = read_one(&package) {
                out.push(app);
            }
        }
        out.sort_by_key(|a| a.name.to_ascii_lowercase());
        Ok(out)
    }

    fn read_one(package: &windows::ApplicationModel::Package) -> Option<StoreApp> {
        let id = package.Id().ok()?;
        let full_name = id.FullName().ok()?.to_string();
        if !is_valid_package_full_name(&full_name) {
            return None;
        }

        // DisplayName reads the manifest's localized resources. For internal
        // components it either fails outright or hands back the unresolved
        // `ms-resource:` reference; both mean "Windows has no name to show a
        // user for this". The manifest Name is the honest fallback — better
        // than dropping the row — but the fact that it was needed is itself
        // evidence the package is infrastructure, so it is carried forward.
        let display_name = package
            .DisplayName()
            .ok()
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
            .filter(|s| !s.starts_with("ms-resource:"));
        let has_display_name = display_name.is_some();
        let name = display_name.or_else(|| id.Name().ok().map(|s| s.to_string()))?;

        let publisher = package
            .PublisherDisplayName()
            .ok()
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty());

        let version = id
            .Version()
            .ok()
            .map(|v| format!("{}.{}.{}.{}", v.Major, v.Minor, v.Build, v.Revision))
            .unwrap_or_default();

        let install_location = package
            .InstalledPath()
            .ok()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        let install_date = package
            .InstalledDate()
            .ok()
            .and_then(|d| iso_date_from_winrt(d.UniversalTime));

        let is_framework = package.IsFramework().unwrap_or(false);
        let signature = package.SignatureKind().ok();
        let is_system = signature == Some(PackageSignatureKind::System);
        let is_store_signed = signature == Some(PackageSignatureKind::Store);
        let is_resource = package.IsResourcePackage().unwrap_or(false);

        let meta = StoreAppMeta {
            name: id
                .Name()
                .ok()
                .map(|s| s.to_string())
                .unwrap_or_else(|| name.clone()),
            publisher: publisher.clone(),
            has_display_name,
            is_framework,
            is_system,
            is_store_signed,
            is_resource,
        };

        Some(StoreApp {
            id: full_name,
            name,
            publisher,
            version,
            install_location,
            install_date,
            confidence: classify(&meta),
            is_framework,
            is_system,
            hidden: is_framework || is_resource || !has_display_name,
        })
    }

    /// Removes one package for the current user.
    ///
    /// Per-user removal needs no elevation, which is why this path never
    /// goes through `elevation.rs`: asking for administrator rights the
    /// operation does not require would be its own small dishonesty.
    pub fn remove(full_name: &str) -> Result<(), String> {
        if !is_valid_package_full_name(full_name) {
            return Err("invalid package name".to_string());
        }
        let manager = PackageManager::new().map_err(|e| format!("package manager: {e}"))?;
        let operation = manager
            .RemovePackageAsync(&HSTRING::from(full_name))
            .map_err(|e| format!("remove package: {e}"))?;
        let result = operation
            .get()
            .map_err(|e| format!("remove package: {e}"))?;

        // A deployment can report failure through the result rather than as
        // an error on the call, so the extended code is checked explicitly.
        let code = result.ExtendedErrorCode().unwrap_or_default();
        if code.is_err() {
            let text = result
                .ErrorText()
                .map(|s| s.to_string())
                .unwrap_or_else(|_| String::new());
            let detail = if text.trim().is_empty() {
                format!("{code:?}")
            } else {
                text
            };
            return Err(format!("Windows refused the removal: {detail}"));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod platform {
    use super::StoreApp;

    pub fn list() -> Result<Vec<StoreApp>, String> {
        Ok(Vec::new())
    }

    pub fn remove(_full_name: &str) -> Result<(), String> {
        Err("Store apps are a Windows feature".to_string())
    }
}

#[tauri::command(async)]
pub fn list_store_apps() -> Result<Vec<StoreApp>, String> {
    platform::list()
}

#[tauri::command(async)]
pub fn remove_store_app(package_full_name: String) -> Result<(), String> {
    let result = platform::remove(&package_full_name);
    crate::applog::line(&match &result {
        Ok(()) => format!("store package removed: {package_full_name}"),
        Err(e) => format!("store package removal failed: {package_full_name}: {e}"),
    });
    result
}

/// Exercises the real WinRT path on the machine running the tests.
///
/// The pure rules above are covered on any platform; these prove the API
/// calls themselves still bind, which no amount of unit testing can.
#[cfg(all(test, windows))]
mod live_tests {
    use super::*;

    #[test]
    fn the_package_manager_returns_this_machines_packages() {
        let apps = platform::list().expect("enumeration should succeed on Windows");
        assert!(
            apps.len() > 10,
            "a Windows 11 machine has dozens of packages, got {}",
            apps.len()
        );

        for app in &apps {
            assert!(!app.id.is_empty(), "every package needs its full name");
            assert!(
                is_valid_package_full_name(&app.id),
                "enumeration produced a name removal would refuse: {}",
                app.id
            );
            assert!(!app.name.trim().is_empty(), "empty name for {}", app.id);
            assert!(
                !app.confidence.reasons.is_empty(),
                "unexplained verdict for {}",
                app.id
            );
        }
    }

    #[test]
    fn frameworks_are_never_offered_as_safe_on_real_data() {
        // The rule that matters most in practice: a shared runtime must not
        // be presented as a casual removal on this machine's actual mix.
        for app in platform::list().expect("enumeration") {
            if app.is_framework {
                assert_eq!(
                    app.confidence.level,
                    ConfidenceLevel::Keep,
                    "{} is a framework but was banded {:?}",
                    app.id,
                    app.confidence.level
                );
            }
        }
    }

    #[test]
    fn infrastructure_packages_are_hidden_from_the_default_list() {
        // Frameworks and nameless components exist in quantity on a real
        // machine; none of them belong in the list a user opens first.
        for app in platform::list().expect("enumeration") {
            if app.is_framework {
                assert!(app.hidden, "{} is a framework but was listed", app.id);
            }
        }
    }

    #[test]
    fn a_removal_windows_cannot_perform_surfaces_as_an_error() {
        // Well-formed enough to clear the name guard, but no such package is
        // installed, so this drives the whole API path — PackageManager,
        // RemovePackageAsync, the blocking get, and the DeploymentResult
        // error branch — without removing anything that exists.
        let missing = "PcTweakerUninstaller.NoSuchPackage_0.0.0.0_x64__0000000000000";
        assert!(
            is_valid_package_full_name(missing),
            "the guard should pass this"
        );
        let result = platform::remove(missing);
        assert!(
            result.is_err(),
            "removing a missing package must not report success"
        );
    }

    #[test]
    fn removal_refuses_a_name_that_did_not_come_from_enumeration() {
        // No package is touched: the guard rejects the shape before any
        // deployment call is made.
        assert!(platform::remove("not a package name").is_err());
        assert!(platform::remove("").is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(name: &str) -> StoreAppMeta {
        StoreAppMeta {
            name: name.to_string(),
            publisher: Some("Contoso".to_string()),
            has_display_name: true,
            ..Default::default()
        }
    }

    #[test]
    fn a_framework_is_kept_because_other_packages_link_against_it() {
        let m = StoreAppMeta {
            is_framework: true,
            ..meta("Microsoft.VCLibs.140.00")
        };
        let c = classify(&m);
        assert_eq!(c.level, ConfidenceLevel::Keep);
        assert!(c.reasons.contains(&"storeFramework"));
    }

    #[test]
    fn the_shell_host_is_kept_even_though_windows_would_remove_it() {
        let c = classify(&meta("Microsoft.Windows.StartMenuExperienceHost"));
        assert_eq!(c.level, ConfidenceLevel::Keep);
        assert!(c.reasons.contains(&"storeOsComponent"));
    }

    #[test]
    fn os_component_matching_is_case_insensitive() {
        let c = classify(&meta("MICROSOFT.UI.XAML.2.8"));
        assert_eq!(c.level, ConfidenceLevel::Keep);
    }

    #[test]
    fn a_resource_package_is_not_an_application() {
        let m = StoreAppMeta {
            is_resource: true,
            ..meta("Contoso.App.Language.it")
        };
        assert_eq!(classify(&m).level, ConfidenceLevel::Keep);
    }

    #[test]
    fn a_component_windows_gives_no_name_to_is_kept() {
        let m = StoreAppMeta {
            has_display_name: false,
            ..meta("1527c705-839a-4832-9118-54d4bd6a0c89")
        };
        let c = classify(&m);
        assert_eq!(c.level, ConfidenceLevel::Keep);
        assert!(c.reasons.contains(&"storeNoDisplayName"));
    }

    #[test]
    fn a_package_shipped_with_windows_asks_for_review() {
        let m = StoreAppMeta {
            is_system: true,
            ..meta("Microsoft.XboxGameOverlay")
        };
        let c = classify(&m);
        assert_eq!(c.level, ConfidenceLevel::Review);
        assert!(c.reasons.contains(&"storeSystemSigned"));
    }

    #[test]
    fn a_missing_publisher_is_reviewable_not_safe() {
        let m = StoreAppMeta {
            publisher: None,
            ..meta("Some.Sideloaded.Package")
        };
        let c = classify(&m);
        assert_eq!(c.level, ConfidenceLevel::Review);
        assert!(c.reasons.contains(&"storeNoPublisher"));
    }

    #[test]
    fn a_blank_publisher_counts_as_missing() {
        let m = StoreAppMeta {
            publisher: Some("   ".to_string()),
            ..meta("Some.Package")
        };
        assert_eq!(classify(&m).level, ConfidenceLevel::Review);
    }

    #[test]
    fn an_ordinary_store_install_is_safe() {
        let m = StoreAppMeta {
            is_store_signed: true,
            ..meta("Contoso.Notepad")
        };
        let c = classify(&m);
        assert_eq!(c.level, ConfidenceLevel::Safe);
        assert!(c.reasons.contains(&"storeFromStore"));
        assert!(c.reasons.contains(&"storeCleanRemoval"));
    }

    #[test]
    fn keep_wins_over_review_when_both_apply() {
        // A framework that also ships with Windows must land in Keep: the
        // stronger reason decides the band, not the order they were found.
        let m = StoreAppMeta {
            is_framework: true,
            is_system: true,
            ..meta("Microsoft.NET.Native.Framework.2.2")
        };
        assert_eq!(classify(&m).level, ConfidenceLevel::Keep);
    }

    #[test]
    fn every_verdict_explains_itself() {
        for m in [
            meta("Contoso.App"),
            StoreAppMeta {
                is_framework: true,
                ..meta("A.B")
            },
            StoreAppMeta {
                is_system: true,
                ..meta("C.D")
            },
            StoreAppMeta {
                publisher: None,
                ..meta("E.F")
            },
        ] {
            assert!(
                !classify(&m).reasons.is_empty(),
                "silent verdict for {}",
                m.name
            );
        }
    }

    #[test]
    fn a_well_formed_package_name_is_accepted() {
        assert!(is_valid_package_full_name(
            "Microsoft.WindowsCalculator_11.2210.0.0_x64__8wekyb3d8bbwe"
        ));
    }

    #[test]
    fn names_carrying_command_syntax_are_refused() {
        for bad in [
            "",
            "NoUnderscoreHere",
            "Package_1.0; Remove-Item C:\\",
            "Package_1.0 && calc.exe",
            "Package_1.0|calc",
            "Package_1.0`ncalc",
            "Package_1.0 -Force",
            "Package_$(whoami)_x64",
        ] {
            assert!(
                !is_valid_package_full_name(bad),
                "should have refused: {bad:?}"
            );
        }
    }

    #[test]
    fn an_absurdly_long_name_is_refused() {
        let long = format!("A_{}", "b".repeat(600));
        assert!(!is_valid_package_full_name(&long));
    }

    // --- WinRT date conversion -------------------------------------------

    #[test]
    fn the_unix_epoch_converts_exactly() {
        // 1970-01-01 is 134774 days after the 1601 epoch.
        let ticks = 134_774i64 * 86_400 * 10_000_000;
        assert_eq!(iso_date_from_winrt(ticks).as_deref(), Some("1970-01-01"));
    }

    #[test]
    fn a_known_date_round_trips() {
        // 2024-02-29 — a leap day, the case the calendar maths exists for.
        let days = 134_774i64 + 19_782; // 19782 days after 1970-01-01
        let ticks = days * 86_400 * 10_000_000;
        assert_eq!(iso_date_from_winrt(ticks).as_deref(), Some("2024-02-29"));
    }

    #[test]
    fn a_century_non_leap_year_is_handled() {
        // 1900 was not a leap year; 2000 was. 2000-03-01 pins the rule.
        let days = 134_774i64 + 11_017;
        let ticks = days * 86_400 * 10_000_000;
        assert_eq!(iso_date_from_winrt(ticks).as_deref(), Some("2000-03-01"));
    }

    #[test]
    fn an_unset_date_is_absent_rather_than_1601() {
        assert_eq!(iso_date_from_winrt(0), None);
        assert_eq!(iso_date_from_winrt(-1), None);
    }

    #[test]
    fn dates_advance_monotonically_across_a_year() {
        let base = 134_774i64 + 20_000;
        let mut previous = String::new();
        for offset in 0..400 {
            let ticks = (base + offset) * 86_400 * 10_000_000;
            let got = iso_date_from_winrt(ticks).expect("a real date");
            assert!(got > previous, "{got} did not follow {previous}");
            previous = got;
        }
    }
}
