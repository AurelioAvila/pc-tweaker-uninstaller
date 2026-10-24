fn main() {
    tauri_build::build();

    // The app installs per user and relaunches itself elevated, so its own
    // folder is writable without administrator rights. Resolve the exe's
    // static imports from System32 only, so a DLL placed next to it never
    // loads into the elevated child (scripts/check-dll-imports.mjs checks the
    // shipped exe; CRYPTBASE.DLL: see main.rs). Same as PC Tweaker.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg-bins=/DEPENDENTLOADFLAG:0x800");
    }
}
