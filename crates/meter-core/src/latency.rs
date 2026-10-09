//! Ping to the game server, measured passively from the captured connection.
//!
//! When the client sends data, the server's TCP stack acknowledges it; the time from our outgoing segment to the
//! server segment that acknowledges it is one round trip. Nothing is sent: both packets are ones the game already
//! exchanges (its heartbeat alone gives ~19 samples a second).
//!
//! The server may hold an acknowledgement back until it has data of its own to send, which inflates some samples.
//! The reported ping is therefore the lowest sample of the last few seconds, which tracks the network path
//! rather than the server's send schedule. Retransmitted data is never sampled (Karn's rule).

use std::collections::VecDeque;

/// How far back the reported ping looks.
const WINDOW_MS: u64 = 3_000;
/// Segments awaiting acknowledgement that we remember; older ones are dropped.
const MAX_OUTSTANDING: usize = 256;
/// Samples above this are an idle connection or a stall, not a ping.
const MAX_SAMPLE_MS: u64 = 5_000;

/// `a` is at or before `b` in sequence space.
fn seq_le(a: u32, b: u32) -> bool {
    b.wrapping_sub(a) as i32 >= 0
}

#[derive(Default)]
pub struct Latency {
    /// (end sequence number, send time) of client data the server has not acknowledged yet, oldest first.
    outstanding: VecDeque<(u32, u64)>,
    /// Highest sequence number the client has sent; data ending at or before it is a retransmit.
    sent_max: Option<u32>,
    /// (receive time, round trip) of recent samples, oldest first.
    samples: VecDeque<(u64, u64)>,
}

impl Latency {
    /// A segment from the client to the server.
    pub fn client_sent(&mut self, t_ms: u64, seq: u32, len: usize) {
        if len == 0 {
            return;
        }
        let end = seq.wrapping_add(len as u32);
        match self.sent_max {
            Some(max) if seq_le(end, max) => {
                // Retransmit: an acknowledgement could answer either copy, so nothing outstanding can be timed.
                self.outstanding.clear();
                return;
            }
            _ => self.sent_max = Some(end),
        }
        if self.outstanding.len() == MAX_OUTSTANDING {
            self.outstanding.pop_front();
        }
        self.outstanding.push_back((end, t_ms));
    }

    /// A segment from the server carrying acknowledgement number `ack`.
    pub fn server_acked(&mut self, t_ms: u64, ack: u32) {
        let mut newest = None;
        while let Some(&(end, sent)) = self.outstanding.front() {
            if !seq_le(end, ack) {
                break;
            }
            newest = Some(sent);
            self.outstanding.pop_front();
        }
        // Of everything this acknowledgement covers, the newest segment waited least for it.
        if let Some(sent) = newest {
            let rtt = t_ms.saturating_sub(sent);
            if rtt <= MAX_SAMPLE_MS {
                self.samples.push_back((t_ms, rtt));
            }
        }
        while self.samples.front().is_some_and(|&(t, _)| t + WINDOW_MS < t_ms) {
            self.samples.pop_front();
        }
    }

    /// Current ping in milliseconds, or `None` if there has been no sample recently.
    pub fn ping_ms(&self, now_ms: u64) -> Option<u64> {
        self.samples.iter().filter(|&&(t, _)| t + WINDOW_MS >= now_ms).map(|&(_, rtt)| rtt).min()
    }

    /// Time of the latest sample.
    pub fn last_sample_ms(&self) -> Option<u64> {
        self.samples.back().map(|&(t, _)| t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_data_until_the_server_acknowledges_it() {
        let mut l = Latency::default();
        l.client_sent(1_000, 100, 20);
        l.server_acked(1_042, 120);
        assert_eq!(l.ping_ms(1_042), Some(42));
    }

    #[test]
    fn partial_acks_wait_and_cumulative_acks_use_the_newest_segment() {
        let mut l = Latency::default();
        l.client_sent(1_000, 100, 10);
        l.client_sent(1_030, 110, 10);
        l.server_acked(1_020, 105); // covers neither segment fully
        assert_eq!(l.ping_ms(1_020), None);
        l.server_acked(1_065, 120); // covers both; the second waited 35 ms
        assert_eq!(l.ping_ms(1_065), Some(35));
    }

    #[test]
    fn reports_the_lowest_recent_sample() {
        let mut l = Latency::default();
        let mut seq = 0u32;
        for (i, rtt) in [80, 40, 95, 60].into_iter().enumerate() {
            let t = 1_000 + i as u64 * 100;
            l.client_sent(t, seq, 10);
            seq += 10;
            l.server_acked(t + rtt, seq);
        }
        assert_eq!(l.ping_ms(1_400), Some(40));
        // Once those samples age out of the window there is no reading.
        assert_eq!(l.ping_ms(1_400 + WINDOW_MS + 100), None);
    }

    #[test]
    fn retransmitted_data_is_not_timed() {
        let mut l = Latency::default();
        l.client_sent(1_000, 100, 10);
        l.client_sent(1_300, 100, 10); // retransmit
        l.server_acked(1_310, 110);
        assert_eq!(l.ping_ms(1_310), None);
        l.client_sent(1_400, 110, 10);
        l.server_acked(1_450, 120);
        assert_eq!(l.ping_ms(1_450), Some(50));
    }

    #[test]
    fn handles_sequence_wraparound() {
        let mut l = Latency::default();
        l.client_sent(1_000, u32::MAX - 4, 10);
        l.server_acked(1_025, 5);
        assert_eq!(l.ping_ms(1_025), Some(25));
    }
}
