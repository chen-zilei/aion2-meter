//! Combat events → encounters and DPS.

use crate::parser::{EntityId, Event};
use serde::{Deserialize, Serialize};
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
    /// Healing done, including heals over time. Overhealing can't be told apart, so it is included.
    pub healing: u64,
    pub hps: f64,
    /// Damage taken from monsters.
    pub damage_taken: u64,
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
    by_actor: HashMap<EntityId, ActorTotals>,
    by_target: HashMap<EntityId, u64>,
}

#[derive(Default)]
struct ActorTotals {
    damage: u64,
    hits: u32,
    crits: u32,
    skills: HashMap<u32, SkillStats>,
    healing: u64,
    taken: u64,
}

/// Who is who, kept across meter restarts: AION 2 only names a player when they come into view or on a loading screen,
/// so a restart mid-zone would otherwise leave you unidentified (and your own row hidden) until the next zone.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct KnownIdentities {
    pub self_id: Option<EntityId>,
    pub players: Vec<(EntityId, String)>,
    pub party: Vec<String>,
}

/// Class skills are 8 digits; NPC skills and item procs aren't, and a zero class digit pair marks a shared skill.
fn is_class_skill(code: u32) -> bool {
    (10_000_000..=19_999_999).contains(&code) && !(code / 10_000).is_multiple_of(100)
}

/// NPC skills are 7 digits. Players fire some too (item and godstone procs), so they only mark an actor not yet
/// known as a player.
fn is_npc_skill(code: u32) -> bool {
    (1_000_000..=9_999_999).contains(&code)
}

/// What the name tables say about an NPC template.
#[derive(Debug, Clone, PartialEq)]
pub struct NpcInfo {
    pub name: String,
    pub is_boss: bool,
}

/// Looks up an NPC template code, e.g. in the name tables. `None` names it `NPC <code>`.
pub type NpcNamer = Box<dyn Fn(u32) -> Option<NpcInfo> + Send>;

/// Instance (dungeon) map ids.
fn is_instance(map_id: u32) -> bool {
    (600_000..700_000).contains(&map_id)
}

/// In an instance, a boss fight stays one encounter through downtime (phase changes, invulnerable stretches) until the
/// boss dies, you leave, or nobody has hit anything for this long (a wipe).
const BOSS_FIGHT_IDLE_MS: u64 = 180_000;

pub struct Tracker {
    opts: TrackerOptions,
    pub npc_namer: Option<NpcNamer>,
    names: HashMap<EntityId, String>,
    players: std::collections::HashSet<EntityId>,
    /// Monsters and other NPCs: spawned as one, or seen using monster skills.
    npcs: std::collections::HashSet<EntityId>,
    /// Living bosses, per the name tables.
    bosses: std::collections::HashSet<EntityId>,
    /// The map loaded last, 0 until the first loading screen.
    map_id: u32,
    self_id: Option<EntityId>,
    current: Option<Encounter>,
    /// Ids whose names came from [`Tracker::restore`] rather than this run; dropped if the game shows they are stale.
    restored: std::collections::HashSet<EntityId>,
    /// Whether your id has shown up in a hit. Until it has, "only my fights" counts everything: an id that never
    /// fights is stale or not the one combat uses, and filtering on it would hide all of your damage.
    self_in_combat: bool,
    /// Bumped whenever who-is-who changes, so callers know when to save [`Tracker::identities`].
    pub identity_version: u64,
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
            npcs: Default::default(),
            bosses: Default::default(),
            map_id: 0,
            self_id: None,
            current: None,
            restored: Default::default(),
            self_in_combat: false,
            identity_version: 0,
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

    /// Named players, you and your party, for [`Tracker::restore`] after a restart.
    pub fn identities(&self) -> KnownIdentities {
        let mut players: Vec<_> =
            self.players.iter().filter_map(|id| self.names.get(id).map(|n| (*id, n.clone()))).collect();
        players.sort();
        let mut party: Vec<_> = self.party.iter().cloned().collect();
        party.sort();
        KnownIdentities { self_id: self.self_id, players, party }
    }

