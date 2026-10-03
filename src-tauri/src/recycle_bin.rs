//! Makes sure an item will really land in the Recycle Bin before cleanup
//! hands it to the shell.
//!
//! The shell recycles with confirmations turned off, so when an item cannot
//! be recycled (the bin is turned off for that drive, the item is larger
//! than the bin, or the drive has no bin at all, like network and removable
//! drives) Windows deletes it permanently without asking. Cleanup promises
//! that nothing is deleted permanently, so those items are refused up front
//! and left in place for the user to handle.

use std::path::Path;

/// What the Recycle Bin of one volume accepts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BinLimits {
    pub enabled: bool,
    pub max_kb: u64,
}

/// Why an item of `size_kb` cannot safely be recycled on a volume with
/// `limits`, or `None` when it can. `limits` is `None` when the item is not
/// on a local fixed drive or the bin could not be inspected.
pub fn refusal(limits: Option<BinLimits>, size_kb: Option<u64>) -> Option<&'static str> {
    let Some(limits) = limits else {
        return Some("not on a local drive with a Recycle Bin");
    };
    if !limits.enabled {
        return Some("the Recycle Bin is turned off for this drive");
    }
    let Some(size_kb) = size_kb else {
        return Some("too large to measure, so it may not fit in the Recycle Bin");
    };
    if size_kb >= limits.max_kb {
        return Some("larger than the Recycle Bin of this drive");
    }
    None
}

/// Bin size assumed when the user never set one: 5% of the volume, which is
/// at or below what Windows reserves by default on any volume size.
pub fn default_max_kb(volume_total_kb: u64) -> u64 {
    volume_total_kb / 20
}

/// Size of a file or folder in KB. `None` when the folder is too large to
/// measure quickly or cannot be read.
pub fn item_size_kb(path: &Path) -> Option<u64> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if meta.is_dir() {
        let mut cap = 20_000u32;
        crate::uninstall_exec::dir_size_capped(path, &mut cap).map(|b| b / 1024)
    } else {
        Some(meta.len().div_ceil(1024))
    }
}

/// Limits of the Recycle Bin that would receive `path`.
#[cfg(windows)]
pub fn limits_for(path: &Path) -> Option<BinLimits> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetVolumeNameForVolumeMountPointW, GetVolumePathNameW,
    };
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    const DRIVE_FIXED: u32 = 3;

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut root = [0u16; 512];
    // SAFETY: both buffers are valid, `wide` is NUL-terminated and the length
    // passed matches `root`.
    if unsafe { GetVolumePathNameW(wide.as_ptr(), root.as_mut_ptr(), root.len() as u32) } == 0 {
        return None;
    }
    // SAFETY: `root` was NUL-terminated by the call above.
    if unsafe { GetDriveTypeW(root.as_ptr()) } != DRIVE_FIXED {
        return None;
    }

    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let disabled = RegKey::predef(hive)
            .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Policies\Explorer")
            .and_then(|k| k.get_value::<u32, _>("NoRecycleFiles"))
            .is_ok_and(|v| v != 0);
        if disabled {
            return Some(BinLimits {
                enabled: false,
                max_kb: 0,
            });
        }
    }

    let mut volume = [0u16; 64];
    // SAFETY: `root` is a NUL-terminated mount point and the length passed
    // matches `volume`.
    let named = unsafe {
        GetVolumeNameForVolumeMountPointW(root.as_ptr(), volume.as_mut_ptr(), volume.len() as u32)
    } != 0;
    let settings = named
        .then(|| {
            let end = volume.iter().position(|&c| c == 0).unwrap_or(volume.len());
            let name = String::from_utf16_lossy(&volume[..end]);
            let guid = name
                .find('{')
                .map(|i| name[i..].trim_end_matches('\\').to_string())?;
            RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey(format!(
                    r"Software\Microsoft\Windows\CurrentVersion\Explorer\BitBucket\Volume\{guid}"
                ))
                .ok()
        })
        .flatten();

    if let Some(key) = &settings {
        if key
            .get_value::<u32, _>("NukeOnDelete")
            .is_ok_and(|v| v != 0)
        {
            return Some(BinLimits {
                enabled: false,
                max_kb: 0,
            });
        }
        if let Ok(max_mb) = key.get_value::<u32, _>("MaxCapacity") {
            return Some(BinLimits {
                enabled: true,
                max_kb: u64::from(max_mb) * 1024,
            });
        }
    }

    let mut total: u64 = 0;
    // SAFETY: `root` is NUL-terminated; null out-pointers are allowed.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            root.as_ptr(),
            std::ptr::null_mut(),
            &mut total,
            std::ptr::null_mut(),
        )
    } != 0;
    ok.then(|| BinLimits {
        enabled: true,
        max_kb: default_max_kb(total / 1024),
    })
}

#[cfg(not(windows))]
pub fn limits_for(_path: &Path) -> Option<BinLimits> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIN: BinLimits = BinLimits {
        enabled: true,
        max_kb: 1_000,
    };

    #[test]
    fn an_item_that_fits_an_enabled_bin_is_accepted() {
        assert_eq!(refusal(Some(BIN), Some(999)), None);
    }

    #[test]
    fn anything_the_bin_cannot_take_is_refused() {
        assert!(refusal(None, Some(1)).is_some());
        let off = BinLimits {
            enabled: false,
            ..BIN
        };
        assert!(refusal(Some(off), Some(1)).is_some());
        assert!(refusal(Some(BIN), Some(1_000)).is_some());
        assert!(refusal(Some(BIN), None).is_some());
    }

    #[test]
    fn the_assumed_bin_size_is_a_twentieth_of_the_volume() {
        const GB: u64 = 1024 * 1024;
        assert_eq!(default_max_kb(200 * GB), 10 * GB);
    }
}
