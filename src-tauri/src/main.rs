// GUI subsystem unconditionally (not just in release): the elevated headless
// child (`--elevated-uninstall`) is this same executable, and a console
// subsystem would flash a black console window at the user on every elevated
// uninstall — including during `tauri dev`. DO NOT REMOVE.
#![windows_subsystem = "windows"]

fn main() {
    // Headless elevated branch: `app.exe --elevated-uninstall <source> <id>`
    // runs ONE uninstall (restore point + execution + report file) and exits.
    // One UAC consent covers exactly one action; the GUI never runs elevated.
    // The child re-validates and re-derives everything from these two
    // arguments — see uninstall_exec's module docs for the trust model.
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 4 && args[1] == "--elevated-uninstall" {
        // Nothing, not even the log below, may write through a linked data folder.
        let plain = pc_tweaker_uninstaller_lib::uninstall_exec::fixed_data_dir()
            .and_then(|dir| pc_tweaker_uninstaller_lib::elevation::ensure_plain_app_data_dir(&dir));
        if plain.is_err() {
            std::process::exit(1);
        }
        // The child is headless and its only channel back to the GUI is the
        // report file. When that file does not appear, this log is the only
        // place that can say why.
        pc_tweaker_uninstaller_lib::applog::init(env!("CARGO_PKG_VERSION"), "elevated-child");
        std::process::exit(
            pc_tweaker_uninstaller_lib::uninstall_exec::run_elevated_child(&args[2], &args[3]),
        );
    }

    pc_tweaker_uninstaller_lib::run()
}