    /// Restores what an earlier run knew. Anything this run already learned wins.
    pub fn restore(&mut self, known: KnownIdentities) {
        for (id, name) in known.players {
            if self.players.insert(id) {
                self.names.insert(id, name);
                self.restored.insert(id);
            }
        }
        if self.self_id.is_none() {
            self.self_id = known.self_id.filter(|id| self.players.contains(id));
        }
        if self.party.is_empty() {
            self.party = known.party.into_iter().collect();
        }
        self.identity_version += 1;
    }

    pub fn self_name(&self) -> Option<&str> {
        self.self_id.and_then(|id| self.names.get(&id)).map(String::as_str)
    }

    pub fn event(&mut self, ev: &Event) {
        match ev {
            Event::Identity { id, name, is_self, .. } => {
                if *is_self && self.self_id.is_some_and(|old| old != *id && self.restored.contains(&old)) {
                    // Our own id moved while restored names were in use: the zone changed while the meter was off,
                    // so every restored id may now belong to someone else.
                    for old in std::mem::take(&mut self.restored) {
                        self.names.remove(&old);
                        self.players.remove(&old);
                    }
                }
                self.restored.remove(id);
                self.npcs.remove(id);
                self.names.insert(*id, name.clone());
                self.players.insert(*id);
                if *is_self && self.self_id != Some(*id) {
                    self.self_id = Some(*id);
                    self.self_in_combat = false;
                }
                self.identity_version += 1;
            }
            Event::Damage { t_ms, actor, target, skill, amount, flags } => {
                self.tick(*t_ms);
                // Anyone using class skills is a player, named or not, so an unnamed you is never hidden as a monster.
                if is_class_skill(*skill) && self.players.insert(*actor) {
                    self.npcs.remove(actor);
                    self.identity_version += 1;
                } else if is_npc_skill(*skill) && !self.players.contains(actor) {
                    self.npcs.insert(*actor);
                }
                if !self.in_my_fight(*actor, *target) {
                    return;
                }
                let target_is_player = self.is_player(*target);
                let actor_is_npc = self.npcs.contains(actor);
                if target_is_player && self.is_player(*actor) {
                    return; // PvP and duels are not tracked
                }
                if actor_is_npc && self.npcs.contains(target) {
                    return; // monsters fighting each other
                }
                let enc = self.encounter(*t_ms);
                enc.last_ms = enc.last_ms.max(*t_ms);
                // A monster's hits are damage taken by whatever it hit (a player not identified yet, or a summon),
                // never damage dealt.
                if target_is_player || actor_is_npc {
                    enc.by_actor.entry(*target).or_default().taken += *amount as u64;
                    return;
                }
                let a = enc.by_actor.entry(*actor).or_default();
                a.damage += *amount as u64;
                a.hits += 1;
                a.crits += flags.crit as u32;
                let s = a.skills.entry(*skill).or_insert_with(|| SkillStats { skill: *skill, ..Default::default() });
                s.damage += *amount as u64;
                s.hits += 1;
                s.crits += flags.crit as u32;
                s.max_hit = s.max_hit.max(*amount);
                *enc.by_target.entry(*target).or_default() += *amount as u64;
            }
            Event::Heal { actor, target, amount, .. } => {
                // Heals join a fight but never start or extend one, so a heal over time can't keep it open.
                let mine = !self.self_in_combat || !self.opts.only_my_fights || self.is_ours(*actor) || self.is_ours(*target);
                if let Some(enc) = self.current.as_mut().filter(|_| mine) {
                    enc.by_actor.entry(*actor).or_default().healing += *amount as u64;
                }
            }
            Event::NpcSpawn { id, npc_code, .. } => {
                // Ids are reused once an entity is gone, so a respawn replaces whatever name the id had.
                if !self.players.contains(id) {
                    self.npcs.insert(*id);
                    let info = self.npc_namer.as_ref().and_then(|f| f(*npc_code));
                    if info.as_ref().is_some_and(|i| i.is_boss) {
                        self.bosses.insert(*id);
                    } else {
                        self.bosses.remove(id);
                    }
                    self.names.insert(*id, info.map(|i| i.name).unwrap_or_else(|| format!("NPC {npc_code}")));
                }
            }
            Event::Party { members, .. } => {
                self.party = members.iter().cloned().collect();
                self.identity_version += 1;
            }
            Event::Death { id, .. } => {
                // The boss you were fighting died: that fight is over, whatever downtime it had.
                let fought = self.current.as_ref().is_some_and(|e| e.by_target.contains_key(id));
                if self.bosses.remove(id) && fought {
                    self.finish();
                }
            }
            Event::ZoneChange { map_id, .. } => {
                if *map_id != self.map_id {
                    self.finish();
                    self.bosses.clear();
                    self.map_id = *map_id;
                }
            }
        }
    }

