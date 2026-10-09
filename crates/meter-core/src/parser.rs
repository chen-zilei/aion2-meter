//! Game packets → combat events.
//!
//! Layouts are community findings (see docs/PROTOCOL.md) and deliberately strict: a record that does not
//! validate is dropped and counted, because a misread varint shows up as a phantom million-damage hit.

use crate::{opcodes as op, wire};
use serde::Serialize;
use std::collections::BTreeMap;

pub type EntityId = u32;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct HitFlags {
    pub crit: bool,
    pub back: bool,
    pub parry: bool,
    pub perfect: bool,
    pub double: bool,
    pub dot: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Event {
    Damage { t_ms: u64, actor: EntityId, target: EntityId, skill: u32, amount: u32, flags: HitFlags },
    Heal { t_ms: u64, actor: EntityId, target: EntityId, skill: u32, amount: u32 },
    Identity { t_ms: u64, id: EntityId, name: String, is_self: bool },
    Death { t_ms: u64, id: EntityId },
    /// A monster or other NPC appeared. `npc_code` is its template id, the key into the NPC name tables.
    NpcSpawn { t_ms: u64, id: EntityId, npc_code: u32, max_hp: Option<u64> },
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct ParserStats {
    pub heartbeats: u64,
    pub damage_records: u64,
    pub damage_rejected: u64,
    /// Packet count per opcode, for reverse-engineering (`replay --opcodes`).
    pub opcode_counts: BTreeMap<u16, u64>,
}

#[derive(Default)]
pub struct Parser {
    pub stats: ParserStats,
}

fn is_entity(id: u32) -> bool {
    (1..=9_999_999).contains(&id)
}

fn is_skill(id: u32) -> bool {
    (1..=299_999_999).contains(&id)
}

impl Parser {
    pub fn packet(&mut self, t_ms: u64, opcode: u16, body: &[u8], out: &mut dyn FnMut(Event)) {
        *self.stats.opcode_counts.entry(opcode).or_default() += 1;
        match opcode {
            op::HEARTBEAT => self.stats.heartbeats += 1,
            op::DAMAGE => self.damage(t_ms, body, out),
            op::DOT => dot(t_ms, body, out),
            op::SELF_INFO | op::PLAYER_INFO => {
                identity(t_ms, body, opcode == op::SELF_INFO, out);
                scan_embedded_bundles(t_ms, body, out);
            }
            op::DEATH => death(t_ms, body, out),
            op::SPAWN => spawn(t_ms, body, out),
            // Identity records also ride inside other packets, mid-body and inside LZ4 bundles embedded in a larger
            // packet. The game re-sends names that way (the own record every few minutes), so without this a player
            // already in view when the meter starts stays `#id` until they leave and come back.
            _ if body.len() >= 16 => {
                scan_identities(t_ms, body, out);
                scan_embedded_bundles(t_ms, body, out);
            }
            _ => {}
        }
    }

    /// `04 38`: one record, optionally followed by more records each prefixed with `01 00`.
    fn damage(&mut self, t_ms: u64, b: &[u8], out: &mut dyn FnMut(Event)) {
        let mut o = 0;
        let mut first = true;
        while o < b.len() {
            if !first {
                if b.get(o..o + 2) != Some(&[0x01, 0x00]) {
                    break;
                }
                o += 2;
            }
            match damage_record(t_ms, b, &mut o) {
                Some(Record::Event(ev)) => {
                    self.stats.damage_records += 1;
                    out(ev);
                }
                Some(Record::CastMarker) => return,
                None => {
                    if first {
                        self.stats.damage_rejected += 1;
                    }
                    break;
                }
            }
            first = false;
        }
    }
}

enum Record {
    Event(Event),
    /// Layout 0 records announce a cast and carry no damage.
    CastMarker,
}

/// ```text
/// target varint, switch varint (layout = switch & 0x0F in 4..=7, 0x20 = extra hits follow), flag varint,
/// actor varint, skill u32, hit uid u8, dmg type varint (3 = crit), layout block, [0 pad] scalar varint, damage varint
/// ```
/// The tail after the damage (extra hit list etc.) is not decoded yet, which is why only cleanly-ending chains are
/// followed. Improving this is a good first reverse-engineering task.
fn damage_record(t_ms: u64, b: &[u8], o: &mut usize) -> Option<Record> {
    let target = wire::varint(b, o).filter(|&v| is_entity(v))?;
    let sw = wire::varint(b, o)?;
    let layout = sw & 0x0F;
    if layout == 0 {
        return Some(Record::CastMarker);
    }
    if !(4..=7).contains(&layout) {
        return None;
    }
    wire::varint(b, o)?; // flag
    let actor = wire::varint(b, o).filter(|&v| is_entity(v))?;
    let skill = wire::u32(b, o).filter(|&v| is_skill(v))?;
    wire::u8(b, o)?; // per-hit uid
    let dmg_type = wire::varint(b, o)?;

    let mut flags = HitFlags { crit: dmg_type == 3, ..Default::default() };
    let block_len = if layout == 4 { 8 } else { 11 };
    if layout >= 5 {
        let mods = *b.get(*o)?;
        let dir = *b.get(*o + 2)?;
        flags.parry = mods & 0x02 != 0;
        flags.perfect = mods & 0x04 != 0;
        flags.double = mods & 0x08 != 0;
        flags.back = dir == 0x01;
    }
    *o += block_len;

    let mut v1 = wire::varint(b, o)?;
    if v1 == 0 {
        v1 = wire::varint(b, o)?; // zero pad
    }
    let after_v1 = *o;
    let v2 = wire::varint(b, o)?;
    // Usually v1 is a power scalar and v2 the damage. Some layout-6 crits carry the damage first and no scalar.
    let amount = if layout == 6 && flags.crit && (1_000..=5_000_000).contains(&v1) && v2 <= 25 {
        *o = after_v1;
        v1
    } else {
        v2
    };
    if !(1..=99_999_999).contains(&amount) {
        return None;
    }

    // Consume the rest of the record up to the next chain marker or the end.
    while *o < b.len() && b.get(*o..*o + 2) != Some(&[0x01, 0x00]) {
        *o += 1;
    }

    Some(Record::Event(if actor == target {
        Event::Heal { t_ms, actor, target, skill, amount }
    } else {
        Event::Damage { t_ms, actor, target, skill, amount, flags }
    }))
}

/// `05 38`: `target varint, effect u8, actor varint, varint, skill u32 (x100), amount varint`.
fn dot(t_ms: u64, b: &[u8], out: &mut dyn FnMut(Event)) {
    let mut o = 0;
    let parse = |o: &mut usize| -> Option<Event> {
        let target = wire::varint(b, o).filter(|&v| is_entity(v))?;
        let effect = wire::u8(b, o)?;
        let actor = wire::varint(b, o).filter(|&v| is_entity(v))?;
        wire::varint(b, o)?;
        let skill = wire::u32(b, o)? / 100;
        let amount = wire::varint(b, o).filter(|&v| (1..=99_999_999).contains(&v))?;
        if !is_skill(skill) {
            return None;
        }
        match effect {
            0x02 | 0x0A if actor != target => Some(Event::Damage {
                t_ms,
                actor,
                target,
                skill,
                amount,
                flags: HitFlags { dot: true, ..Default::default() },
            }),
            0x01 | 0x09 | 0x0B => Some(Event::Heal { t_ms, actor, target, skill, amount }),
            _ => None,
        }
    };
    if let Some(ev) = parse(&mut o) {
        out(ev);
    }
}

/// `33 36` / `45 36`: `id varint, u32 mask, u8 flags (bit 0 = has name), u8 len, utf8 name, ...`.
fn identity(t_ms: u64, b: &[u8], is_self: bool, out: &mut dyn FnMut(Event)) {
    let mut o = 0;
    let Some(id) = wire::varint(b, &mut o).filter(|&v| is_entity(v)) else { return };
    o += 4;
    let (Some(&mask), Some(&len)) = (b.get(o), b.get(o + 1)) else { return };
    o += 2;
    if mask & 0x01 == 0 {
        return;
    }
    let Some(name) = b.get(o..o + len as usize).and_then(character_name) else { return };
    out(Event::Identity { t_ms, id, name: name.to_owned(), is_self });
}

/// A character name: up to 16 letters and digits (any script), at least one letter. This strictness is what makes it
/// safe to probe for identity records at guessed offsets. Tutorial characters are `$` + random characters, so they
/// never match.
fn character_name(raw: &[u8]) -> Option<&str> {
    let name = std::str::from_utf8(raw).ok().filter(|_| (1..=36).contains(&raw.len()))?;
    let ok = name.chars().count() <= 16
        && name.chars().all(char::is_alphanumeric)
        && name.chars().any(char::is_alphabetic);
    ok.then_some(name)
}

/// Identity records embedded anywhere in `b`, found by their opcode bytes.
fn scan_identities(t_ms: u64, b: &[u8], out: &mut dyn FnMut(Event)) {
    for i in 0..b.len().saturating_sub(10) {
        if b[i + 1] == 0x36 && matches!(b[i], 0x33 | 0x45) {
            identity(t_ms, &b[i + 2..], b[i] == 0x33, out);
        }
    }
}

const MAX_EMBEDDED_BUNDLE: usize = 8 * 1024 * 1024;

/// LZ4 bundles embedded in a packet body (`varint len, FF FF, u32 raw_size, lz4 block`), scanned for identity records.
fn scan_embedded_bundles(t_ms: u64, b: &[u8], out: &mut dyn FnMut(Event)) {
    let mut i = 1;
    while i + 8 < b.len() {
        if b[i] == 0xFF && b[i + 1] == 0xFF {
            if let Some(end) = (1..=3).rev().find_map(|n| embedded_bundle(t_ms, b, i, n, out)) {
                i = end;
                continue;
            }
        }
        i += 1;
    }
}

/// The bundle whose `FF FF` is at `i` and whose length varint is the `n` bytes before it; returns where it ends.
fn embedded_bundle(t_ms: u64, b: &[u8], i: usize, n: usize, out: &mut dyn FnMut(Event)) -> Option<usize> {
    let at = i.checked_sub(n)?;
    let (len, vlen) = wire::peek_varint(b, at).filter(|&(_, vlen)| vlen == n)?;
    let end = (at + len as usize + vlen).checked_sub(4).filter(|&e| len >= 12 && e <= b.len() && e > i + 6)?;
    let mut o = i + 2;
    let raw = wire::u32(b, &mut o).map(|r| r as usize).filter(|&r| r > 0 && r <= MAX_EMBEDDED_BUNDLE)?;
    let inner = lz4_flex::block::decompress(&b[i + 6..end], raw).ok()?;
    scan_identities(t_ms, &inner, out);
    Some(end)
}

/// `42 36`: `id varint, varint, flag varint (1 or 3 = dead)`.
fn death(t_ms: u64, b: &[u8], out: &mut dyn FnMut(Event)) {
    let mut o = 0;
    let Some(id) = wire::varint(b, &mut o).filter(|&v| is_entity(v)) else { return };
    if wire::varint(b, &mut o).is_none() {
        return;
    }
    if let Some(1 | 3) = wire::varint(b, &mut o) {
        out(Event::Death { t_ms, id });
    }
}

/// Spawn kinds (first byte of the mask) for summons, spirits, pets and lingering skill effects. They carry template
/// codes too but are not monsters, so they are left for owner attribution later.
const SUMMON_KINDS: [u8; 5] = [0x5F, 0x1C, 0x1F, 0x1D, 0x5D];

/// `41 36`: `id varint, u32 mask (first byte = kind), ..., npc_code u24, marker 00 (00|40) 02, x y z f32, ...,
/// 01 cur_hp varint max_hp varint, ...`.
///
/// Fields between the mask and the marker vary in length, so the marker is searched for. Spawn ids above 1,000,000
/// are folded into the id space damage records use.
fn spawn(t_ms: u64, b: &[u8], out: &mut dyn FnMut(Event)) {
    let mut o = 0;
    let Some(raw_id) = wire::varint(b, &mut o).filter(|&v| v != 0) else { return };
    let id = if raw_id > 1_000_000 { (raw_id & 0x3FFF) | 0x4000 } else { raw_id };
    let mask_start = o;
    let Some(&kind) = b.get(mask_start) else { return };
    if SUMMON_KINDS.contains(&kind) {
        return;
    }

    let end = b.len().saturating_sub(2).min(mask_start + 60);
    let Some(marker) = (mask_start + 3..end).find(|&i| b[i] == 0x00 && matches!(b[i + 1], 0x00 | 0x40) && b[i + 2] == 0x02)
    else {
        return;
    };
    let npc_code = b[marker - 3] as u32 | (b[marker - 2] as u32) << 8 | (b[marker - 1] as u32) << 16;
    if npc_code == 0 {
        return;
    }

    // HP: `01 cur max` somewhere in the 64 bytes after the marker, with 0 < cur <= max.
    let hp_start = marker + 3;
    let hp_end = b.len().saturating_sub(2).min(hp_start + 64);
    let max_hp = (hp_start..hp_end).filter(|&h| b[h] == 0x01).find_map(|h| {
        let mut p = h + 1;
        let cur = wire::varint(b, &mut p).filter(|&c| c > 0)?;
        let max = wire::varint(b, &mut p).filter(|&m| m >= cur)?;
        Some(max as u64)
    });

    out(Event::NpcSpawn { t_ms, id, npc_code, max_hp });
}

/// Builds a `04 38` body the way the server lays it out. Used by tests and the demo source.
pub fn encode_damage(target: u32, actor: u32, skill: u32, amount: u32, crit: bool) -> Vec<u8> {
    use crate::frame::encode_varint as v;
    let mut b = v(target);
    b.extend(v(0x06)); // layout 6
    b.extend(v(0)); // flag
    b.extend(v(actor));
    b.extend(skill.to_le_bytes());
    b.push(0x01); // hit uid
    b.extend(v(if crit { 3 } else { 1 }));
    b.extend([0u8; 11]); // layout block: no modifiers, front
    b.extend(v(12_345)); // power scalar
    b.extend(v(amount));
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(opcode: u16, body: &[u8]) -> Vec<Event> {
        let mut p = Parser::default();
        let mut evs = Vec::new();
        p.packet(0, opcode, body, &mut |e| evs.push(e));
        evs
    }

    #[test]
    fn decodes_a_damage_record() {
        let evs = parse(op::DAMAGE, &encode_damage(5001, 42, 11_020_000, 31_337, true));
        assert_eq!(evs.len(), 1);
        match &evs[0] {
            Event::Damage { actor, target, skill, amount, flags, .. } => {
                assert_eq!((*actor, *target, *skill, *amount), (42, 5001, 11_020_000, 31_337));
                assert!(flags.crit);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn follows_chained_records() {
        let mut body = encode_damage(5001, 42, 11_020_000, 100, false);
        body.extend([0x01, 0x00]);
        body.extend(encode_damage(5002, 42, 11_020_000, 200, false));
        assert_eq!(parse(op::DAMAGE, &body).len(), 2);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(op::DAMAGE, &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]).is_empty());
    }

    fn spawn_body(raw_id: u32, kind: u8, npc_code: u32) -> Vec<u8> {
        let mut b = crate::frame::encode_varint(raw_id);
        b.extend([kind, 0x00, 0x00, 0x00]); // mask
        b.extend([0x00, 0x07, 0x00]); // variable fields before the code
        b.extend(&npc_code.to_le_bytes()[..3]);
        b.extend([0x00, 0x40, 0x02]); // marker
        b.extend([0u8; 12]); // x y z
        b.extend([0x05, 0x01]);
        b.extend(crate::frame::encode_varint(48_000)); // current hp
        b.extend(crate::frame::encode_varint(50_000)); // max hp
        b
    }

    #[test]
    fn decodes_npc_spawn() {
        assert_eq!(
            parse(op::SPAWN, &spawn_body(5001, 0x0C, 2_000_002)),
            vec![Event::NpcSpawn { t_ms: 0, id: 5001, npc_code: 2_000_002, max_hp: Some(50_000) }]
        );
    }

    #[test]
    fn folds_large_spawn_ids_and_skips_summons() {
        match &parse(op::SPAWN, &spawn_body(3_000_123, 0x0C, 2_000_002))[..] {
            [Event::NpcSpawn { id, .. }] => assert_eq!(*id, (3_000_123 & 0x3FFF) | 0x4000),
            other => panic!("unexpected {other:?}"),
        }
        assert!(parse(op::SPAWN, &spawn_body(5001, 0x5F, 2_000_002)).is_empty());
    }

    fn identity_body(id: u32, name: &str) -> Vec<u8> {
        let mut b = crate::frame::encode_varint(id);
        b.extend([0, 0, 0, 0, 0x01, name.len() as u8]);
        b.extend(name.as_bytes());
        b.extend([0xE8, 0x03, 0x18, 0, 0, 0]); // server, class
        b
    }

    #[test]
    fn finds_identities_inside_other_packets() {
        let mut body = vec![0x07; 9];
        body.extend([0x45, 0x36]);
        body.extend(identity_body(77, "Faelis"));
        body.extend([0x00; 8]);
        assert_eq!(parse(0x1234, &body), vec![Event::Identity { t_ms: 0, id: 77, name: "Faelis".into(), is_self: false }]);
    }

    #[test]
    fn finds_identities_inside_embedded_bundles() {
        let mut inner = vec![0x01; 5];
        inner.extend([0x33, 0x36]);
        inner.extend(identity_body(42, "Zilei"));
        let block = lz4_flex::block::compress(&inner);
        let mut bundle = vec![0xFF, 0xFF];
        bundle.extend((inner.len() as u32).to_le_bytes());
        bundle.extend(&block);
        let mut body = vec![0x09; 6];
        body.extend(crate::frame::encode_varint(bundle.len() as u32 + 4));
        body.extend(bundle);
        body.extend([0x00; 4]);
        // LZ4 keeps short inputs as literals, so scan the bundle path on its own to prove it decompresses.
        let mut found = Vec::new();
        scan_embedded_bundles(0, &body, &mut |e| found.push(e));
        assert_eq!(found, vec![Event::Identity { t_ms: 0, id: 42, name: "Zilei".into(), is_self: true }]);
    }

    #[test]
    fn rejects_placeholder_and_junk_names() {
        assert!(parse(op::PLAYER_INFO, &identity_body(77, "$x9a2")).is_empty());
        assert!(parse(op::PLAYER_INFO, &identity_body(77, "a b")).is_empty());
        assert!(parse(op::PLAYER_INFO, &identity_body(77, "1234")).is_empty());
        assert_eq!(parse(op::PLAYER_INFO, &identity_body(77, "엘리시온")).len(), 1);
    }

    #[test]
    fn decodes_identity() {
        let mut body = crate::frame::encode_varint(42);
        body.extend([0, 0, 0, 0, 0x01, 5]);
        body.extend(b"Zilei");
        assert_eq!(
            parse(op::SELF_INFO, &body),
            vec![Event::Identity { t_ms: 0, id: 42, name: "Zilei".into(), is_self: true }]
        );
    }
}
