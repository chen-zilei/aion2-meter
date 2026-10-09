//! Turns the numbers in combat packets (skill codes, NPC template codes) into the names the game shows.
//!
//! The packets carry no names for these: every open-source AION 2 meter resolves them from lookup tables that were
//! extracted from the game client's data. This module reads the table layout most of those meters share:
//!
//! ```text
//! <dir>/skills/<lang>.json   { "11020000": "Keen Strike", … }
//! <dir>/npcs/<lang>.json     { "2000002": { "name": "Draconute Ranger", "isBoss": false, "isDummy": false }, … }
//! <dir>/skills.json          optional hand-written corrections, same shape, applied last
//! <dir>/npcs.json            (values may also be a bare name string)
//! ```
//!
//! English is loaded first and the chosen language on top, so anything a translation lacks keeps its English name.
//! Every file is optional: without them the meter shows `Skill 11020000` / `NPC 2000002`.

use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Npc {
    pub name: String,
    pub is_boss: bool,
    pub is_dummy: bool,
}

#[derive(Debug, Default, Clone)]
pub struct Names {
    pub skills: HashMap<u32, String>,
    pub npcs: HashMap<u32, Npc>,
}

impl Names {
    /// Loads `<dir>` in `lang` (English underneath). Missing or malformed files are skipped.
    pub fn load(dir: &Path, lang: &str) -> Self {
        let mut names = Self::default();
        let layers: &[&str] = if lang == "en" { &["en"] } else { &["en", lang] };
        for l in layers {
            names.add_skills(&read(&dir.join("skills").join(format!("{l}.json"))));
            names.add_npcs(&read(&dir.join("npcs").join(format!("{l}.json"))));
        }
        names.add_skills(&read(&dir.join("skills.json")));
        names.add_npcs(&read(&dir.join("npcs.json")));
        names
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty() && self.npcs.is_empty()
    }

    /// Adds a `skills/<lang>.json` table on top of what is loaded.
    pub fn add_skills(&mut self, table: &Value) {
        for (code, name) in entries(table) {
            if let Some(name) = name.as_str().filter(|n| !n.is_empty()) {
                self.skills.insert(code, name.to_owned());
            }
        }
    }

    /// Adds an `npcs/<lang>.json` table on top of what is loaded. Flags a layer leaves out keep their earlier value.
    pub fn add_npcs(&mut self, table: &Value) {
        for (code, v) in entries(table) {
            let prev = self.npcs.get(&code).cloned();
            let flag = |key: &str, old: Option<bool>| v.get(key).and_then(Value::as_bool).or(old).unwrap_or(false);
            let name = match v {
                Value::String(s) => Some(s.as_str()),
                _ => v.get("name").and_then(Value::as_str),
            }
            .filter(|n| !n.is_empty())
            .map(str::to_owned)
            .or_else(|| prev.as_ref().map(|p| p.name.clone()));
            let Some(name) = name else { continue };
            let npc = Npc {
                name,
                is_boss: flag("isBoss", prev.as_ref().map(|p| p.is_boss)),
                is_dummy: flag("isDummy", prev.as_ref().map(|p| p.is_dummy)),
            };
            self.npcs.insert(code, npc);
        }
    }

    /// The skill's name, if the tables know it or its base skill.
    ///
    /// Player skill codes are 8 digits (class × 1,000,000 + skill × 10,000 + level / specialisation); the tables list
    /// many but not all variants, so an unknown variant falls back to its base code (`11020047` → `11020000`).
    pub fn skill(&self, code: u32) -> Option<&str> {
        self.skills.get(&code).or_else(|| self.skills.get(&base_skill(code))).map(String::as_str)
    }

    /// The skill's name, or `Skill <code>` when the tables don't know it.
    pub fn skill_or_code(&self, code: u32) -> String {
        self.skill(code).map_or_else(|| format!("Skill {code}"), str::to_owned)
    }

    pub fn npc(&self, code: u32) -> Option<&Npc> {
        self.npcs.get(&code)
    }

    /// The NPC's name, or `NPC <code>` when the tables don't know it.
    pub fn npc_or_code(&self, code: u32) -> String {
        self.npc(code).map_or_else(|| format!("NPC {code}"), |n| n.name.clone())
    }
}

/// Class skills (10,000,000..=19,999,999) without their last four digits; any other code unchanged.
pub fn base_skill(code: u32) -> u32 {
    if (10_000_000..=19_999_999).contains(&code) {
        code - code % 10_000
    } else {
        code
    }
}

fn read(path: &Path) -> Value {
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}

fn entries(table: &Value) -> impl Iterator<Item = (u32, &Value)> {
    table.as_object().into_iter().flatten().filter_map(|(k, v)| Some((k.parse().ok()?, v)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn names() -> Names {
        let mut n = Names::default();
        n.add_skills(&json!({ "11020000": "Keen Strike", "11020010": "Keen Strike II", "1000": "Rest", "bad": "x" }));
        n.add_npcs(&json!({
            "2000000": { "name": "Punching Bag", "isBoss": false, "isDummy": true },
            "2400101": { "name": "Field Boss", "isBoss": true },
        }));
        n
    }

    #[test]
    fn exact_then_base_skill() {
        let n = names();
        assert_eq!(n.skill(11_020_010), Some("Keen Strike II"));
        assert_eq!(n.skill(11_020_047), Some("Keen Strike"));
        assert_eq!(n.skill(1000), Some("Rest"));
        assert_eq!(n.skill_or_code(11_030_000), "Skill 11030000");
        assert_eq!(n.skills.len(), 3);
    }

    #[test]
    fn npcs_and_layering() {
        let mut n = names();
        assert!(n.npc(2_000_000).unwrap().is_dummy);
        // A translation layer renames without dropping the flags, and a bare string works too.
        n.add_npcs(&json!({ "2400101": { "name": "Feldboss" }, "2000000": "Sandsack" }));
        assert_eq!(n.npc(2_400_101), Some(&Npc { name: "Feldboss".into(), is_boss: true, is_dummy: false }));
        assert_eq!(n.npc(2_000_000).unwrap().name, "Sandsack");
        assert!(n.npc(2_000_000).unwrap().is_dummy);
        assert_eq!(n.npc_or_code(7), "NPC 7");
    }

    #[test]
    fn loads_a_folder_with_fallback_to_english() {
        let dir = std::env::temp_dir().join(format!("meter-names-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("skills")).unwrap();
        std::fs::create_dir_all(dir.join("npcs")).unwrap();
        std::fs::write(dir.join("skills/en.json"), r#"{"11020000":"Keen Strike","1000":"Rest"}"#).unwrap();
        std::fs::write(dir.join("skills/de.json"), r#"{"11020000":"Scharfer Schlag"}"#).unwrap();
        std::fs::write(dir.join("skills.json"), r#"{"1000":"Resting"}"#).unwrap();
        std::fs::write(dir.join("npcs/en.json"), "not json").unwrap();
        let n = Names::load(&dir, "de");
        assert_eq!(n.skill(11_020_000), Some("Scharfer Schlag"));
        assert_eq!(n.skill(1000), Some("Resting"));
        assert!(n.npcs.is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
