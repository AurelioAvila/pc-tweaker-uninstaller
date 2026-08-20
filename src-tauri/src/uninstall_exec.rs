//! Phase 3: the uninstall executor — dry-run plan, restore point, allowlisted
//! execution, report.
//!
//! Trust model. The webview supplies exactly two strings: a `source` view
//! name and a registry key name. Both are validated by shape, then EVERYTHING
//! else — the command, the arguments, whether execution is allowed at all —
//! is re-derived fresh from the registry at execution time, behind the same
//! fail-closed parser the listing uses ([`crate::uninstall_command`]). The
//! preview the user confirmed is never itself executed; it is rebuilt, so a
//! registry change between preview and click cannot smuggle a different
//! command past the confirmation.
//!
//! Execution rules, in order:
//! - MSI: the command line is rebuilt from scratch as
//!   `%SystemRoot%\System32\msiexec.exe /x {GUID} /qb /norestart` — the GUID
//!   is the only registry-derived byte in it, and it already passed a strict
//!   shape check.
//! - Executable: the parsed `.exe` path must be absolute and must exist as a
//!   regular file on disk (the second gate). Arguments are passed as an
//!   argv array to `CreateProcess` — no shell exists anywhere in this path.
//! - Everything else (interpreters, non-.exe, unparsable): refused.
//!
//! Elevation. Machine-wide uninstalls relaunch this executable elevated in a
//! headless mode (`--elevated-uninstall <source> <id>`): one UAC consent per
//! action, and the elevated child re-validates and re-derives everything —
//! it trusts nothing from the parent but those two validated strings. The
//! child writes its report to a FIXED path under the app's own data dir
//! (never a path taken from argv) and the parent reads it back.

use crate::programs::{self, RawEntry};
use crate::restore_point::{self, RestorePointOutcome};
use crate::uninstall_command::{self, Classification};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

/// Must match `tauri.conf.json`'s identifier: the headless elevated child has
/// no Tauri context, so the app data dir is derived from this constant.
const APP_IDENTIFIER: &str = "com.aurel.pc-tweaker-uninstaller";
const REPORT_FILE: &str = "last-uninstall-report.json";

/// One uninstall at a time, app-wide. Uninstallers mutate shared machine
/// state; running two concurrently is never what the user meant.
static EXEC_LOCK: Mutex<()> = Mutex::new(());

// ---- Input validation ------------------------------------------------------

/// Registry key names cannot contain backslashes; everything else here is a
/// sanity cap so a hostile webview (or argv) cannot feed pathological input.
pub fn validate_key_name(id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err("Missing program identifier.".to_string());
    }
    if id.len() > 255 {
        return Err("Program identifier is too long.".to_string());
    }
    if id.contains(['\\', '/']) || id.chars().any(char::is_control) {
        return Err("Program identifier contains invalid characters.".to_string());
    }
    Ok(())
}

pub fn validate_source(source: &str) -> Result<(), String> {
    match source {
        "machine64" | "machine32" | "user" => Ok(()),
        _ => Err("Unknown program source.".to_string()),
    }
}

/// Machine-wide entries live under HKLM and (almost always) install into
/// protected paths: run those elevated up front, one UAC consent, and get a
/// restore point out of it. Per-user entries run at the user's own integrity
/// level first.
fn needs_elevation(source: &str) -> bool {
    source != "user"
}

// ---- Plan (dry run) --------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PlanKind {
    Msi,
    Executable,
}

/// What the confirmation dialog shows. Display only — execution re-derives
/// its own copy of all of this.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UninstallPlan {
    pub program_name: String,
    pub kind: PlanKind,
    /// The exact argv that will run: `command[0]` is the program.
    pub command: Vec<String>,
    pub needs_elevation: bool,
    pub will_attempt_restore_point: bool,
    pub warnings: Vec<String>,
}

