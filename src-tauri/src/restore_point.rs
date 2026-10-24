//! Windows System Restore around a removal: the restore point requested
//! (elevated only) right before an uninstall runs, what that request really
//! produced, and a forecast the confirmation shows before anything runs.
//!
//! Design constraints, in order:
//! - **No shell.** `Checkpoint-Computer` would mean launching PowerShell;
//!   this module calls `SRSetRestorePointW`, reads the points back through
//!   WMI and reads the Application event log, all natively.
//! - **No hard link-time dependency on srclient.dll.** It does not exist on
//!   Windows editions without System Restore (notably Windows Server), and a
//!   static import would crash the whole app at startup there. The DLL is
//!   loaded dynamically and its absence is just another honest failure.
//! - **"Success" from Windows is not a new restore point.** Windows makes at
//!   most one point per `SystemRestorePointCreationFrequency` (24 hours by
//!   default); inside that window `SRSetRestorePointW` returns success and
//!   creates nothing. So the newest point is read before and after the
//!   request, and only a higher sequence number made during this operation
//!   counts as created. Anything else is reported as exactly what it is. The
//!   policy value is never changed to force a point.
//! - **Best effort.** No outcome blocks an uninstall the user asked for: the
//!   confirmation says beforehand what to expect, the report says what
//!   Windows actually did.
//!
//! The description written into the restore point is a compile-time constant:
//! no registry-derived text (program names are attacker-influencable) is ever
//! embedded in system state.

use serde::{Deserialize, Serialize};

/// What a removal's restore point request really produced, shown in the
/// report and kept in the ledger. `Skipped` is used by callers when the
/// attempt was never made (per-user uninstall, no elevation). Times are Unix
/// seconds.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RestorePointOutcome {
    /// A new point: a higher sequence number than before the request, made
    /// during this operation.
    Created {
        at: u64,
    },
    /// Windows accepted the request and kept its newest existing point.
    Reused {
        at: u64,
    },
    /// Windows accepted the request, but the points could not be read back,
    /// so whether a new one exists is unknown.
    Unverified {
        reason: String,
    },
    Skipped {
        reason: String,
    },
    Failed {
        #[serde(default)]
        cause: FailCause,
        reason: String,
    },
}

/// Why no point was made, so the UI can say it in the user's language.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FailCause {
    /// System Restore is turned off.
    Disabled,
    /// Not enough free disk space for a new point.
    DiskSpace,
    /// This Windows edition has no System Restore.
    Unavailable,
    #[default]
    Error,
}

impl RestorePointOutcome {
    /// The ledger's one English line (the UI shows the localized outcome).
    pub fn ledger_line(&self) -> String {
        match self {
            Self::Created { at } => format!("created {}", utc(*at)),
            Self::Reused { at } => format!(
                "not created: Windows reused the restore point from {}",
                utc(*at)
            ),
            Self::Unverified { reason } => format!("not verified: {reason}"),
            Self::Skipped { reason } => format!("skipped: {reason}"),
            Self::Failed { reason, .. } => format!("failed: {reason}"),
        }
    }
}

/// The newest restore point Windows lists.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Newest {
    pub sequence: u32,
    pub created_at: u64,
}

/// A refused request: why, and the English detail for logs and the ledger.
pub type Failure = (FailCause, String);

/// Clock granularity between this process and the System Restore service.
const SLACK_S: u64 = 5;