    fn encounter(&mut self, t_ms: u64) -> &mut Encounter {
        self.current.get_or_insert_with(|| {
            let id = self.next_id;
            self.next_id += 1;
            Encounter { id, first_ms: t_ms, ..Default::default() }
        })
    }

    fn is_player(&self, id: EntityId) -> bool {
        self.players.contains(&id) || Some(id) == self.self_id
    }

    /// You, or a party member matched by name.
    fn is_ours(&self, id: EntityId) -> bool {
        Some(id) == self.self_id || (self.players.contains(&id) && self.names.get(&id).is_some_and(|n| self.party.contains(n)))
    }

    /// Whether a hit belongs to your fight, marking what you or your party hit (or what hit you) as part of it.
    fn in_my_fight(&mut self, actor: EntityId, target: EntityId) -> bool {
        if !self.self_in_combat && self.self_id.is_some_and(|me| actor == me || target == me) {
            self.self_in_combat = true;
            // What was counted before you joined in is other people's fighting: start your fight clean.
            if self.opts.only_my_fights {
                self.current = None;
            }
        }
        if !self.self_in_combat || !self.opts.only_my_fights {
            return true;
        }
        let (actor_ours, target_ours) = (self.is_ours(actor), self.is_ours(target));
        if actor_ours {
            self.engaged.insert(target);
        } else if target_ours {
            self.engaged.insert(actor);
        }
        target_ours || self.engaged.contains(&target)
    }

    /// Closes the current encounter once it has been idle long enough. Call with the latest packet time.
    pub fn tick(&mut self, now_ms: u64) {
        let timeout = if self.in_boss_fight() { BOSS_FIGHT_IDLE_MS.max(self.opts.idle_timeout_ms) } else { self.opts.idle_timeout_ms };
        let idle = self.current.as_ref().is_some_and(|e| now_ms.saturating_sub(e.last_ms) > timeout);
        if idle {
            self.finish();
        }
    }

