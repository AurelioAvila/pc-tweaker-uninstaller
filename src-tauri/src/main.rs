// GUI subsystem unconditionally (not just in release): the elevated headless
// child (`--elevated-uninstall`) is this same executable, and a console
// subsystem would flash a black console window at the user on every elevated
// uninstall — including during `tauri dev`. DO NOT REMOVE.
#![windows_subsystem = "windows"]

// ADVAPI32's SystemFunction036 (the random source behind getrandom) is a
// forwarder into CRYPTBASE.DLL, and Windows resolves forwarders by plain name,
// install folder first. Importing CRYPTBASE.DLL itself makes it an import of
// the exe, which /DEPENDENTLOADFLAG (build.rs) maps from System32 before any
// forwarder is resolved. By ordinal, under a name of our own: every CRYPTBASE
// export name is also in the ADVAPI32 import libraries, so a by-name import
// would bind there. Never called. Same as PC Tweaker.
#[cfg(all(windows, target_env = "msvc"))]
#[link(name = "cryptbase", kind = "raw-dylib")]
extern "system" {
    #[link_ordinal(1)]
    fn pcu_cryptbase_import();
}

fn main() {
    // Keeps the CRYPTBASE.DLL import above in the exe.
    #[cfg(all(windows, target_env = "msvc"))]
    std::hint::black_box(pcu_cryptbase_import as unsafe extern "system" fn());
    // Headless elevated branch: `app.exe --elevated-uninstall <source> <id>`
    // runs ONE uninstall (restore point + execution + report file) and exits;
    // `--elevated-forget` removes one broken entry the same way (forget.rs).
    // One UAC consent covers exactly one action; the GUI never runs elevated.
    // The child re-validates and re-derives everything from these two
    // arguments — see uninstall_exec's module docs for the trust model.
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 4 && (args[1] == "--elevated-uninstall" || args[1] == "--elevated-forget") {
        // First, before anything can load a DLL by name from the per-user
        // install folder or the working directory.
        pc_tweaker_uninstaller_lib::elevation::restrict_dll_search_to_system32();
        use pc_tweaker_uninstaller_lib::elevation::decode_elevated_arg;
        let (Ok(source), Ok(id)) = (decode_elevated_arg(&args[2]), decode_elevated_arg(&args[3]))
        else {
            std::process::exit(2);
        };
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
        std::process::exit(if args[1] == "--elevated-forget" {
            pc_tweaker_uninstaller_lib::forget::run_elevated_child(&source, &id)
        } else {
            pc_tweaker_uninstaller_lib::uninstall_exec::run_elevated_child(&source, &id)
        });
    }

    pc_tweaker_uninstaller_lib::run()
}