/// The decision, kept pure so it is tested with recorded readings. `before`
/// and `after` are the newest point listed around the request; `started` and
/// `finished` bound the operation.
pub fn verdict(
    before: Result<Option<Newest>, String>,
    request: Result<(), Failure>,
    after: Result<Option<Newest>, String>,
    started: u64,
    finished: u64,
) -> RestorePointOutcome {
    if let Err((cause, reason)) = request {
        return RestorePointOutcome::Failed { cause, reason };
    }
    let (before, after) = match (before, after) {
        (Ok(before), Ok(after)) => (before, after),
        (Err(e), _) | (_, Err(e)) => {
            return RestorePointOutcome::Unverified {
                reason: format!("the restore points could not be read: {e}"),
            }
        }
    };
    let Some(after) = after else {
        return RestorePointOutcome::Failed {
            cause: FailCause::Error,
            reason: "Windows reported success, but lists no restore point.".to_string(),
        };
    };
    let higher = before.is_none_or(|b| after.sequence > b.sequence);
    let during = after.created_at + SLACK_S >= started && after.created_at <= finished + SLACK_S;
    if higher && during {
        RestorePointOutcome::Created {
            at: after.created_at,
        }
    } else {
        RestorePointOutcome::Reused {
            at: after.created_at,
        }
    }
}

/// What the confirmation can honestly say before anything runs, read without
/// administrator rights (the points themselves need them).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Forecast {
    /// System Restore is turned off: no point will be made.
    Off,
    /// Windows made a point at `at`, inside its creation window, so it will
    /// most likely reuse that one instead of making a new point.
    Reuse { at: u64 },
    /// No point inside the window: Windows should make a new one.
    New,
}

/// Windows' default `SystemRestorePointCreationFrequency`, in minutes.
const DEFAULT_FREQUENCY_MIN: u32 = 1440;

