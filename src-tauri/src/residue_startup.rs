//! Startup entries whose program is provably gone.
//!
//! Ported from PC Tweaker (`startup.rs`: `extract_exe_path`,
//! `expand_env_vars`; `regclean.rs`: `local_path`, `provably_missing`, the
//! `Disk` trait) and trimmed to what the first-launch sweep needs.
//!
//! The one rule that matters: a command is judged only when the answer is
//! certain. A shape this module cannot read, a relative or network path, an
//! 8.3 short name, an unexpandable variable, a removable or unmounted drive
//! or anything under the Windows folder is "cannot judge", which is never
//! reported. A dead row left on screen costs nothing; a live startup entry
//! reported as dead would.

use std::path::{Path, PathBuf};

/// File system facts, behind a trait so tests can use a fake.
pub trait Disk {
    fn exists(&self, path: &Path) -> bool;
    /// A fixed (non-removable, non-network) drive is mounted at `letter:\`.
    fn fixed_drive(&self, letter: char) -> bool;
}

pub struct WinDisk;

impl Disk for WinDisk {
    fn exists(&self, path: &Path) -> bool {
        // A path Windows cannot even query (access denied) counts as present:
        // only a clean "not found" is proof of absence.
        match std::fs::symlink_metadata(path) {
            Ok(_) => true,
            Err(e) => {
                !matches!(e.kind(), std::io::ErrorKind::NotFound) && e.raw_os_error() != Some(3)
            }
        }
    }

    #[cfg(windows)]
    fn fixed_drive(&self, letter: char) -> bool {
        use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;
        const DRIVE_FIXED: u32 = 3;
        if !letter.is_ascii_alphabetic() {
            return false;
        }
        let root: Vec<u16> = format!("{letter}:\\")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        // SAFETY: `root` is NUL-terminated.
        unsafe { GetDriveTypeW(root.as_ptr()) == DRIVE_FIXED }
    }

    #[cfg(not(windows))]
    fn fixed_drive(&self, _letter: char) -> bool {
        false
    }
}

/// Pulls the executable out of a Run command line, or `None` when the shape
/// is not one this can read confidently. Quoted paths, unquoted paths through
/// the first `.exe`, and environment variables are handled; a bare name such
/// as `rundll32.exe` (resolved through PATH at boot) is never a path here.
pub fn extract_exe_path(command: &str) -> Option<PathBuf> {
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    let candidate = if let Some(rest) = command.strip_prefix('"') {
        rest.split('"').next()?.to_string()
    } else {
        let lower = command.to_ascii_lowercase();
        let idx = lower.find(".exe")?;
        // ASCII lowercasing keeps every byte offset.
        command[..idx + 4].to_string()
    };
    let expanded = expand_env_vars(&candidate);
    if expanded.trim().is_empty() {
        return None;
    }
    let path = PathBuf::from(expanded.trim());
    if path.parent().is_none_or(|p| p.as_os_str().is_empty()) {
        return None;
    }
    Some(path)
}

/// Expands `%VAR%` references. An unset variable makes the whole expansion
/// unusable (an empty string), never a half-substituted path.
pub fn expand_env_vars(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            out.push_str(rest);
            return out;
        };
        let Ok(value) = std::env::var(&after[..end]) else {
            return String::new();
        };
        out.push_str(&rest[..start]);
        out.push_str(&value);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// `text` as an absolute local path, or `None` when it is not one this module
/// can reason about (relative, UNC, unexpandable variable, 8.3 short name).
pub fn local_path(text: &str) -> Option<PathBuf> {
    let text = text.trim().trim_matches('"').trim();
    let expanded = if text.contains('%') {
        expand_env_vars(text)
    } else {
        text.to_string()
    };
    let bytes = expanded.as_bytes();
    if bytes.len() < 4
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || bytes[2] != b'\\'
        || expanded.contains('~')
        || expanded.contains("\\..")
        || expanded.contains(['"', '*', '?', '<', '>', '|'])
    {
        return None;
    }
    Some(PathBuf::from(expanded))
}

