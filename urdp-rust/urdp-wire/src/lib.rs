//! URDP wire format helpers.
//!
//! This crate contains helper functions for encoding and decoding
//! numbers and headers on the URDP wire.  The API is intentionally
//! simple and does not expose any network transports; those are left
//! to higher layers.

use thiserror::Error;

/// Error type for wire operations.
#[derive(Debug, Error)]
pub enum WireError {
    #[error("varint encoding overflow")]
    VarintOverflow,
    #[error("unexpected end of input")]
    UnexpectedEof,
}

/// Encode a 64-bit integer as a little-endian base-128 (varint).
pub fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    while value >= 0x80 {
        buf.push(((value & 0x7F) as u8) | 0x80);
        value >>= 7;
    }
    buf.push(value as u8);
    buf
}

/// Decode a varint, returning the value and the number of bytes consumed.
pub fn decode_varint(input: &[u8]) -> Result<(u64, usize), WireError> {
    let mut value = 0u64;
    let mut shift = 0;
    for (i, &byte) in input.iter().enumerate() {
        let bits = (byte & 0x7F) as u64;
        value |= bits << shift;
        if (byte & 0x80) == 0 {
            return Ok((value, i + 1));
        }
        shift += 7;
        if shift >= 64 {
            return Err(WireError::VarintOverflow);
        }
    }
    Err(WireError::UnexpectedEof)
}

/// Pack a URDP block header from its fields.
///
/// A header consists of a varint `block_id`, followed by a single
/// byte containing the codex slot (lower 4 bits), lane (bits 4–5)
/// and flags (bits 6–7), and then an 8-byte session tag.
pub fn pack_header(block_id: u64, codex_slot: u8, lane: u8, flags: u8, session_tag: [u8; 8]) -> Vec<u8> {
    let mut buf = encode_varint(block_id);
    let header_byte = (codex_slot & 0x0F) | ((lane & 0x03) << 4) | ((flags & 0x03) << 6);
    buf.push(header_byte);
    buf.extend_from_slice(&session_tag);
    buf
}

/// Unpack a URDP block header into its components.
pub fn unpack_header(input: &[u8]) -> Result<(u64, u8, u8, u8, [u8; 8], usize), WireError> {
    let (block_id, varint_len) = decode_varint(input)?;
    if input.len() < varint_len + 1 + 8 {
        return Err(WireError::UnexpectedEof);
    }
    let header_byte = input[varint_len];
    let codex_slot = header_byte & 0x0F;
    let lane = (header_byte >> 4) & 0x03;
    let flags = (header_byte >> 6) & 0x03;
    let mut tag = [0u8; 8];
    tag.copy_from_slice(&input[varint_len + 1..varint_len + 9]);
    Ok((block_id, codex_slot, lane, flags, tag, varint_len + 1 + 8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_roundtrip() {
        let values = [0, 1, 127, 128, 255, 256, 16384, u32::MAX as u64, u64::MAX];
        for &v in &values {
            let encoded = encode_varint(v);
            let (decoded, len) = decode_varint(&encoded).unwrap();
            assert_eq!(decoded, v);
            assert_eq!(len, encoded.len());
        }
    }

    #[test]
    fn test_header_roundtrip() {
        let id = 5;
        let slot = 2;
        let lane = 1;
        let flags = 1;
        let tag = [1, 2, 3, 4, 5, 6, 7, 8];
        let packed = pack_header(id, slot, lane, flags, tag);
        let (rid, rslot, rlane, rflags, rtag, consumed) = unpack_header(&packed).unwrap();
        assert_eq!(rid, id);
        assert_eq!(rslot, slot);
        assert_eq!(rlane, lane);
        assert_eq!(rflags, flags);
        assert_eq!(rtag, tag);
        assert_eq!(consumed, packed.len());
    }
}
