//! Who is who, saved so a meter restart mid-zone still knows you and the players around you.

use meter_core::combat::KnownIdentities;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Older than this, ids have likely been reissued (zone changes, relogs), so the file is ignored.
const MAX_AGE_MS: u64 = 45 * 60 * 1000;

#[derive(Serialize, Deserialize)]
struct Saved {
    saved_at_ms: u64,
    known: KnownIdentities,
}

pub fn load(path: &Path, now_ms: u64) -> Option<KnownIdentities> {
    let saved: Saved = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    (now_ms.saturating_sub(saved.saved_at_ms) <= MAX_AGE_MS).then_some(saved.known)
}

pub fn save(path: &Path, known: KnownIdentities, now_ms: u64) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string(&Saved { saved_at_ms: now_ms, known }) {
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}
