//! Ping to the game server, measured passively from the game's own messages.
//!
//! TCP acknowledgements can't be used: the game connects through a nearby Cloudflare relay, which acknowledges
//! segments itself, so they only time the hop to the relay (~2 ms). Instead:
//!
//! - Every 10 s the client sends a ping (encrypted; its size changes between logins) and the server answers with a
//!   pong: `00 00`, u64 echo of the client's send time on the game's own clock, u64 server Unix ms stamped on
//!   receipt. `server stamp − send time` is the uplink plus the PC↔server clock offset (`uplink` below).
//! - About 20 times a second the server sends a heartbeat (`00 36`) holding its Unix ms when created.
//!   `arrival − value` is the downlink, minus the same clock offset, plus however long the server held it before
//!   sending. The lowest of the last few seconds is the heartbeat that went out right away.
//!
//! The sum cancels the clock offset and leaves the round trip, refreshed with every heartbeat. It is capped by the
//! latest raw ping/pong round trip, which is shown on its own until a heartbeat arrives.
//!
//! The game's clock is monotonic, so `capture time − echo` (`clock` below) shifts whenever Windows corrects the PC
//! clock. It is never assumed: each pong is matched to the outgoing frame it echoes. The ping goes out every 10 s
//! on the game's clock, so the right frame gives the same `clock` from one pong to the next (to the millisecond),
//! and among frames that do, the ping is the rarest size. After that each pong only has to land on a frame of that
//! size near the known value, which also follows slow drift; a clock step loses the match and it is learnt again.
//!
//! The pong is recognised by its shape rather than its opcode, so a renumbering patch doesn't break it.
//! Nothing is sent: these are messages the game exchanges anyway.

use crate::opcodes as op;
use std::collections::{HashMap, HashSet, VecDeque};

/// How far back the heartbeat minimum looks.
const WINDOW_MS: u64 = 3_000;
/// A pong this long after its ping is not its answer.
const MAX_PONG_WAIT_MS: u64 = 3_000;
/// The uplink measurement is trusted for this long (pings come every 10 s).
const UPLINK_VALID_MS: u64 = 35_000;
/// A clock offset beyond this means we misread a field.
const MAX_OFFSET_MS: i64 = 3_600_000;
/// Two `clock` values within this are the same clock (capture times are whole milliseconds).
const CLOCK_TOLERANCE_MS: i64 = 2;
/// Longest the server holds a packet before sending it (its send tick has been ~50–110 ms).
const MAX_SEND_WAIT_MS: i64 = 250;
/// Pong body: `00 00`, u64 echo, u64 server ms.
const PONG_LEN: usize = 18;

/// Counters for diagnosing the ping readout (`replay --ping`).
#[derive(Debug, Default, Clone)]
pub struct LatencyStats {
    pub pongs: u64,
    pub pongs_matched: u64,
    pub heartbeats: u64,
    /// Size of the client frame identified as the ping.
    pub ping_frame_size: Option<usize>,
    /// Capture time minus the game's clock, in ms.
    pub clock_ms: Option<i64>,
}

#[derive(Default)]
pub struct Latency {
    pub stats: LatencyStats,
    /// (time, size) of the client's frames in the last few seconds.
    sent: VecDeque<(u64, usize)>,
    /// How many client frames of each size have been seen.
    sizes: HashMap<usize, u64>,
    /// (capture time − echo, size) for each frame before the previous unmatched pong.
    candidates: Vec<(i64, usize)>,
    /// Capture time minus the game's clock, once known.
    clock: Option<i64>,
    /// Latest server ms from a heartbeat, to recognise pongs by their stamp.
    server_ms: Option<i64>,
    /// Opcodes seen with a body that can't be a pong.
    not_pong: HashSet<u16>,
    /// (time, server stamp − client send) from the latest ping/pong.
    uplink: Option<(u64, i64)>,
    /// Raw round trip of the latest ping/pong.
    raw: Option<(u64, u64)>,
    /// (arrival, arrival − heartbeat value) of recent heartbeats, oldest first.
    downlink: VecDeque<(u64, i64)>,
}

fn u64_at(body: &[u8], at: usize) -> Option<i64> {
    Some(u64::from_le_bytes(body.get(at..at + 8)?.try_into().ok()?) as i64)
}

impl Latency {
    /// A complete frame from the client, `size` bytes long.
    pub fn client_frame(&mut self, t_ms: u64, size: usize) {
        *self.sizes.entry(size).or_default() += 1;
        self.sent.push_back((t_ms, size));
        while self.sent.front().is_some_and(|&(t, _)| t + MAX_PONG_WAIT_MS < t_ms) {
            self.sent.pop_front();
        }
    }

