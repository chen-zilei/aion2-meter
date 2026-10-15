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
    /// Ignore other players' fights nearby (open world); see `TrackerOptions::only_my_fights`.
    pub only_my_fights: bool,
    /// Show only you and your party members.
    pub party_only: bool,
    /// Save the raw game traffic to disk, so a fight can be decoded again after a parser fix.
    pub record_packets: bool,
    pub hotkeys: Hotkeys,
}

/// Global shortcuts, in the format `Ctrl+Shift+L`; an empty string leaves the action unbound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Hotkeys {
    pub lock_overlay: String,
    pub toggle_overlay: String,
    pub new_encounter: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self { lock_overlay: "Ctrl+Shift+L".into(), toggle_overlay: "Ctrl+Shift+O".into(), new_encounter: "Ctrl+Shift+R".into() }
    }
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
            only_my_fights: true,
            party_only: false,
            record_packets: false,
            hotkeys: Hotkeys::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_saved_before_hotkeys_existed_get_the_defaults() {
        let s: Settings = serde_json::from_str(r#"{"overlayLocked": true}"#).unwrap();
        assert!(s.overlay_locked);
        assert_eq!(s.hotkeys, Hotkeys::default());
    }

    #[test]
    fn one_rebound_hotkey_keeps_the_others() {
        let s: Settings = serde_json::from_str(r#"{"hotkeys": {"newEncounter": "Alt+F9"}}"#).unwrap();
        assert_eq!(s.hotkeys.new_encounter, "Alt+F9");
        assert_eq!(s.hotkeys.lock_overlay, "Ctrl+Shift+L");
    }
}
