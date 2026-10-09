//! Primitive readers for the AION 2 wire format. Multi-byte integers are little-endian.

/// LEB128 unsigned varint, at most 5 bytes. Advances `o` only on success.
pub fn varint(b: &[u8], o: &mut usize) -> Option<u32> {
    let mut value: u32 = 0;
    for i in 0..5 {
        let byte = *b.get(*o + i)?;
        value |= ((byte & 0x7F) as u32) << (7 * i);
        if byte & 0x80 == 0 {
            *o += i + 1;
            return Some(value);
        }
    }
    None
}

/// Varint at `o` plus its encoded length, without advancing.
pub fn peek_varint(b: &[u8], o: usize) -> Option<(u32, usize)> {
    let mut p = o;
    varint(b, &mut p).map(|v| (v, p - o))
}

pub fn u8(b: &[u8], o: &mut usize) -> Option<u8> {
    let v = *b.get(*o)?;
    *o += 1;
    Some(v)
}

pub fn u16(b: &[u8], o: &mut usize) -> Option<u16> {
    let s = b.get(*o..*o + 2)?;
    *o += 2;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

pub fn u32(b: &[u8], o: &mut usize) -> Option<u32> {
    let s = b.get(*o..*o + 4)?;
    *o += 4;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_varints() {
        let b = [0x0E, 0xAC, 0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F];
        let mut o = 0;
        assert_eq!(varint(&b, &mut o), Some(14));
        assert_eq!(varint(&b, &mut o), Some(300));
        assert_eq!(varint(&b, &mut o), Some(u32::MAX));
        assert_eq!(o, b.len());
    }

    #[test]
    fn truncated_varint_does_not_advance() {
        let mut o = 0;
        assert_eq!(varint(&[0x80, 0x80], &mut o), None);
        assert_eq!(o, 0);
    }
}
