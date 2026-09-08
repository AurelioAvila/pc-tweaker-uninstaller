//! Read icons from registered local applications; never execute their code.
use base64::{engine::general_purpose::STANDARD, Engine};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn local_path(raw: &str) -> Option<std::path::PathBuf> {
    let p = std::path::PathBuf::from(raw);
    let b = raw.as_bytes();
    // No UNC/network shares, device paths, relative names or alternate streams.
    if b.len() < 4
        || !b[0].is_ascii_alphabetic()
        || b[1] != b':'
        || !matches!(b[2], b'\\' | b'/')
        || raw[2..].contains(':')
        || raw.contains('\0')
    {
        return None;
    }
    p.is_file().then_some(p)
}

#[cfg(windows)]
fn extract(raw: &str) -> Option<String> {
    use windows_sys::Win32::{
        Graphics::Gdi::*,
        UI::{Shell::ExtractIconExW, WindowsAndMessaging::*},
    };
    let (path, index) = match raw.rsplit_once(',') {
        Some((p, i)) if i.trim().parse::<i32>().is_ok() => (p, i.trim().parse().ok()?),
        _ => (raw, 0),
    };
    let path = local_path(path.trim().trim_matches('"'))?;
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    if !matches!(extension.as_str(), "exe" | "dll" | "ico") {
        return None;
    }
    let wide: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect();
    unsafe {
        let mut icon = std::ptr::null_mut();
        if ExtractIconExW(wide.as_ptr(), index, &mut icon, std::ptr::null_mut(), 1) == 0
            || icon.is_null()
        {
            return None;
        }
        let mut info: ICONINFO = std::mem::zeroed();
        if GetIconInfo(icon, &mut info) == 0 {
            DestroyIcon(icon);
            return None;
        }
        let mut bitmap: BITMAP = std::mem::zeroed();
        let valid = !info.hbmColor.is_null()
            && GetObjectW(
                info.hbmColor,
                std::mem::size_of::<BITMAP>() as i32,
                &mut bitmap as *mut _ as _,
            ) != 0;
        let result = (|| {
            if !valid
                || bitmap.bmWidth <= 0
                || bitmap.bmHeight <= 0
                || bitmap.bmWidth > 256
                || bitmap.bmHeight > 256
            {
                return None;
            }
            let (w, h) = (bitmap.bmWidth as u32, bitmap.bmHeight as u32);
            let mut bi: BITMAPINFO = std::mem::zeroed();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = w as i32;
            bi.bmiHeader.biHeight = -(h as i32);
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB;
            let dc = CreateCompatibleDC(std::ptr::null_mut());
            if dc.is_null() {
                return None;
            }
            let mut pixels = vec![0u8; (w * h * 4) as usize];
            let rows = GetDIBits(
                dc,
                info.hbmColor,
                0,
                h,
                pixels.as_mut_ptr() as _,
                &mut bi,
                DIB_RGB_COLORS,
            );
            DeleteDC(dc);
            if rows != h as i32 {
                return None;
            }
            let alpha = pixels.chunks_exact(4).any(|p| p[3] != 0);
            for p in pixels.chunks_exact_mut(4) {
                p.swap(0, 2);
                if !alpha {
                    p[3] = 255;
                }
            }
            let mut bytes = Vec::new();
            {
                let mut encoder = png::Encoder::new(&mut bytes, w, h);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .ok()?
                    .write_image_data(&pixels)
                    .ok()?;
            }
            Some(format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
        })();
        if !info.hbmColor.is_null() {
            DeleteObject(info.hbmColor);
        }
        if !info.hbmMask.is_null() {
            DeleteObject(info.hbmMask);
        }
        DestroyIcon(icon);
        result
    }
}

#[cfg(windows)]
fn load(source: &str, id: &str) -> Option<String> {
    if source == "store" {
        use windows::{core::HSTRING, Management::Deployment::PackageManager};
        let manager = PackageManager::new().ok()?;
        let packages = manager.FindPackagesByUserSecurityId(&HSTRING::new()).ok()?;
        for p in packages {
            if p.Id().ok()?.FullName().ok()? != id {
                continue;
            }
            let uri = p.Logo().ok()?.AbsoluteUri().ok()?.to_string();
            let path = url::Url::parse(&uri).ok()?.to_file_path().ok()?;
            let path = local_path(path.to_str()?)?;
            if path.metadata().ok()?.len() > 1_048_576 {
                return None;
            }
            let bytes = std::fs::read(path).ok()?;
            if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                return None;
            }
            return Some(format!("data:image/png;base64,{}", STANDARD.encode(bytes)));
        }
        return None;
    }
    let entry = crate::programs::read_raw_entry(source, id).ok()?;
    extract(entry.display_icon.as_deref()?)
}
#[cfg(not(windows))]
fn load(_: &str, _: &str) -> Option<String> {
    None
}

#[tauri::command]
pub async fn program_icon(source: String, id: String) -> Option<String> {
    if !matches!(
        source.as_str(),
        "machine64" | "machine32" | "user" | "store"
    ) || id.len() > 512
        || id.contains(['\\', '/', '\0'])
    {
        return None;
    }
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    let key = format!("{source}:{id}");
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(value) = cache.lock().ok()?.get(&key).cloned() {
        return value;
    }
    let result = tauri::async_runtime::spawn_blocking(move || load(&source, &id))
        .await
        .ok()
        .flatten();
    if let Ok(mut entries) = cache.lock() {
        if entries.len() < 2000 {
            entries.insert(key, result.clone());
        }
    }
    result
}
