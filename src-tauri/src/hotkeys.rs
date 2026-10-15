//! The global shortcuts, rebindable from Settings.

use crate::settings::Hotkeys;
use std::str::FromStr;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    LockOverlay,
    ToggleOverlay,
    NewEncounter,
}

impl Action {
    pub const ALL: [Action; 3] = [Action::LockOverlay, Action::ToggleOverlay, Action::NewEncounter];

    /// The name the settings and the page use for it.
    pub fn id(self) -> &'static str {
        match self {
            Action::LockOverlay => "lockOverlay",
            Action::ToggleOverlay => "toggleOverlay",
            Action::NewEncounter => "newEncounter",
        }
    }

    pub fn from_id(id: &str) -> Option<Action> {
        Action::ALL.into_iter().find(|a| a.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::LockOverlay => "Lock overlay",
            Action::ToggleOverlay => "Show / hide overlay",
            Action::NewEncounter => "New encounter",
        }
    }

    pub fn keys(self, h: &Hotkeys) -> &str {
        match self {
            Action::LockOverlay => &h.lock_overlay,
            Action::ToggleOverlay => &h.toggle_overlay,
            Action::NewEncounter => &h.new_encounter,
        }
    }

    pub fn keys_mut(self, h: &mut Hotkeys) -> &mut String {
        match self {
            Action::LockOverlay => &mut h.lock_overlay,
            Action::ToggleOverlay => &mut h.toggle_overlay,
            Action::NewEncounter => &mut h.new_encounter,
        }
    }
}

/// `None` for an unbound action; an error for text that isn't a shortcut.
pub fn parse(keys: &str) -> Result<Option<Shortcut>, String> {
    if keys.trim().is_empty() {
        return Ok(None);
    }
    Shortcut::from_str(keys).map(Some).map_err(|_| format!("\"{keys}\" isn't a shortcut"))
}

/// Which action, if any, `keys` would clash with besides `action` itself.
pub fn conflict(h: &Hotkeys, action: Action, keys: &str) -> Option<Action> {
    let wanted = parse(keys).ok().flatten()?;
    Action::ALL.into_iter().filter(|&a| a != action).find(|&a| parse(a.keys(h)).ok().flatten() == Some(wanted))
}

pub fn action_for(h: &Hotkeys, pressed: &Shortcut) -> Option<Action> {
    Action::ALL.into_iter().find(|&a| parse(a.keys(h)).ok().flatten().as_ref() == Some(pressed))
}

/// Replaces every registered shortcut with `h`; returns the actions whose shortcut Windows refused
/// (usually because another app already holds it).
pub fn register(app: &AppHandle, h: &Hotkeys) -> Vec<Action> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut failed = Vec::new();
    for a in Action::ALL {
        match parse(a.keys(h)) {
            Ok(None) => {}
            Ok(Some(s)) => {
                if let Err(e) = gs.register(s) {
                    eprintln!("could not register shortcut {s:?}: {e}");
                    failed.push(a);
                }
            }
            Err(e) => {
                eprintln!("{e}");
                failed.push(a);
            }
        }
    }
    failed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_parse_and_differ() {
        let h = Hotkeys::default();
        for a in Action::ALL {
            assert!(parse(a.keys(&h)).unwrap().is_some(), "{a:?}");
            assert_eq!(conflict(&h, a, a.keys(&h)), None);
        }
    }

    #[test]
    fn the_page_formats_parse() {
        for keys in ["Ctrl+Alt+Numpad1", "Shift+F9", "F10", "Ctrl+Shift+Minus", "Super+ArrowUp", "Ctrl+Backquote", "Alt+PageDown"] {
            assert!(parse(keys).unwrap().is_some(), "{keys}");
        }
        assert!(parse("Ctrl+Shift+Nope").is_err());
        assert_eq!(parse("").unwrap(), None);
    }

    #[test]
    fn finds_conflicts_whatever_the_spelling() {
        let h = Hotkeys::default();
        assert_eq!(conflict(&h, Action::NewEncounter, "shift+ctrl+KeyL"), Some(Action::LockOverlay));
        assert_eq!(conflict(&h, Action::NewEncounter, "Ctrl+Shift+N"), None);
        // An unbound action never clashes.
        let mut h = h;
        h.lock_overlay.clear();
        assert_eq!(conflict(&h, Action::NewEncounter, ""), None);
    }

    #[test]
    fn matches_pressed_shortcut_to_action() {
        let h = Hotkeys::default();
        let pressed = Shortcut::from_str("Ctrl+Shift+O").unwrap();
        assert_eq!(action_for(&h, &pressed), Some(Action::ToggleOverlay));
        assert_eq!(action_for(&h, &Shortcut::from_str("Ctrl+Shift+P").unwrap()), None);
    }
}
