//! Splits one server→client byte stream into game packets.
//!
//! ```text
//! frame   = varint len, [one byte in 0xF0..=0xFE], payload      (frame size = len + varint bytes − 4)
//! payload = opcode(2) body
//!         | FF FF u32 raw_size lz4-block                         (the block holds more frames, possibly nested)
//! ```
//! `0x00` bytes between frames are padding. After lost data the decoder hunts for the next heartbeat frame
//! (`0E 00 36`, about 19 per second) instead of guessing at varints, which would produce phantom damage.

use crate::wire;

pub const MAX_FRAME: usize = 65_535;
const MAX_UNBUNDLED_WAIT: usize = 16_384;
const MAX_BUNDLE: usize = 8 * 1024 * 1024;
const MAX_HUNT_BYTES: usize = 256 * 1024;
const MAX_DEPTH: u8 = 4;
const HEARTBEATS: [[u8; 3]; 2] = [[0x0E, 0x00, 0x36], [0x06, 0x00, 0x36]];

#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
pub struct FrameStats {
    pub frames: u64,
    pub bundles: u64,
    pub bad_bundles: u64,
    pub resyncs: u64,
    pub dropped_bytes: u64,
}

pub struct FrameDecoder {
    buf: Vec<u8>,
    hunting: bool,
    hunted: usize,
    pub stats: FrameStats,
}

impl FrameDecoder {
    /// `starts_aligned`: the stream was captured from its first byte (SYN seen). Otherwise the first segment may
    /// start mid-frame, so the decoder begins by hunting for a heartbeat.
    pub fn new(starts_aligned: bool) -> Self {
        Self { buf: Vec::with_capacity(128 * 1024), hunting: !starts_aligned, hunted: 0, stats: FrameStats::default() }
    }

    /// Call when the TCP layer skipped missing bytes: the next byte is not a frame boundary.
    pub fn reset(&mut self) {
        self.buf.clear();
        self.hunting = true;
        self.hunted = 0;
    }

    pub fn feed(&mut self, data: &[u8], on_packet: &mut dyn FnMut(u16, &[u8])) {
        self.buf.extend_from_slice(data);
        let mut pos = 0;
        let buf = std::mem::take(&mut self.buf);

        while pos < buf.len() {
            let span = &buf[pos..];

            if self.hunting {
                match find_heartbeat(span) {
                    Some(i) => {
                        pos += i;
                        self.stats.dropped_bytes += i as u64;
                        self.hunting = false;
                        continue;
                    }
                    None => {
                        let skip = span.len().saturating_sub(2); // a signature may straddle two segments
                        pos += skip;
                        self.stats.dropped_bytes += skip as u64;
                        self.hunted += skip;
                        if self.hunted > MAX_HUNT_BYTES {
                            self.hunting = false; // signature changed after a patch? fall back to byte resync
                        }
                        break;
                    }
                }
            }

            if span[0] == 0x00 {
                pos += 1;
                continue;
            }

            let Some((len, vlen)) = wire::peek_varint(span, 0) else {
                if span.len() < 5 {
                    break; // need more bytes
                }
                pos += self.resync();
                continue;
            };

            let size = (len as usize + vlen).saturating_sub(4);
            if len < 6 || size > MAX_FRAME {
                pos += self.resync();
                continue;
            }
            if size > span.len() {
                if size > MAX_UNBUNDLED_WAIT && !is_bundle(span, vlen) {
                    pos += self.resync();
                    continue;
                }
                break; // wait for the rest of the frame
            }

            self.handle_frame(&span[..size], vlen, 0, on_packet);
            pos += size;
        }

        self.buf = buf;
        self.buf.drain(..pos);
    }

    fn resync(&mut self) -> usize {
        self.stats.resyncs += 1;
        self.stats.dropped_bytes += 1;
        if self.hunted <= MAX_HUNT_BYTES {
            self.hunting = true;
            self.hunted = 0;
        }
        1
    }

    fn handle_frame(&mut self, frame: &[u8], vlen: usize, depth: u8, on_packet: &mut dyn FnMut(u16, &[u8])) {
        self.stats.frames += 1;
        let mut o = vlen;
        if matches!(frame.get(o), Some(0xF0..=0xFE)) {
            o += 1;
        }
        if o + 2 > frame.len() {
            return;
        }

        if frame[o] == 0xFF && frame[o + 1] == 0xFF {
            self.stats.bundles += 1;
            o += 2;
            let raw = match wire::u32(frame, &mut o) {
                Some(n) if n > 0 && (n as usize) <= MAX_BUNDLE && depth < MAX_DEPTH => n as usize,
                _ => {
                    self.stats.bad_bundles += 1;
                    return;
                }
            };
            let mut out = vec![0u8; raw];
            match lz4_flex::block::decompress_into(&frame[o..], &mut out) {
                Ok(n) if n > 0 => {
                    out.truncate(n);
                    self.walk_inner(&out, depth + 1, on_packet);
                }
                _ => self.stats.bad_bundles += 1,
            }
            return;
        }

        let opcode = (frame[o] as u16) << 8 | frame[o + 1] as u16;
        on_packet(opcode, &frame[o + 2..]);
    }