fn in_windows_folder(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    let windir = std::env::var("SystemRoot")
        .unwrap_or_else(|_| r"C:\Windows".into())
        .to_ascii_lowercase();
    let windir = windir.trim_end_matches('\\');
    path == windir || path.starts_with(&format!("{windir}\\"))
}

/// Proven absent: on a fixed drive that is mounted, outside the Windows
/// folder (a 32-bit installer writes `System32` for what is really
/// `SysWOW64`), and not there.
pub fn provably_missing(path: &Path, disk: &dyn Disk) -> bool {
    let text = path.to_string_lossy();
    let Some(letter) = text.chars().next() else {
        return false;
    };
    if in_windows_folder(&text) {
        return false;
    }
    disk.fixed_drive(letter.to_ascii_uppercase()) && !disk.exists(path)
}

/// The executable a startup command runs, when it is provably not there.
///
/// This process is 64-bit, so `%ProgramFiles%` is the 64-bit folder while a
/// 32-bit installer that wrote the same text meant the (x86) one: such a
/// command is missing only if it is missing in both.
pub fn missing_target(command: &str, disk: &dyn Disk) -> Option<PathBuf> {
    let lower = command.to_ascii_lowercase();
    let mut variants = vec![command.to_string()];
    for (needle, alt) in [
        ("%programfiles%", "%ProgramFiles(x86)%"),
        ("%commonprogramfiles%", "%CommonProgramFiles(x86)%"),
    ] {
        if let Some(at) = lower.find(needle) {
            variants.push(format!(
                "{}{alt}{}",
                &command[..at],
                &command[at + needle.len()..]
            ));
        }
    }
    let mut first = None;
    for variant in variants {
        let exe = extract_exe_path(&variant)?;
        let exe = local_path(&exe.to_string_lossy())?;
        if !provably_missing(&exe, disk) {
            return None;
        }
        first.get_or_insert(exe);
    }
    first
}

/// The raw target of a `.lnk` shortcut (`SLGP_RAWPATH`: variables unexpanded,
/// nothing resolved or searched for, so reading it can never launch or fix
/// anything). `None` for an unreadable shortcut or one with no file target.
#[cfg(windows)]
pub fn shortcut_target(path: &Path) -> Option<String> {
    use windows::core::{Interface, HSTRING};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH};

    let path = path.to_path_buf();
    // COM state belongs to the thread, so a thread of its own keeps the
    // caller's apartment untouched.
    std::thread::spawn(move || {
        // SAFETY: plain COM calls on this thread's own apartment, balanced by
        // CoUninitialize; the buffer is as long as the length passed.
        unsafe {
            let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
            let target = (|| {
                let link: IShellLinkW =
                    CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
                let file: IPersistFile = link.cast().ok()?;
                file.Load(&HSTRING::from(path.as_os_str()), STGM_READ)
                    .ok()?;
                let mut buf = [0u16; 1024];
                link.GetPath(&mut buf, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)
                    .ok()?;
                let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                let text = String::from_utf16_lossy(&buf[..len]);
                (!text.trim().is_empty()).then_some(text)
            })();
            if initialized {
                CoUninitialize();
            }
            target
        }
    })
    .join()
    .ok()
    .flatten()
}

