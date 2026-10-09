//! Opens release notes on GitHub. Checking for and installing updates is done by the updater plugin.

const REPO: &str = "chen-zilei/aion2-meter";

/// Opens a release page in the default browser. Only this repo's release pages are allowed.
#[tauri::command]
pub fn open_release_page(url: String) -> Result<(), String> {
    if !url.starts_with(&format!("https://github.com/{REPO}/releases/")) {
        return Err("not a release page".into());
    }
    #[cfg(windows)]
    let opener = "explorer";
    #[cfg(target_os = "macos")]
    let opener = "open";
    #[cfg(not(any(windows, target_os = "macos")))]
    let opener = "xdg-open";
    std::process::Command::new(opener).arg(&url).spawn().map(|_| ()).map_err(|e| e.to_string())
}