    /// Any decoded server packet: heartbeats and pongs are picked out. Besides its usual opcode, the pong is known
    /// by its shape (length, `00 00`, a server stamp near the heartbeat's) from any opcode that has never carried
    /// anything else.
    pub fn server_packet(&mut self, t_ms: u64, opcode: u16, body: &[u8]) {
        if opcode == op::HEARTBEAT {
            self.heartbeat(t_ms, body);
            return;
        }
        if body.len() != PONG_LEN || body[..2] != [0, 0] {
            self.not_pong.insert(opcode);
            return;
        }
        let stamp_ok = matches!((u64_at(body, 10), self.server_ms), (Some(stamp), Some(hb)) if (stamp - hb).abs() < 60_000);
        if opcode == op::PONG || (stamp_ok && !self.not_pong.contains(&opcode)) {
            self.pong(t_ms, body);
        }
    }

    /// A server pong answering the client's ping.
    fn pong(&mut self, t_ms: u64, body: &[u8]) {
        self.stats.pongs += 1;
        let (Some(echo), Some(stamp)) = (u64_at(body, 2), u64_at(body, 10)) else { return };
        let now: Vec<(i64, usize)> =
            self.sent.iter().filter(|&&(t, _)| t <= t_ms).map(|&(t, size)| (t as i64 - echo, size)).collect();
        let near = |c: i64, k: i64| (c - k).abs() <= CLOCK_TOLERANCE_MS;

        // Known clock: the ping-sized frame that lands on it (closest, to follow drift). Checking the size keeps
        // a clock step that happens to be a multiple of the heartbeat interval from matching a heartbeat.
        let ping_size = self.stats.ping_frame_size;
        let mut clock = self.clock.and_then(|k| {
            now.iter()
                .filter(|&&(c, size)| Some(size) == ping_size && near(c, k))
                .min_by_key(|&&(c, _)| (c - k).abs())
                .map(|&(c, _)| c)
        });
        if clock.is_none() {
            // Learn it (again, after a clock step): frames whose `clock` repeats from the previous pong, the rarest size first.
            let repeated = now.iter().filter(|&&(c, size)| self.candidates.iter().any(|&(p, s)| s == size && near(c, p)));
            if let Some(&(c, size)) = repeated.min_by_key(|&&(_, size)| self.sizes.get(&size).copied().unwrap_or(0)) {
                clock = Some(c);
                self.stats.ping_frame_size = Some(size);
            }
        }
        let Some(clock) = clock else {
            self.candidates = now;
            return;
        };
        self.clock = Some(clock);
        self.candidates.clear();
        self.stats.clock_ms = Some(clock);

        let send = echo + clock;
        let uplink = stamp - send;
        if uplink.abs() > MAX_OFFSET_MS {
            return;
        }
        self.stats.pongs_matched += 1;
        self.uplink = Some((t_ms, uplink));
        self.raw = Some((t_ms, (t_ms as i64 - send) as u64));
    }

    /// Server heartbeat `00 36`.
    fn heartbeat(&mut self, t_ms: u64, body: &[u8]) {
        self.stats.heartbeats += 1;
        let Some(value) = u64_at(body, 0) else { return };
        self.server_ms = Some(value);
        let down = t_ms as i64 - value;
        if down.abs() > MAX_OFFSET_MS {
            return;
        }
        self.downlink.push_back((t_ms, down));
        while self.downlink.front().is_some_and(|&(t, _)| t + WINDOW_MS < t_ms) {
            self.downlink.pop_front();
        }
    }

    /// Current ping in milliseconds, or `None` without a recent reading.
    pub fn ping_ms(&self, now_ms: u64) -> Option<u64> {
        let (pong_t, uplink) = self.uplink?;
        if pong_t + UPLINK_VALID_MS < now_ms {
            return None;
        }
        let down = self.downlink.iter().filter(|&&(t, _)| t + WINDOW_MS >= now_ms).map(|&(_, d)| d).min();
        // The raw ping/pong round trip includes any wait for the server's send tick, so it is an upper bound; the
        // heartbeat estimate can sit a few ms above it when no heartbeat leaves the instant it is stamped.
        // Far below it, the PC clock was stepped since the last pong and the halves no longer share a clock.
        let raw = self.raw.map(|(_, rtt)| rtt);
        match (down, raw) {
            (Some(d), Some(raw)) => {
                let estimate = uplink + d;
                Some(if estimate < raw as i64 - MAX_SEND_WAIT_MS { raw } else { (estimate.max(0) as u64).min(raw) })
            }
            (Some(d), None) => Some((uplink + d).max(0) as u64),
            (None, raw) => raw,
        }
    }

    /// Time of the latest reading, to pick the live connection when there are several.
    pub fn last_sample_ms(&self) -> Option<u64> {
        self.downlink.back().map(|&(t, _)| t).max(self.uplink.map(|(t, _)| t))
    }
}

