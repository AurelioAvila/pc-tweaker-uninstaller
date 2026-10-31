//! Phase 3: the uninstall executor — dry-run plan, restore point, allowlisted
//! execution, report.
//!
//! Trust model. The webview supplies a `source` view name, a registry key
//! name and the fingerprint of the plan the user confirmed. The first two are
//! validated by shape, then EVERYTHING else — the command, the arguments,
//! whether execution is allowed at all — is re-derived fresh from the
//! registry at execution time, behind the same fail-closed parser the listing
//! uses ([`crate::uninstall_command`]). The preview the user confirmed is
//! never itself executed; it is rebuilt, so the webview can never supply a
//! command. The rebuilt plan must carry the confirmed fingerprint (the
//! entry's uninstall strings and the command derived from them), checked
//! here and again inside the elevated child: a registry change made after the
//! confirmation, even during the UAC prompt, stops the removal instead of
//! running a command nobody saw.
//!
//! Execution rules, in order:
//! - MSI: the command line is rebuilt from scratch as
//!   `%SystemRoot%\System32\msiexec.exe /x {GUID} /qn /norestart` — the GUID
//!   is the only registry-derived byte in it, and it already passed a strict
//!   shape check.
//! - Executable: the parsed `.exe` path must be absolute and must exist as a
//!   regular file on disk (the second gate). Arguments are passed as an
//!   argv array to `CreateProcess` — no shell exists anywhere in this path.
//! - Everything else (interpreters, non-.exe, unparsable): refused.
//!
//! Elevation. Machine-wide uninstalls relaunch this executable elevated in a
//! headless mode (`--elevated-uninstall <source> <id> <fingerprint>`): one
//! UAC consent per action, and the elevated child re-validates and re-derives
//! everything — it trusts nothing from the parent but those values. It
//! sends its report back over a pipe the parent created (elevation.rs) and
//! writes nothing to disk; the folders it relies on come from Windows, not
//! from environment variables the user's programs can change.

use crate::programs::{self, RawEntry};
use crate::restore_point::{self, RestorePointOutcome};
use crate::uninstall_command::{self, Classification};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Must match `tauri.conf.json`'s identifier: the headless elevated child has
/// no Tauri context, so the app data dir is derived from this constant.
const APP_IDENTIFIER: &str = "com.aurel.pc-tweaker-uninstaller";

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

/// Per-user entries live in HKCU, which any program running as this user can
/// write. Elevating such an entry would let that program borrow this app's
/// signed UAC prompt to start whatever it registered. A per-user uninstaller
/// is therefore run elevated only when its executable sits under Program
/// Files AND only administrators can change it and every folder between it
/// and Program Files: an installer that loosened its folder's permissions
/// (Users: Modify is common) leaves a place where anyone can swap the
/// uninstaller or drop a DLL beside it. `None` when elevation may go ahead,
/// else the reason shown to the user.
fn elevation_refusal(source: &str, kind: &PlanKind, command: &[String]) -> Option<&'static str> {
    if source != "user" {
        return None;
    }
    // Windows' own folder is left out on purpose: its tools are signed and
    // protected, but a per-user entry can still point one of them at a
    // script, and elevating that would run the script as administrator.
    let exe = match kind {
        PlanKind::Msi => None,
        PlanKind::Executable => command.first(),
    };
    let roots = program_roots();
    match exe {
        Some(exe) if is_under_protected_root(exe, &roots) => {
            (!admin_only_path(exe, &roots)).then_some(PER_USER_ELEVATION_WRITABLE)
        }
        _ => Some(PER_USER_ELEVATION_REFUSED),
    }
}

/// Windows' well-known accounts that may change an elevated uninstaller:
/// SYSTEM, Administrators and TrustedInstaller, plus CREATOR OWNER and OWNER
/// RIGHTS, which only ever stand for an owner that must itself be one of the
/// first three.
const ADMIN_SIDS: &[&str] = &[
    "S-1-5-18",
    "S-1-5-32-544",
    "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464",
    "S-1-3-0",
    "S-1-3-4",
];
/// Rights that let someone replace a file or folder, add one beside it, or
/// take it over: write/add file, append/add folder, delete child, delete,
/// change permissions, take ownership, and the generic write/all bits.
const CHANGE_RIGHTS: u32 =
    0x2 | 0x4 | 0x40 | 0x1_0000 | 0x4_0000 | 0x8_0000 | 0x1000_0000 | 0x4000_0000;

