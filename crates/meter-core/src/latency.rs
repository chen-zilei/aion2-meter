//! Ping to the game server, measured passively from the game's own messages.
//!
//! TCP acknowledgements can't be used: the game connects through a nearby Cloudflare relay, which acknowledges
//! segments itself, so they only time the hop to the relay (~2 ms). Instead:
//!
//! - Every 10 s the client sends a ping (encrypted; its size changes between logins) and the server answers with
//!   `03 36`: `00 00`, u64 echo of the client's send time, u64 server Unix ms stamped on receipt. The echo is the
//!   PC's own Unix ms minus a fixed game epoch, so `echo + epoch` is our send time on our clock, and
//!   `server stamp − send time` is the uplink plus the PC↔server clock offset (`uplink` below).
//! - About 20 times a second the server sends a heartbeat (`00 36`) holding its Unix ms when created.
//!   `arrival − value` is the downlink, minus the same clock offset, plus however long the server held it before
//!   sending. The lowest of the last few seconds is the heartbeat that went out right away.
//!
//! The sum cancels the clock offset and leaves the round trip, refreshed with every heartbeat. It is capped by the
//! latest raw ping/pong round trip, which is shown on its own until a heartbeat arrives.
//!
//! The epoch was read from captures and held across game restarts and logins. If a patch moves it, pongs stop
//! matching and `replay --ping` shows it (`pongs` vs `matched`). Nothing is sent: these are messages the game exchanges anyway.

use std::collections::VecDeque;

/// PC Unix ms minus the client's echoed clock (2026-03-27 17:49:10.910 UTC), seen across game restarts.
pub const GAME_EPOCH_MS: i64 = 1_774_633_750_910;
/// How far back the heartbeat minimum looks.
const WINDOW_MS: u64 = 3_000;
/// A pong this long after its ping is not its answer.
const MAX_PONG_WAIT_MS: i64 = 3_000;
/// The uplink measurement is trusted for this long (pings come every 10 s).
const UPLINK_VALID_MS: u64 = 35_000;
/// A clock offset beyond this means we misread a field.
const MAX_OFFSET_MS: i64 = 3_600_000;

/// Counters for diagnosing the ping readout (`replay --ping`).
#[derive(Debug, Default, Clone)]
pub struct LatencyStats {
    pub pongs: u64,
    pub pongs_matched: u64,
    pub heartbeats: u64,
    /// `arrival − echo` of the latest pong: the epoch plus the round trip, for re-finding the epoch.
    pub last_arrival_minus_echo: Option<i64>,
}

#[derive(Default)]
pub struct Latency {
    pub stats: LatencyStats,
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
    /// Server `03 36` answering the client's ping.
    pub fn pong(&mut self, t_ms: u64, body: &[u8]) {
        self.stats.pongs += 1;
        let (Some(echo), Some(stamp)) = (u64_at(body, 2), u64_at(body, 10)) else { return };
        let arrival = t_ms as i64;
        self.stats.last_arrival_minus_echo = Some(arrival - echo);
        let send = echo + GAME_EPOCH_MS;
        let uplink = stamp - send;
        if send > arrival || arrival - send > MAX_PONG_WAIT_MS || uplink.abs() > MAX_OFFSET_MS {
            return;
        }
        self.stats.pongs_matched += 1;
        self.uplink = Some((t_ms, uplink));
        self.raw = Some((t_ms, (arrival - send) as u64));
    }

    /// Server heartbeat `00 36`.
    pub fn heartbeat(&mut self, t_ms: u64, body: &[u8]) {
        self.stats.heartbeats += 1;
        let Some(value) = u64_at(body, 0) else { return };
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
        let raw = self.raw.map(|(_, rtt)| rtt);
        match down {
            Some(d) => Some(((uplink + d).max(0) as u64).min(raw.unwrap_or(u64::MAX))),
            None => raw,
        }
    }

    /// Time of the latest reading, to pick the live connection when there are several.
    pub fn last_sample_ms(&self) -> Option<u64> {
        self.downlink.back().map(|&(t, _)| t).max(self.uplink.map(|(t, _)| t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pong for a ping sent at `sent` (PC ms) under `epoch`, stamped `stamp` by the server.
    fn pong_body(sent: u64, epoch: i64, stamp: u64) -> Vec<u8> {
        let mut b = vec![0, 0];
        b.extend(((sent as i64 - epoch) as u64).to_le_bytes());
        b.extend(stamp.to_le_bytes());
        b
    }

    const T0: u64 = 1_791_515_750_000;

    /// Server clock 850 ms ahead of ours, 60 ms each way, heartbeats held 0–100 ms before sending.
    #[test]
    fn adds_uplink_and_lowest_downlink_cancelling_the_clock_offset() {
        let mut l = Latency::default();
        // Ping at T0; the server stamps it on receipt (T0+60 ours = T0+910 its) and answers 100 ms later.
        l.pong(T0 + 160, &pong_body(T0, GAME_EPOCH_MS, T0 + 910));
        assert_eq!(l.ping_ms(T0 + 160), Some(160)); // only the raw round trip so far
        for (made, held) in [(T0 + 950, 90), (T0 + 1_000, 40), (T0 + 1_050, 0), (T0 + 1_100, 70)] {
            // Made at `made` server time, sent `held` later, arrives 60 ms after that, in our clock.
            l.heartbeat(made - 850 + held + 60, &made.to_le_bytes());
        }
        assert_eq!(l.ping_ms(T0 + 400), Some(120));
        assert_eq!(l.ping_ms(T0 + 160 + UPLINK_VALID_MS + 1), None);
    }

    #[test]
    fn needs_no_particular_client_message() {
        // No outgoing segments seen at all: the echo alone gives the send time.
        let mut l = Latency::default();
        l.pong(T0 + 117, &pong_body(T0, GAME_EPOCH_MS, T0 + 900));
        assert_eq!(l.ping_ms(T0 + 117), Some(117));
    }

    #[test]
    fn never_reads_above_the_raw_round_trip() {
        let mut l = Latency::default();
        // Raw 118 ms; every heartbeat waited 6 ms before leaving, so the estimate alone would say 124.
        l.pong(T0 + 118, &pong_body(T0, GAME_EPOCH_MS, T0 + 909));
        l.heartbeat(T0 + 200, &(T0 + 200 + 850 - 59 - 6).to_le_bytes());
        assert_eq!(l.ping_ms(T0 + 200), Some(118));
    }

    #[test]
    fn ignores_garbage_pongs() {
        let mut l = Latency::default();
        l.pong(T0, &[0, 0, 1]);
        l.pong(T0, &pong_body(T0 + 60_000, GAME_EPOCH_MS, T0)); // "sent" in the future
        assert_eq!(l.ping_ms(T0), None);
    }
}
