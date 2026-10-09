fn main() {
    // Delay-load Npcap so the app still starts (and can tell the user to install Npcap) when wpcap.dll is missing.
    // Without this Windows refuses to launch the exe with "wpcap.dll was not found".
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_FEATURE_LIVE").is_ok()
    {
        println!("cargo:rustc-link-arg=/DELAYLOAD:wpcap.dll");
        println!("cargo:rustc-link-lib=delayimp");
    }
    tauri_build::build()
}
