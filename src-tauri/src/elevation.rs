#[cfg(windows)]
pub fn is_elevated() -> bool {
    is_elevated::is_elevated()
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

/// Re-launches this executable (absolute path, started in System32) through a
/// UAC consent prompt for one headless action (`--elevated-uninstall` or
/// `--elevated-forget` with its values) and waits for it. Nothing else about
/// the app runs elevated. The values cross encoded, and the relaunched
/// process re-validates and re-derives everything from them. Returns what the
/// action sent back on its report pipe, empty when it sent nothing.
pub fn run_elevated_args(flag: &str, values: &[&str]) -> Result<Vec<u8>, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;

    #[cfg(windows)]
    {
        if in_shared_download_folder(&exe) {
            return Err(
                "administrator actions are not started while the app runs from a \
                        temporary or Downloads folder; install PC Tweaker Uninstaller and \
                        try again"
                    .to_string(),
            );
        }
        let pipe = ReportPipe::create()?;
        let parameters = std::iter::once(flag.to_string())
            .chain(values.iter().map(|v| encode_elevated_arg(v)))
            .chain(std::iter::once(encode_elevated_arg(&pipe.name)))
            .collect::<Vec<_>>()
            .join(" ");
        match shell_runas(&exe, &parameters)? {
            (0, pid) => Ok(pipe.read_from(pid)),
            // main.rs exits 2 before doing anything with arguments it refuses.
            (2, _) => Err("the elevated action refused its arguments".to_string()),
            // Any other exit may come after the action ran (a report that
            // could not be handed over, a crash): no report, and no claim
            // that nothing changed.
            (code, _) => {
                crate::applog::line(&format!("elevated action exited with code {code}"));
                Ok(Vec::new())
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (exe, flag, values);
        Err("elevation is not implemented on this platform yet".to_string())
    }
}

/// Folders where other software drops files next to whatever runs there
/// (browser downloads, installers' temporary copies). The elevated child is
/// this executable, so it is never started from one of them.
#[cfg(windows)]
fn in_shared_download_folder(exe: &std::path::Path) -> bool {
    let mut shared = vec![std::env::temp_dir()];
    if let Some(home) = std::env::var_os("USERPROFILE") {
        shared.push(std::path::Path::new(&home).join("Downloads"));
    }
    exe.parent().is_some_and(|dir| is_inside_any(dir, &shared))
}

/// Case-insensitive and through links where they resolve.
fn is_inside_any(dir: &std::path::Path, roots: &[std::path::PathBuf]) -> bool {
    let norm = |p: &std::path::Path| {
        let p = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        std::path::PathBuf::from(p.to_string_lossy().to_lowercase())
    };
    let dir = norm(dir);
    roots.iter().any(|root| dir.starts_with(norm(root)))
}

/// `ShellExecuteExW` with the `runas` verb, waiting for the helper's exit
/// code and process id. The working directory is System32, never the caller's.
#[cfg(windows)]
fn shell_runas(exe: &std::path::Path, parameters: &str) -> Result<(u32, u32), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, WAIT_OBJECT_0},
        System::{
            Com::{
                CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
            },
            Threading::{GetExitCodeProcess, GetProcessId, WaitForSingleObject, INFINITE},
        },
        UI::{
            Shell::{
                FOLDERID_System, ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS,
                SHELLEXECUTEINFOW,
            },
            WindowsAndMessaging::SW_NORMAL,
        },
    };
    let wide = |s: &std::ffi::OsStr| s.encode_wide().chain([0]).collect::<Vec<u16>>();
    let system32 = known_folder(&FOLDERID_System).unwrap_or_else(|| r"C:\Windows\System32".into());
    let verb = wide("runas".as_ref());
    let file = wide(exe.as_os_str());
    let parameters = wide(parameters.as_ref());
    let directory = wide(system32.as_ref());
    // SAFETY: the structure is zeroed, sized, and points at buffers that
    // outlive the call; the process handle is closed and a successful
    // CoInitializeEx is balanced on this same thread.
    unsafe {
        let com = CoInitializeEx(
            std::ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        );
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOASYNC | SEE_MASK_NOCLOSEPROCESS;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = parameters.as_ptr();
        info.lpDirectory = directory.as_ptr();
        info.nShow = SW_NORMAL;
        let launched = ShellExecuteExW(&mut info) != 0 && !info.hProcess.is_null();
        let error = std::io::Error::last_os_error();
        if com >= 0 {
            CoUninitialize();
        }
        if !launched {
            return Err(format!("elevation was cancelled or failed: {error}"));
        }
        let mut code = 0u32;
        let pid = GetProcessId(info.hProcess);
        let ok = WaitForSingleObject(info.hProcess, INFINITE) == WAIT_OBJECT_0
            && GetExitCodeProcess(info.hProcess, &mut code) != 0;
        CloseHandle(info.hProcess);
        if ok {
            Ok((code, pid))
        } else {
            Err("could not read the elevated action's result".to_string())
        }
    }
}