/// One entry of an access list as [`read_access`] reports it. Entry types
/// other than plain allow and deny are reported as an allow of everything to
/// nobody known, so they count against the path.
#[derive(Debug, Clone)]
pub struct AccessEntry {
    pub allow: bool,
    pub inherit_only: bool,
    pub mask: u32,
    pub sid: String,
}

/// The pure rule: the owner is SYSTEM, Administrators or TrustedInstaller,
/// and no entry that applies to this object lets anyone else change it. No
/// access list at all means everyone may do anything. Deny entries are
/// ignored, which can only make the answer stricter.
pub fn only_admins_can_change(owner: &str, dacl: Option<&[AccessEntry]>) -> bool {
    let Some(entries) = dacl else {
        return false;
    };
    ADMIN_SIDS[..3].contains(&owner)
        && entries.iter().all(|e| {
            !e.allow
                || e.inherit_only
                || e.mask & CHANGE_RIGHTS == 0
                || ADMIN_SIDS.contains(&e.sid.as_str())
        })
}

/// True when the real file behind `exe` (links resolved) is still under one
/// of `roots`, and it and every folder up to that root pass
/// [`only_admins_can_change`]. Anything unreadable counts as changeable.
fn admin_only_path(exe: &str, roots: &[String]) -> bool {
    let Ok(real) = std::fs::canonicalize(exe) else {
        return false;
    };
    let real = real.to_string_lossy();
    let real = real.strip_prefix(r"\\?\").unwrap_or(&real).to_string();
    let Some(root) = roots
        .iter()
        .find(|root| is_under_protected_root(&real, std::slice::from_ref(root)))
    else {
        return false;
    };
    let root = root.trim_end_matches('\\').to_lowercase();
    let mut path = std::path::Path::new(&real);
    while path.to_string_lossy().to_lowercase() != root {
        match read_access(path) {
            Some((owner, dacl)) if only_admins_can_change(&owner, dacl.as_deref()) => {}
            _ => return false,
        }
        let Some(parent) = path.parent() else {
            return false;
        };
        path = parent;
    }
    true
}

/// The owner and access list of a file or folder, SIDs as `S-1-…` strings.
/// `None` when they cannot be read; an inner `None` is a NULL access list.
#[cfg(windows)]
#[allow(clippy::type_complexity)]
pub fn read_access(path: &std::path::Path) -> Option<(String, Option<Vec<AccessEntry>>)> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::{LocalFree, ERROR_SUCCESS},
        Security::{
            AclSizeInformation,
            Authorization::{ConvertSidToStringSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT},
            GetAce, GetAclInformation, ACE_HEADER, ACL, ACL_SIZE_INFORMATION,
            DACL_SECURITY_INFORMATION, INHERIT_ONLY_ACE, OWNER_SECURITY_INFORMATION, PSID,
        },
    };
    // SAFETY: a NUL-terminated SID string Windows allocated; freed with
    // LocalFree as documented.
    unsafe fn sid_string(sid: PSID) -> Option<String> {
        let mut raw: *mut u16 = std::ptr::null_mut();
        if sid.is_null() || ConvertSidToStringSidW(sid, &mut raw) == 0 || raw.is_null() {
            return None;
        }
        let len = (0..).take_while(|&i| *raw.add(i) != 0).count();
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(raw, len));
        LocalFree(raw as _);
        Some(text)
    }
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let mut owner: PSID = std::ptr::null_mut();
    let mut dacl: *mut ACL = std::ptr::null_mut();
    let mut descriptor = std::ptr::null_mut();
    // SAFETY: valid out pointers; owner and dacl point into `descriptor`,
    // which is freed with LocalFree after the last read, also on failure.
    unsafe {
        let status = GetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            std::ptr::null_mut(),
            &mut dacl,
            std::ptr::null_mut(),
            &mut descriptor,
        );
        let result = (|| {
            if status != ERROR_SUCCESS {
                return None;
            }
            let owner = sid_string(owner)?;
            if dacl.is_null() {
                return Some((owner, None));
            }
            let mut info: ACL_SIZE_INFORMATION = std::mem::zeroed();
            if GetAclInformation(
                dacl,
                &mut info as *mut _ as *mut _,
                std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            ) == 0
            {
                return None;
            }
            let mut entries = Vec::new();
            for index in 0..info.AceCount {
                let mut ace: *mut std::ffi::c_void = std::ptr::null_mut();
                if GetAce(dacl, index, &mut ace) == 0 || ace.is_null() {
                    return None;
                }
                let header = &*(ace as *const ACE_HEADER);
                let inherit_only = u32::from(header.AceFlags) & INHERIT_ONLY_ACE != 0;
                // ACCESS_ALLOWED_ACE_TYPE 0 and ACCESS_DENIED_ACE_TYPE 1 share
                // one layout: header, mask, then the SID.
                entries.push(match header.AceType {
                    0 | 1 => {
                        let mask =
                            std::ptr::read_unaligned((ace as *const u8).add(4).cast::<u32>());
                        AccessEntry {
                            allow: header.AceType == 0,
                            inherit_only,
                            mask,
                            sid: sid_string((ace as *mut u8).add(8) as PSID)?,
                        }
                    }
                    _ => AccessEntry {
                        allow: true,
                        inherit_only,
                        mask: u32::MAX,
                        sid: String::new(),
                    },
                });
            }
            Some((owner, Some(entries)))
        })();
        LocalFree(descriptor as _);
        result
    }
}

