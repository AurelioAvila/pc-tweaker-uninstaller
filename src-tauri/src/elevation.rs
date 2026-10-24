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
/// process re-validates and re-derives everything from them.
pub fn run_elevated_args(flag: &str, values: &[&str]) -> Result<(), String> {
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
        let parameters = std::iter::once(flag.to_string())
            .chain(values.iter().map(|v| encode_elevated_arg(v)))
            .collect::<Vec<_>>()
            .join(" ");
        match shell_runas(&exe, &parameters)? {
            0 => Ok(()),
            code => Err(format!("the elevated action exited with code {code}")),
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
/// code. The working directory is System32, never the caller's.
#[cfg(windows)]
fn shell_runas(exe: &std::path::Path, parameters: &str) -> Result<u32, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, WAIT_OBJECT_0},
        System::{
            Com::{
                CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
            },
            Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE},
        },
        UI::{
            Shell::{
                ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
            },
            WindowsAndMessaging::SW_NORMAL,
        },
    };
    let wide = |s: &std::ffi::OsStr| s.encode_wide().chain([0]).collect::<Vec<u16>>();
    let system32 = std::path::Path::new(
        &std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into()),
    )
    .join("System32");
    let verb = wide("runas".as_ref());
    let file = wide(exe.as_os_str());
    let parameters = wide(parameters.as_ref());
    let directory = wide(system32.as_os_str());
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
        let ok = WaitForSingleObject(info.hProcess, INFINITE) == WAIT_OBJECT_0
            && GetExitCodeProcess(info.hProcess, &mut code) != 0;
        CloseHandle(info.hProcess);
        if ok {
            Ok(code)
        } else {
            Err("could not read the elevated action's result".to_string())
        }
    }
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

/// The elevated child writes its report into the user's app data folder,
/// which any process running as that user can change. A junction or symbolic
/// link in place of that folder would send the administrator's write to
/// another location, so the child refuses to run through one. A missing
/// folder is fine: it is created as a plain directory.
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
