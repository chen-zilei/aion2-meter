//! User settings, saved as JSON in the app's config folder.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Server port(s) to decode. 13328 in community captures; change it here if a patch moves it.
    pub game_ports: Vec<u16>,
    pub idle_timeout_s: u64,
    /// Show synthetic fights instead of live data, for trying the UI without the game.
    pub demo: bool,
    pub overlay_visible: bool,
    /// Click-through: the overlay ignores the mouse so it never steals clicks from the game.
    pub overlay_locked: bool,
    pub overlay_opacity: f64,
    /// Hide monsters and unidentified entities from the meter.
    pub players_only: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            game_ports: vec![meter_core::pipeline::DEFAULT_GAME_PORT],
            idle_timeout_s: 8,
            demo: false,
            overlay_visible: true,
            overlay_locked: false,
            overlay_opacity: 0.85,
            players_only: true,
        }
    }
}

impl Settings {
    pub fn load(path: &PathBuf) -> Self {
        std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &PathBuf) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}
