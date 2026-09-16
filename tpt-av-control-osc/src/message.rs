//! OSC messages: owned representation, encoding, and the real-time-safe
//! zero-copy parser.

use crate::types::{self, tags};
use tpt_av_control_utils::ControlError;

/// An OSC argument value.
#[derive(Debug, Clone, PartialEq)]
pub enum OscArg {
    /// `i` — 32-bit integer.
    Int(i32),
    /// `f` — 32-bit float.
    Float(f32),
    /// `s` — OSC-string.
    String(String),
    /// `b` — OSC-blob.
    Blob(Vec<u8>),
    /// `h` — 64-bit integer.
    Long(i64),
    /// `t` — OSC-timetag (64-bit fixed point NTP).
    Time(u64),
    /// `d` — 64-bit float.
    Double(f64),
    /// `S` — symbol; kept as a string.
    Symbol(String),
    /// `c` — ASCII character.
    Char(char),
    /// `r` — 32-bit RGBA color (as `0xRRGGBBAA`).
    Color(u32),
    /// `m` — 4-byte MIDI message (port id, status byte, data1, data2).
    Midi(u32),
    /// `T` / `F` — boolean.
    Bool(bool),
    /// `N` — nil.
    Nil,
    /// `I` — infinitum.
    Inf,
}

impl OscArg {
    /// The OSC type tag character for this value.
    pub fn type_tag(&self) -> char {
        match self {
            OscArg::Int(_) => 'i',
            OscArg::Float(_) => 'f',
            OscArg::String(_) => 's',
            OscArg::Blob(_) => 'b',
            OscArg::Long(_) => 'h',
            OscArg::Time(_) => 't',
            OscArg::Double(_) => 'd',
            OscArg::Symbol(_) => 'S',
            OscArg::Char(_) => 'c',
            OscArg::Color(_) => 'r',
            OscArg::Midi(_) => 'm',
            OscArg::Bool(true) => 'T',
            OscArg::Bool(false) => 'F',
            OscArg::Nil => 'N',
            OscArg::Inf => 'I',
        }
    }

    /// Numeric view for mapping: `Int`, `Long`, `Float`, `Double`, `Bool`.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            OscArg::Int(v) => Some(*v as f32),
            OscArg::Long(v) => Some(*v as f32),
            OscArg::Float(v) => Some(*v),
            OscArg::Double(v) => Some(*v as f32),
            OscArg::Bool(v) => Some(if *v { 1.0 } else { 0.0 }),
            _ => None,
        }
    }
}

/// An OSC message.
///
/// The type tag string (the leading `,` plus one tag per argument) is
/// derived from the arguments; access it with [`OscMessage::type_tags`].
#[derive(Debug, Clone, PartialEq)]
pub struct OscMessage {
    /// OSC address (e.g. `/track/1/volume`).
    pub address: String,
    /// Arguments in wire order.
    pub arguments: Vec<OscArg>,
}

impl OscMessage {
    /// Creates a message, validating the address.
    pub fn new(address: impl Into<String>, arguments: &[OscArg]) -> Result<Self, ControlError> {
        let address = address.into();
        validate_address(&address)?;
        Ok(Self {
            address,
            arguments: arguments.to_vec(),
        })
    }

    /// Creates a message without address validation (for internal use where
    /// the address is already known good).
    pub fn new_unchecked(address: impl Into<String>, arguments: Vec<OscArg>) -> Self {
        Self {
            address: address.into(),
            arguments,
        }
    }

    /// The type tag string including the leading comma, e.g. `,iff`.
    pub fn type_tags(&self) -> String {
        let mut tags = String::with_capacity(self.arguments.len() + 1);
        tags.push(',');
        for arg in &self.arguments {
            tags.push(arg.type_tag());
        }
        tags
    }

