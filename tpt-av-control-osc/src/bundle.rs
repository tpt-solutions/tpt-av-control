//! OSC bundles: timestamped collections of messages and nested bundles.

use crate::message::OscMessage;
use crate::types;
use tpt_av_control_utils::ControlError;

/// The `#bundle` marker that begins every bundle.
const BUNDLE_MARKER: &[u8] = b"#bundle";

/// Maximum nesting depth accepted when parsing bundles (defends against
/// stack exhaustion from hostile packets).
pub const MAX_BUNDLE_DEPTH: u8 = 32;

/// An element of an OSC bundle: either a message or a nested bundle.
#[derive(Debug, Clone, PartialEq)]
pub enum OscPacket {
    /// A complete OSC message.
    Message(OscMessage),
    /// A nested OSC bundle.
    Bundle(OscBundle),
}

/// An OSC bundle (collection of packets with a timestamp).
///
/// `timestamp` is the 64-bit NTP time tag; `None` means "immediate"
/// (wire value `1`).
#[derive(Debug, Clone, PartialEq)]
pub struct OscBundle {
    /// NTP timestamp; `None` = immediate.
    pub timestamp: Option<u64>,
    /// The packets carried by the bundle, in wire order.
    pub elements: Vec<OscPacket>,
}

impl OscBundle {
    /// Creates a bundle with the given NTP timestamp (`None` = immediate).
    pub fn new(timestamp: Option<u64>, elements: Vec<OscPacket>) -> Self {
        Self {
            timestamp,
            elements,
        }
    }

    /// Encodes the bundle into a new byte buffer.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(64);
        self.encode_into(&mut buf);
        buf
    }

    /// Encodes the bundle, appending to `buf`.
    pub fn encode_into(&self, buf: &mut Vec<u8>) {
        types::padded_into(buf, BUNDLE_MARKER);
        // OSC's "immediate" time tag is the wire value 1.
        buf.extend_from_slice(&self.timestamp.unwrap_or(1).to_be_bytes());
        for element in &self.elements {
            let size: u32 = match element {
                OscPacket::Message(m) => m.encode().len() as u32,
                OscPacket::Bundle(b) => b.encode().len() as u32,
            };
            buf.extend_from_slice(&size.to_be_bytes());
            match element {
                OscPacket::Message(m) => m.encode_into(buf),
                OscPacket::Bundle(b) => b.encode_into(buf),
            }
        }
    }

    /// Decodes a bundle from bytes.
    pub fn decode(data: &[u8]) -> Result<Self, ControlError> {
        parse_bundle(data)
    }
}

/// Parses a bundle from raw bytes.
pub fn parse_bundle(data: &[u8]) -> Result<OscBundle, ControlError> {
    parse_bundle_at(data, 0)
}

fn parse_bundle_at(data: &[u8], depth: u8) -> Result<OscBundle, ControlError> {
    if depth >= MAX_BUNDLE_DEPTH {
        return Err(ControlError::InvalidData(
            "bundle nesting too deep".to_string(),
        ));
    }
    let (marker, mut offset) = types::read_null_terminated(data, 0)?;
    if marker != BUNDLE_MARKER {
        return Err(ControlError::InvalidData(
            "bundle must begin with '#bundle'".to_string(),
        ));
    }
    let timetag = types::read_u64(data, offset)?;
    offset += 8;
    let timestamp = (timetag != 1).then_some(timetag);

    let mut elements = Vec::new();
    while offset < data.len() {
        let size = types::read_u32(data, offset)? as usize;
        offset += 4;
        let end = offset
            .checked_add(size)
            .filter(|&e| e <= data.len())
            .ok_or_else(|| ControlError::InvalidData("bundle element overruns packet".into()))?;
        let element = &data[offset..end];
        elements.push(parse_packet_at(element, depth + 1)?);
        offset = end;
    }
    Ok(OscBundle {
        timestamp,
        elements,
    })
}

/// Parses a bundle element (message or nested bundle).
pub fn parse_packet(data: &[u8]) -> Result<OscPacket, ControlError> {
    parse_packet_at(data, 0)
}

fn parse_packet_at(data: &[u8], depth: u8) -> Result<OscPacket, ControlError> {
    if data.first() == Some(&b'#') {
        Ok(OscPacket::Bundle(parse_bundle_at(data, depth)?))
    } else {
        Ok(OscPacket::Message(
            crate::message::parse_osc_message(data)?.to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::OscArg;

    #[test]
    fn roundtrips_nested_bundles() {
        let leaf = OscPacket::Message(OscMessage::new("/leaf", &[OscArg::Float(1.0)]).unwrap());
        let inner = OscPacket::Bundle(OscBundle::new(Some(42), vec![leaf]));
        let outer = OscBundle::new(
            Some(0x0000_0001_0000_0000),
            vec![
                OscPacket::Message(OscMessage::new("/a", &[]).unwrap()),
                inner,
            ],
        );
        let bytes = outer.encode();
        let back = OscBundle::decode(&bytes).unwrap();
        assert_eq!(back, outer);
        assert_eq!(back.timestamp, Some(0x0000_0001_0000_0000));
    }

    #[test]
    fn immediate_encodes_as_timetag_one() {
        let b = OscBundle::new(None, vec![]);
        let bytes = b.encode();
        // #bundle\0\0 (8 bytes) + timetag 1 (8 bytes).
        assert_eq!(&bytes[8..16], &1u64.to_be_bytes());
        let back = OscBundle::decode(&bytes).unwrap();
        assert_eq!(back.timestamp, None);
    }

    #[test]
    fn empty_bundle_roundtrips() {
        let b = OscBundle::new(Some(7), vec![]);
        assert_eq!(OscBundle::decode(&b.encode()).unwrap(), b);
    }

    #[test]
    fn rejects_bad_marker() {
        let msg = OscMessage::new("/not-a-bundle", &[]).unwrap().encode();
        assert!(parse_bundle(&msg).is_err());
        assert!(parse_bundle(&[0u8; 8]).is_err());
    }

    #[test]
    fn rejects_truncated_elements() {
        let b = OscBundle::new(
            None,
            vec![OscPacket::Message(
                OscMessage::new("/m", &[OscArg::Int(1)]).unwrap(),
            )],
        );
        let good = b.encode();
        // Prefixes shorter than marker+timetag (16 bytes) must fail; an
        // empty-element bundle at exactly 16 bytes is legitimately valid.
        for len in 0..16usize {
            assert!(OscBundle::decode(&good[..len]).is_err(), "len {len}");
        }
        for len in 17..good.len() {
            assert!(OscBundle::decode(&good[..len]).is_err(), "len {len}");
        }
        // Element size lying about its length.
        let mut hostile = good.clone();
        let elem_size_pos = 16usize;
        hostile[elem_size_pos..elem_size_pos + 4].copy_from_slice(&4096u32.to_be_bytes());
        assert!(OscBundle::decode(&hostile).is_err());
    }
}