/// A folder as Windows records it, never from an environment variable: the
/// user's own variables (HKCU\Environment, which any program running as the
/// user can change) reach the elevated child too, so `%SystemRoot%` or
/// `%ProgramFiles%` there could point an administrator's action anywhere.
#[cfg(windows)]
pub fn known_folder(id: &windows_sys::core::GUID) -> Option<String> {
    use windows_sys::Win32::{System::Com::CoTaskMemFree, UI::Shell::SHGetKnownFolderPath};
    let mut raw: *mut u16 = std::ptr::null_mut();
    // SAFETY: a valid folder id and out pointer; the returned buffer is
    // NUL-terminated and is freed with CoTaskMemFree, as documented, also
    // when the call fails.
    unsafe {
        let hr = SHGetKnownFolderPath(id, 0, std::ptr::null_mut(), &mut raw);
        let path = (hr >= 0 && !raw.is_null()).then(|| {
            let len = (0..).take_while(|&i| *raw.add(i) != 0).count();
            String::from_utf16_lossy(std::slice::from_raw_parts(raw, len))
        });
        CoTaskMemFree(raw as *const _);
        path
    }
}

/// Every elevated report pipe is named this, then the parent's process id, a
/// counter and a time stamp in hex. The child accepts nothing else as its
/// report target.
const REPORT_PIPE_PREFIX: &str = r"\\.\pipe\pc-tweaker-uninstaller-report-";
/// Largest report the child sends. Well inside the pipe's buffer, so writing
/// it never waits for a reader: the parent reads only once the child exited.
pub const REPORT_LIMIT: usize = 32 * 1024;

/// The elevated child's only way back to this process: a one-instance pipe
/// made before the UAC prompt. The child writes its report into it and
/// nothing to disk. Its earlier report file sat in the user's app data
/// folder, where a program running as the user could swap the folder for a
/// link during a long uninstall and send the administrator's write anywhere,
/// and where a standard user who typed an administrator's password never saw
/// it, because the child then ran under that administrator's profile.
#[cfg(windows)]
struct ReportPipe {
    name: String,
    pipe: std::fs::File,
}