    /// Encodes the message into a new byte buffer.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(64);
        self.encode_into(&mut buf);
        buf
    }

    /// Encodes the message, appending to `buf`.
    pub fn encode_into(&self, buf: &mut Vec<u8>) {
        types::padded_into(buf, self.address.as_bytes());
        let mut tag_bytes = Vec::with_capacity(self.arguments.len() + 2);
        tag_bytes.push(b',');
        for arg in &self.arguments {
            tag_bytes.push(arg.type_tag() as u8);
        }
        types::padded_into(buf, &tag_bytes);
        for arg in &self.arguments {
            encode_arg(buf, arg);
        }
    }

    /// Decodes an owned message from bytes (allocates). For the
    /// real-time-safe path, use [`parse_osc_message`] instead.
    pub fn decode(data: &[u8]) -> Result<Self, ControlError> {
        parse_osc_message(data).map(|r| r.to_owned())
    }
}

fn encode_arg(buf: &mut Vec<u8>, arg: &OscArg) {
    match arg {
        OscArg::Int(v) => buf.extend_from_slice(&v.to_be_bytes()),
        OscArg::Float(v) => buf.extend_from_slice(&v.to_bits().to_be_bytes()),
        OscArg::String(s) | OscArg::Symbol(s) => types::padded_into(buf, s.as_bytes()),
        OscArg::Blob(b) => {
            buf.extend_from_slice(&(b.len() as u32).to_be_bytes());
            let pad = types::padded_len(b.len()) - b.len();
            buf.extend_from_slice(b);
            buf.extend(std::iter::repeat(0u8).take(pad));
        }
        OscArg::Long(v) => buf.extend_from_slice(&v.to_be_bytes()),
        OscArg::Time(v) => buf.extend_from_slice(&v.to_be_bytes()),
        OscArg::Double(v) => buf.extend_from_slice(&v.to_bits().to_be_bytes()),
        OscArg::Char(c) => buf.extend_from_slice(&(*c as u32).to_be_bytes()),
        OscArg::Color(v) | OscArg::Midi(v) => buf.extend_from_slice(&v.to_be_bytes()),
        OscArg::Bool(_) | OscArg::Nil | OscArg::Inf => {}
    }
}

/// Validates an outgoing OSC address: starts with `/`, printable ASCII,
/// no spaces.
pub fn validate_address(address: &str) -> Result<(), ControlError> {
    let bad = !address.starts_with('/')
        || address.is_empty()
        || !address.bytes().all(|b| (0x21..=0x7E).contains(&b));
    if bad {
        return Err(ControlError::InvalidAddress(address.to_string()));
    }
    Ok(())
}