    /// In an instance, fighting a boss that is still alive.
    fn in_boss_fight(&self) -> bool {
        is_instance(self.map_id)
            && self.current.as_ref().is_some_and(|e| e.by_target.keys().any(|id| self.bosses.contains(id)))
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
        let total: u64 = enc.by_actor.values().map(|a| a.damage).sum();
        let mut actors: Vec<ActorStats> = enc
            .by_actor
            .iter()
            .map(|(&id, a)| {
                let mut skills: Vec<SkillStats> = a.skills.values().cloned().collect();
                skills.sort_by_key(|s| std::cmp::Reverse(s.damage));
                ActorStats {
                    id,
                    name: self.name_of(id),
                    is_self: Some(id) == self.self_id,
                    is_player: self.players.contains(&id),
                    damage: a.damage,
                    dps: a.damage as f64 / duration_s,
                    share: if total > 0 { a.damage as f64 / total as f64 } else { 0.0 },
                    hits: a.hits,
                    crits: a.crits,
                    skills,
                    healing: a.healing,
                    hps: a.healing as f64 / duration_s,
                    damage_taken: a.taken,
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
        t.event(&hit_on(0, 2, 800, 999)); // before I fight, everything counts (my id might not be the combat one)
        assert_eq!(t.snapshot().unwrap().total_damage, 999);
        t.event(&hit_on(100, 1, 900, 50)); // I engage 900, and the strangers' encounter is dropped
        t.event(&hit_on(200, 2, 900, 30)); // the stranger helps on my mob: counted
        t.event(&hit_on(300, 2, 800, 999)); // their own mob: not counted
        t.event(&hit_on(400, 3, 1, 7)); // a mob hits me: it joins my fight, as damage taken
        t.event(&hit_on(500, 1, 3, 20));
        let s = t.snapshot().unwrap();
        assert_eq!(s.total_damage, 50 + 30 + 20);
        assert_eq!(s.actors.iter().find(|a| a.is_self).unwrap().damage_taken, 7);
        assert_eq!(s.main_target, "#900");
    }

    #[test]
    fn counts_party_members_fights() {
        let mut t = Tracker::new(TrackerOptions::default());
        t.event(&Event::Identity { t_ms: 0, id: 1, name: "Me".into(), is_self: true });
        t.event(&Event::Identity { t_ms: 0, id: 2, name: "Faelis".into(), is_self: false });
        t.event(&Event::Identity { t_ms: 0, id: 4, name: "Stranger".into(), is_self: false });
        t.event(&Event::Party { t_ms: 0, members: vec!["Me".into(), "Faelis".into()] });
        t.event(&hit_on(0, 900, 1, 0)); // something hits me, so my id is known to fight
        t.event(&hit_on(0, 2, 800, 40)); // party member's mob I never touched
        t.event(&hit_on(100, 4, 700, 999)); // a stranger's mob
        assert_eq!(t.snapshot().unwrap().total_damage, 40);
    }

    #[test]
    fn tracks_healing_and_damage_taken() {
        let heal = |actor, target, amount| Event::Heal { t_ms: 0, actor, target, skill: 18_120_000, amount };
        let mut t = Tracker::new(TrackerOptions::default());
        t.event(&Event::Identity { t_ms: 0, id: 1, name: "Me".into(), is_self: true });
        t.event(&Event::Identity { t_ms: 0, id: 2, name: "Healer".into(), is_self: false });
        t.event(&Event::Identity { t_ms: 0, id: 4, name: "Stranger".into(), is_self: false });
        t.event(&Event::Party { t_ms: 0, members: vec!["Me".into(), "Healer".into()] });
        t.event(&heal(2, 1, 99)); // no fight yet: ignored
        t.event(&hit_on(0, 900, 1, 300)); // the mob hits me
        t.event(&hit_on(1_000, 1, 900, 500));
        t.event(&heal(2, 1, 250)); // party healer heals me
        t.event(&heal(1, 1, 40)); // I heal myself
        t.event(&heal(4, 4, 999)); // a stranger heals themselves
        t.event(&hit_on(1_500, 4, 1, 1)); // PvP: ignored
        let s = t.snapshot().unwrap();
        let get = |id| s.actors.iter().find(|a| a.id == id).unwrap();
        assert_eq!((get(1).damage, get(1).healing, get(1).damage_taken), (500, 40, 300));
        assert_eq!((get(2).damage, get(2).healing), (0, 250));
        assert!(s.actors.iter().all(|a| a.id != 4));
        assert_eq!(s.total_damage, 500);
        assert_eq!(s.main_target, "#900");
    }

    #[test]
    fn restores_identities_and_drops_them_when_stale() {
        let mut old = Tracker::new(TrackerOptions::default());
        old.event(&Event::Identity { t_ms: 0, id: 1, name: "Me".into(), is_self: true });
        old.event(&Event::Identity { t_ms: 0, id: 2, name: "Faelis".into(), is_self: false });
        old.event(&Event::Party { t_ms: 0, members: vec!["Me".into(), "Faelis".into()] });

        let mut t = Tracker::new(TrackerOptions::default());
        t.restore(old.identities());
        assert_eq!(t.self_name(), Some("Me"));
        t.event(&hit_on(0, 1, 900, 50));
        let s = t.snapshot().unwrap();
        assert!(s.actors[0].is_self && s.actors[0].is_player);
        assert_eq!(t.identities(), old.identities());

        // A new own id means the zone changed while the meter was off: restored ids are no longer trusted.
        t.event(&Event::Identity { t_ms: 0, id: 7, name: "Me".into(), is_self: true });
        assert_eq!(t.identities().players, vec![(7, "Me".into())]);
    }

    #[test]
    fn a_self_id_that_never_fights_does_not_hide_your_damage() {
        let mut t = Tracker::new(TrackerOptions::default());
        t.event(&Event::Identity { t_ms: 0, id: 1, name: "Me".into(), is_self: true });
        t.event(&hit_on(0, 5, 900, 50)); // I fight under another id
        t.event(&hit_on(100, 6, 800, 20));
        assert_eq!(t.snapshot().unwrap().total_damage, 70);
    }

    #[test]
    fn monster_hits_are_never_damage_dealt() {
        let mut t = Tracker::new(TrackerOptions::default());
        t.event(&Event::NpcSpawn { t_ms: 0, id: 900, npc_code: 2_000_002, max_hp: None });
        let hit = |actor, target, skill, amount| Event::Damage { t_ms: 0, actor, target, skill, amount, flags: HitFlags::default() };
        t.event(&hit(5, 900, 11_020_001, 100)); // an unnamed player hits the spawned mob
        t.event(&hit(900, 5, 2_000_100, 40)); // the mob hits back
        t.event(&hit(901, 8, 2_000_200, 30)); // an unspawned mob, known by its skill, hits an unnamed summon
        t.event(&hit(900, 901, 2_000_100, 999)); // mobs fighting each other
        let s = t.snapshot().unwrap();
        assert_eq!(s.total_damage, 100);
        assert_eq!(s.actors.iter().filter(|a| a.damage > 0).map(|a| a.id).collect::<Vec<_>>(), vec![5]);
        assert_eq!(s.actors.iter().find(|a| a.id == 5).unwrap().damage_taken, 40);
        assert_eq!(s.main_target, "NPC 2000002");
    }

    /// Loads `map_id`, then spawns a boss (900) and a trash mob (800), as the game does after a loading screen.
    fn boss_tracker(map_id: u32) -> Tracker {
        let mut t = Tracker::new(TrackerOptions::default());
        t.event(&Event::ZoneChange { t_ms: 0, map_id });
        t.npc_namer = Some(Box::new(|code| Some(NpcInfo { name: format!("N{code}"), is_boss: code == 2_400_101 })));
        t.event(&Event::NpcSpawn { t_ms: 0, id: 900, npc_code: 2_400_101, max_hp: None });
        t.event(&Event::NpcSpawn { t_ms: 0, id: 800, npc_code: 2_000_002, max_hp: None });
        t
    }

    #[test]
    fn instance_boss_fights_survive_downtime_until_the_boss_dies() {
        let mut t = boss_tracker(600_123);
        t.event(&hit_on(0, 1, 900, 100));
        t.tick(60_000); // a long phase change
        t.event(&hit_on(60_000, 1, 900, 100));
        assert!(t.snapshot().unwrap().active);
        assert_eq!(t.snapshot().unwrap().total_damage, 200);
        t.event(&Event::Death { t_ms: 61_000, id: 900 });
        assert!(!t.snapshot().unwrap().active);
        assert_eq!(t.history.len(), 1);
    }

    #[test]
    fn trash_and_open_world_keep_the_normal_idle_timeout() {
        let mut t = boss_tracker(600_123);
        t.event(&hit_on(0, 1, 800, 100)); // trash in an instance
        t.tick(9_000);
        assert!(!t.snapshot().unwrap().active);

        let mut t = boss_tracker(100_200); // open world, even against a boss
        t.event(&hit_on(0, 1, 900, 100));
        t.tick(9_000);
        assert!(!t.snapshot().unwrap().active);
    }

    #[test]
    fn leaving_the_zone_ends_the_fight() {
        let mut t = boss_tracker(600_123);
        t.event(&hit_on(0, 1, 900, 100));
        t.event(&Event::ZoneChange { t_ms: 5_000, map_id: 600_123 }); // same map: an in-map teleport
        assert!(t.snapshot().unwrap().active);
        t.event(&Event::ZoneChange { t_ms: 6_000, map_id: 100_200 });
        assert!(!t.snapshot().unwrap().active);
    }

    #[test]
    fn class_skills_mark_players() {
        let mut t = Tracker::new(TrackerOptions::default());
        t.event(&Event::Damage { t_ms: 0, actor: 5, target: 900, skill: 11_020_001, amount: 10, flags: HitFlags::default() });
        t.event(&Event::Damage { t_ms: 0, actor: 6, target: 900, skill: 2_000_100, amount: 10, flags: HitFlags::default() });
        let s = t.snapshot().unwrap();
        assert!(s.actors.iter().find(|a| a.id == 5).unwrap().is_player);
        assert!(s.actors.iter().all(|a| a.id != 6)); // a monster skill: not a player, and not damage dealt
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

        t.npc_namer =
            Some(Box::new(|code| (code == 2_000_002).then(|| NpcInfo { name: "Draconute Ranger".into(), is_boss: false })));
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