#[cfg(not(windows))]
#[allow(clippy::type_complexity)]
pub fn read_access(_path: &std::path::Path) -> Option<(String, Option<Vec<AccessEntry>>)> {
    None
}

/// From Windows, not `%ProgramFiles%`: the elevated child re-checks this, and
/// the user's own environment reaches it (elevation.rs, `known_folder`).
fn program_roots() -> Vec<String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::{FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86};
        [FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86]
            .iter()
            .filter_map(crate::elevation::known_folder)
            .filter(|root| !root.trim().is_empty())
            .collect()
    }
    #[cfg(not(windows))]
    Vec::new()
}

/// Case-insensitive prefix test on a component boundary. Any `..` component
/// is refused outright, so a path cannot start inside a root and climb out.
pub fn is_under_protected_root(path: &str, roots: &[String]) -> bool {
    let normalized = path.replace('/', "\\");
    if normalized.split('\\').any(|part| part == "..") {
        return false;
    }
    let lower = normalized.to_lowercase();
    roots.iter().any(|root| {
        let root = root.trim_end_matches('\\').to_lowercase();
        lower.len() > root.len() + 1
            && lower.starts_with(&root)
            && lower.as_bytes()[root.len()] == b'\\'
    })
}

const PER_USER_ELEVATION_REFUSED: &str =
    "This program was installed for your account only, and its \
    uninstaller asks for administrator rights from a folder your account can change. It was not \
    started with elevation. If you trust it, run its uninstaller from Windows Settings.";

const PER_USER_ELEVATION_WRITABLE: &str =
    "This program was installed for your account only, and its \
    uninstaller asks for administrator rights. Its folder is under Program Files, but accounts \
    without administrator rights can change it, so it was not started with elevation. If you \
    trust it, run its uninstaller from Windows Settings.";

const COMMAND_CHANGED: &str = "This program's uninstall command changed after you confirmed it, \
    so it was not run. Start the removal again to review the new command.";

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
    /// Removal Confidence Score for the Removal Brief (display only).
    pub confidence: crate::confidence::Confidence,
    /// The registry's own size claim — an estimate, labeled as one.
    pub estimated_size_kb: Option<u32>,
    /// What confirming this plan binds the run to ([`command_fingerprint`]):
    /// `execute_uninstall` and the elevated child refuse any other.
    pub fingerprint: String,
}

/// SHA-256 over the entry's uninstall strings exactly as the registry holds
/// them and the command derived from them, with the entry's identity. Any
/// change to either string, or to how it parses, changes the fingerprint.
pub fn command_fingerprint(source: &str, id: &str, entry: &RawEntry, command: &[String]) -> String {
    use sha2::{Digest, Sha256};
    let material = serde_json::json!([
        source,
        id,
        entry.uninstall_string,
        entry.quiet_uninstall_string,
        command
    ]);
    format!("{:x}", Sha256::digest(material.to_string()))
}

