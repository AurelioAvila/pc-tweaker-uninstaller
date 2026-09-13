//! A local diagnostic log, because until now there was nothing to ask for.
//!
//! The Rust core did not contain one `println!`, and the binary is built for
//! the GUI subsystem unconditionally (`main.rs` explains why: the elevated
//! headless child is this same executable and a console subsystem would flash
//! a black window on every elevated uninstall). So when a user wrote "it did
//! not work", there was no artefact to ask them to attach — not a timestamp,
//! not an exit code, not even a panic message, because a panic in a
//! GUI-subsystem process writes to a stderr nobody is reading.
//!
//! What this is not: telemetry. Nothing is uploaded, nothing is scheduled,
//! there is no reporting service. The file sits next to the removal ledger in
//! the app's own data directory, the user opens the folder from the
//! application, and it goes anywhere only if they attach it to an email
//! themselves. PRIVACY.md says exactly that.
//!
//! No logging crate for this. `log` plus a backend is three dependencies and
//! an init dance to end up with what forty lines of std already do, and the
//! things worth recording here are a handful of decision points, not a
//! firehose with levels and filters.

use std::io::Write;
use std::path::PathBuf;

const LOG_FILE: &str = "uninstaller.log";

/// Past this, the file is rotated once. One generation is the right amount:
/// enough that a session's history survives the rotation that a long batch
/// run triggers, not so much that a support attachment is unopenable.
const MAX_BYTES: u64 = 1_048_576;

fn log_path() -> Result<PathBuf, String> {
    Ok(crate::uninstall_exec::fixed_data_dir()?.join(LOG_FILE))
}

/// Seconds since the Unix epoch, the same stamp the removal ledger records,
/// so a log line and a receipt can be lined up against each other.
fn stamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Appends one line. Every failure is swallowed: logging must never be the
/// reason an uninstall fails, and a user whose APPDATA is unwritable has a
/// bigger problem than a missing log line.
pub fn line(message: &str) {
    let Ok(path) = log_path() else { return };
    if let Some(dir) = path.parent() {
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
    }
    if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) >= MAX_BYTES {
        // Replaces the previous generation. Ignored on failure: a rotation
        // that cannot happen is not a reason to stop recording.
        let _ = std::fs::rename(&path, path.with_extension("log.1"));
    }
    // One line, single-quoted nothing, no structure: this is read by a person
    // in Notepad, not parsed.
    let entry = format!("{} {}\n", stamp(), message.replace('\n', " "));
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(entry.as_bytes()));
}

/// Installs the panic hook and records the start of a run.
///
/// The hook is the reason this module exists at all. A panic in a
/// GUI-subsystem process is completely silent: the window vanishes and the
/// user reports that the application "closed itself", with nothing to go on.
/// Now the payload and the source location land in the file before the
/// process dies.
pub fn init(version: &str, role: &str) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown location".to_string());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "no message".to_string());
        line(&format!("PANIC at {location}: {payload}"));
        previous(info);
    }));
    line(&format!("--- start {role} version {version}"));
}

/// Opens the folder holding the log and the removal ledger. The file is never
/// sent anywhere by the application; this is how a user gets to it.
#[tauri::command]
pub fn open_log_folder(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_opener::OpenerExt;
    let path = log_path()?;
    let directory = path
        .parent()
        .ok_or_else(|| "The log folder could not be located.".to_string())?
        .to_path_buf();
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(directory.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| format!("The folder could not be opened: {e}"))?;
    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line has to survive the round trip and carry a stamp, otherwise the
    /// file is a wall of text with no way to line it up against a removal
    /// receipt. Reading the file back is the only honest check, since every
    /// write path here deliberately swallows its errors.
    #[test]
    fn a_logged_line_reaches_the_file_with_a_timestamp() {
        let Ok(path) = log_path() else { return };
        line("test marker for a_logged_line_reaches_the_file_with_a_timestamp");
        let body = std::fs::read_to_string(&path).unwrap_or_default();
        let recorded = body
            .lines()
            .rev()
            .find(|l| l.contains("test marker for a_logged_line_reaches"))
            .expect("the line was not written");
        let stamp_text = recorded.split(' ').next().unwrap_or("");
        let recorded_stamp: u64 = stamp_text.parse().expect("no unix timestamp on the line");
        assert!(
            recorded_stamp > 1_700_000_000,
            "implausible timestamp: {recorded_stamp}"
        );
    }

    /// A newline in a message would otherwise split one event across two
    /// lines and make the file unreadable at exactly the moment it matters —
    /// an error string carrying a multi-line message from Windows.
    #[test]
    fn a_multi_line_message_stays_on_one_line() {
        let Ok(path) = log_path() else { return };
        line("multi marker\nsecond half of the same event");
        let body = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            body.lines()
                .any(|l| l.contains("multi marker") && l.contains("second half of the same event")),
            "the message was split across lines"
        );
    }
}
