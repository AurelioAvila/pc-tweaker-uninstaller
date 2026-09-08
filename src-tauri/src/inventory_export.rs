use tauri::Manager;
fn cell(value: &str) -> String {
    let escaped = value.replace('"', "\"\"");
    let prefix = if value
        .trim_start()
        .starts_with(['=', '+', '-', '@', '\t', '\r', '\n'])
    {
        "'"
    } else {
        ""
    };
    format!("\"{prefix}{escaped}\"")
}
#[tauri::command]
pub async fn export_inventory(app: tauri::AppHandle, keys: Vec<String>) -> Result<String, String> {
    if keys.len() > 5000 {
        return Err("Too many inventory entries.".into());
    }
    let directory = app.path().download_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let keys: std::collections::HashSet<_> = keys.into_iter().collect();
        let mut csv = String::from(
            "\u{feff}Name,Publisher,Version,Reported size KB,Recorded date,Source\r\n",
        );
        for p in crate::programs::list_programs()? {
            if !keys.contains(&format!("{}:{}", p.source, p.id)) {
                continue;
            }
            let fields = [
                p.name,
                p.publisher.unwrap_or_default(),
                p.version.unwrap_or_default(),
                p.estimated_size_kb
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
                p.install_date.unwrap_or_default(),
                p.source.into(),
            ];
            csv.push_str(&fields.iter().map(|x| cell(x)).collect::<Vec<_>>().join(","));
            csv.push_str("\r\n");
        }
        for p in crate::store_apps::list_store_apps()? {
            if !keys.contains(&format!("store:{}", p.id)) {
                continue;
            }
            let fields = [
                p.name,
                p.publisher.unwrap_or_default(),
                p.version,
                String::new(),
                p.install_date.unwrap_or_default(),
                "store".into(),
            ];
            csv.push_str(&fields.iter().map(|x| cell(x)).collect::<Vec<_>>().join(","));
            csv.push_str("\r\n");
        }
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();
        let file = directory.join(format!("PC-Tweaker-Inventory-{ts}.csv"));
        use std::io::Write;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&file)
            .map_err(|e| e.to_string())?;
        output
            .write_all(csv.as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(file.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spreadsheet_formulas_are_not_executable() {
        assert_eq!(cell("=CMD()"), "\"'=CMD()\"");
        assert_eq!(cell("a,\"b\""), "\"a,\"\"b\"\"\"");
    }
}