#[cfg(windows)]
impl ReportPipe {
    fn create() -> Result<Self, String> {
        use std::os::windows::{ffi::OsStrExt, io::FromRawHandle};
        use windows_sys::Win32::{
            Foundation::INVALID_HANDLE_VALUE,
            Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_INBOUND},
            System::Pipes::{
                CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE,
                PIPE_WAIT,
            },
        };
        static COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let count = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let name = format!(
            "{REPORT_PIPE_PREFIX}{:x}-{count:x}-{stamp:x}",
            std::process::id()
        );
        let wide: Vec<u16> = std::ffi::OsStr::new(&name)
            .encode_wide()
            .chain([0])
            .collect();
        // SAFETY: a NUL-terminated name and the default security descriptor
        // (creator, administrators and SYSTEM may write). One instance only,
        // and the first: nobody else can serve this name.
        let handle = unsafe {
            CreateNamedPipeW(
                wide.as_ptr(),
                PIPE_ACCESS_INBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                0,
                2 * REPORT_LIMIT as u32,
                0,
                std::ptr::null(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(format!(
                "the elevated action's report channel could not be opened: {}",
                std::io::Error::last_os_error()
            ));
        }
        // SAFETY: a valid handle that nothing else owns.
        let pipe = unsafe { std::fs::File::from_raw_handle(handle) };
        Ok(Self { name, pipe })
    }

    /// What process `pid` wrote before it exited. Nothing when no one
    /// connected or another process did: a program running as the user can
    /// open this pipe too, and its words are not the child's report.
    fn read_from(&self, pid: u32) -> Vec<u8> {
        use std::io::Read;
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::Pipes::GetNamedPipeClientProcessId;
        let mut client = 0u32;
        // SAFETY: the pipe handle this struct owns and a valid out pointer.
        let known = unsafe { GetNamedPipeClientProcessId(self.pipe.as_raw_handle(), &mut client) };
        let mut bytes = Vec::new();
        if known != 0 && client == pid {
            // The child has exited, so its end is closed: this ends at the
            // last byte it wrote instead of waiting.
            let _ = (&self.pipe)
                .take(REPORT_LIMIT as u64)
                .read_to_end(&mut bytes);
        }
        bytes
    }
}

/// The elevated child's half: hands its report to the parent's pipe. The
/// name arrived on the command line, which whoever started this process
/// chose, so it must be exactly the shape the parent creates: anything else
/// (a file path, a `..`) is refused rather than written to as administrator.
/// The pipe is opened for identification only, so its creator cannot act
/// with this process's rights.
pub fn send_report_bytes(pipe: &str, bytes: &[u8]) -> Result<(), String> {
    if !is_report_pipe_name(pipe) {
        return Err("the report channel name is not one this app creates".to_string());
    }
    #[cfg(windows)]
    {
        use std::io::Write;
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::SECURITY_IDENTIFICATION;
        std::fs::OpenOptions::new()
            .write(true)
            .security_qos_flags(SECURITY_IDENTIFICATION)
            .open(pipe)
            .and_then(|mut channel| channel.write_all(bytes))
            .map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = bytes;
        Err("elevation is not implemented on this platform yet".to_string())
    }
}

fn is_report_pipe_name(name: &str) -> bool {
    name.strip_prefix(REPORT_PIPE_PREFIX).is_some_and(|rest| {
        !rest.is_empty()
            && rest.len() <= 48
            && rest.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
    })
}

/// Marks an argument that crossed the UAC relaunch encoded.
const ENCODED_ARG_PREFIX: &str = "b64:";

/// URL-safe base64 is only letters, digits, `-` and `_`: a registry key name
/// with spaces or quotes crosses the relaunch with no quoting to get wrong.
pub fn encode_elevated_arg(value: &str) -> String {
    use base64::Engine as _;
    format!(
        "{ENCODED_ARG_PREFIX}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(value)
    )
}

/// The child's side of `encode_elevated_arg`. A malformed payload is refused
/// rather than guessed at. An unprefixed argument is taken as it is: an app
/// still running from before an update relaunches the new binary that way.
pub fn decode_elevated_arg(arg: &str) -> Result<String, String> {
    use base64::Engine as _;
    let Some(encoded) = arg.strip_prefix(ENCODED_ARG_PREFIX) else {
        return Ok(arg.to_owned());
    };
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or_else(|| "the elevated action received a malformed argument".to_string())
}

/// The elevated child is this same executable, installed per user, so the
/// folder it starts from is writable without administrator rights. Any DLL
/// it loads by name afterwards (WMI, COM, a system library) comes from
/// System32 only, never from a file next to the exe or in the working
/// directory. Static imports are covered at link time (build.rs).
pub fn restrict_dll_search_to_system32() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::LibraryLoader::{
            SetDefaultDllDirectories, SetDllDirectoryW, LOAD_LIBRARY_SEARCH_SYSTEM32,
        };
        // SAFETY: a flag value, and an empty NUL-terminated string.
        unsafe {
            SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32);
            SetDllDirectoryW([0u16].as_ptr());
        }
    }
}

/// A folder of the app's own data (the registry backups) must be a real
/// folder: a junction or symbolic link in its place would send the write to
/// another location, so it is refused. A missing folder is fine: it is
/// created as a plain directory.
pub fn ensure_plain_app_data_dir(dir: &std::path::Path) -> Result<(), String> {
    match std::fs::symlink_metadata(dir) {
        Ok(metadata) if is_link(&metadata) => Err(format!(
            "{} is a link to another location; the elevated action was refused",
            dir.display()
        )),
        Ok(metadata) if !metadata.is_dir() => Err(format!(
            "{} is not a folder; the elevated action was refused",
            dir.display()
        )),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("could not inspect {}: {}", dir.display(), e)),
    }
}

fn is_link(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_REPARSE_POINT covers junctions as well as symbolic links.
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    false
}

