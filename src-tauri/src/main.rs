// Prevents an additional console window on Windows in release. DO NOT REMOVE.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Phase 3 adds the `--elevated-*` headless branch here (one privileged
    // action per UAC consent, same pattern as PC Tweaker's main.rs).
    pc_tweaker_uninstaller_lib::run()
}