#[cfg(not(windows))]
pub fn shortcut_target(_path: &Path) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeDisk {
        present: Vec<&'static str>,
        fixed: Vec<char>,
    }
    impl Disk for FakeDisk {
        fn exists(&self, path: &Path) -> bool {
            self.present
                .iter()
                .any(|p| p.eq_ignore_ascii_case(&path.to_string_lossy()))
        }
        fn fixed_drive(&self, letter: char) -> bool {
            self.fixed.contains(&letter)
        }
    }
    fn disk() -> FakeDisk {
        FakeDisk {
            present: vec![r"C:\Program Files\Live\live.exe"],
            fixed: vec!['C'],
        }
    }

    #[test]
    fn the_executable_is_extracted_from_every_command_shape() {
        assert_eq!(
            extract_exe_path(r#""C:\Program Files\App\app.exe" --silent"#),
            Some(PathBuf::from(r"C:\Program Files\App\app.exe"))
        );
        assert_eq!(
            extract_exe_path(r"C:\Tools\thing.exe /background"),
            Some(PathBuf::from(r"C:\Tools\thing.exe"))
        );
        assert_eq!(
            extract_exe_path(r"C:\Program Files\My App\run.exe -q"),
            Some(PathBuf::from(r"C:\Program Files\My App\run.exe"))
        );
        for unreadable in [
            "rundll32.exe shell32.dll,Control_RunDLL",
            "",
            "   ",
            "no executable at all",
            r"%NONEXISTENT_VAR_XYZ%\app.exe",
        ] {
            assert_eq!(extract_exe_path(unreadable), None, "{unreadable}");
        }
    }

    #[test]
    fn only_a_clean_not_found_on_a_fixed_drive_is_proof() {
        let d = disk();
        let gone = r#""C:\Program Files\Gone\gone.exe" /boot"#;
        assert_eq!(
            missing_target(gone, &d),
            Some(PathBuf::from(r"C:\Program Files\Gone\gone.exe"))
        );
        // Present, removable drive, 8.3 name, relative, UNC, Windows folder,
        // unknown variable, and an exe looked up through PATH: cannot judge.
        for command in [
            r"C:\Program Files\Live\live.exe",
            r"E:\Portable\tool.exe",
            r"C:\PROGRA~1\Gone\gone.exe",
            r"Gone\gone.exe",
            r"\\server\share\gone.exe",
            r"C:\Windows\System32\gone.exe",
            r"%NOT_SET_ANYWHERE%\gone.exe",
            "cmd.exe /c start",
        ] {
            assert_eq!(missing_target(command, &d), None, "{command}");
        }
    }

    /// The Run value of a 32-bit program says `%ProgramFiles%`, which this
    /// 64-bit process expands to the other folder: it must exist in neither.
    #[test]
    fn program_files_is_checked_in_both_views() {
        let pf = std::env::var("ProgramFiles").unwrap_or_default();
        let x86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();
        if pf.is_empty() || x86.is_empty() {
            return;
        }
        let in_x86: &'static str = Box::leak(format!(r"{x86}\Old32\old.exe").into_boxed_str());
        let d = FakeDisk {
            present: vec![in_x86],
            fixed: vec!['C'],
        };
        assert_eq!(missing_target(r"%ProgramFiles%\Old32\old.exe", &d), None);
        assert!(missing_target(r"%ProgramFiles%\Nowhere\x.exe", &d).is_some());
    }

    #[cfg(windows)]
    #[test]
    fn a_shortcut_target_is_read_without_resolving_it() {
        use windows::core::{Interface, HSTRING};
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED,
        };
        use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
        let file = std::env::temp_dir().join(format!("pcu-lnk-{}.lnk", std::process::id()));
        let file2 = file.clone();
        let made = std::thread::spawn(move || unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let ok = (|| {
                let link: IShellLinkW =
                    CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
                link.SetPath(&HSTRING::from(r"C:\Definitely Gone\gone.exe"))
                    .ok()?;
                let persist: IPersistFile = link.cast().ok()?;
                persist.Save(&HSTRING::from(file2.as_os_str()), true).ok()
            })()
            .is_some();
            CoUninitialize();
            ok
        })
        .join()
        .unwrap();
        assert!(made, "could not create a test shortcut");
        assert_eq!(
            shortcut_target(&file).as_deref(),
            Some(r"C:\Definitely Gone\gone.exe")
        );
        std::fs::remove_file(&file).ok();
        assert_eq!(shortcut_target(&file), None);
    }
}
