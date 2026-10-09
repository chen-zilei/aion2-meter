//! Captured link-layer packets → combat tracker. One instance per meter; feed it from live capture or a replay.

use crate::combat::{Tracker, TrackerOptions};
use crate::frame::{FrameDecoder, FrameStats};
use crate::net::{self, FlowKey, LinkType};
use crate::parser::{Event, Parser};
use crate::tcp::{Chunk, Reassembler};
use std::collections::HashMap;

/// Server port the Global client talked to in community captures. Configurable because it may change.
pub const DEFAULT_GAME_PORT: u16 = 13328;

pub type PacketTap = Box<dyn FnMut(u64, u16, &[u8]) + Send>;
pub type EventTap = Box<dyn FnMut(&Event) + Send>;

struct Flow {
    tcp: Reassembler,
    frames: FrameDecoder,
}

pub struct Pipeline {
    pub game_ports: Vec<u16>,
    flows: HashMap<FlowKey, Flow>,
    pub parser: Parser,
    pub tracker: Tracker,
    /// Optional raw packet tap for debugging / reverse engineering.
    pub on_packet: Option<PacketTap>,
    pub on_event: Option<EventTap>,
    pub last_ms: u64,
}

impl Pipeline {
    pub fn new(game_ports: Vec<u16>, opts: TrackerOptions) -> Self {
        Self {
            game_ports,
            flows: HashMap::new(),
            parser: Parser::default(),
            tracker: Tracker::new(opts),
            on_packet: None,
            on_event: None,
            last_ms: 0,
        }
    }

    pub fn push(&mut self, link: LinkType, t_ms: u64, data: &[u8]) {
        self.last_ms = self.last_ms.max(t_ms);
        let Some(seg) = net::parse(link, data) else { return };
        // Only server → client: the server is the side using the game port.
        if !self.game_ports.contains(&seg.flow.src.port()) {
            return;
        }
        if seg.rst || seg.fin {
            self.flows.remove(&seg.flow);
            return;
        }
        let flow = self.flows.entry(seg.flow).or_insert_with(|| Flow {
            tcp: Reassembler::default(),
            frames: FrameDecoder::new(seg.syn),
        });

        let Self { parser, tracker, on_packet, on_event, .. } = self;
        let Flow { tcp, frames } = flow;
        tcp.push(seg.seq, seg.syn, seg.payload, &mut |chunk| match chunk {
            Chunk::Gap => frames.reset(),
            Chunk::Data(bytes) => frames.feed(bytes, &mut |opcode, body| {
                if let Some(tap) = on_packet.as_mut() {
                    tap(t_ms, opcode, body);
                }
                parser.packet(t_ms, opcode, body, &mut |ev| {
                    if let Some(cb) = on_event.as_mut() {
                        cb(&ev);
                    }
                    tracker.event(&ev);
                });
            }),
        });
    }

    /// Lets idle encounters close even when no packets arrive. `now_ms` is in capture time.
    pub fn tick(&mut self, now_ms: u64) {
        self.tracker.tick(now_ms);
    }

    pub fn frame_stats(&self) -> FrameStats {
        self.flows.values().fold(FrameStats::default(), |mut acc, f| {
            acc.frames += f.frames.stats.frames;
            acc.bundles += f.frames.stats.bundles;
            acc.bad_bundles += f.frames.stats.bad_bundles;
            acc.resyncs += f.frames.stats.resyncs;
            acc.dropped_bytes += f.frames.stats.dropped_bytes;
            acc
        })
    }

    pub fn has_game_flow(&self) -> bool {
        !self.flows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::encode_frame;
    use crate::opcodes as op;
    use crate::parser::encode_damage;
    use etherparse::PacketBuilder;

    fn tcp_packet(seq: u32, src_port: u16, payload: &[u8]) -> Vec<u8> {
        let builder = PacketBuilder::ethernet2([1; 6], [2; 6])
            .ipv4([10, 0, 0, 1], [192, 168, 1, 2], 64)
            .tcp(src_port, 50_000, seq, 65_535);
        let mut out = Vec::with_capacity(builder.size(payload.len()));
        builder.write(&mut out, payload).unwrap();
        out
    }

    #[test]
    fn decodes_damage_end_to_end_from_ethernet_frames() {
        let mut stream = encode_frame(op::HEARTBEAT, &[0; 8]);
        stream.extend(encode_frame(op::DAMAGE, &encode_damage(9000, 7, 11_020_000, 2_500, false)));
        stream.extend(encode_frame(op::DAMAGE, &encode_damage(9000, 7, 11_020_000, 7_500, true)));

        let mut pipe = Pipeline::new(vec![DEFAULT_GAME_PORT], TrackerOptions::default());
        let (a, b) = stream.split_at(20);
        // Joined mid-connection (no SYN), split across segments, with an unrelated flow mixed in.
        pipe.push(LinkType::Ethernet, 1_000, &tcp_packet(1_000, DEFAULT_GAME_PORT, a));
        pipe.push(LinkType::Ethernet, 1_000, &tcp_packet(5, 443, b"not the game"));
        pipe.push(LinkType::Ethernet, 3_000, &tcp_packet(1_000 + a.len() as u32, DEFAULT_GAME_PORT, b));

        let snap = pipe.tracker.snapshot().expect("an encounter");
        assert_eq!(snap.total_damage, 10_000);
        assert_eq!(snap.actors[0].crits, 1);
        assert_eq!(pipe.parser.stats.heartbeats, 1);
    }
}
