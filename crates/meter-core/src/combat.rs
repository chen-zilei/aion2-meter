//! Combat events → encounters and DPS.

use crate::parser::{EntityId, Event};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TrackerOptions {
    /// An encounter ends after this long without any damage.
    pub idle_timeout_ms: u64,
    /// Finished encounters kept for the history view.
    pub history_len: usize,
    /// Count only fights you are part of: damage to something you hit or that hit you. Other players' fights nearby
    /// are ignored, so they neither inflate the meter nor keep an encounter alive. Everything counts until your own
    /// character is known.
    pub only_my_fights: bool,
}

impl Default for TrackerOptions {
    fn default() -> Self {
        Self { idle_timeout_ms: 8_000, history_len: 30, only_my_fights: true }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillStats {
    pub skill: u32,
    pub damage: u64,
    pub hits: u32,
    pub crits: u32,
    pub max_hit: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorStats {
    pub id: EntityId,
    pub name: String,
    pub is_self: bool,
    pub is_player: bool,
    pub damage: u64,
    pub dps: f64,
    pub share: f64,
    pub hits: u32,
    pub crits: u32,
    pub skills: Vec<SkillStats>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: u64,
    pub active: bool,
    pub started_ms: u64,
    pub duration_s: f64,
    pub total_damage: u64,
    pub party_dps: f64,
    /// Name or id of the target that took the most damage.
    pub main_target: String,
    /// Sorted by damage, highest first.
    pub actors: Vec<ActorStats>,
}

#[derive(Default)]
struct Encounter {
    id: u64,
    first_ms: u64,
    last_ms: u64,
    by_actor: HashMap<EntityId, (u64, u32, u32, HashMap<u32, SkillStats>)>,
    by_target: HashMap<EntityId, u64>,
}

/// Turns an NPC template code into a display name, e.g. from the name tables. `None` falls back to `NPC <code>`.
pub type NpcNamer = Box<dyn Fn(u32) -> Option<String> + Send>;

pub struct Tracker {
    opts: TrackerOptions,
    pub npc_namer: Option<NpcNamer>,
    names: HashMap<EntityId, String>,
    players: std::collections::HashSet<EntityId>,
    self_id: Option<EntityId>,
    current: Option<Encounter>,
    /// Party members by character name, from the latest party list.
    party: std::collections::HashSet<String>,
    /// Entities in your fight: what you or your party hit, and what hit you or them. Cleared when the encounter ends.
    engaged: std::collections::HashSet<EntityId>,
    next_id: u64,
    pub history: Vec<Snapshot>,
}

impl Tracker {
    pub fn new(opts: TrackerOptions) -> Self {
        Self {
            opts,
            npc_namer: None,
            names: HashMap::new(),
            players: Default::default(),
            self_id: None,
            current: None,
            party: Default::default(),
            engaged: Default::default(),
            next_id: 1,
            history: Vec::new(),
        }
    }

    /// Names a non-player entity (a boss, a dummy) without counting it as a player.
    pub fn set_name(&mut self, id: EntityId, name: impl Into<String>) {
        self.names.insert(id, name.into());
    }

    pub fn self_name(&self) -> Option<&str> {
        self.self_id.and_then(|id| self.names.get(&id)).map(String::as_str)
    }

    pub fn event(&mut self, ev: &Event) {
        match ev {
            Event::Identity { id, name, is_self, .. } => {
                self.names.insert(*id, name.clone());
                self.players.insert(*id);
                if *is_self {
                    self.self_id = Some(*id);
                }
            }
            Event::Damage { t_ms, actor, target, skill, amount, flags } => {
                self.tick(*t_ms);
                if !self.in_my_fight(*actor, *target) {
                    return;
                }
                let enc = self.current.get_or_insert_with(|| {
                    let id = self.next_id;
                    self.next_id += 1;
                    Encounter { id, first_ms: *t_ms, ..Default::default() }
                });
                enc.last_ms = enc.last_ms.max(*t_ms);
                let a = enc.by_actor.entry(*actor).or_default();
                a.0 += *amount as u64;
                a.1 += 1;
                a.2 += flags.crit as u32;
                let s = a.3.entry(*skill).or_insert_with(|| SkillStats { skill: *skill, ..Default::default() });
                s.damage += *amount as u64;
                s.hits += 1;
                s.crits += flags.crit as u32;
                s.max_hit = s.max_hit.max(*amount);
                *enc.by_target.entry(*target).or_default() += *amount as u64;
            }
            Event::NpcSpawn { id, npc_code, .. } => {
                // Ids are reused once an entity is gone, so a respawn replaces whatever name the id had.
                if !self.players.contains(id) {
                    let name = self.npc_namer.as_ref().and_then(|f| f(*npc_code)).unwrap_or_else(|| format!("NPC {npc_code}"));
                    self.names.insert(*id, name);
                }
            }
            Event::Party { members, .. } => self.party = members.iter().cloned().collect(),
            Event::Heal { .. } | Event::Death { .. } => {}
        }
    }

    /// Whether a hit belongs to your fight, marking what you or your party hit (or what hit you) as part of it.
    fn in_my_fight(&mut self, actor: EntityId, target: EntityId) -> bool {
        let Some(me) = self.self_id.filter(|_| self.opts.only_my_fights) else { return true };
        let ours = |id: EntityId| {
            id == me || (self.players.contains(&id) && self.names.get(&id).is_some_and(|n| self.party.contains(n)))
        };
        let (actor_ours, target_ours) = (ours(actor), ours(target));
        if actor_ours {
            self.engaged.insert(target);
        } else if target_ours {
            self.engaged.insert(actor);
        }
        target_ours || self.engaged.contains(&target)
    }

    /// Closes the current encounter once it has been idle long enough. Call with the latest packet time.
    pub fn tick(&mut self, now_ms: u64) {
        let idle = self.current.as_ref().is_some_and(|e| now_ms.saturating_sub(e.last_ms) > self.opts.idle_timeout_ms);
        if idle {
            self.finish();
        }
    }

    /// Ends the current encounter now (a "reset" button, or the end of a replay).
    pub fn finish(&mut self) {
        self.engaged.clear();
        if let Some(enc) = self.current.take() {
            let mut snap = self.snapshot_of(&enc);
            snap.active = false;
            self.history.insert(0, snap);
            self.history.truncate(self.opts.history_len);
        }
    }

    /// The live encounter, or the most recent finished one.
    pub fn snapshot(&self) -> Option<Snapshot> {
        match &self.current {
            Some(enc) => Some(self.snapshot_of(enc)),
            None => self.history.first().cloned(),
        }
    }

    fn name_of(&self, id: EntityId) -> String {
        self.names.get(&id).cloned().unwrap_or_else(|| format!("#{id}"))
    }

    fn snapshot_of(&self, enc: &Encounter) -> Snapshot {
        // At least one second, so a single opening hit does not read as millions of DPS.
        let duration_s = ((enc.last_ms - enc.first_ms) as f64 / 1000.0).max(1.0);
        let total: u64 = enc.by_actor.values().map(|a| a.0).sum();
        let mut actors: Vec<ActorStats> = enc
            .by_actor
            .iter()
            .map(|(&id, (damage, hits, crits, skills))| {
                let mut skills: Vec<SkillStats> = skills.values().cloned().collect();
                skills.sort_by_key(|s| std::cmp::Reverse(s.damage));
                ActorStats {
                    id,
                    name: self.name_of(id),
                    is_self: Some(id) == self.self_id,
                    is_player: self.players.contains(&id),
                    damage: *damage,
                    dps: *damage as f64 / duration_s,
                    share: if total > 0 { *damage as f64 / total as f64 } else { 0.0 },
                    hits: *hits,
                    crits: *crits,
                    skills,
                }
            })
            .collect();
        actors.sort_by_key(|a| std::cmp::Reverse(a.damage));
        let party: u64 = actors.iter().filter(|a| a.is_player).map(|a| a.damage).sum();
        let main_target = enc.by_target.iter().max_by_key(|(_, d)| **d).map(|(&id, _)| self.name_of(id)).unwrap_or_default();
        Snapshot {
            id: enc.id,
            active: true,
            started_ms: enc.first_ms,
            duration_s,
            total_damage: total,
            party_dps: party as f64 / duration_s,
            main_target,
            actors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::HitFlags;

    fn hit(t_ms: u64, actor: u32, amount: u32) -> Event {
        Event::Damage { t_ms, actor, target: 900, skill: 1, amount, flags: HitFlags::default() }
    }

    fn hit_on(t_ms: u64, actor: u32, target: u32, amount: u32) -> Event {
        Event::Damage { t_ms, actor, target, skill: 1, amount, flags: HitFlags::default() }
    }

    #[test]
    fn ignores_fights_you_are_not_in() {
        let mut t = Tracker::new(TrackerOptions::default());
        t.event(&Event::Identity { t_ms: 0, id: 1, name: "Me".into(), is_self: true });
        t.event(&hit_on(0, 2, 800, 999)); // a stranger's mob: no encounter
        assert!(t.snapshot().is_none());
        t.event(&hit_on(100, 1, 900, 50)); // I engage 900
        t.event(&hit_on(200, 2, 900, 30)); // the stranger helps on my mob: counted
        t.event(&hit_on(300, 2, 800, 999)); // their own mob: not counted
        t.event(&hit_on(400, 3, 1, 7)); // a mob hits me: it joins my fight
        t.event(&hit_on(500, 1, 3, 20));
        let s = t.snapshot().unwrap();
        assert_eq!(s.total_damage, 50 + 30 + 7 + 20);
        assert_eq!(s.main_target, "#900");
    }

    #[test]
    fn counts_party_members_fights() {
        let mut t = Tracker::new(TrackerOptions::default());
        t.event(&Event::Identity { t_ms: 0, id: 1, name: "Me".into(), is_self: true });
        t.event(&Event::Identity { t_ms: 0, id: 2, name: "Faelis".into(), is_self: false });
        t.event(&Event::Identity { t_ms: 0, id: 4, name: "Stranger".into(), is_self: false });
        t.event(&Event::Party { t_ms: 0, members: vec!["Me".into(), "Faelis".into()] });
        t.event(&hit_on(0, 2, 800, 40)); // party member's mob I never touched
        t.event(&hit_on(100, 4, 700, 999)); // a stranger's mob
        assert_eq!(t.snapshot().unwrap().total_damage, 40);
    }

    #[test]
    fn strangers_do_not_keep_your_encounter_alive() {
        let mut t = Tracker::new(TrackerOptions::default());
        t.event(&Event::Identity { t_ms: 0, id: 1, name: "Me".into(), is_self: true });
        t.event(&hit_on(0, 1, 900, 50));
        for i in 1..20 {
            t.event(&hit_on(i * 1_000, 2, 800, 10));
        }
        assert!(!t.snapshot().unwrap().active);
        assert_eq!(t.history.len(), 1);
    }

    #[test]
    fn counts_everything_until_you_are_known_or_when_turned_off() {
        let mut t = Tracker::new(TrackerOptions::default());
        t.event(&hit_on(0, 2, 800, 10));
        assert_eq!(t.snapshot().unwrap().total_damage, 10);

        let mut t = Tracker::new(TrackerOptions { only_my_fights: false, ..Default::default() });
        t.event(&Event::Identity { t_ms: 0, id: 1, name: "Me".into(), is_self: true });
        t.event(&hit_on(0, 2, 800, 10));
        assert_eq!(t.snapshot().unwrap().total_damage, 10);
    }

    #[test]
    fn names_spawned_npcs() {
        let mut t = Tracker::new(TrackerOptions::default());
        t.event(&Event::NpcSpawn { t_ms: 0, id: 900, npc_code: 2_000_002, max_hp: None });
        t.event(&hit(0, 1, 10));
        assert_eq!(t.snapshot().unwrap().main_target, "NPC 2000002");

        t.npc_namer = Some(Box::new(|code| (code == 2_000_002).then(|| "Draconute Ranger".to_owned())));
        t.event(&Event::NpcSpawn { t_ms: 0, id: 900, npc_code: 2_000_002, max_hp: None });
        assert_eq!(t.snapshot().unwrap().main_target, "Draconute Ranger");
    }

    #[test]
    fn computes_dps_and_splits_encounters_on_idle() {
        let mut t = Tracker::new(TrackerOptions::default());
        t.event(&Event::Identity { t_ms: 0, id: 1, name: "Me".into(), is_self: true });
        t.event(&hit(0, 1, 1_000));
        t.event(&hit(5_000, 1, 4_000));
        t.event(&hit(5_000, 2, 6_000));
        let s = t.snapshot().unwrap();
        assert_eq!(s.total_damage, 11_000);
        assert_eq!(s.actors[0].name, "#2");
        assert_eq!(s.actors[1].dps, 1_000.0);
        assert!(s.actors[1].is_self);

        t.event(&hit(20_000, 1, 1)); // more than 8 s later: new encounter
        assert_eq!(t.history.len(), 1);
        assert_eq!(t.snapshot().unwrap().total_damage, 1);
    }
}
