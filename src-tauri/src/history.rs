//! Finished encounters, saved so the History page survives a restart or an update.

use meter_core::combat::Snapshot;
use std::path::Path;

pub fn load(path: &Path) -> Vec<Snapshot> {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save(path: &Path, history: &[Snapshot]) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string(history) {
        // Write then rename, so quitting mid-write can't leave a half file that loads as empty.
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}
