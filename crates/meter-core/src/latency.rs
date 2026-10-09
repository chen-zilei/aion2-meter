//! Ping to the game server, measured passively from the game's own messages.
//!
//! TCP acknowledgements can't be used: the game connects through a nearby Cloudflare relay, which acknowledges
//! segments itself, so they only time the hop to the relay (~2 ms). Instead:
//!
//! - Every 10 s the client sends a 13-byte ping frame (encrypted, recognisable only by its size) and the server
//!   answers with `03 36`: `00 00, u64 echo of the client's clock, u64 server Unix ms` stamped on receipt.
//!   `server stamp − our send time` is the uplink plus the PC↔server clock offset (`uplink` below).
//! - About 20 times a second the server sends a heartbeat (`00 36`) holding its Unix ms when created.
//!   `arrival − value` is the downlink, minus the same clock offset, plus however long the server held it before
//!   sending. The lowest of the last few seconds is the heartbeat that went out right away.
//!
//! The sum cancels the clock offset and leaves the round trip, refreshed with every heartbeat. Until both halves
//! are known the raw ping/pong round trip is shown, which also includes the server's ~100 ms send tick.
//! Nothing is sent: these are messages the game exchanges anyway.

use std::collections::VecDeque;

/// Total size of the client's ping frame, length prefix included.
pub const PING_FRAME_LEN: usize = 13;
/// How far back the heartbeat minimum looks.
const WINDOW_MS: u64 = 3_000;
/// A pong this long after the ping is not its answer.
const MAX_PONG_WAIT_MS: u64 = 3_000;
/// The uplink measurement is trusted for this long (pings come every 10 s).
const UPLINK_VALID_MS: u64 = 35_000;
/// A clock offset beyond this means we misread a field.
const MAX_OFFSET_MS: i64 = 3_600_000;

#[derive(Default)]
pub struct Latency {
    /// Capture time of the client's latest ping frame, until its pong arrives.
    ping_sent: Option<u64>,
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
    /// A complete frame from the client to the server, `size` bytes long.
    pub fn client_frame(&mut self, t_ms: u64, size: usize) {
        if size == PING_FRAME_LEN {
            self.ping_sent = Some(t_ms);
        }
    }

    /// Server `03 36` answering the client's ping.
    pub fn pong(&mut self, t_ms: u64, body: &[u8]) {
        let (Some(sent), Some(stamp)) = (self.ping_sent, u64_at(body, 10)) else { return };
        if t_ms < sent || t_ms - sent > MAX_PONG_WAIT_MS {
            return;
        }
        self.ping_sent = None;
        let uplink = stamp - sent as i64;
        if uplink.abs() > MAX_OFFSET_MS {
            return;
        }
        self.uplink = Some((t_ms, uplink));
        self.raw = Some((t_ms, t_ms - sent));
    }

    /// Server heartbeat `00 36`.
    pub fn heartbeat(&mut self, t_ms: u64, body: &[u8]) {
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
        match down {
            Some(d) => Some((uplink + d).max(0) as u64),
            None => self.raw.map(|(_, rtt)| rtt),
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

    fn pong_body(echo: u64, stamp: u64) -> Vec<u8> {
        let mut b = vec![0, 0];
        b.extend(echo.to_le_bytes());
        b.extend(stamp.to_le_bytes());
        b
    }

    /// Server clock 850 ms ahead of ours, 60 ms each way, heartbeats held 0–100 ms before sending.
    #[test]
    fn adds_uplink_and_lowest_downlink_cancelling_the_clock_offset() {
        let mut l = Latency::default();
        l.client_frame(10_000, PING_FRAME_LEN);
        // Server stamps on receipt (10_060 ours = 10_910 its), answers on its next tick 40 ms later.
        l.pong(10_160, &pong_body(123, 10_910));
        assert_eq!(l.ping_ms(10_160), Some(160)); // only the raw round trip so far
        for (made, held) in [(10_950u64, 90u64), (11_000, 40), (11_050, 0), (11_100, 70)] {
            // Made at `made` server time, sent `held` later, arrives 60 ms after that, in our clock.
            l.heartbeat(made - 850 + held + 60, &made.to_le_bytes());
        }
        assert_eq!(l.ping_ms(10_400), Some(120));
    }

    #[test]
    fn ignores_pongs_without_a_recent_ping_and_goes_stale() {
        let mut l = Latency::default();
        l.pong(5_000, &pong_body(1, 9_000));
        assert_eq!(l.ping_ms(5_000), None);
        l.client_frame(6_000, 11); // a heartbeat-sized frame is not a ping
        l.pong(6_100, &pong_body(1, 9_000));
        assert_eq!(l.ping_ms(6_100), None);
        l.client_frame(7_000, PING_FRAME_LEN);
        l.pong(7_130, &pong_body(1, 7_900));
        assert_eq!(l.ping_ms(7_130), Some(130));
        assert_eq!(l.ping_ms(7_130 + UPLINK_VALID_MS + 1), None);
    }

    #[test]
    fn splits_client_frames_by_their_length_prefix() {
        let mut f = ClientFrames::default();
        let mut sizes = Vec::new();
        let hb = [0x0E, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let ping = [0x10, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let mut stream = hb.to_vec();
        stream.extend(ping);
        stream.extend(hb);
        let (a, b) = stream.split_at(15);
        f.feed(a, &mut |s| sizes.push(s));
        f.feed(b, &mut |s| sizes.push(s));
        assert_eq!(sizes, [11, 13, 11]);
        // After a gap, the next chunk starts a frame again.
        f.reset();
        f.feed(&ping, &mut |s| sizes.push(s));
        assert_eq!(sizes, [11, 13, 11, 13]);
    }
}
