#[cfg(windows)]
pub fn is_elevated() -> bool {
    is_elevated::is_elevated()
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

/// Re-launches the current executable with a UAC consent prompt, asking it to
/// run a single headless action (`--elevated-apply <id>` or
/// `--elevated-rollback <id>`) and exit. Nothing else about the app runs
/// elevated: only this one action does, and only after the user approves the
/// prompt.
pub fn run_elevated_action(action_flag: &str, tweak_id: &str) -> Result<(), String> {
    run_elevated_args(&[action_flag, tweak_id])
}

/// Same contract as [`run_elevated_action`] for actions that need more than
/// one argument. The relaunched process re-validates and re-derives
/// everything from these arguments — nothing else crosses the trust boundary.
pub fn run_elevated_args(args: &[&str]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;

    #[cfg(windows)]
    {
        let mut command = runas::Command::new(exe);
        for arg in args {
            command.arg(arg);
        }
        let status = command
            .gui(true)
            .status()
            .map_err(|e| format!("elevation was cancelled or failed: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "the elevated action exited with code {:?}",
                status.code()
            ))
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (exe, args);
        Err("elevation is not implemented on this platform yet".to_string())
    }
}