/// The plan as rebuilt now must be the one the user confirmed.
fn check_confirmed(plan: &UninstallPlan, confirmed: &str) -> Result<(), String> {
    if plan.fingerprint == confirmed {
        Ok(())
    } else {
        crate::applog::line(&format!(
            "uninstall command changed after confirmation: {}",
            plan.program_name
        ));
        Err(COMMAND_CHANGED.to_string())
    }
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
/// `/qn` runs fully silent — our own dialog is the progress UI, so no stray
/// Windows Installer window ever pops over the app; `/norestart` keeps the
/// reboot decision with the user (exit code 3010 reports it instead).
pub fn msi_argv(product_code: &str, system_root: &str) -> Vec<String> {
    vec![
        format!(
            r"{}\System32\msiexec.exe",
            system_root.trim_end_matches('\\')
        ),
        "/x".to_string(),
        product_code.to_string(),
        "/qn".to_string(),
        "/norestart".to_string(),
    ]
}

/// The Windows folder as Windows reports it: msiexec runs elevated from here,
/// so `%SystemRoot%` from the user's environment must not choose it.
fn system_root() -> String {
    #[cfg(windows)]
    {
        crate::elevation::known_folder(&windows_sys::Win32::UI::Shell::FOLDERID_Windows)
            .unwrap_or_else(|| r"C:\Windows".to_string())
    }
    #[cfg(not(windows))]
    r"C:\Windows".to_string()
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
    let hidden = !programs::is_listable(&entry);
    let confidence =
        crate::confidence::assess(&entry, hidden, &programs::summarize_uninstall(&entry));
    let estimated_size_kb = entry.estimated_size_kb;
    let (kind, command) = derive_argv(&entry)?;
    let fingerprint = command_fingerprint(source, id, &entry, &command);

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
        confidence,
        estimated_size_kb,
        fingerprint,
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

/// The app's fixed data directory, derived from a constant so the elevated
/// headless child (no Tauri context), the parent, and the ledger all resolve
/// the identical path — and so no path ever rides argv.
pub fn fixed_data_dir() -> Result<PathBuf, String> {
    let base = std::env::var_os("APPDATA")
        .ok_or_else(|| "APPDATA is not set; cannot locate the app data directory.".to_string())?;
    Ok(PathBuf::from(base).join(APP_IDENTIFIER))
}

// ---- Verified space measurement -------------------------------------------

/// Bounded recursive directory size. `None` when the walk hits the entry cap
/// or the directory is unreadable: an unmeasurable folder yields no number,
/// never a wrong one.
pub fn dir_size_capped(path: &std::path::Path, cap: &mut u32) -> Option<u64> {
    let mut total: u64 = 0;
    let entries = std::fs::read_dir(path).ok()?;
    for entry in entries.flatten() {
        if *cap == 0 {
            return None;
        }
        *cap -= 1;
        let meta = entry.metadata().ok()?;
        if meta.is_dir() {
            total += dir_size_capped(&entry.path(), cap)?;
        } else {
            total += meta.len();
        }
    }
    Some(total)
}

/// Measures the install folder the entry recorded before the removal, if it
/// exists. After the removal the entry is gone, so the same recorded path is
/// measured again: a folder still there yields its remaining size.
fn measure_install_dir(location: Option<&str>) -> Option<u64> {
    let path = std::path::Path::new(location?);
    if !path.is_dir() {
        return None;
    }
    // ~20k entries covers normal applications; beyond that, honesty over
    // minutes of disk churn.
    dir_size_capped(path, &mut 20_000)
}

/// Best-effort ledger receipt. A receipt failure never fails the uninstall —
/// the report the user sees is already complete without it.
fn record_receipt(
    source: &str,
    report: &UninstallReport,
    pre_bytes: Option<u64>,
    post_bytes: Option<u64>,
    estimated_size_kb: Option<u32>,
) {
    let verified_freed_kb = match (pre_bytes, post_bytes) {
        // Folder measured before and now gone: everything it held is freed.
        (Some(pre), None) => Some(pre / 1024),
        (Some(pre), Some(post)) if pre > post => Some((pre - post) / 1024),
        _ => None,
    };
    let method = report
        .command
        .first()
        .map(|c| {
            if c.to_ascii_lowercase().ends_with("msiexec.exe") {
                "msi"
            } else {
                "executable"
            }
        })
        .unwrap_or("unknown");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = crate::ledger::append(&crate::ledger::RemovalReceipt {
        ts,
        program_name: report.program_name.clone(),
        source: source.to_string(),
        method: method.to_string(),
        success: report.success,
        exit_code: report.exit_code,
        reboot_required: report.reboot_required,
        restore_point: report.restore_point.ledger_line(),
        restore_point_outcome: Some(report.restore_point.clone()),
        estimated_size_kb,
        verified_freed_kb,
        message: report.message.clone(),
        removed_entry: None,
    });
}

/// The elevated child's last step: its report goes back over the parent's
/// pipe, never into a file. Exit code 0 only when it was handed over.
pub fn send_report(pipe: &str, report: &UninstallReport) -> i32 {
    let mut bytes = serde_json::to_vec(report).unwrap_or_default();
    if bytes.len() > crate::elevation::REPORT_LIMIT {
        // A registry entry can carry megabytes of text; the outcome is what
        // the parent needs, not every byte of the name or command.
        let short = UninstallReport {
            program_name: report.program_name.chars().take(256).collect(),
            command: Vec::new(),
            message: report.message.chars().take(2048).collect(),
            ..report.clone()
        };
        bytes = serde_json::to_vec(&short).unwrap_or_default();
    }
    match crate::elevation::send_report_bytes(pipe, &bytes) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn parse_report(bytes: &[u8]) -> Result<UninstallReport, String> {
    if bytes.is_empty() {
        crate::applog::line("elevated child sent no report");
        return Err("The elevated uninstall finished but left no report.".to_string());
    }
    serde_json::from_slice(bytes).map_err(|_| {
        crate::applog::line(&format!(
            "elevated report unparseable, {} bytes",
            bytes.len()
        ));
        "The elevated uninstall report could not be read.".to_string()
    })
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
    crate::applog::line(&format!(
        "uninstall finished: {program_name} exit {exit_code:?} success {success} after {}ms",
        started.elapsed().as_millis()
    ));
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

/// Many third-party (especially NSIS/Inno-based) uninstallers copy
/// themselves to a temp file, relaunch that copy, and exit almost
/// immediately — so `Command::status()` returns success while the real
/// removal work (deleting the registry key, the install folder) is still
/// running in the detached child. MSI uninstalls via `msiexec /qn` don't
/// have this problem: `msiexec.exe` itself performs the removal and only
/// exits once it's done.
///
/// So after a successful *executable*-kind uninstall, poll the registry
/// briefly for the key to actually disappear before returning — otherwise
/// the list the caller refreshes right after still shows the "removed"
/// program. Bounded: a genuinely stuck/buggy uninstaller must not hang the
/// report forever, so this gives up silently after the cap and returns the
/// report exactly as before.
fn wait_for_removal(source: &str, id: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if programs::read_raw_entry(source, id).is_err() {
            return; // Key is gone — confirmed, no need to wait further.
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// `ERROR_ELEVATION_REQUIRED`: the target executable's manifest demands
/// administrator rights, so a plain spawn is refused by Windows.
const ELEVATION_REQUIRED: i32 = 740;
/// Internal sentinel: tells `execute_sync` to retry through the elevated path.
const NEEDS_ELEVATION_MARKER: &str = "__needs_elevation__";

/// The elevated flow: relaunch self elevated and headless, then take the
/// report the child sent back. The child checks the fingerprint itself.
fn run_via_elevation(source: &str, id: &str, confirmed: &str) -> Result<UninstallReport, String> {
    run_elevated_flag("--elevated-uninstall", source, id, &[confirmed])
}

/// Serializes every registry-changing action this app runs: uninstallers
/// change shared machine state, and two at once is never what was meant.
pub fn lock_exec() -> Result<std::sync::MutexGuard<'static, ()>, String> {
    EXEC_LOCK
        .try_lock()
        .map_err(|_| "Another uninstall is already running. Wait for it to finish.".to_string())
}

/// Relaunches this executable elevated for one headless action and returns
/// the report it sent back. The child keeps no log of its own (it writes
/// nothing to disk), so its outcome is recorded here.
pub fn run_elevated_flag(
    flag: &str,
    source: &str,
    id: &str,
    extra: &[&str],
) -> Result<UninstallReport, String> {
    crate::applog::line(&format!("elevating for {source}:{id}"));
    let values: Vec<&str> = [source, id]
        .into_iter()
        .chain(extra.iter().copied())
        .collect();
    let bytes = crate::elevation::run_elevated_args(flag, &values).map_err(|e| {
        crate::applog::line(&format!("elevation failed for {source}:{id}: {e}"));
        format!("The uninstall did not run: {e}. No changes were made by this app.")
    })?;
    let report = parse_report(&bytes)?;
    crate::applog::line(&format!(
        "elevated {flag} for {source}:{id}: exit {:?} success {} after {}ms",
        report.exit_code, report.success, report.duration_ms
    ));
    Ok(report)
}

fn execute_sync(source: &str, id: &str, confirmed: &str) -> Result<UninstallReport, String> {
    let _guard = lock_exec()?;

    // Re-derive everything fresh; the previewed plan is display-only, but it
    // must still be the one that was confirmed.
    let plan = build_plan(source, id)?;
    check_confirmed(&plan, confirmed)?;

    if plan.needs_elevation {
        return run_via_elevation(source, id, confirmed);
    }

    let skipped = RestorePointOutcome::Skipped {
        reason: "Restore points require administrator rights; this per-user uninstall \
                 runs without one."
            .to_string(),
    };
    match run_and_report(plan.program_name, plan.kind, plan.command, skipped) {
        Err(marker) if marker == NEEDS_ELEVATION_MARKER => {
            // The target's manifest demands admin: one UAC consent, retry,
            // unless the entry could have been planted without admin rights.
            let plan = build_plan(source, id)?;
            check_confirmed(&plan, confirmed)?;
            if let Some(reason) = elevation_refusal(source, &plan.kind, &plan.command) {
                crate::applog::line(&format!("per-user elevation refused for {source}:{id}"));
                return Err(reason.to_string());
            }
            run_via_elevation(source, id, confirmed)
        }
        other => other,
    }
}

/// Entry point for the headless elevated child (`main.rs` routes here for
/// `--elevated-uninstall <source> <id> <fingerprint> <pipe>`). Re-validates
/// and re-derives everything from source and id, runs only the plan whose
/// fingerprint the user confirmed, and sends the report into the parent's
/// pipe. The process exit code only says "the report was handed over" — the
/// uninstall's own outcome lives inside the report.
pub fn run_elevated_child(source: &str, id: &str, confirmed: &str, pipe: &str) -> i32 {
    let report = (|| -> Result<UninstallReport, String> {
        let plan = build_plan(source, id)?;
        // Re-checked here: the child is a plain CLI entry point and must not
        // trust that its caller was this app, and the UAC prompt may have
        // waited long enough for the entry to change.
        check_confirmed(&plan, confirmed)?;
        if let Some(reason) = elevation_refusal(source, &plan.kind, &plan.command) {
            return Err(reason.to_string());
        }
        let restore = restore_point::create_restore_point();
        run_and_report(plan.program_name, plan.kind, plan.command, restore)
    })();

    match report {
        Ok(report) => send_report(pipe, &report),
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
            send_report(pipe, &refused)
        }
    }
}

// ---- Tauri commands --------------------------------------------------------

/// Dry-run preview for the confirmation dialog. Read-only.
#[tauri::command(async)]
pub fn plan_uninstall(source: String, id: String) -> Result<UninstallPlan, String> {
    build_plan(&source, &id)
}

/// Executes the uninstall the user confirmed (`fingerprint` from its plan).
/// Blocking work runs off the async runtime's core threads; the webview stays
/// responsive for the duration. The install folder is measured before and
/// after so the ledger's "space freed" is verified, not just the registry's
/// claim. The entry as read before the run is what the leftover scan uses
/// afterwards (residue.rs), never a path from the webview.
#[tauri::command]
pub async fn execute_uninstall(
    source: String,
    id: String,
    fingerprint: String,
) -> Result<UninstallReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let entry = programs::read_raw_entry(&source, &id).ok();
        let location = entry.as_ref().and_then(|e| e.install_location.clone());
        let estimated_size_kb = entry.as_ref().and_then(|e| e.estimated_size_kb);
        let pre = measure_install_dir(location.as_deref());
        let result = execute_sync(&source, &id, &fingerprint);
        if let Ok(report) = &result {
            // Executable-kind uninstallers can report success before the
            // registry key is actually gone (see `wait_for_removal`); give
            // it a chance to catch up before the caller measures freed
            // space and reloads the program list.
            let is_msi = report
                .command
                .first()
                .is_some_and(|c| c.to_ascii_lowercase().ends_with("msiexec.exe"));
            if report.success && !is_msi {
                wait_for_removal(&source, &id);
            }
            let post = measure_install_dir(location.as_deref());
            record_receipt(&source, report, pre, post, estimated_size_kb);
            if let (true, Some(entry)) = (report.success, &entry) {
                crate::residue::remember_removed(&source, &id, entry);
            }
        }
        result
    })
    .await
    .map_err(|e| format!("The uninstall task failed to run: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots() -> Vec<String> {
        vec![r"C:\Program Files".to_string(), r"C:\Windows\".to_string()]
    }

    #[test]
    fn protected_root_accepts_paths_inside_it() {
        assert!(is_under_protected_root(
            r"C:\Program Files\Vendor\uninst.exe",
            &roots()
        ));
        assert!(is_under_protected_root(
            r"c:\program files\vendor\uninst.exe",
            &roots()
        ));
        assert!(is_under_protected_root(
            r"C:\Windows\System32\msiexec.exe",
            &roots()
        ));
    }

    #[test]
    fn protected_root_refuses_lookalikes_and_escapes() {
        assert!(!is_under_protected_root(
            r"C:\Program Files Evil\uninst.exe",
            &roots()
        ));
        assert!(!is_under_protected_root(r"C:\Program Files", &roots()));
        assert!(!is_under_protected_root(
            r"C:\Program Files\..\Users\me\x.exe",
            &roots()
        ));
        assert!(!is_under_protected_root(
            r"C:\Users\me\AppData\Local\x\uninst.exe",
            &roots()
        ));
    }

    #[test]
    fn machine_entries_may_always_elevate() {
        let cmd = vec![r"D:\Anywhere\uninst.exe".to_string()];
        assert!(elevation_refusal("machine64", &PlanKind::Executable, &cmd).is_none());
        assert!(elevation_refusal("machine32", &PlanKind::Executable, &cmd).is_none());
    }

    #[test]
    fn per_user_entries_outside_protected_folders_never_elevate() {
        let cmd = vec![r"C:\Users\me\AppData\Local\Evil\uninst.exe".to_string()];
        for (kind, cmd) in [
            (PlanKind::Executable, cmd),
            (PlanKind::Msi, Vec::new()),
            (PlanKind::Executable, Vec::new()),
        ] {
            assert_eq!(
                elevation_refusal("user", &kind, &cmd),
                Some(PER_USER_ELEVATION_REFUSED)
            );
        }
        if let Ok(root) = std::env::var("SystemRoot") {
            let tool = vec![format!(r"{root}\System32\diskpart.exe")];
            assert!(elevation_refusal("user", &PlanKind::Executable, &tool).is_some());
        }
    }

    fn ace(allow: bool, inherit_only: bool, mask: u32, sid: &str) -> AccessEntry {
        AccessEntry {
            allow,
            inherit_only,
            mask,
            sid: sid.into(),
        }
    }

    const ADMINS: &str = "S-1-5-32-544";
    const USERS: &str = "S-1-5-32-545";
    const TRUSTED_INSTALLER: &str =
        "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464";

    /// The access list Program Files folders normally carry: full control for
    /// TrustedInstaller, SYSTEM and Administrators, read and run for Users,
    /// full control for CREATOR OWNER on what is created inside only.
    fn program_files_acl() -> Vec<AccessEntry> {
        vec![
            ace(true, false, 0x1F_01FF, TRUSTED_INSTALLER),
            ace(true, false, 0x1F_01FF, "S-1-5-18"),
            ace(true, false, 0x1F_01FF, ADMINS),
            ace(true, false, 0x12_00A9, USERS),
            ace(true, true, 0x1000_0000, "S-1-3-0"),
            ace(true, false, 0x12_00A9, "S-1-15-2-1"),
        ]
    }

    #[test]
    fn an_admin_only_folder_may_hold_an_elevated_uninstaller() {
        let acl = program_files_acl();
        assert!(only_admins_can_change(TRUSTED_INSTALLER, Some(&acl)));
        assert!(only_admins_can_change(ADMINS, Some(&acl)));
        // A deny entry can only take rights away.
        let mut denied = acl.clone();
        denied.push(ace(false, false, 0x1F_01FF, USERS));
        assert!(only_admins_can_change("S-1-5-18", Some(&denied)));
    }

    #[test]
    fn a_folder_others_can_change_never_holds_an_elevated_uninstaller() {
        // Users: Modify, the loosened permission some installers set.
        let mut modify = program_files_acl();
        modify.push(ace(true, false, 0x13_01BF, USERS));
        assert!(!only_admins_can_change(ADMINS, Some(&modify)));
        // Only "create files" for Authenticated Users is enough to plant a DLL.
        let mut add_file = program_files_acl();
        add_file.push(ace(true, false, 0x2, "S-1-5-11"));
        assert!(!only_admins_can_change(ADMINS, Some(&add_file)));
        // So are delete, change permissions and take ownership, and generic write.
        for mask in [0x1_0000, 0x4_0000, 0x8_0000, 0x4000_0000, 0x40] {
            let mut acl = program_files_acl();
            acl.push(ace(true, false, mask, "S-1-1-0"));
            assert!(!only_admins_can_change(ADMINS, Some(&acl)), "{mask:#x}");
        }
        // An owner who is a standard user can rewrite the list at will.
        let user = "S-1-5-21-1-2-3-1001";
        assert!(!only_admins_can_change(user, Some(&program_files_acl())));
        // No access list at all lets everyone in.
        assert!(!only_admins_can_change(ADMINS, None));
        // An entry type this app does not parse counts against the folder.
        let mut unknown = program_files_acl();
        unknown.push(ace(true, false, u32::MAX, ""));
        assert!(!only_admins_can_change(ADMINS, Some(&unknown)));
        // An inherit-only grant does not apply to the folder itself.
        let mut inherit_only = program_files_acl();
        inherit_only.push(ace(true, true, 0x1F_01FF, USERS));
        assert!(only_admins_can_change(ADMINS, Some(&inherit_only)));
    }

    /// The real access lists: Windows' own msiexec.exe and its folders are
    /// administrator-only; a folder this test creates in the user's temporary
    /// folder is not, so an uninstaller there would never be elevated even if
    /// that folder were treated as a protected root.
    #[cfg(windows)]
    #[test]
    fn real_access_lists_are_read_and_judged() {
        let windows =
            crate::elevation::known_folder(&windows_sys::Win32::UI::Shell::FOLDERID_Windows)
                .unwrap();
        let msiexec = format!(r"{windows}\System32\msiexec.exe");
        assert!(admin_only_path(&msiexec, std::slice::from_ref(&windows)));

        let dir = std::env::temp_dir().join(format!("pcu-acl-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Vendor")).unwrap();
        let exe = dir.join(r"Vendor\uninst.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        let root = dir.to_string_lossy().into_owned();
        let exe = exe.to_string_lossy().into_owned();
        assert!(is_under_protected_root(&exe, std::slice::from_ref(&root)));
        assert!(!admin_only_path(&exe, std::slice::from_ref(&root)));
        // A missing file or one outside every root is never admin-only.
        assert!(!admin_only_path(
            &format!(r"{root}\Vendor\missing.exe"),
            std::slice::from_ref(&root)
        ));
        assert!(!admin_only_path(&msiexec, std::slice::from_ref(&root)));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_fingerprint_binds_both_uninstall_strings_and_the_command() {
        let entry = entry_with(Some(r#""C:\App\unins.exe""#), None);
        let command = vec![r"C:\App\unins.exe".to_string()];
        let confirmed = command_fingerprint("user", "TestKey", &entry, &command);
        assert_eq!(confirmed.len(), 64);
        assert_eq!(
            confirmed,
            command_fingerprint("user", "TestKey", &entry, &command)
        );
        let changed = [
            command_fingerprint(
                "user",
                "TestKey",
                &entry_with(Some(r#""C:\App\unins.exe" /x"#), None),
                &command,
            ),
            command_fingerprint(
                "user",
                "TestKey",
                &entry_with(Some(r#""C:\App\unins.exe""#), Some("x")),
                &command,
            ),
            command_fingerprint("user", "TestKey", &entry, &[r"C:\Evil\x.exe".to_string()]),
            command_fingerprint("machine64", "TestKey", &entry, &command),
            command_fingerprint("user", "OtherKey", &entry, &command),
        ];
        for other in changed {
            assert_ne!(other, confirmed);
        }
    }

    #[test]
    fn a_plan_that_changed_after_confirmation_is_refused() {
        let entry = entry_with(Some(r#""C:\App\unins.exe""#), None);
        let command = vec![r"C:\App\unins.exe".to_string()];
        let plan = UninstallPlan {
            program_name: "Test App".into(),
            kind: PlanKind::Executable,
            command: command.clone(),
            needs_elevation: false,
            will_attempt_restore_point: false,
            warnings: Vec::new(),
            confidence: crate::confidence::assess(
                &entry,
                false,
                &programs::summarize_uninstall(&entry),
            ),
            estimated_size_kb: None,
            fingerprint: command_fingerprint("user", "TestKey", &entry, &command),
        };
        assert!(check_confirmed(&plan, &plan.fingerprint.clone()).is_ok());
        assert_eq!(
            check_confirmed(&plan, &"0".repeat(64)).unwrap_err(),
            COMMAND_CHANGED
        );
        assert_eq!(check_confirmed(&plan, "").unwrap_err(), COMMAND_CHANGED);
    }

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
                "/qn",
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
            restore_point: RestorePointOutcome::Reused { at: 1_792_780_121 },
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
        assert_eq!(
            back.restore_point,
            RestorePointOutcome::Reused { at: 1_792_780_121 }
        );
    }
}
