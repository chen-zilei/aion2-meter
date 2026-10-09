//! Skill and NPC name tables: where they live on disk, how they get there, and the commands the UI uses to read them.
//!
//! The tables are not shipped in this repository. AION 2 names are NCSOFT's game data, and the community tables that
//! hold them are published under GPL-3.0, so the app downloads them onto the user's machine instead: once on first
//! start, and again from Settings. The source is pinned to one commit so a table can't change under us unnoticed;
//! bump [`SOURCE_COMMIT`] after a game patch adds skills.

use meter_core::names::Names;
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// taengu/A2Tools-DPS-Meter (GPL-3.0), `src/data/i18n`: tables extracted from the game client, kept up to date by
/// that project and also used by cyberbadger6969/aion2-dps-meter and mazixs/A2Tools-DPS-Meter.
pub const SOURCE_REPO: &str = "taengu/A2Tools-DPS-Meter";
pub const SOURCE_COMMIT: &str = "82e53c1008ac4c2974446cc703703f473bbbed81";
const TABLES: [&str; 2] = ["skills", "npcs"];

pub struct GameData {
    dir: PathBuf,
    lang: String,
    names: RwLock<Names>,
    state: RwLock<State>,
}

#[derive(Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum State {
    Ready,
    Downloading,
    Failed { message: String },
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    #[serde(flatten)]
    state: State,
    skills: usize,
    npcs: usize,
    folder: String,
    source: String,
}

#[derive(Serialize, Default)]
pub struct Lookup {
    skills: HashMap<u32, String>,
    npcs: HashMap<u32, String>,
}

impl GameData {
    pub fn new(dir: PathBuf) -> Arc<Self> {
        let lang = "en".to_owned();
        let names = Names::load(&dir, &lang);
        Arc::new(Self { dir, lang, names: RwLock::new(names), state: RwLock::new(State::Ready) })
    }

    pub fn is_empty(&self) -> bool {
        self.names.read().is_empty()
    }

    pub fn status(&self) -> Status {
        let names = self.names.read();
        Status {
            state: self.state.read().clone(),
            skills: names.skills.len(),
            npcs: names.npcs.len(),
            folder: self.dir.display().to_string(),
            source: format!("https://github.com/{SOURCE_REPO}/tree/{SOURCE_COMMIT}/src/data/i18n"),
        }
    }

    /// Names for the codes asked about; codes the tables don't know are left out.
    pub fn lookup(&self, skills: &[u32], npcs: &[u32]) -> Lookup {
        let names = self.names.read();
        Lookup {
            skills: skills.iter().filter_map(|&c| Some((c, names.skill(c)?.to_owned()))).collect(),
            npcs: npcs.iter().filter_map(|&c| Some((c, names.npc(c)?.name.clone()))).collect(),
        }
    }

    /// Downloads the tables on a background thread, then reloads them. `done` runs afterwards either way.
    pub fn download_in_background(self: &Arc<Self>, done: impl FnOnce() + Send + 'static) {
        {
            let mut state = self.state.write();
            if *state == State::Downloading {
                return;
            }
            *state = State::Downloading;
        }
        let this = self.clone();
        std::thread::spawn(move || {
            let result = download(&this.dir, &this.lang);
            *this.names.write() = Names::load(&this.dir, &this.lang);
            *this.state.write() = match result {
                Ok(()) => State::Ready,
                Err(e) => State::Failed { message: format!("{e:#}") },
            };
            done();
        });
    }
}

fn download(dir: &Path, lang: &str) -> anyhow::Result<()> {
    use anyhow::Context;
    let langs: &[&str] = if lang == "en" { &["en"] } else { &["en", lang] };
    for table in TABLES {
        for l in langs {
            let url = format!("https://raw.githubusercontent.com/{SOURCE_REPO}/{SOURCE_COMMIT}/src/data/i18n/{table}/{l}.json");
            let body = ureq::get(&url).call().with_context(|| format!("downloading {table}/{l}.json"))?.into_string()?;
            // Refuse anything that isn't a JSON object rather than overwrite a good table with an error page.
            let parsed: serde_json::Value = serde_json::from_str(&body).with_context(|| format!("{table}/{l}.json is not JSON"))?;
            anyhow::ensure!(parsed.is_object(), "{table}/{l}.json is not a table");
            let path = dir.join(table).join(format!("{l}.json"));
            std::fs::create_dir_all(path.parent().unwrap())?;
            let tmp = path.with_extension("json.part");
            std::fs::write(&tmp, body)?;
            std::fs::rename(&tmp, &path)?;
        }
    }
    Ok(())
}
