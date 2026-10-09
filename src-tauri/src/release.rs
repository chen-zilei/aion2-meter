//! Asks GitHub whether a newer release than this build has been published.

use serde::{Deserialize, Serialize};

const REPO: &str = "chen-zilei/aion2-meter";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub version: String,
    pub url: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
}

/// The latest published release if it is newer than this build. Drafts and pre-releases are skipped by GitHub,
/// and any failure (offline, rate limited, repo not visible) just means no banner.
#[tauri::command]
pub async fn check_release() -> Option<Release> {
    tauri::async_runtime::spawn_blocking(|| {
        let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
        let latest: GhRelease = ureq::get(&url)
            .set("Accept", "application/vnd.github+json")
            .set("User-Agent", concat!("aion2-meter/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(10))
            .call()
            .ok()?
            .into_string()
            .ok()
            .and_then(|body| serde_json::from_str(&body).ok())?;
        let version = latest.tag_name.trim_start_matches('v').to_string();
        is_newer(&version, env!("CARGO_PKG_VERSION")).then(|| Release { version, url: latest.html_url })
    })
    .await
    .ok()
    .flatten()
}

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

/// Compares dotted versions numerically ("0.10.0" > "0.9.3"). Anything after a '-' is ignored.
fn is_newer(candidate: &str, current: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.split('-').next().unwrap_or("").split('.').map(|p| p.parse().unwrap_or(0)).collect()
    };
    let (a, b) = (parts(candidate), parts(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_numerically() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("0.10.0", "0.9.3"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("0.1.0-beta", "0.1.0"));
    }
}