/// Picks the command execution will use: the quiet string when it parses to
/// something executable, else the standard one. Interpreter/manual entries
/// are refused with a reason the UI can show verbatim.
pub fn classify_for_execution(entry: &RawEntry) -> Result<Classification, String> {
    let candidates = [
        entry.quiet_uninstall_string.as_deref(),
        entry.uninstall_string.as_deref(),
    ];
    let mut saw_any = false;
    for candidate in candidates.into_iter().flatten() {
        saw_any = true;
        match uninstall_command::parse(candidate) {
            Ok(Classification::ManualOnly { .. }) | Err(_) => {}
            Ok(executable) => return Ok(executable),
        }
    }
    if saw_any {
        Err(
            "This program's uninstall command is not safe to run automatically. \
             Use the program's own uninstaller instead."
                .to_string(),
        )
    } else {
        Err("This program does not declare an uninstall command.".to_string())
    }
}

/// Rebuilds the canonical msiexec argv from the validated GUID alone.
/// `/qb` shows progress without questions; `/norestart` keeps the reboot
/// decision with the user (exit code 3010 reports it instead).
pub fn msi_argv(product_code: &str, system_root: &str) -> Vec<String> {
    vec![
        format!(
            r"{}\System32\msiexec.exe",
            system_root.trim_end_matches('\\')
        ),
        "/x".to_string(),
        product_code.to_string(),
        "/qb".to_string(),
        "/norestart".to_string(),
    ]
}

fn system_root() -> String {
    std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string())
}

/// Pure shape half of the on-disk gate: the executable path must be absolute
/// (`X:\...`) so the command cannot resolve against a cwd an attacker chose.
pub fn validate_exe_path_shape(path: &str) -> Result<(), String> {
    let bytes = path.as_bytes();
    let absolute = bytes.len() > 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/');
    if !absolute {
        return Err("The uninstaller path is not absolute, so it will not be run.".to_string());
    }
    Ok(())
}

/// Full on-disk second gate: shape, then the file must actually exist as a
/// regular file. A vanished uninstaller is refused, not guessed at.
fn gate_executable(path: &str) -> Result<(), String> {
    validate_exe_path_shape(path)?;
    let meta = std::fs::metadata(path)
        .map_err(|_| "The uninstaller executable no longer exists on disk.".to_string())?;
    if !meta.is_file() {
        return Err("The uninstaller path is not a regular file.".to_string());
    }
    Ok(())
}

/// Derives the argv for an entry, applying every gate. This is THE function
/// both the preview and the executor call — there is no second code path.
fn derive_argv(entry: &RawEntry) -> Result<(PlanKind, Vec<String>), String> {
    match classify_for_execution(entry)? {
        Classification::Msi { product_code } => {
            Ok((PlanKind::Msi, msi_argv(&product_code, &system_root())))
        }
        Classification::Executable { path, args } => {
            gate_executable(&path)?;
            let mut argv = vec![path];
            argv.extend(args);
            Ok((PlanKind::Executable, argv))
        }
        Classification::ManualOnly { .. } => {
            Err("This program's uninstall command must be run manually.".to_string())
        }
    }
}

fn build_plan(source: &str, id: &str) -> Result<UninstallPlan, String> {
    validate_source(source)?;
    validate_key_name(id)?;
    let entry = programs::read_raw_entry(source, id)?;
    let program_name = entry.display_name.clone().unwrap_or_else(|| id.to_string());
    let (kind, command) = derive_argv(&entry)?;

    let elevated = needs_elevation(source);
    let mut warnings = Vec::new();
    if !elevated {
        warnings.push(
            "Per-user uninstall: no restore point will be created (that requires \
             administrator rights)."
                .to_string(),
        );
    }
    if kind == PlanKind::Executable {
        warnings.push(
            "This uninstaller is the program's own. It may show its own windows and \
             ask its own questions."
                .to_string(),
        );
    }

    Ok(UninstallPlan {
        program_name,
        kind,
        command,
        needs_elevation: elevated,
        will_attempt_restore_point: elevated,
        warnings,
    })
}

// ---- Execution + report ----------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UninstallReport {
    pub program_name: String,
    pub command: Vec<String>,
    pub restore_point: RestorePointOutcome,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub reboot_required: bool,
    pub message: String,
    pub duration_ms: u64,
}

