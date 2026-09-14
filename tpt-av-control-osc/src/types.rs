//! Low-level OSC wire-format primitives: big-endian scalar reads/writes and
//! 4-byte-aligned padding. All readers borrow; none allocate.

use tpt_av_control_utils::ControlError;

/// OSC type tag characters.
pub mod tags {
    /// `i` — 32-bit integer.
    pub const INT: u8 = b'i';
    /// `f` — 32-bit float.
    pub const FLOAT: u8 = b'f';
    /// `s` — OSC-string.
    pub const STRING: u8 = b's';
    /// `b` — OSC-blob.
    pub const BLOB: u8 = b'b';
    /// `h` — 64-bit integer.
    pub const LONG: u8 = b'h';
    /// `t` — OSC-timetag.
    pub const TIME: u8 = b't';
    /// `d` — 64-bit float.
    pub const DOUBLE: u8 = b'd';
    /// `S` — symbol (alternate string).
    pub const SYMBOL: u8 = b'S';
    /// `c` — ASCII character.
    pub const CHAR: u8 = b'c';
    /// `r` — 32-bit RGBA color.
    pub const COLOR: u8 = b'r';
    /// `m` — 4-byte MIDI message.
    pub const MIDI: u8 = b'm';
    /// `T` — boolean true.
    pub const TRUE: u8 = b'T';
    /// `F` — boolean false.
    pub const FALSE: u8 = b'F';
    /// `N` — nil.
    pub const NIL: u8 = b'N';
    /// `I` — infinitum.
    pub const INFINITUM: u8 = b'I';
}

/// Rounds `len` up to the next multiple of 4 (OSC's alignment rule).
pub const fn padded_len(len: usize) -> usize {
    (len + 3) & !3
}

/// Encodes `s` (with its terminating NUL) padded to a 4-byte boundary.
pub fn string_to_padded_bytes(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(padded_len(s.len() + 1));
    padded_into(&mut out, s.as_bytes());
    out
}

/// Appends `payload` plus a NUL terminator, padded to a 4-byte boundary.
pub fn padded_into(buf: &mut Vec<u8>, payload: &[u8]) {
    buf.extend_from_slice(payload);
    let pad = padded_len(payload.len() + 1) - payload.len();
    buf.extend(std::iter::repeat(0u8).take(pad));
}

/// Reads a `u32` at `offset`.
pub fn read_u32(data: &[u8], offset: usize) -> Result<u32, ControlError> {
    let end = offset
        .checked_add(4)
        .filter(|&e| e <= data.len())
        .ok_or_else(|| invalid(offset, 4, data.len()))?;
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&data[offset..end]);
    Ok(u32::from_be_bytes(bytes))
}

/// Reads an `i32` at `offset`.
pub fn read_i32(data: &[u8], offset: usize) -> Result<i32, ControlError> {
    Ok(read_u32(data, offset)? as i32)
}

/// Reads a `u64` at `offset`.
pub fn read_u64(data: &[u8], offset: usize) -> Result<u64, ControlError> {
    let end = offset
        .checked_add(8)
        .filter(|&e| e <= data.len())
        .ok_or_else(|| invalid(offset, 8, data.len()))?;
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[offset..end]);
    Ok(u64::from_be_bytes(bytes))
}

/// Reads an `i64` at `offset`.
pub fn read_i64(data: &[u8], offset: usize) -> Result<i64, ControlError> {
    Ok(read_u64(data, offset)? as i64)
}

/// Reads an IEEE-754 `f32` at `offset`.
pub fn read_f32(data: &[u8], offset: usize) -> Result<f32, ControlError> {
    Ok(f32::from_bits(read_u32(data, offset)?))
}

/// Reads an IEEE-754 `f64` at `offset`.
pub fn read_f64(data: &[u8], offset: usize) -> Result<f64, ControlError> {
    Ok(f64::from_bits(read_u64(data, offset)?))
}

/// Reads a NUL-terminated byte string starting at `offset`. Returns the
/// payload and the offset *after* its 4-byte-aligned padding.
pub fn read_null_terminated(data: &[u8], offset: usize) -> Result<(&[u8], usize), ControlError> {
    let nul = data[offset..]
        .iter()
        .position(|&b| b == 0)
        .map(|p| offset + p)
        .ok_or_else(|| {
            ControlError::InvalidData("unterminated OSC string".to_string())
        })?;
    let payload = &data[offset..nul];
    let after_pad = offset + padded_len(nul - offset + 1);
    if after_pad > data.len() {
        return Err(ControlError::InvalidData(format!(
            "string padding overruns packet (end {after_pad}, len {})",
            data.len()
        )));
    }
    Ok((payload, after_pad))
}

fn invalid(offset: usize, need: usize, have: usize) -> ControlError {
    ControlError::InvalidData(format!(
        "truncated OSC data: need {need} bytes at offset {offset}, have {have}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_len_rules() {
        assert_eq!(padded_len(0), 0);
        assert_eq!(padded_len(1), 4);
        assert_eq!(padded_len(3), 4);
        assert_eq!(padded_len(4), 4);
        assert_eq!(padded_len(5), 8);
    }

    #[test]
    fn string_padding() {
        assert_eq!(string_to_padded_bytes("").len(), 4);
        assert_eq!(string_to_padded_bytes("a").len(), 4);
        assert_eq!(string_to_padded_bytes("abcd").len(), 8);
        assert_eq!(string_to_padded_bytes("abcde").len(), 8);
        assert_eq!(string_to_padded_bytes("ab"), b"ab\0\0");
    }

    #[test]
    fn read_scalars() {
        let data = [0x00, 0x00, 0x01, 0x00, 0x40, 0xCC, 0x00, 0x00];
        assert_eq!(read_u32(&data, 0).unwrap(), 256);
        assert_eq!(read_i32(&data, 0).unwrap(), 256);
        assert!((read_f32(&data, 4).unwrap() - 6.375).abs() < 1e-6);
        assert!(read_u32(&data, 5).is_err());
    }

    #[test]
    fn read_null_terminated_pads() {
        // "abc\0" is already 4-byte aligned; "de\0\0" follows.
        let data = b"abc\0de\0\0";
        let (payload, next) = read_null_terminated(data, 0).unwrap();
        assert_eq!(payload, b"abc");
        assert_eq!(next, 4);
        let (payload2, _) = read_null_terminated(data, 4).unwrap();
        assert_eq!(payload2, b"de");
        // No NUL anywhere.
        assert!(read_null_terminated(b"abcd", 0).is_err());
    }
}
