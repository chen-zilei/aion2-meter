//! Opcodes as their two bytes appear on the wire, written `0xAABB` where AA is the first byte.
//!
//! These are community findings for the post-June-2026 client (what Global launched on), not an official spec.
//! Expect them to move when the game patches; `replay --opcodes` helps re-find them.

pub const HEARTBEAT: u16 = 0x0036;
/// Answer to the client's ping every 10 s: `00 00`, u64 echo of the client's clock, u64 server Unix ms.
pub const PONG: u16 = 0x0336;
pub const SELF_INFO: u16 = 0x3336;
pub const PLAYER_INFO: u16 = 0x4536;
pub const SPAWN: u16 = 0x4136;
pub const DEATH: u16 = 0x4236;
pub const MAP_LOAD: u16 = 0x2136;
pub const CAST: u16 = 0x0238;
pub const DAMAGE: u16 = 0x0438;
pub const DOT: u16 = 0x0538;
pub const REMAIN_HP: u16 = 0x008D;
pub const PARTY_ROSTER: u16 = 0x0297;

pub fn name(op: u16) -> String {
    match op {
        HEARTBEAT => "Heartbeat".into(),
        PONG => "Pong".into(),
        SELF_INFO => "SelfInfo".into(),
        PLAYER_INFO => "PlayerInfo".into(),
        SPAWN => "Spawn".into(),
        DEATH => "Death".into(),
        MAP_LOAD => "MapLoad".into(),
        CAST => "Cast".into(),
        DAMAGE => "Damage".into(),
        DOT => "Dot".into(),
        REMAIN_HP => "RemainHp".into(),
        PARTY_ROSTER => "PartyRoster".into(),
        _ => format!("{:02x} {:02x}", op >> 8, op & 0xFF),
    }
}