/// Maps an exit code to what the user should be told. MSI codes are
/// documented Windows Installer error values; plain executables only promise
/// the 0-is-success convention.
pub fn interpret_exit(kind: &PlanKind, code: Option<i32>) -> (bool, bool, String) {
    match (kind, code) {
        (_, Some(0)) => (true, false, "Uninstalled successfully.".to_string()),
        (PlanKind::Msi, Some(3010)) => (
            true,
            true,
            "Uninstalled successfully. A restart is required to finish removing files.".to_string(),
        ),
        (PlanKind::Msi, Some(1602)) => (
            false,
            false,
            "The uninstall was cancelled by the user.".to_string(),
        ),
        (PlanKind::Msi, Some(1605)) => (
            false,
            false,
            "Windows Installer reports this product is not installed — it may already \
             be removed. Refresh the list to check."
                .to_string(),
        ),
        (_, Some(code)) => (
            false,
            false,
            format!(
                "The uninstaller exited with code {code}. The program may not be fully \
                 removed — refresh the list to check."
            ),
        ),
        (_, None) => (
            false,
            false,
            "The uninstaller was terminated before reporting a result.".to_string(),
        ),
    }
}

/// Fixed report location under the app's own data directory. Derived from a
/// constant so the elevated headless child (which has no Tauri context) and
/// the parent resolve the identical path — and so no path ever rides argv.
fn report_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("APPDATA")
        .ok_or_else(|| "APPDATA is not set; cannot locate the app data directory.".to_string())?;
    Ok(PathBuf::from(base).join(APP_IDENTIFIER).join(REPORT_FILE))
}