/// A borrowed OSC argument produced by the zero-copy parser.
#[derive(Debug, Clone, PartialEq)]
pub enum OscArgRef<'a> {
    /// `i` — 32-bit integer.
    Int(i32),
    /// `f` — 32-bit float.
    Float(f32),
    /// `s` — OSC-string, borrowed.
    Str(&'a str),
    /// `b` — OSC-blob, borrowed (length prefix already consumed).
    Blob(&'a [u8]),
    /// `h` — 64-bit integer.
    Long(i64),
    /// `t` — OSC-timetag.
    Time(u64),
    /// `d` — 64-bit float.
    Double(f64),
    /// `S` — symbol, borrowed.
    Symbol(&'a str),
    /// `c` — ASCII character.
    Char(char),
    /// `r` — 32-bit RGBA color.
    Color(u32),
    /// `m` — 4-byte MIDI message.
    Midi(u32),
    /// `T` / `F` — boolean.
    Bool(bool),
    /// `N` — nil.
    Nil,
    /// `I` — infinitum.
    Inf,
}

impl OscArgRef<'_> {
    /// The OSC type tag character for this value.
    pub fn type_tag(&self) -> char {
        match self {
            OscArgRef::Int(_) => 'i',
            OscArgRef::Float(_) => 'f',
            OscArgRef::Str(_) => 's',
            OscArgRef::Blob(_) => 'b',
            OscArgRef::Long(_) => 'h',
            OscArgRef::Time(_) => 't',
            OscArgRef::Double(_) => 'd',
            OscArgRef::Symbol(_) => 'S',
            OscArgRef::Char(_) => 'c',
            OscArgRef::Color(_) => 'r',
            OscArgRef::Midi(_) => 'm',
            OscArgRef::Bool(true) => 'T',
            OscArgRef::Bool(false) => 'F',
            OscArgRef::Nil => 'N',
            OscArgRef::Inf => 'I',
        }
    }

    /// Numeric view for mapping.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            OscArgRef::Int(v) => Some(*v as f32),
            OscArgRef::Long(v) => Some(*v as f32),
            OscArgRef::Float(v) => Some(*v),
            OscArgRef::Double(v) => Some(*v as f32),
            OscArgRef::Bool(v) => Some(if *v { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// Converts to an owned argument (allocates).
    pub fn to_owned_arg(&self) -> OscArg {
        match self {
            OscArgRef::Int(v) => OscArg::Int(*v),
            OscArgRef::Float(v) => OscArg::Float(*v),
            OscArgRef::Str(s) => OscArg::String((*s).to_string()),
            OscArgRef::Blob(b) => OscArg::Blob(b.to_vec()),
            OscArgRef::Long(v) => OscArg::Long(*v),
            OscArgRef::Time(v) => OscArg::Time(*v),
            OscArgRef::Double(v) => OscArg::Double(*v),
            OscArgRef::Symbol(s) => OscArg::Symbol((*s).to_string()),
            OscArgRef::Char(c) => OscArg::Char(*c),
            OscArgRef::Color(v) => OscArg::Color(*v),
            OscArgRef::Midi(v) => OscArg::Midi(*v),
            OscArgRef::Bool(v) => OscArg::Bool(*v),
            OscArgRef::Nil => OscArg::Nil,
            OscArgRef::Inf => OscArg::Inf,
        }
    }
}

/// A parsed OSC message that borrows directly from the packet buffer.
///
/// Produced by [`parse_osc_message`]; performs no allocation, so it is safe
/// to construct and read from real-time audio threads.
#[derive(Debug, Clone, PartialEq)]
pub struct OscMessageRef<'a> {
    data: &'a [u8],
    address: &'a str,
    tags: &'a str,
    args_offset: usize,
}

impl<'a> OscMessageRef<'a> {
    /// The message address.
    pub fn address(&self) -> &'a str {
        self.address
    }

    /// The type tags without the leading comma (e.g. `iff`).
    pub fn type_tags(&self) -> &'a str {
        self.tags
    }

    /// The number of arguments.
    pub fn arg_count(&self) -> usize {
        self.tags.len()
    }

    /// Returns the `index`-th argument, parsed on demand from the borrowed
    /// bytes. Repeated access re-parses; for many passes use
    /// [`OscMessageRef::to_owned`] once off the real-time thread.
    pub fn arg(&self, index: usize) -> Option<OscArgRef<'a>> {
        if index >= self.tags.len() {
            return None;
        }
        let mut offset = self.args_offset;
        for (i, tag) in self.tags.bytes().enumerate() {
            let arg = read_arg(self.data, &mut offset, tag).ok()?;
            if i == index {
                return Some(arg);
            }
        }
        None
    }

    /// Iterates over all arguments.
    pub fn args(&self) -> impl Iterator<Item = OscArgRef<'a>> + '_ {
        (0..self.arg_count()).map(move |i| self.arg(i).expect("index < arg_count"))
    }

    /// Converts to an owned [`OscMessage`] (allocates).
    pub fn to_owned(&self) -> OscMessage {
        OscMessage {
            address: self.address.to_string(),
            arguments: self.args().map(|a| a.to_owned_arg()).collect(),
        }
    }
}

/// Parses an OSC message from raw bytes without allocating.
///
/// Real-time safe: the returned [`OscMessageRef`] borrows address, type
/// tags, string, and blob data directly from `data`.
pub fn parse_osc_message(data: &[u8]) -> Result<OscMessageRef<'_>, ControlError> {
    if data.first() != Some(&b'/') {
        return Err(ControlError::InvalidData(
            "OSC message must start with '/'".to_string(),
        ));
    }
    let (address_bytes, mut offset) = types::read_null_terminated(data, 0)?;
    let address = std::str::from_utf8(address_bytes)
        .map_err(|_| ControlError::InvalidData("address is not UTF-8".to_string()))?;
    validate_address(address)?;

    let (tag_bytes, after_tags) = types::read_null_terminated(data, offset)?;
    let tag_str = std::str::from_utf8(tag_bytes)
        .map_err(|_| ControlError::InvalidData("type tags are not ASCII".to_string()))?;
    let tags_str = tag_str
        .strip_prefix(',')
        .ok_or_else(|| ControlError::InvalidData("type tag string must start with ','".into()))?;
    offset = after_tags;

    let msg = OscMessageRef {
        data,
        address,
        tags: tags_str,
        args_offset: offset,
    };
    // Validate that every argument parses within the buffer up front.
    let mut check = offset;
    for tag in tags_str.bytes() {
        read_arg(data, &mut check, tag)?;
    }
    Ok(msg)
}