/// `enabled`: System Restore on (None = unknown); `frequency_min`: the policy
/// value if set; `newest`: when the event log last recorded a created point.
pub fn predict(
    enabled: Option<bool>,
    frequency_min: Option<u32>,
    newest: Option<u64>,
    now: u64,
) -> Forecast {
    if enabled == Some(false) {
        return Forecast::Off;
    }
    let window = u64::from(frequency_min.unwrap_or(DEFAULT_FREQUENCY_MIN)) * 60;
    match newest {
        Some(at) if window > 0 && now.saturating_sub(at) < window => Forecast::Reuse { at },
        _ => Forecast::New,
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Days since 1970-01-01 for a proleptic Gregorian date (H. Hinnant).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// "2026-10-23 18:28:41 UTC", for the ledger's English line.
fn utc(ts: u64) -> String {
    let days = (ts / 86_400) as i64;
    let secs = ts % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} UTC",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

/// Unix seconds from date and time fields, after range checks.
fn unix(y: i64, mo: i64, d: i64, h: i64, mi: i64, s: i64) -> Option<i64> {
    let valid = (1..=12).contains(&mo)
        && (1..=31).contains(&d)
        && (0..24).contains(&h)
        && (0..60).contains(&mi)
        && (0..=60).contains(&s);
    valid.then(|| days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + s)
}

fn digits(s: &str, range: std::ops::Range<usize>) -> Option<i64> {
    let part = s.get(range)?;
    if !part.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

/// WMI's CIM_DATETIME, "yyyymmddHHMMSS.mmmmmmsUUU" with UUU the offset from
/// UTC in minutes, as Unix seconds.
pub fn parse_cim_datetime(s: &str) -> Option<u64> {
    let b = s.as_bytes();
    if b.len() != 25 || b[14] != b'.' || !matches!(b[21], b'+' | b'-') {
        return None;
    }
    let local = unix(
        digits(s, 0..4)?,
        digits(s, 4..6)?,
        digits(s, 6..8)?,
        digits(s, 8..10)?,
        digits(s, 10..12)?,
        digits(s, 12..14)?,
    )?;
    let offset = digits(s, 22..25)? * if b[21] == b'-' { -1 } else { 1 };
    u64::try_from(local - offset * 60).ok()
}

/// The `SystemTime='2026-10-23T18:28:41.9964916Z'` of a rendered event (UTC).
pub fn parse_event_time(xml: &str) -> Option<u64> {
    let start = xml.find("SystemTime=")? + "SystemTime=".len() + 1;
    let t = xml.get(start..start + 20)?;
    let b = t.as_bytes();
    let shape = b[4] == b'-' && b[7] == b'-' && b[10] == b'T' && b[13] == b':' && b[16] == b':';
    if !shape || !matches!(b[19], b'.' | b'Z') {
        return None;
    }
    let ts = unix(
        digits(t, 0..4)?,
        digits(t, 5..7)?,
        digits(t, 8..10)?,
        digits(t, 11..13)?,
        digits(t, 14..16)?,
        digits(t, 17..19)?,
    )?;
    u64::try_from(ts).ok()
}

#[cfg(windows)]
mod imp {
    use super::{FailCause, Failure, Newest};
    use windows::core::{w, IUnknown, Param, BSTR, HRESULT, PCWSTR, VARIANT};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_MULTITHREADED, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
    };
    use windows::Win32::System::Wmi::{
        IEnumWbemClassObject, IWbemLocator, IWbemServices, WbemLocator, WBEM_FLAG_FORWARD_ONLY,
        WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_GENERIC_FLAG_TYPE,
    };
    use windows_sys::Win32::Foundation::{FreeLibrary, BOOL};
    use windows_sys::Win32::System::EventLog::{
        EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtQueryReverseDirection, EvtRender,
        EvtRenderEventXml,
    };
    use windows_sys::Win32::System::LibraryLoader::{
        GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32,
    };

    const BEGIN_SYSTEM_CHANGE: u32 = 100;
    const END_SYSTEM_CHANGE: u32 = 101;
    const APPLICATION_UNINSTALL: u32 = 1;
    /// `ERROR_SERVICE_DISABLED`: System Restore is turned off.
    const SERVICE_DISABLED: u32 = 1058;
    /// `ERROR_DISK_FULL`: System Restore is in standby for lack of space.
    const DISK_FULL: u32 = 112;
    // From rpcdce.h and winerror.h/wbemcli.h.
    const RPC_C_AUTHN_WINNT: u32 = 10;
    const RPC_C_AUTHZ_NONE: u32 = 0;
    const RPC_E_CHANGED_MODE: HRESULT = HRESULT(0x8001_0106_u32 as i32);
    const WBEM_S_TIMEDOUT: HRESULT = HRESULT(0x0004_0004);
    /// A provider that stops answering must not hang the removal.
    const TIMEOUT_MS: i32 = 30_000;

    /// Mirrors `RESTOREPOINTINFOW` from sr.h. `#[repr(C)]` matches the SDK
    /// layout: the i64 lands at its natural 8-byte offset.
    #[repr(C)]
    struct RestorePointInfoW {
        event_type: u32,
        restore_pt_type: u32,
        sequence_number: i64,
        description: [u16; 256],
    }

    /// Mirrors `STATEMGRSTATUS` from sr.h.
    #[repr(C)]
    struct StateMgrStatus {
        status: u32,
        sequence_number: i64,
    }

    type SrSetRestorePointW =
        unsafe extern "system" fn(*const RestorePointInfoW, *mut StateMgrStatus) -> BOOL;

    /// Asks Windows for a point. `Ok` only means Windows accepted the
    /// request; the sequence number it names is logged, never trusted.
    pub fn request() -> Result<i64, Failure> {
        // Fixed, compile-time description — see the module header for why no
        // program name is included.
        const DESCRIPTION: &str = "Before uninstall (PC Tweaker Uninstaller)";

        let dll_name: Vec<u16> = "srclient.dll\0".encode_utf16().collect();
        // System32 only: this runs elevated from a per-user install folder,
        // where a planted srclient.dll would otherwise be found first.
        // SAFETY: `dll_name` is a valid NUL-terminated UTF-16 string that
        // outlives the call.
        let module = unsafe {
            LoadLibraryExW(
                dll_name.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
        if module.is_null() {
            return Err((
                FailCause::Unavailable,
                "System Restore is not available on this Windows edition.".to_string(),
            ));
        }

        // SAFETY: `module` is a live module handle; the proc name is a valid
        // NUL-terminated ANSI string.
        let proc = unsafe { GetProcAddress(module, c"SRSetRestorePointW".as_ptr() as *const u8) };
        let Some(proc) = proc else {
            // SAFETY: `module` came from LoadLibraryExW above.
            unsafe { FreeLibrary(module) };
            return Err((
                FailCause::Unavailable,
                "SRSetRestorePointW was not found in srclient.dll.".to_string(),
            ));
        };
        // SAFETY: the signature below matches the documented export.
        let set_restore_point = unsafe {
            std::mem::transmute::<unsafe extern "system" fn() -> isize, SrSetRestorePointW>(proc)
        };

        let mut description = [0u16; 256];
        for (slot, unit) in description
            .iter_mut()
            .zip(DESCRIPTION.encode_utf16().take(255))
        {
            *slot = unit;
        }

        let info = RestorePointInfoW {
            event_type: BEGIN_SYSTEM_CHANGE,
            restore_pt_type: APPLICATION_UNINSTALL,
            sequence_number: 0,
            description,
        };
        let mut status = StateMgrStatus {
            status: 0,
            sequence_number: 0,
        };

        // SAFETY: both pointers reference live, correctly-shaped structs for
        // the duration of the call.
        let ok = unsafe { set_restore_point(&info, &mut status) };
        if ok != 0 {
            // The documented pairing: close the change the begin call opened,
            // by its sequence number, so Windows finalises the restore point.
            let end = RestorePointInfoW {
                event_type: END_SYSTEM_CHANGE,
                restore_pt_type: 0,
                sequence_number: status.sequence_number,
                description: [0u16; 256],
            };
            let mut end_status = StateMgrStatus {
                status: 0,
                sequence_number: 0,
            };
            // SAFETY: as above, both structs live for the duration of the call.
            unsafe { set_restore_point(&end, &mut end_status) };
        }
        // SAFETY: `module` came from LoadLibraryExW above.
        unsafe { FreeLibrary(module) };

        match (ok != 0, status.status) {
            (true, _) => Ok(status.sequence_number),
            (false, SERVICE_DISABLED) => Err((
                FailCause::Disabled,
                "System Restore is disabled on this system.".to_string(),
            )),
            (false, DISK_FULL) => Err((
                FailCause::DiskSpace,
                "there is not enough free disk space for a restore point.".to_string(),
            )),
            (false, code) => Err((
                FailCause::Error,
                format!("Windows refused to create a restore point (status {code})."),
            )),
        }
    }

    /// WMI's own codes (0x8004xxxx) carry no system message.
    fn wmi_failed(e: windows::core::Error) -> String {
        match e.code().0 as u32 {
            0x8004_1003 => "access denied (administrator rights needed)".to_string(),
            code if e.message().is_empty() => format!("WMI error {code:#010x}"),
            code => format!("{} ({code:#010x})", e.message()),
        }
    }

    /// COM on the calling thread for one read. A thread already in a
    /// single-threaded apartment keeps it (RPC_E_CHANGED_MODE): COM works
    /// there too, and that initialisation is not ours to undo.
    struct Com(bool);

    impl Drop for Com {
        fn drop(&mut self) {
            if self.0 {
                // SAFETY: balances the successful CoInitializeEx in `newest`,
                // on the same thread.
                unsafe { CoUninitialize() };
            }
        }
    }

    /// WMI proxies must impersonate the caller; the default blanket only
    /// identifies it. The blanket belongs to one proxy, so it is set on each.
    fn impersonate(proxy: impl Param<IUnknown>) -> Result<(), String> {
        // SAFETY: `proxy` is a live COM proxy; no principal or auth info.
        unsafe {
            CoSetProxyBlanket(
                proxy,
                RPC_C_AUTHN_WINNT,
                RPC_C_AUTHZ_NONE,
                PCWSTR::null(),
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            )
        }
        .map_err(wmi_failed)
    }

    /// The highest-numbered point in `ROOT\DEFAULT:SystemRestore`. Needs
    /// administrator rights (a standard token gets access denied).
    pub fn newest() -> Result<Option<Newest>, String> {
        // SAFETY: no reserved pointer; balanced by `Com` when it succeeded.
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let _com = if hr == RPC_E_CHANGED_MODE {
            Com(false)
        } else {
            hr.ok().map_err(wmi_failed)?;
            Com(true)
        };
        // SAFETY: COM is initialised on this thread while `_com` lives, and
        // every interface below is released before it (declared later).
        let services: IWbemServices = unsafe {
            let locator: IWbemLocator =
                CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER).map_err(wmi_failed)?;
            locator.ConnectServer(
                &BSTR::from(r"ROOT\DEFAULT"),
                &BSTR::new(),
                &BSTR::new(),
                &BSTR::new(),
                0,
                &BSTR::new(),
                None,
            )
        }
        .map_err(wmi_failed)?;
        impersonate(&services)?;
        // SAFETY: both BSTRs outlive the call.
        let rows: IEnumWbemClassObject = unsafe {
            services.ExecQuery(
                &BSTR::from("WQL"),
                &BSTR::from("SELECT SequenceNumber, CreationTime FROM SystemRestore"),
                WBEM_GENERIC_FLAG_TYPE(WBEM_FLAG_FORWARD_ONLY.0 | WBEM_FLAG_RETURN_IMMEDIATELY.0),
                None,
            )
        }
        .map_err(wmi_failed)?;
        impersonate(&rows)?;

        let mut newest: Option<Newest> = None;
        loop {
            let mut row = [None];
            let mut returned = 0u32;
            // SAFETY: `row` has room for the one object asked for and takes
            // ownership of it; `returned` is a live out-parameter.
            let hr = unsafe { rows.Next(TIMEOUT_MS, &mut row, &mut returned) };
            if hr == WBEM_S_TIMEDOUT {
                return Err("WMI did not answer in time".to_string());
            }
            hr.ok().map_err(wmi_failed)?;
            let [Some(point)] = row else { break };
            if returned == 0 {
                break;
            }
            let mut sequence = VARIANT::new();
            let mut created = VARIANT::new();
            // SAFETY: `point` is a live instance; the VARIANTs are live
            // out-parameters that own what they receive.
            unsafe {
                point
                    .Get(w!("SequenceNumber"), 0, &mut sequence, None, None)
                    .map_err(wmi_failed)?;
                point
                    .Get(w!("CreationTime"), 0, &mut created, None, None)
                    .map_err(wmi_failed)?;
            }
            let sequence = u32::try_from(&sequence)
                .map_err(|_| "a restore point has no sequence number".to_string())?;
            let created_at = BSTR::try_from(&created)
                .ok()
                .and_then(|t| super::parse_cim_datetime(&t.to_string()))
                .ok_or_else(|| "a restore point has an unreadable creation time".to_string())?;
            if newest.is_none_or(|n| sequence > n.sequence) {
                newest = Some(Newest {
                    sequence,
                    created_at,
                });
            }
        }
        Ok(newest)
    }

    /// When the Application log last recorded a created point ("System
    /// Restore", event 8194). Readable without administrator rights; `None`
    /// when there is none or the log cannot be read.
    pub fn newest_logged() -> Option<u64> {
        let path: Vec<u16> = "Application\0".encode_utf16().collect();
        let query: Vec<u16> = "*[System[Provider[@Name='System Restore'] and (EventID=8194)]]\0"
            .encode_utf16()
            .collect();
        // SAFETY: both strings are NUL-terminated and outlive the call.
        let results = unsafe {
            EvtQuery(
                0,
                path.as_ptr(),
                query.as_ptr(),
                EvtQueryChannelPath | EvtQueryReverseDirection,
            )
        };
        if results == 0 {
            return None;
        }
        let mut event = 0isize;
        let mut returned = 0u32;
        // SAFETY: `results` is a live query handle; room for one event.
        let got = unsafe { EvtNext(results, 1, &mut event, 5_000, 0, &mut returned) } != 0;
        let time = (got && returned == 1).then(|| render_time(event)).flatten();
        // SAFETY: both handles came from the calls above and are closed once.
        unsafe {
            if got && returned == 1 {
                EvtClose(event);
            }
            EvtClose(results);
        }
        time
    }

    fn render_time(event: isize) -> Option<u64> {
        let (mut used, mut count) = (0u32, 0u32);
        // SAFETY: a sizing call with no buffer; it only writes the two counts.
        unsafe {
            EvtRender(
                0,
                event,
                EvtRenderEventXml,
                0,
                std::ptr::null_mut(),
                &mut used,
                &mut count,
            )
        };
        if used == 0 || used > 1 << 20 {
            return None;
        }
        let mut buffer = vec![0u16; used.div_ceil(2) as usize];
        // SAFETY: `buffer` holds `used` bytes and outlives the call.
        let ok = unsafe {
            EvtRender(
                0,
                event,
                EvtRenderEventXml,
                (buffer.len() * 2) as u32,
                buffer.as_mut_ptr().cast(),
                &mut used,
                &mut count,
            )
        };
        if ok == 0 {
            return None;
        }
        super::parse_event_time(&String::from_utf16_lossy(&buffer))
    }

    /// System Restore on/off (`RPSessionInterval` is 0 when protection is
    /// off; the policy can also turn it off) and the creation frequency
    /// policy, in minutes.
    pub fn settings() -> (Option<bool>, Option<u32>) {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
        let value =
            |path: &str, name: &str| hklm.open_subkey(path).ok()?.get_value::<u32, _>(name).ok();
        const SR: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore";
        let policy_off = value(
            r"SOFTWARE\Policies\Microsoft\Windows NT\SystemRestore",
            "DisableSR",
        ) == Some(1);
        let enabled = if policy_off {
            Some(false)
        } else {
            value(SR, "RPSessionInterval").map(|v| v >= 1)
        };
        (enabled, value(SR, "SystemRestorePointCreationFrequency"))
    }
}

/// Requests a restore point and reports what really happened. Requires
/// elevation; callers that are not elevated should report `Skipped` instead.
pub fn create_restore_point() -> RestorePointOutcome {
    #[cfg(windows)]
    {
        let before = imp::newest();
        let started = now();
        let request = imp::request();
        let named = request.as_ref().ok().copied();
        let after = match &request {
            Ok(_) => imp::newest(),
            Err(_) => Ok(None),
        };
        crate::applog::line(&format!(
            "restore point: newest before {before:?}, Windows named {named:?}, newest after {after:?}"
        ));
        verdict(before, request.map(|_| ()), after, started, now())
    }
    #[cfg(not(windows))]
    {
        RestorePointOutcome::Failed {
            cause: FailCause::Unavailable,
            reason: "System Restore only exists on Windows.".to_string(),
        }
    }
}

/// The confirmation's forecast. Read-only, no administrator rights needed.
#[tauri::command(async)]
pub fn restore_point_forecast() -> Forecast {
    #[cfg(windows)]
    {
        let (enabled, frequency) = imp::settings();
        predict(enabled, frequency, imp::newest_logged(), now())
    }
    #[cfg(not(windows))]
    {
        Forecast::Off
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-23 18:28:41 UTC: the Windows Update point recorded on the
    /// development PC on 24 Oct 2026.
    const WU: u64 = 1_792_780_121;

    fn point(sequence: u32, created_at: u64) -> Result<Option<Newest>, String> {
        Ok(Some(Newest {
            sequence,
            created_at,
        }))
    }

    #[test]
    fn a_higher_sequence_made_during_the_request_is_created() {
        let start = WU + 86_400 + 60;
        let outcome = verdict(
            point(166, WU),
            Ok(()),
            point(167, start + 12),
            start,
            start + 30,
        );
        assert_eq!(outcome, RestorePointOutcome::Created { at: start + 12 });
    }

    #[test]
    fn the_first_point_ever_is_created_when_made_during_the_request() {
        let outcome = verdict(Ok(None), Ok(()), point(1, WU + 3), WU, WU + 20);
        assert_eq!(outcome, RestorePointOutcome::Created { at: WU + 3 });
    }

    #[test]
    fn success_with_the_same_sequence_is_a_reused_point() {
        // Windows' throttle: success, nothing new, same newest point.
        let start = WU + 3600;
        let outcome = verdict(point(166, WU), Ok(()), point(166, WU), start, start + 2);
        assert_eq!(outcome, RestorePointOutcome::Reused { at: WU });
    }

    #[test]
    fn a_higher_sequence_from_outside_the_operation_is_not_claimed() {
        let start = WU + 3600;
        let outcome = verdict(Ok(None), Ok(()), point(166, WU), start, start + 2);
        assert_eq!(outcome, RestorePointOutcome::Reused { at: WU });
        let late = verdict(
            point(1, WU),
            Ok(()),
            point(2, start + 600),
            start,
            start + 2,
        );
        assert_eq!(late, RestorePointOutcome::Reused { at: start + 600 });
    }

    #[test]
    fn success_with_no_point_listed_is_a_failure() {
        let outcome = verdict(Ok(None), Ok(()), Ok(None), WU, WU + 2);
        assert!(matches!(
            outcome,
            RestorePointOutcome::Failed {
                cause: FailCause::Error,
                ..
            }
        ));
    }

    #[test]
    fn unreadable_points_never_count_as_created() {
        let denied = || Err("Access denied (0x80041003)".to_string());
        let before = verdict(denied(), Ok(()), point(167, WU), WU, WU + 2);
        let after = verdict(point(166, WU - 9), Ok(()), denied(), WU, WU + 2);
        for outcome in [before, after] {
            assert!(matches!(outcome, RestorePointOutcome::Unverified { .. }));
        }
    }

    #[test]
    fn a_refused_request_keeps_its_cause() {
        for cause in [
            FailCause::Disabled,
            FailCause::DiskSpace,
            FailCause::Unavailable,
            FailCause::Error,
        ] {
            let outcome = verdict(
                point(166, WU),
                Err((cause, "x".into())),
                Ok(None),
                WU,
                WU + 1,
            );
            assert_eq!(
                outcome,
                RestorePointOutcome::Failed {
                    cause,
                    reason: "x".into()
                }
            );
        }
    }

    #[test]
    fn cim_datetimes_parse_with_their_offset() {
        assert_eq!(parse_cim_datetime("20261023182841.996491-000"), Some(WU));
        assert_eq!(parse_cim_datetime("20261023202841.000000+120"), Some(WU));
        assert_eq!(parse_cim_datetime("20261023172841.000000-060"), Some(WU));
        assert_eq!(parse_cim_datetime("19700101000000.000000+000"), Some(0));
        assert_eq!(
            parse_cim_datetime("20000229120000.000000+000"),
            Some(951_825_600)
        );
        for bad in [
            "",
            "2026102318284.996491-000",
            "20261323182841.996491-000",
            "2026+023182841.996491-000",
            "20261023182841-996491-000",
            "20261023182841.996491*000",
            "19691231235959.000000+000",
        ] {
            assert_eq!(parse_cim_datetime(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_recorded_event_time_parses() {
        // Rendered event 8194 from the development PC's Application log.
        let xml = "<Event xmlns='http://schemas.microsoft.com/win/2004/08/events/event'><System><Provider Name='System Restore'/><EventID Qualifiers='0'>8194</EventID><Version>0</Version><Level>4</Level><Task>0</Task><Opcode>0</Opcode><Keywords>0x80000000000000</Keywords><TimeCreated SystemTime='2026-10-23T18:28:41.9964916Z'/><EventRecordID>44138</EventRecordID><Correlation/><Execution ProcessID='48000' ThreadID='0'/><Channel>Application</Channel><Computer>DESKTOP-1E118SC</Computer><Security/></System><EventData><Data>C:\\WINDOWS\\system32\\svchost.exe -k netsvcs -p -s wuauserv</Data><Data>Windows Update</Data></EventData></Event>";
        assert_eq!(parse_event_time(xml), Some(WU));
        assert_eq!(
            parse_event_time("<TimeCreated SystemTime=\"2026-10-23T18:28:41Z\"/>"),
            Some(WU)
        );
        assert_eq!(
            parse_event_time("<TimeCreated SystemTime='2026-10-23 18:28'/>"),
            None
        );
        assert_eq!(parse_event_time("<Event/>"), None);
    }

    #[test]
    fn the_forecast_follows_windows_creation_window() {
        assert_eq!(predict(Some(false), None, Some(WU), WU + 60), Forecast::Off);
        assert_eq!(
            predict(Some(true), None, Some(WU), WU + 86_399),
            Forecast::Reuse { at: WU }
        );
        assert_eq!(
            predict(Some(true), None, Some(WU), WU + 86_400),
            Forecast::New
        );
        assert_eq!(predict(None, Some(60), Some(WU), WU + 3_600), Forecast::New);
        assert_eq!(
            predict(None, Some(60), Some(WU), WU + 3_599),
            Forecast::Reuse { at: WU }
        );
        // Frequency 0 turns the throttle off.
        assert_eq!(
            predict(Some(true), Some(0), Some(WU), WU + 1),
            Forecast::New
        );
        assert_eq!(predict(Some(true), None, None, WU), Forecast::New);
        // A clock behind the log still reads as recent, never as new.
        assert_eq!(
            predict(Some(true), None, Some(WU), WU - 30),
            Forecast::Reuse { at: WU }
        );
    }

    #[test]
    fn outcomes_serialize_with_a_kind_tag_for_the_ui() {
        let json = serde_json::to_value(RestorePointOutcome::Skipped {
            reason: "per-user uninstall".to_string(),
        })
        .unwrap();
        assert_eq!(json["kind"], "skipped");
        assert_eq!(json["reason"], "per-user uninstall");

        let reused = serde_json::to_value(RestorePointOutcome::Reused { at: WU }).unwrap();
        assert_eq!(reused, serde_json::json!({"kind": "reused", "at": WU}));
        let failed = serde_json::to_value(RestorePointOutcome::Failed {
            cause: FailCause::DiskSpace,
            reason: "x".into(),
        })
        .unwrap();
        assert_eq!(failed["cause"], "diskSpace");
        let forecast = serde_json::to_value(Forecast::Reuse { at: WU }).unwrap();
        assert_eq!(forecast, serde_json::json!({"kind": "reuse", "at": WU}));
    }

    #[test]
    fn outcomes_round_trip_through_the_report_file_format() {
        let original = RestorePointOutcome::Failed {
            cause: FailCause::Disabled,
            reason: "System Restore is disabled on this system.".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: RestorePointOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(back, original);
        // A failure written without a cause reads as a generic error.
        let old: RestorePointOutcome =
            serde_json::from_str(r#"{"kind":"failed","reason":"old"}"#).unwrap();
        assert!(matches!(
            old,
            RestorePointOutcome::Failed {
                cause: FailCause::Error,
                ..
            }
        ));
    }

    #[test]
    fn ledger_lines_never_say_created_for_a_reused_point() {
        assert_eq!(
            RestorePointOutcome::Reused { at: WU }.ledger_line(),
            "not created: Windows reused the restore point from 2026-10-23 18:28:41 UTC"
        );
        assert_eq!(
            RestorePointOutcome::Created { at: 951_825_600 }.ledger_line(),
            "created 2000-02-29 12:00:00 UTC"
        );
        assert_eq!(
            RestorePointOutcome::Created { at: 0 }.ledger_line(),
            "created 1970-01-01 00:00:00 UTC"
        );
    }

    /// Live, read-only: the forecast and the WMI read on this PC. Run with
    /// `cargo test live_ -- --ignored --nocapture`; never creates anything.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn live_readings_on_this_pc() {
        println!("settings {:?}", imp::settings());
        println!("newest logged {:?}", imp::newest_logged().map(utc));
        println!("forecast {:?}", restore_point_forecast());
        println!("WMI newest {:?}", imp::newest());
    }
}