fn write_report(report: &UninstallReport) -> Result<(), String> {
    let path = report_path()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
    // Atomic-enough for a single-writer report: temp then rename.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

fn read_report() -> Result<UninstallReport, String> {
    let path = report_path()?;
    let bytes = std::fs::read(&path)
        .map_err(|_| "The elevated uninstall finished but left no report.".to_string())?;
    serde_json::from_slice(&bytes)
        .map_err(|_| "The elevated uninstall report could not be read.".to_string())
}

/// Runs the derived argv and assembles the report. `restore_point` is decided
/// by the caller (only the elevated path attempts one).
fn run_and_report(
    program_name: String,
    kind: PlanKind,
    argv: Vec<String>,
    restore_point: RestorePointOutcome,
) -> Result<UninstallReport, String> {
    let started = Instant::now();
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| "Internal error: empty command.".to_string())?;

    let status = std::process::Command::new(program)
        .args(args)
        .status()
        .map_err(|e| {
            if e.raw_os_error() == Some(ELEVATION_REQUIRED) {
                NEEDS_ELEVATION_MARKER.to_string()
            } else {
                format!("The uninstaller could not be started: {e}")
            }
        })?;

    let exit_code = status.code();
    let (success, reboot_required, message) = interpret_exit(&kind, exit_code);
    Ok(UninstallReport {
        program_name,
        command: argv,
        restore_point,
        exit_code,
        success,
        reboot_required,
        message,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

/// `ERROR_ELEVATION_REQUIRED`: the target executable's manifest demands
/// administrator rights, so a plain spawn is refused by Windows.
const ELEVATION_REQUIRED: i32 = 740;
/// Internal sentinel: tells `execute_sync` to retry through the elevated path.
const NEEDS_ELEVATION_MARKER: &str = "__needs_elevation__";

/// The elevated flow: clear any stale report, relaunch self elevated and
/// headless, then read back the report the child wrote.
fn run_via_elevation(source: &str, id: &str) -> Result<UninstallReport, String> {
    if let Ok(path) = report_path() {
        // Best effort: a stale report must never be mistaken for this run's.
        let _ = std::fs::remove_file(path);
    }
    crate::elevation::run_elevated_args(&["--elevated-uninstall", source, id]).map_err(|e| {
        format!("The uninstall did not run: {e}. No changes were made by this app.")
    })?;
    read_report()
}

fn execute_sync(source: &str, id: &str) -> Result<UninstallReport, String> {
    let _guard = EXEC_LOCK
        .try_lock()
        .map_err(|_| "Another uninstall is already running. Wait for it to finish.".to_string())?;

    // Re-derive everything fresh; the previewed plan is display-only.
    let plan = build_plan(source, id)?;

    if plan.needs_elevation {
        return run_via_elevation(source, id);
    }

    let skipped = RestorePointOutcome::Skipped {
        reason: "Restore points require administrator rights; this per-user uninstall \
                 runs without one."
            .to_string(),
    };
    match run_and_report(plan.program_name, plan.kind, plan.command, skipped) {
        Err(marker) if marker == NEEDS_ELEVATION_MARKER => {
            // The target's manifest demands admin: one UAC consent, retry.
            run_via_elevation(source, id)
        }
        other => other,
    }
}

/// Entry point for the headless elevated child (`main.rs` routes here for
/// `--elevated-uninstall <source> <id>`). Re-validates and re-derives
/// everything from the two arguments; writes the report to the fixed path.
/// The process exit code only says "a report was produced" — the uninstall's
/// own outcome lives inside the report.
pub fn run_elevated_child(source: &str, id: &str) -> i32 {
    let report = (|| -> Result<UninstallReport, String> {
        let plan = build_plan(source, id)?;
        let restore = restore_point::create_restore_point();
        run_and_report(plan.program_name, plan.kind, plan.command, restore)
    })();

    match report {
        Ok(report) => match write_report(&report) {
            Ok(()) => 0,
            Err(_) => 1,
        },
        Err(reason) => {
            // Even a refusal is reported, so the parent can show the reason.
            // (The elevation marker cannot occur here — the child IS elevated —
            // but translate it defensively rather than leak a sentinel string.)
            let reason = if reason == NEEDS_ELEVATION_MARKER {
                "Windows refused to start the uninstaller even with elevation.".to_string()
            } else {
                reason
            };
            let refused = UninstallReport {
                program_name: id.to_string(),
                command: Vec::new(),
                restore_point: RestorePointOutcome::Skipped {
                    reason: "The uninstall was refused before it started.".to_string(),
                },
                exit_code: None,
                success: false,
                reboot_required: false,
                message: reason,
                duration_ms: 0,
            };
            match write_report(&refused) {
                Ok(()) => 0,
                Err(_) => 1,
            }
        }
    }
}

// ---- Tauri commands --------------------------------------------------------

/// Dry-run preview for the confirmation dialog. Read-only.
#[tauri::command]
pub fn plan_uninstall(source: String, id: String) -> Result<UninstallPlan, String> {
    build_plan(&source, &id)
}

/// Executes the uninstall. Blocking work runs off the async runtime's core
/// threads; the webview stays responsive for the duration.
#[tauri::command]
pub async fn execute_uninstall(source: String, id: String) -> Result<UninstallReport, String> {
    tauri::async_runtime::spawn_blocking(move || execute_sync(&source, &id))
        .await
        .map_err(|e| format!("The uninstall task failed to run: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_with(uninstall: Option<&str>, quiet: Option<&str>) -> RawEntry {
        RawEntry {
            key_name: "TestKey".into(),
            display_name: Some("Test App".into()),
            uninstall_string: uninstall.map(str::to_string),
            quiet_uninstall_string: quiet.map(str::to_string),
            ..Default::default()
        }
    }

    // ---- Input validation ---------------------------------------------------

    #[test]
    fn key_names_with_separators_or_controls_are_rejected() {
        assert!(validate_key_name("7-Zip").is_ok());
        assert!(validate_key_name("{6F340107-F9AA-47C6-B54C-C3A19F11553C}").is_ok());
        assert!(validate_key_name("").is_err());
        assert!(validate_key_name("   ").is_err());
        assert!(validate_key_name(r"..\..\Escape").is_err());
        assert!(validate_key_name("a/b").is_err());
        assert!(validate_key_name("evil\u{0}name").is_err());
        assert!(validate_key_name(&"x".repeat(256)).is_err());
    }

    #[test]
    fn only_the_three_known_sources_are_accepted() {
        for good in ["machine64", "machine32", "user"] {
            assert!(validate_source(good).is_ok());
        }
        for bad in ["", "MACHINE64", "hklm", "machine64 "] {
            assert!(validate_source(bad).is_err(), "should reject: {bad}");
        }
    }

    // ---- Classification preference ------------------------------------------

    #[test]
    fn quiet_string_wins_when_it_is_executable() {
        let entry = entry_with(
            Some(r#""C:\App\unins.exe""#),
            Some("MsiExec.exe /X{6F340107-F9AA-47C6-B54C-C3A19F11553C}"),
        );
        assert!(matches!(
            classify_for_execution(&entry).unwrap(),
            Classification::Msi { .. }
        ));
    }

    #[test]
    fn a_broken_quiet_string_falls_back_to_the_standard_one() {
        let entry = entry_with(
            Some(r#""C:\App\unins.exe" /SILENT"#),
            Some("msiexec.exe /x {not-a-guid}"),
        );
        assert!(matches!(
            classify_for_execution(&entry).unwrap(),
            Classification::Executable { .. }
        ));
    }

    #[test]
    fn interpreter_only_entries_are_refused_for_execution() {
        let entry = entry_with(Some("cmd.exe /c cleanup.bat"), None);
        assert!(classify_for_execution(&entry).is_err());
        assert!(classify_for_execution(&entry_with(None, None)).is_err());
    }

    // ---- MSI command rebuild --------------------------------------------------

    #[test]
    fn msi_argv_is_rebuilt_from_the_guid_alone() {
        let argv = msi_argv("{6F340107-F9AA-47C6-B54C-C3A19F11553C}", r"C:\Windows");
        assert_eq!(
            argv,
            vec![
                r"C:\Windows\System32\msiexec.exe",
                "/x",
                "{6F340107-F9AA-47C6-B54C-C3A19F11553C}",
                "/qb",
                "/norestart",
            ]
        );
        // A trailing backslash in SystemRoot must not double up.
        let argv = msi_argv("{6F340107-F9AA-47C6-B54C-C3A19F11553C}", "C:\\Windows\\");
        assert_eq!(argv[0], r"C:\Windows\System32\msiexec.exe");
    }

    // ---- Executable path gate --------------------------------------------------

    #[test]
    fn relative_and_bare_paths_never_pass_the_shape_gate() {
        assert!(validate_exe_path_shape(r"C:\Program Files\Foo\unins.exe").is_ok());
        assert!(validate_exe_path_shape("unins.exe").is_err());
        assert!(validate_exe_path_shape(r".\unins.exe").is_err());
        assert!(validate_exe_path_shape(r"..\..\unins.exe").is_err());
        assert!(validate_exe_path_shape(r"\\server\share\unins.exe").is_err());
        assert!(validate_exe_path_shape("").is_err());
    }

    // ---- Exit code interpretation ----------------------------------------------

    #[test]
    fn msi_exit_codes_map_to_honest_messages() {
        let (ok, reboot, _) = interpret_exit(&PlanKind::Msi, Some(0));
        assert!(ok && !reboot);
        let (ok, reboot, _) = interpret_exit(&PlanKind::Msi, Some(3010));
        assert!(ok && reboot);
        let (ok, _, msg) = interpret_exit(&PlanKind::Msi, Some(1602));
        assert!(!ok && msg.contains("cancelled"));
        let (ok, _, _) = interpret_exit(&PlanKind::Msi, Some(1605));
        assert!(!ok);
        let (ok, _, msg) = interpret_exit(&PlanKind::Executable, Some(2));
        assert!(!ok && msg.contains("code 2"));
        let (ok, _, _) = interpret_exit(&PlanKind::Executable, None);
        assert!(!ok);
    }

    // ---- Report round trip -------------------------------------------------------

    #[test]
    fn report_round_trips_through_json() {
        let report = UninstallReport {
            program_name: "Test App".into(),
            command: vec![r"C:\Windows\System32\msiexec.exe".into(), "/x".into()],
            restore_point: RestorePointOutcome::Created,
            exit_code: Some(3010),
            success: true,
            reboot_required: true,
            message: "Uninstalled successfully.".into(),
            duration_ms: 1234,
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: UninstallReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.exit_code, Some(3010));
        assert!(back.reboot_required);
        assert_eq!(back.restore_point, RestorePointOutcome::Created);
    }
}
