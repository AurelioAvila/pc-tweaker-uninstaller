// Prevents an additional console window on Windows in release. DO NOT REMOVE.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Headless elevated branch: `app.exe --elevated-uninstall <source> <id>`
    // runs ONE uninstall (restore point + execution + report file) and exits.
    // One UAC consent covers exactly one action; the GUI never runs elevated.
    // The child re-validates and re-derives everything from these two
    // arguments — see uninstall_exec's module docs for the trust model.
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 4 && args[1] == "--elevated-uninstall" {
        std::process::exit(
            pc_tweaker_uninstaller_lib::uninstall_exec::run_elevated_child(&args[2], &args[3]),
        );
    }

    pc_tweaker_uninstaller_lib::run()
}