    /// Frames inside a decompressed bundle. No resync here: an invalid length ends the bundle.
    fn walk_inner(&mut self, data: &[u8], depth: u8, on_packet: &mut dyn FnMut(u16, &[u8])) {
        let mut pos = 0;
        while pos < data.len() {
            if data[pos] == 0x00 {
                pos += 1;
                continue;
            }
            let span = &data[pos..];
            let Some((len, vlen)) = wire::peek_varint(span, 0) else { return };
            let size = (len as usize + vlen).saturating_sub(4);
            if len < 6 || size > span.len() {
                return;
            }
            self.handle_frame(&span[..size], vlen, depth, on_packet);
            pos += size;
        }
    }
}

fn find_heartbeat(span: &[u8]) -> Option<usize> {
    HEARTBEATS.iter().filter_map(|sig| span.windows(sig.len()).position(|w| w == sig)).min()
}

fn is_bundle(frame: &[u8], vlen: usize) -> bool {
    let mut o = vlen;
    if matches!(frame.get(o), Some(0xF0..=0xFE)) {
        o += 1;
    }
    frame.get(o) == Some(&0xFF) && frame.get(o + 1) == Some(&0xFF)
}

/// Encodes one frame the way the server does. Used by tests and the demo source.
pub fn encode_frame(opcode: u16, body: &[u8]) -> Vec<u8> {
    let payload_len = 2 + body.len();
    // size = len + vlen - 4 and size = vlen + payload  =>  len = payload + 4
    let len = (payload_len + 4) as u32;
    let mut out = encode_varint(len);
    out.extend_from_slice(&opcode.to_be_bytes());
    out.extend_from_slice(body);
    out
}

pub fn encode_varint(mut v: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(5);
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return out;
        }
        out.push(byte | 0x80);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(dec: &mut FrameDecoder, data: &[u8]) -> Vec<(u16, Vec<u8>)> {
        let mut got = Vec::new();
        dec.feed(data, &mut |op, body| got.push((op, body.to_vec())));
        got
    }

    fn heartbeat() -> Vec<u8> {
        encode_frame(0x0036, &[0; 8])
    }

    #[test]
    fn heartbeat_encodes_like_the_wire() {
        assert_eq!(&heartbeat()[..3], &[0x0E, 0x00, 0x36]);
    }

    #[test]
    fn splits_frames_across_segments() {
        let mut stream = heartbeat();
        stream.extend(encode_frame(0x0438, &[1, 2, 3, 4]));
        stream.push(0); // padding
        stream.extend(heartbeat());

        let mut dec = FrameDecoder::new(true);
        let mut got = collect(&mut dec, &stream[..7]);
        got.extend(collect(&mut dec, &stream[7..]));
        let ops: Vec<u16> = got.iter().map(|g| g.0).collect();
        assert_eq!(ops, [0x0036, 0x0438, 0x0036]);
        assert_eq!(got[1].1, [1, 2, 3, 4]);
    }

    #[test]
    fn unpacks_lz4_bundles() {
        let mut inner = encode_frame(0x0438, &[9, 9, 9]);
        inner.extend(encode_frame(0x0538, &[7]));
        let compressed = lz4_flex::block::compress(&inner);
        let mut body = (inner.len() as u32).to_le_bytes().to_vec();
        body.extend(compressed);
        let bundle = encode_frame(0xFFFF, &body);

        let mut dec = FrameDecoder::new(true);
        let ops: Vec<u16> = collect(&mut dec, &bundle).iter().map(|g| g.0).collect();
        assert_eq!(ops, [0x0438, 0x0538]);
        assert_eq!(dec.stats.bundles, 1);
    }

    #[test]
    fn resyncs_on_heartbeat_after_joining_mid_stream() {
        let mut stream = vec![0x83, 0x44, 0x12, 0x99]; // tail of a frame we never saw the start of
        stream.extend(heartbeat());
        stream.extend(encode_frame(0x0438, &[5]));

        let mut dec = FrameDecoder::new(false);
        let ops: Vec<u16> = collect(&mut dec, &stream).iter().map(|g| g.0).collect();
        assert_eq!(ops, [0x0036, 0x0438]);
        assert_eq!(dec.stats.dropped_bytes, 4);
    }
}