/// Reads one argument of type `tag` at `*offset`, advancing `*offset`.
fn read_arg<'a>(
    data: &'a [u8],
    offset: &mut usize,
    tag: u8,
) -> Result<OscArgRef<'a>, ControlError> {
    let arg = match tag {
        tags::INT => OscArgRef::Int(types::read_i32(data, *offset)?),
        tags::FLOAT => OscArgRef::Float(types::read_f32(data, *offset)?),
        tags::STRING | tags::SYMBOL => {
            let (bytes, next) = types::read_null_terminated(data, *offset)?;
            let s = std::str::from_utf8(bytes)
                .map_err(|_| ControlError::InvalidData("string argument is not UTF-8".into()))?;
            *offset = next;
            return Ok(if tag == tags::STRING {
                OscArgRef::Str(s)
            } else {
                OscArgRef::Symbol(s)
            });
        }
        tags::BLOB => {
            let size = types::read_u32(data, *offset)? as usize;
            let start = *offset + 4;
            let end = start
                .checked_add(size)
                .filter(|&e| e <= data.len())
                .ok_or_else(|| {
                    ControlError::InvalidData("blob length overruns packet".to_string())
                })?;
            *offset = start + types::padded_len(size);
            if *offset > data.len() {
                return Err(ControlError::InvalidData(
                    "blob padding overruns packet".into(),
                ));
            }
            return Ok(OscArgRef::Blob(&data[start..end]));
        }
        tags::LONG => OscArgRef::Long(types::read_i64(data, *offset)?),
        tags::TIME => OscArgRef::Time(types::read_u64(data, *offset)?),
        tags::DOUBLE => OscArgRef::Double(types::read_f64(data, *offset)?),
        tags::TRUE => OscArgRef::Bool(true),
        tags::FALSE => OscArgRef::Bool(false),
        tags::NIL => OscArgRef::Nil,
        tags::INFINITUM => OscArgRef::Inf,
        tags::CHAR => {
            let bits = types::read_u32(data, *offset)?;
            OscArgRef::Char(char::from_u32(bits).ok_or_else(|| {
                ControlError::InvalidData("invalid character argument".to_string())
            })?)
        }
        tags::COLOR => OscArgRef::Color(types::read_u32(data, *offset)?),
        tags::MIDI => OscArgRef::Midi(types::read_u32(data, *offset)?),
        other => {
            return Err(ControlError::InvalidData(format!(
                "unknown OSC type tag {:?}",
                other as char
            )))
        }
    };
    *offset += match tag {
        tags::LONG | tags::TIME | tags::DOUBLE => 8,
        _ => 4,
    };
    Ok(arg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(addr: &str, args: &[OscArg]) -> OscMessage {
        OscMessage::new(addr, args).unwrap()
    }

    #[test]
    fn encodes_spec_example_exactly() {
        // The OSC 1.0 spec's example: /oscillator/4/frequency with f32 440.
        let m = msg("/oscillator/4/frequency", &[OscArg::Float(440.0)]);
        let bytes = m.encode();
        let expected: &[u8] = &[
            b'/', b'o', b's', b'c', b'i', b'l', b'l', b'a', b't', b'o', b'r', b'/', b'4', b'/',
            b'f', b'r', b'e', b'q', b'u', b'e', b'n', b'c', b'y', 0, // address + NUL pad
            b',', b'f', 0, 0, // tags + pad
            0x43, 0xDC, 0x00, 0x00, // 440.0f32
        ];
        assert_eq!(bytes, expected);
    }

    #[test]
    fn encodes_int_example_exactly() {
        let m = msg("/foo", &[OscArg::Int(1000)]);
        let expected: &[u8] = &[
            b'/', b'f', b'o', b'o', 0, 0, 0, 0, //
            b',', b'i', 0, 0, //
            0x00, 0x00, 0x03, 0xE8,
        ];
        assert_eq!(m.encode(), expected);
    }

    #[test]
    fn roundtrips_all_arg_types() {
        let args = vec![
            OscArg::Int(-1),
            OscArg::Int(i32::MAX),
            OscArg::Float(-2.5),
            OscArg::Float(f32::MIN_POSITIVE),
            OscArg::String("héllo world".into()),
            OscArg::String(String::new()),
            OscArg::Blob(vec![1, 2, 3, 4, 5]),
            OscArg::Blob(vec![]),
            OscArg::Long(i64::MIN),
            OscArg::Time(0x8000_0000_0000_0001),
            OscArg::Double(std::f64::consts::PI),
            OscArg::Symbol("sym".into()),
            OscArg::Char('Z'),
            OscArg::Color(0x1122_3344),
            OscArg::Midi(0x90_3C_64_00),
            OscArg::Bool(true),
            OscArg::Bool(false),
            OscArg::Nil,
            OscArg::Inf,
        ];
        let m = msg("/round/trip", &args);
        let back = OscMessage::decode(&m.encode()).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn blob_padding_all_cases() {
        for size in 0..12usize {
            let m = msg("/b", &[OscArg::Blob(vec![0xAA; size])]);
            let back = OscMessage::decode(&m.encode()).unwrap();
            assert_eq!(back.arguments[0], OscArg::Blob(vec![0xAA; size]));
        }
    }

    #[test]
    fn type_tags_derive() {
        let m = msg(
            "/t",
            &[OscArg::Int(1), OscArg::Float(2.0), OscArg::Bool(true)],
        );
        assert_eq!(m.type_tags(), ",ifT");
        let empty = msg("/t", &[]);
        assert_eq!(empty.type_tags(), ",");
    }

    #[test]
    fn zero_copy_parser_borrows() {
        let bytes = msg("/vol/1", &[OscArg::Float(0.5), OscArg::Int(3)]).encode();
        let r = parse_osc_message(&bytes).unwrap();
        assert_eq!(r.address(), "/vol/1");
        assert_eq!(r.type_tags(), "fi");
        assert_eq!(r.arg_count(), 2);
        assert_eq!(r.arg(0), Some(OscArgRef::Float(0.5)));
        assert_eq!(r.arg(1), Some(OscArgRef::Int(3)));
        assert_eq!(r.arg(2), None);
        // Owned conversion matches owned decode.
        assert_eq!(r.to_owned(), OscMessage::decode(&bytes).unwrap());
    }

    #[test]
    fn rejects_malformed_input() {
        assert!(parse_osc_message(b"").is_err(), "empty");
        assert!(parse_osc_message(b"no-slash\0\0\0\0").is_err(), "no slash");
        assert!(parse_osc_message(b"/a\0").is_err(), "truncated address pad");

        // Truncated at every length.
        let good = msg("/x", &[OscArg::Float(1.0), OscArg::String("ab".into())]).encode();
        for len in 0..good.len() {
            assert!(
                OscMessage::decode(&good[..len]).is_err(),
                "expected failure at len {len}"
            );
        }

        // Missing tag comma.
        let bad: &[u8] = b"/x\0\0\0\0i\0\0\0\x01";
        assert!(OscMessage::decode(bad).is_err());

        // Unknown tag.
        let bad: &[u8] = b"/x\0\0\0\0,\x7F\0\0";
        assert!(OscMessage::decode(bad).is_err());
    }

    #[test]
    fn address_validation() {
        assert!(OscMessage::new("/ok", &[]).is_ok());
        assert!(OscMessage::new("no-leading-slash", &[]).is_err());
        assert!(OscMessage::new("", &[]).is_err());
        assert!(OscMessage::new("/has space", &[]).is_err());
        assert!(OscMessage::new("/non\u{1F600}ascii", &[]).is_err());
    }
}
