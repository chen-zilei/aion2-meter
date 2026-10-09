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
            op::SELF_INFO => identity(t_ms, body, true, out),
            op::PLAYER_INFO => identity(t_ms, body, false, out),
            op::DEATH => death(t_ms, body, out),
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
    if mask & 0x01 == 0 || !(1..=36).contains(&len) {
        return;
    }
    let Some(raw) = b.get(o..o + len as usize) else { return };
    let Ok(name) = std::str::from_utf8(raw) else { return };
    if name.chars().any(char::is_control) {
        return;
    }
    out(Event::Identity { t_ms, id, name: name.to_owned(), is_self });
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
