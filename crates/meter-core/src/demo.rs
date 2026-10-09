//! Synthetic fights so the UI can be built and shown without the game running.

use crate::parser::{Event, HitFlags};

pub struct Demo {
    t_ms: u64,
    seed: u64,
}

const PARTY: [(u32, &str, u32); 4] = [(101, "You", 18_000), (102, "Faelis", 22_000), (103, "Brannoc", 14_000), (104, "Ilsae", 9_000)];
const SKILLS: [u32; 5] = [11_020_000, 11_030_000, 11_210_000, 11_400_000, 11_700_000];
const BOSS: u32 = 900_001;

impl Demo {
    pub fn new(start_ms: u64) -> Self {
        Self { t_ms: start_ms, seed: 0x9E37_79B9_7F4A_7C15 }
    }

    pub const BOSS_ID: u32 = BOSS;
    pub const BOSS_NAME: &'static str = "Training Dummy";

    /// Party identities to send once at start.
    pub fn intro(&self) -> Vec<Event> {
        PARTY
            .iter()
            .enumerate()
            .map(|(i, (id, name, _))| Event::Identity { t_ms: self.t_ms, id: *id, name: (*name).into(), is_self: i == 0 })
            .collect()
    }

    fn rand(&mut self) -> u64 {
        // xorshift64*: deterministic, no dependency.
        self.seed ^= self.seed >> 12;
        self.seed ^= self.seed << 25;
        self.seed ^= self.seed >> 27;
        self.seed.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Advances `dt_ms` and returns the hits that happened in that time.
    pub fn step(&mut self, dt_ms: u64) -> Vec<Event> {
        self.t_ms += dt_ms;
        let mut out = Vec::new();
        for (id, _, base) in PARTY {
            if !self.rand().is_multiple_of(3) {
                continue;
            }
            let crit = self.rand().is_multiple_of(4);
            let spread = (self.rand() % 40) as u32 + 80;
            let amount = base * spread / 100 * if crit { 2 } else { 1 };
            let skill = SKILLS[(self.rand() % SKILLS.len() as u64) as usize];
            out.push(Event::Damage {
                t_ms: self.t_ms,
                actor: id,
                target: BOSS,
                skill,
                amount,
                flags: HitFlags { crit, back: self.rand().is_multiple_of(5), ..Default::default() },
            });
        }
        out
    }

    /// A plausible ping that drifts a little each second, for the overlay's ping readout.
    pub fn ping_ms(&self) -> u64 {
        38 + (self.t_ms / 1000).wrapping_mul(7) % 13
    }

    pub fn now_ms(&self) -> u64 {
        self.t_ms
    }
}