/// Splits the client's outgoing stream into frame sizes. Bodies are encrypted, so only the length prefix
/// (same varint as the server's: frame size = len + varint bytes − 4) is read.
#[derive(Default)]
pub struct ClientFrames {
    buf: Vec<u8>,
    /// The next byte is a frame boundary. Lost after a gap or a bad length; the client writes whole messages, so
    /// the start of the next chunk is taken as a boundary again.
    aligned: bool,
}

impl ClientFrames {
    pub fn reset(&mut self) {
        self.buf.clear();
        self.aligned = false;
    }

    pub fn feed(&mut self, data: &[u8], on_frame: &mut dyn FnMut(usize)) {
        if !self.aligned {
            self.buf.clear();
            self.aligned = true;
        }
        self.buf.extend_from_slice(data);
        let mut pos = 0;
        while pos < self.buf.len() {
            let span = &self.buf[pos..];
            if span[0] == 0x00 {
                pos += 1;
                continue;
            }
            let Some((len, vlen)) = crate::wire::peek_varint(span, 0) else {
                if span.len() >= 5 {
                    self.reset();
                    return;
                }
                break;
            };
            let size = (len as usize + vlen).saturating_sub(4);
            if len < 6 || size > crate::frame::MAX_FRAME {
                self.reset();
                return;
            }
            if size > span.len() {
                break;
            }
            on_frame(size);
            pos += size;
        }
        self.buf.drain(..pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 1_791_515_750_000;
    const PING: usize = 12;
    const HEARTBEAT: usize = 11;

    fn pong_body(echo: i64, stamp: u64) -> Vec<u8> {
        let mut b = vec![0, 0];
        b.extend((echo as u64).to_le_bytes());
        b.extend(stamp.to_le_bytes());
        b
    }

    /// A client whose game clock is `pc − clock`, sending heartbeats every 50 ms on the same grid as its ping
    /// (the hardest case: they repeat from one ping to the next too), and a server 850 ms ahead.
    struct Sim {
        l: Latency,
        clock: i64,
    }

    impl Sim {
        fn new(clock: i64) -> Self {
            Self { l: Latency::default(), clock }
        }

        /// Ten seconds ending in a ping at `ping`, answered `rtt` later (stamped halfway).
        fn round(&mut self, ping: u64, rtt: u64) -> Option<u64> {
            for i in (1..200).rev() {
                self.l.client_frame(ping - i * 50, HEARTBEAT);
            }
            self.l.client_frame(ping, PING);
            self.l.client_frame(ping + 50, HEARTBEAT);
            let echo = ping as i64 - self.clock;
            self.l.server_packet(ping + rtt, op::PONG, &pong_body(echo, ping + rtt / 2 + 850));
            self.l.ping_ms(ping + rtt)
        }
    }

    #[test]
    fn learns_the_games_clock_from_two_pings() {
        let mut sim = Sim::new(1_774_633_750_910);
        assert_eq!(sim.round(T0, 117), None);
        assert_eq!(sim.round(T0 + 10_000, 118), Some(118));
        assert_eq!(sim.round(T0 + 20_000, 116), Some(116));
        assert_eq!(sim.l.stats.ping_frame_size, Some(PING));
        assert_eq!(sim.l.stats.clock_ms, Some(1_774_633_750_910));
    }

    #[test]
    fn relearns_after_windows_steps_the_clock() {
        let mut sim = Sim::new(1_774_633_750_910);
        sim.round(T0, 117);
        assert_eq!(sim.round(T0 + 10_000, 117), Some(117));
        // Time sync moves the PC clock forward; the game's clock doesn't follow. A step of whole heartbeat
        // intervals lines a heartbeat up with the old clock, which must not be taken for the ping.
        for (i, step) in [1_313u64, 1_300].into_iter().enumerate() {
            let t = T0 + 20_000 + i as u64 * 20_000;
            sim.clock += step as i64;
            let after_step = sim.round(t + step, 118);
            assert!(after_step.is_none_or(|ms| ms < 1_000), "{after_step:?}"); // never the shifted reading
            assert_eq!(sim.round(t + step + 10_000, 119), Some(119));
        }
    }

    #[test]
    fn follows_slow_drift() {
        let mut sim = Sim::new(5_000);
        sim.round(T0, 100);
        for i in 1..10u64 {
            sim.clock += 1; // 1 ms per ping
            assert_eq!(sim.round(T0 + i * 10_000 + i, 100), Some(100));
        }
    }

    /// Server clock 850 ms ahead of ours, 60 ms each way, heartbeats held 0–100 ms before sending.
    #[test]
    fn adds_uplink_and_lowest_downlink_cancelling_the_clock_offset() {
        let mut sim = Sim::new(1_000);
        sim.round(T0, 160);
        // Ping at T1; the server stamps it on receipt (T1+60 ours = T1+910 its) and answers 100 ms later.
        let t1 = T0 + 10_000;
        for i in (1..200).rev() {
            sim.l.client_frame(t1 - i * 50, HEARTBEAT);
        }
        sim.l.client_frame(t1, PING);
        sim.l.server_packet(t1 + 160, op::PONG, &pong_body(t1 as i64 - 1_000, t1 + 910));
        assert_eq!(sim.l.ping_ms(t1 + 160), Some(160)); // only the raw round trip so far
        for (made, held) in [(t1 + 950, 90), (t1 + 1_000, 40), (t1 + 1_050, 0), (t1 + 1_100, 70)] {
            // Made at `made` server time, sent `held` later, arrives 60 ms after that, in our clock.
            sim.l.server_packet(made - 850 + held + 60, op::HEARTBEAT, &made.to_le_bytes());
        }
        assert_eq!(sim.l.ping_ms(t1 + 400), Some(120));
        assert_eq!(sim.l.ping_ms(t1 + 160 + UPLINK_VALID_MS + 1), None);
    }

    #[test]
    fn never_reads_above_the_raw_round_trip() {
        let mut sim = Sim::new(1_000);
        sim.round(T0, 118);
        let t1 = T0 + 10_000;
        assert_eq!(sim.round(t1, 118), Some(118));
        // Every heartbeat waited 6 ms before leaving, so the estimate alone would say 124.
        sim.l.server_packet(t1 + 200, op::HEARTBEAT, &(t1 + 200 + 850 - 59 - 6).to_le_bytes());
        assert_eq!(sim.l.ping_ms(t1 + 200), Some(118));
    }

    #[test]
    fn recognises_pongs_by_shape_under_any_opcode() {
        let mut sim = Sim::new(1_000);
        let moved = 0x0337; // a patch renumbered the pong
        let pong = |sim: &mut Sim, ping: u64| {
            for i in (1..60).rev() {
                sim.l.client_frame(ping - i * 50, HEARTBEAT);
            }
            sim.l.client_frame(ping, PING);
            sim.l.server_packet(ping + 100, op::HEARTBEAT, &(ping + 900).to_le_bytes());
            sim.l.server_packet(ping + 117, moved, &pong_body(ping as i64 - 1_000, ping + 910));
            sim.l.ping_ms(ping + 117)
        };
        pong(&mut sim, T0);
        assert!(pong(&mut sim, T0 + 10_000).is_some());
        assert_eq!(sim.l.stats.pongs_matched, 1);

        // An opcode that also carries other bodies is never taken for a pong, even when one fits the shape.
        let mut l = Latency::default();
        l.server_packet(T0, op::HEARTBEAT, &(T0 + 850).to_le_bytes());
        l.server_packet(T0, 0x2b38, &[1; 30]);
        l.server_packet(T0, 0x2b38, &pong_body(5, T0 + 850));
        assert_eq!(l.stats.pongs, 0);
    }

    #[test]
    fn ignores_garbage_pongs() {
        let mut l = Latency::default();
        l.server_packet(T0, op::HEARTBEAT, &(T0 + 850).to_le_bytes());
        l.server_packet(T0, op::PONG, &[0, 0, 1]);
        l.server_packet(T0, op::PONG, &pong_body(5, T0 + 850));
        assert_eq!(l.ping_ms(T0), None);
    }

    #[test]
    fn falls_back_to_the_raw_round_trip_when_the_estimate_collapses() {
        let mut sim = Sim::new(1_000);
        sim.round(T0, 118);
        let t1 = T0 + 10_000;
        assert_eq!(sim.round(t1, 118), Some(118));
        // The PC clock steps 1313 ms back: heartbeats now look like they arrived before they were made.
        sim.l.server_packet(t1 + 200, op::HEARTBEAT, &(t1 + 200 + 850 + 1_313).to_le_bytes());
        assert_eq!(sim.l.ping_ms(t1 + 200), Some(118));
    }

    #[test]
    fn splits_client_frames_by_their_length_prefix() {
        let mut f = ClientFrames::default();
        let mut sizes = Vec::new();
        let hb = [0x0E, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let ping = [0x0F, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let mut stream = hb.to_vec();
        stream.extend(ping);
        stream.extend(hb);
        let (a, b) = stream.split_at(15);
        f.feed(a, &mut |s| sizes.push(s));
        f.feed(b, &mut |s| sizes.push(s));
        assert_eq!(sizes, [11, 12, 11]);
        // After a gap, the next chunk starts a frame again.
        f.reset();
        f.feed(&ping, &mut |s| sizes.push(s));
        assert_eq!(sizes, [11, 12, 11, 12]);
    }
}