#[cfg(test)]
mod app_data_dir_tests {
    use super::ensure_plain_app_data_dir;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pctu-elevation-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_missing_or_plain_folder_is_accepted() {
        let dir = temp("plain");
        assert!(ensure_plain_app_data_dir(&dir).is_ok());
        std::fs::create_dir_all(&dir).unwrap();
        assert!(ensure_plain_app_data_dir(&dir).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_file_in_place_of_the_folder_is_refused() {
        let dir = temp("file");
        std::fs::write(&dir, b"x").unwrap();
        assert!(ensure_plain_app_data_dir(&dir).is_err());
        std::fs::remove_file(&dir).ok();
    }
}

#[cfg(test)]
mod relaunch_tests {
    use super::*;

    #[test]
    fn values_cross_the_relaunch_unchanged() {
        for value in ["machine64", "{1A2B-3C4D}", r#"Some "App" 2.0 \x"#, "ÿ ü"] {
            let encoded = encode_elevated_arg(value);
            assert!(encoded[4..]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
            assert_eq!(decode_elevated_arg(&encoded).unwrap(), value);
        }
        assert_eq!(decode_elevated_arg("machine32").unwrap(), "machine32");
        assert!(decode_elevated_arg("b64:not base64!").is_err());
    }

    #[test]
    fn only_the_apps_own_pipe_names_are_report_targets() {
        assert!(is_report_pipe_name(
            r"\\.\pipe\pc-tweaker-uninstaller-report-1a2b-18f3c2d4e5f60718"
        ));
        for name in [
            r"\\.\pipe\pc-tweaker-uninstaller-report-",
            r"\\.\pipe\pc-tweaker-uninstaller-report-..\..\C:\Windows\win.ini",
            r"\\.\pipe\pc-tweaker-uninstaller-report-1a2b\x",
            r"\\.\pipe\other-report-1a2b",
            r"C:\Users\Public\report.json",
            r"\\?\pipe\pc-tweaker-uninstaller-report-1a2b",
        ] {
            assert!(!is_report_pipe_name(name), "{name}");
            assert!(send_report_bytes(name, b"{}").is_err(), "{name}");
        }
    }

    /// The parent's side and the child's side meet as they do across the UAC
    /// relaunch: the report arrives once the writer has closed its end, only
    /// for the writer's process id, and an empty pipe answers at once.
    #[cfg(windows)]
    #[test]
    fn the_report_pipe_carries_the_childs_bytes_only() {
        let quiet = ReportPipe::create().unwrap();
        assert!(quiet.read_from(std::process::id()).is_empty());

        let pipe = ReportPipe::create().unwrap();
        assert!(ReportPipe::create().is_ok(), "names must not collide");
        send_report_bytes(&pipe.name, br#"{"success":true}"#).unwrap();
        assert!(pipe.read_from(std::process::id() + 1).is_empty());

        let pipe = ReportPipe::create().unwrap();
        send_report_bytes(&pipe.name, br#"{"success":true}"#).unwrap();
        assert_eq!(pipe.read_from(std::process::id()), br#"{"success":true}"#);
    }

    /// Run in a child process with planted variables, so the other tests,
    /// which read the real ones, never see them.
    #[cfg(windows)]
    #[test]
    fn folders_come_from_windows_not_the_environment() {
        use windows_sys::Win32::UI::Shell::{FOLDERID_ProgramFiles, FOLDERID_System};
        const PLANTED: &str = r"C:\Users\Public\planted";
        if std::env::var_os("PCU_PLANTED_ENV").is_some() {
            for id in [FOLDERID_ProgramFiles, FOLDERID_System] {
                let folder = known_folder(&id).unwrap();
                assert!(!folder.contains("planted"), "{folder}");
            }
            return;
        }
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "elevation::relaunch_tests::folders_come_from_windows_not_the_environment",
            ])
            .env("PCU_PLANTED_ENV", "1")
            .env("SystemRoot", PLANTED)
            .env("windir", PLANTED)
            .env("ProgramFiles", PLANTED)
            .env("ProgramFiles(x86)", PLANTED)
            .env("ProgramW6432", PLANTED)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn download_folders_match_case_insensitively() {
        let roots = [std::path::PathBuf::from(r"C:\Users\Ann\Downloads")];
        assert!(is_inside_any(
            std::path::Path::new(r"c:\users\ann\downloads\tools"),
            &roots
        ));
        assert!(!is_inside_any(
            std::path::Path::new(r"C:\Users\Ann\AppData\Local\PC Tweaker Uninstaller"),
            &roots
        ));
    }
}
