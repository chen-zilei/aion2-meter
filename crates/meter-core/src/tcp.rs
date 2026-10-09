//! Orders the TCP segments of one direction of one connection into a byte stream.
//!
//! Retransmits and overlaps are trimmed. Out-of-order segments wait in a small buffer; if a gap does not fill
//! within `MAX_PENDING` segments the missing bytes are skipped and the caller is told, so the frame decoder can
//! resync instead of misreading data.

use std::collections::BTreeMap;

const MAX_PENDING: usize = 64;

pub enum Chunk<'a> {
    Data(&'a [u8]),
    /// Bytes were lost before the next `Data`.
    Gap,
}

#[derive(Default)]
pub struct Reassembler {
    next: Option<u32>,
    pending: BTreeMap<u32, Vec<u8>>,
    pub saw_syn: bool,
    pub gaps: u64,
}

impl Reassembler {
    pub fn push(&mut self, seq: u32, syn: bool, payload: &[u8], out: &mut dyn FnMut(Chunk)) {
        if syn {
            self.saw_syn = true;
            self.next = Some(seq.wrapping_add(1));
            self.pending.clear();
        }
        if payload.is_empty() {
            return;
        }
        let next = *self.next.get_or_insert(seq);

        let ahead = seq.wrapping_sub(next) as i32;
        if ahead > 0 {
            self.pending.entry(seq).or_insert_with(|| payload.to_vec());
            if self.pending.len() > MAX_PENDING {
                self.skip_gap(out);
            }
            return;
        }
        // Starts at or before `next`: drop the part we already delivered.
        let overlap = (-ahead) as usize;
        if overlap >= payload.len() {
            return;
        }
        let fresh = &payload[overlap..];
        out(Chunk::Data(fresh));
        self.next = Some(next.wrapping_add(fresh.len() as u32));
        self.drain_pending(out);
    }

    fn drain_pending(&mut self, out: &mut dyn FnMut(Chunk)) {
        while let Some(next) = self.next {
            let Some((&seq, _)) = self.pending.iter().next() else { return };
            let ahead = seq.wrapping_sub(next) as i32;
            if ahead > 0 {
                return;
            }
            let data = self.pending.remove(&seq).unwrap();
            let overlap = (-ahead) as usize;
            if overlap < data.len() {
                out(Chunk::Data(&data[overlap..]));
                self.next = Some(next.wrapping_add((data.len() - overlap) as u32));
            }
        }
    }

    fn skip_gap(&mut self, out: &mut dyn FnMut(Chunk)) {
        let Some((&seq, _)) = self.pending.iter().next() else { return };
        self.gaps += 1;
        out(Chunk::Gap);
        self.next = Some(seq);
        self.drain_pending(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(segs: &[(u32, &[u8])]) -> (Vec<u8>, usize) {
        let mut r = Reassembler::default();
        let mut bytes = Vec::new();
        let mut gaps = 0;
        for (seq, p) in segs {
            r.push(*seq, false, p, &mut |c| match c {
                Chunk::Data(d) => bytes.extend_from_slice(d),
                Chunk::Gap => gaps += 1,
            });
        }
        (bytes, gaps)
    }

    #[test]
    fn reorders_and_trims_retransmits() {
        let (bytes, gaps) = run(&[(100, b"abc"), (106, b"ghi"), (103, b"def"), (103, b"def"), (104, b"efgh")]);
        assert_eq!(bytes, b"abcdefghi");
        assert_eq!(gaps, 0);
    }

    #[test]
    fn handles_sequence_wraparound() {
        let (bytes, _) = run(&[(u32::MAX - 1, b"ab"), (0, b"cd")]);
        assert_eq!(bytes, b"abcd");
    }
}
