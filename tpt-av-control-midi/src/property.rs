//! MIDI-CI Property Exchange and Profile Configuration (M2-115 message
//! layer).
//!
//! Property Exchange travels inside MIDI-CI Universal SysEx messages
//! (`F0 7E <device> 0D <sub-id2> <version> <source MUID> <dest MUID> …`).
//! Payload layout after the two MUIDs:
//!
//! ```text
//! request-id (1 byte, 7-bit)
//! header-size (2 bytes, 14-bit little 7-bit groups: LSB then MSB)
//! header      (JSON text, 7-bit ASCII)
//! data        (7-bit bytes; present on data-carrying messages)
//! ```
//!
//! Long data sets are split across successive messages sharing the same
//! request id; the JSON header carries `chunkCount`/`chunkNumber`. Use
//! [`PropertyDataSetAssembler`] to reassemble, and [`header_field`] for
//! dependency-free header field extraction.

use crate::ci::MIDI_CI_SUB_ID1;
use crate::midi1::Midi1Message;
use crate::sysex::{self, UNIVERSAL_NON_REALTIME};
use tpt_av_control_utils::ControlError;

/// Get Property Data (initiator asks for a resource).
pub const PE_GET_PROPERTY_DATA: u8 = 0x30;
/// Get Property Data Reply (resource data).
pub const PE_GET_PROPERTY_DATA_REPLY: u8 = 0x31;
/// Set Property Data (initiator writes a resource).
pub const PE_SET_PROPERTY_DATA: u8 = 0x32;
/// Set Property Data Reply (status only).
pub const PE_SET_PROPERTY_DATA_REPLY: u8 = 0x33;
/// Subscribe Property Data (requests change notifications).
pub const PE_SUBSCRIBE_PROPERTY_DATA: u8 = 0x34;
/// Subscription Data (pushed data for an active subscription).
pub const PE_SUBSCRIPTION_DATA: u8 = 0x35;
/// Subscribe Property Data Reply (accepts/declines a subscription).
pub const PE_SUBSCRIBE_PROPERTY_DATA_REPLY: u8 = 0x36;
/// Notify Property Data Changed (header-only hint to re-fetch).
pub const PE_NOTIFY_PROPERTY_DATA_CHANGED: u8 = 0x37;

/// Profile Configuration sub-ID 2 values (M2-115).
pub mod profile {
    /// Profile Inquiry.
    pub const INQUIRY: u8 = 0x20;
    /// Profile Inquiry Reply.
    pub const INQUIRY_REPLY: u8 = 0x21;
    /// Set Profile On.
    pub const SET_ON: u8 = 0x22;
    /// Set Profile On Reply.
    pub const SET_ON_REPLY: u8 = 0x23;
    /// Set Profile Off.
    pub const SET_OFF: u8 = 0x24;
    /// Set Profile Off Reply.
    pub const SET_OFF_REPLY: u8 = 0x25;
}

/// PE status codes for the Set Property Data Reply.
pub mod status {
    /// Success.
    pub const OK: u8 = 0x00;
    /// Unspecified error.
    pub const UNSPECIFIED: u8 = 0x01;
    /// The requested resource does not exist.
    pub const RESOURCE_NOT_FOUND: u8 = 0x02;
    /// The resource is read-only.
    pub const READ_ONLY: u8 = 0x03;
    /// The provided data could not be processed.
    pub const PROCESS_ERROR: u8 = 0x04;
}

/// Whether a PE message carries a `data` section.
fn carries_data(sub_id2: u8) -> bool {
    matches!(
        sub_id2,
        PE_GET_PROPERTY_DATA_REPLY
            | PE_SET_PROPERTY_DATA
            | PE_SUBSCRIPTION_DATA
            | PE_NOTIFY_PROPERTY_DATA_CHANGED
    )
}

/// A MIDI-CI Property Exchange message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyExchangeMessage {
    /// Target device ID (0x7F = broadcast).
    pub device_id: u8,
    /// CI version.
    pub version: u8,
    /// MUID of the sender.
    pub source_muid: u32,
    /// MUID of the peer.
    pub destination_muid: u32,
    /// The PE sub-ID 2 (see the `PE_*` constants).
    pub sub_id2: u8,
    /// Correlates replies with requests; also keys chunked data sets.
    pub request_id: u8,
    /// Status code (only meaningful on Set Property Data Reply).
    pub status: u8,
    /// The JSON header text (7-bit ASCII).
    pub header: String,
    /// The data section, when this message carries one.
    pub data: Vec<u8>,
}

impl PropertyExchangeMessage {
    /// Encodes to a MIDI 1.0 SysEx message.
    pub fn to_midi1(&self) -> Result<Midi1Message, ControlError> {
        for &b in self.header.as_bytes() {
            if b >= 0x7F {
                return Err(ControlError::InvalidData(
                    "PE header must be 7-bit ASCII".into(),
                ));
            }
        }
        for &b in &self.data {
            if b >= 0x80 {
                return Err(ControlError::InvalidData(
                    "PE data must be 7-bit bytes".into(),
                ));
            }
        }
        let mut body: Vec<u8> = Vec::new();
        body.push(self.version & 0x7F);
        push_muid(&mut body, self.source_muid);
        push_muid(&mut body, self.destination_muid);
        body.push(self.request_id & 0x7F);
        if self.sub_id2 == PE_SET_PROPERTY_DATA_REPLY {
            body.push(self.status & 0x7F);
        }
        let header_len = self.header.len();
        if header_len > 0x3FFF {
            return Err(ControlError::OutOfRange {
                value: header_len as i64,
                min: 0,
                max: 0x3FFF,
            });
        }
        body.push((header_len & 0x7F) as u8);
        body.push(((header_len >> 7) & 0x7F) as u8);
        body.extend_from_slice(self.header.as_bytes());
        if carries_data(self.sub_id2) {
            body.extend_from_slice(&self.data);
        }
        Ok(sysex::sysex_universal(
            UNIVERSAL_NON_REALTIME,
            self.device_id & 0x7F,
            MIDI_CI_SUB_ID1,
            self.sub_id2,
            &body,
        ))
    }

    /// Parses a PE message from a MIDI 1.0 SysEx. Non-PE messages fail
    /// with [`ControlError::InvalidData`].
    pub fn from_midi1(message: &Midi1Message) -> Result<Self, ControlError> {
        let view = sysex::parse_sysex(message)?;
        if view.manufacturer != UNIVERSAL_NON_REALTIME || view.sub_id1 != Some(MIDI_CI_SUB_ID1) {
            return Err(ControlError::InvalidData("not a MIDI-CI message".into()));
        }
        let sub_id2 = view.sub_id2.unwrap_or(0xFF);
        if !(PE_GET_PROPERTY_DATA..=PE_NOTIFY_PROPERTY_DATA_CHANGED).contains(&sub_id2) {
            return Err(ControlError::InvalidData(format!(
                "not a Property Exchange sub-id2 ({sub_id2:#04x})"
            )));
        }
        let payload = view.payload;
        if payload.len() < 11 {
            return Err(ControlError::InvalidData("PE body too short".into()));
        }
        let version = payload[0] & 0x7F;
        let source_muid = take_muid(&payload[1..5]);
        let destination_muid = take_muid(&payload[5..9]);
        let mut cursor = 9;
        let request_id = payload[cursor] & 0x7F;
        cursor += 1;
        let mut status = 0;
        if sub_id2 == PE_SET_PROPERTY_DATA_REPLY {
            status = payload[cursor] & 0x7F;
            cursor += 1;
        }
        if payload.len() < cursor + 2 {
            return Err(ControlError::InvalidData("PE header size missing".into()));
        }
        let header_len =
            usize::from(payload[cursor] & 0x7F) | (usize::from(payload[cursor + 1] & 0x7F) << 7);
        cursor += 2;
        let header_end = cursor
            .checked_add(header_len)
            .filter(|&e| e <= payload.len())
            .ok_or_else(|| ControlError::InvalidData("PE header overruns message".into()))?;
        let header = String::from_utf8_lossy(&payload[cursor..header_end]).into_owned();
        cursor = header_end;
        let data = if carries_data(sub_id2) {
            payload[cursor..].to_vec()
        } else {
            Vec::new()
        };
        Ok(Self {
            device_id: view.device_id.unwrap_or(0x7F),
            version,
            source_muid,
            destination_muid,
            sub_id2,
            request_id,
            status,
            header,
            data,
        })
    }
}

fn push_muid(buf: &mut Vec<u8>, muid: u32) {
    let muid = muid & 0x0FFF_FFFF;
    buf.push((muid & 0x7F) as u8);
    buf.push(((muid >> 7) & 0x7F) as u8);
    buf.push(((muid >> 14) & 0x7F) as u8);
    buf.push(((muid >> 21) & 0x7F) as u8);
}

fn take_muid(bytes: &[u8]) -> u32 {
    u32::from(bytes[0])
        | u32::from(bytes[1]) << 7
        | u32::from(bytes[2]) << 14
        | u32::from(bytes[3]) << 21
}

/// Extracts a field from a flat JSON header without a JSON dependency:
/// `header_field(r#"{"resource":"/cap/device"}"#, "resource")` returns
/// `Some("/cap/device")`. Handles string and numeric values; nested
/// objects and escape sequences are out of scope.
/// # Examples
///
/// ```
/// use tpt_av_control_midi::header_field;
/// let header = r#"{"resource":"/cap/device","chunkCount":3}"#;
/// assert_eq!(header_field(header, "resource").as_deref(), Some("/cap/device"));
/// assert_eq!(header_field(header, "chunkCount").as_deref(), Some("3"));
/// ```
pub fn header_field(header: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let key_pos = header.find(&needle)?;
    let after_key = &header[key_pos + needle.len()..];
    let colon = after_key.find(':')?;
    let value = after_key[colon + 1..].trim_start();
    if let Some(rest) = value.strip_prefix('"') {
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    } else {
        let end = value
            .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
            .unwrap_or(value.len());
        let token = value[..end].trim();
        (!token.is_empty()).then(|| token.to_string())
    }
}

/// Extracts a numeric header field.
pub fn header_field_num(header: &str, key: &str) -> Option<usize> {
    header_field(header, key)?.parse().ok()
}

/// Reassembles chunked PE data sets (Get Property Data Reply,
/// Subscription Data, Set Property Data) that share a request id.
///
/// Streams are capped at [`PropertyDataSetAssembler::MAX_CHUNKS`] chunks
/// and [`PropertyDataSetAssembler::MAX_BYTES`] total bytes.
#[derive(Debug, Default)]
pub struct PropertyDataSetAssembler {
    /// `(source muid, request id)` of the in-flight stream, if any.
    key: Option<(u32, u8)>,
    /// Chunk number → bytes (chunk numbers arrive via the JSON header).
    chunks: Vec<(usize, Vec<u8>)>,
    /// Expected total chunk count (from `chunkCount`), when announced.
    count: Option<usize>,
}

impl PropertyDataSetAssembler {
    /// Maximum chunks accepted per stream.
    pub const MAX_CHUNKS: usize = 128;
    /// Maximum total bytes accepted per stream (1 MiB).
    pub const MAX_BYTES: usize = 1024 * 1024;

    /// Feeds one PE message. Returns `(header, full data)` when the data
    /// set is complete — including the trivial single-message case with
    /// no chunk fields.
    pub fn feed(
        &mut self,
        message: &PropertyExchangeMessage,
    ) -> Result<Option<(String, Vec<u8>)>, ControlError> {
        if !carries_data(message.sub_id2) {
            return Ok(None);
        }
        let this_key = (message.source_muid, message.request_id);
        match self.key {
            None => {
                self.key = Some(this_key);
                self.chunks.clear();
                self.count = None;
            }
            Some(k) if k != this_key => {
                // A new stream started before the old one finished: reset.
                self.key = Some(this_key);
                self.chunks.clear();
                self.count = None;
            }
            Some(_) => {}
        }

        let chunk_number = header_field_num(&message.header, "chunkNumber").unwrap_or(0);
        let chunk_count = header_field_num(&message.header, "chunkCount");

        if chunk_count.is_none() && self.chunks.is_empty() && chunk_number == 0 {
            // Single-message data set.
            let header = message.header.clone();
            return Ok(Some((header, message.data.clone())));
        }

        if let Some(c) = chunk_count {
            self.count = Some(c);
        }
        if self.chunks.len() >= Self::MAX_CHUNKS {
            self.reset(&this_key);
            return Err(ControlError::InvalidData(
                "PE data set exceeded chunk limit".into(),
            ));
        }
        let total = self.total_len().saturating_add(message.data.len());
        if total > Self::MAX_BYTES {
            self.reset(&this_key);
            return Err(ControlError::InvalidData(
                "PE data set exceeded byte limit".into(),
            ));
        }
        self.chunks.push((chunk_number, message.data.clone()));

        // Complete when every announced chunk has arrived.
        if let Some(count) = self.count {
            if self.chunks.len() >= count && count > 0 {
                self.chunks.sort_by_key(|(n, _)| *n);
                let data: Vec<u8> = self.chunks.drain(..).flat_map(|(_, d)| d).collect();
                self.key = None;
                self.count = None;
                return Ok(Some((message.header.clone(), data)));
            }
        }
        Ok(None)
    }

    fn total_len(&self) -> usize {
        self.chunks.iter().map(|(_, d)| d.len()).sum()
    }

    fn reset(&mut self, key: &(u32, u8)) {
        self.key = Some(*key);
        self.chunks.clear();
        self.count = None;
    }
}

/// A MIDI-CI Profile Configuration message (M2-115).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileConfigMessage {
    /// Target device ID (0x7F = broadcast).
    pub device_id: u8,
    /// CI version.
    pub version: u8,
    /// MUID of the sender.
    pub source_muid: u32,
    /// MUID of the peer.
    pub destination_muid: u32,
    /// The profile sub-ID 2 (see [`profile`] constants).
    pub sub_id2: u8,
    /// Profiles listed by inquiries: `[id MSB, id LSB, bank]` triplets.
    pub profiles: Vec<[u8; 3]>,
    /// The single profile targeted by on/off switches: `[id MSB, id LSB]`.
    pub profile: [u8; 2],
    /// Target addresses (function block / channel) for on/off switches.
    pub addresses: Vec<u16>,
}

impl ProfileConfigMessage {
    /// Encodes to a MIDI 1.0 SysEx message.
    pub fn to_midi1(&self) -> Result<Midi1Message, ControlError> {
        let mut body: Vec<u8> = Vec::new();
        body.push(self.version & 0x7F);
        push_muid(&mut body, self.source_muid);
        push_muid(&mut body, self.destination_muid);
        match self.sub_id2 {
            profile::INQUIRY | profile::INQUIRY_REPLY => {
                let count = self.profiles.len();
                if count > 0x3FFF {
                    return Err(ControlError::OutOfRange {
                        value: count as i64,
                        min: 0,
                        max: 0x3FFF,
                    });
                }
                body.push((count & 0x7F) as u8);
                body.push(((count >> 7) & 0x7F) as u8);
                for p in &self.profiles {
                    body.push(p[0] & 0x7F);
                    body.push(p[1] & 0x7F);
                    body.push(p[2] & 0x7F);
                }
            }
            profile::SET_ON | profile::SET_ON_REPLY | profile::SET_OFF | profile::SET_OFF_REPLY => {
                body.push(self.profile[0] & 0x7F);
                body.push(self.profile[1] & 0x7F);
                let count = self.addresses.len();
                if count > 127 {
                    return Err(ControlError::OutOfRange {
                        value: count as i64,
                        min: 0,
                        max: 127,
                    });
                }
                body.push(0x00); // address type: 7-bit channel/block ids
                body.push(count as u8);
                for &a in &self.addresses {
                    body.push((a & 0x7F) as u8);
                    body.push(((a >> 7) & 0x7F) as u8);
                }
            }
            other => {
                return Err(ControlError::InvalidData(format!(
                    "not a profile sub-id2 ({other:#04x})"
                )))
            }
        }
        Ok(sysex::sysex_universal(
            UNIVERSAL_NON_REALTIME,
            self.device_id & 0x7F,
            MIDI_CI_SUB_ID1,
            self.sub_id2,
            &body,
        ))
    }

    /// Parses a Profile Configuration message from a MIDI 1.0 SysEx.
    pub fn from_midi1(message: &Midi1Message) -> Result<Self, ControlError> {
        let view = sysex::parse_sysex(message)?;
        if view.manufacturer != UNIVERSAL_NON_REALTIME || view.sub_id1 != Some(MIDI_CI_SUB_ID1) {
            return Err(ControlError::InvalidData("not a MIDI-CI message".into()));
        }
        let sub_id2 = view.sub_id2.unwrap_or(0xFF);
        if !(profile::INQUIRY..=profile::SET_OFF_REPLY).contains(&sub_id2) {
            return Err(ControlError::InvalidData(format!(
                "not a profile sub-id2 ({sub_id2:#04x})"
            )));
        }
        let payload = view.payload;
        if payload.len() < 9 {
            return Err(ControlError::InvalidData("profile body too short".into()));
        }
        let version = payload[0] & 0x7F;
        let source_muid = take_muid(&payload[1..5]);
        let destination_muid = take_muid(&payload[5..9]);
        let mut out = Self {
            device_id: view.device_id.unwrap_or(0x7F),
            version,
            source_muid,
            destination_muid,
            sub_id2,
            profiles: Vec::new(),
            profile: [0, 0],
            addresses: Vec::new(),
        };
        let rest = &payload[9..];
        match sub_id2 {
            profile::INQUIRY | profile::INQUIRY_REPLY => {
                if rest.len() < 2 {
                    return Err(ControlError::InvalidData("profile count missing".into()));
                }
                let count = usize::from(rest[0] & 0x7F) | (usize::from(rest[1] & 0x7F) << 7);
                let bytes = &rest[2..];
                if bytes.len() < count * 3 {
                    return Err(ControlError::InvalidData("profile list truncated".into()));
                }
                out.profiles = bytes[..count * 3]
                    .chunks(3)
                    .map(|c| [c[0] & 0x7F, c[1] & 0x7F, c[2] & 0x7F])
                    .collect();
            }
            _ => {
                if rest.len() < 4 {
                    return Err(ControlError::InvalidData("profile switch truncated".into()));
                }
                out.profile = [rest[0] & 0x7F, rest[1] & 0x7F];
                let count = usize::from(rest[3] & 0x7F);
                let bytes = &rest[4..];
                if bytes.len() < count * 2 {
                    return Err(ControlError::InvalidData("address list truncated".into()));
                }
                out.addresses = bytes[..count * 2]
                    .chunks(2)
                    .map(|c| u16::from(c[0] & 0x7F) | u16::from(c[1] & 0x7F) << 7)
                    .collect();
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi1::parse_midi1;

    fn pe(sample: u8) -> PropertyExchangeMessage {
        PropertyExchangeMessage {
            device_id: 0x7F,
            version: 0x01,
            source_muid: 0x0F1E_2D3C,
            destination_muid: 0x123_4567,
            sub_id2: sample,
            request_id: 42,
            status: status::OK,
            header: r#"{"resource":"/cap/device","mediaType":"application/json"}"#.into(),
            data: vec![1, 2, 3, 4, 5],
        }
    }

    #[test]
    fn pe_roundtrips_all_variants() {
        for sub_id2 in [
            PE_GET_PROPERTY_DATA,
            PE_GET_PROPERTY_DATA_REPLY,
            PE_SET_PROPERTY_DATA,
            PE_SET_PROPERTY_DATA_REPLY,
            PE_SUBSCRIBE_PROPERTY_DATA,
            PE_SUBSCRIPTION_DATA,
            PE_SUBSCRIBE_PROPERTY_DATA_REPLY,
            PE_NOTIFY_PROPERTY_DATA_CHANGED,
        ] {
            let mut message = pe(sub_id2);
            if sub_id2 == PE_SET_PROPERTY_DATA_REPLY {
                message.status = status::RESOURCE_NOT_FOUND;
            }
            if !carries_data(sub_id2) {
                message.data.clear();
            }
            let encoded = message.to_midi1().unwrap();
            let parsed = parse_midi1(&encoded.to_bytes()).unwrap();
            let back = PropertyExchangeMessage::from_midi1(&parsed).unwrap();
            assert_eq!(back, message, "sub-id2 {sub_id2:#04x}");
        }
    }

    #[test]
    fn pe_header_size_is_14_bit() {
        let mut message = pe(PE_GET_PROPERTY_DATA);
        message.header = "x".repeat(200); // needs both 7-bit groups
        let encoded = message.to_midi1().unwrap();
        let parsed = parse_midi1(&encoded.to_bytes()).unwrap();
        let back = PropertyExchangeMessage::from_midi1(&parsed).unwrap();
        assert_eq!(back.header.len(), 200);
    }

    #[test]
    fn pe_rejects_8bit_payloads() {
        let mut message = pe(PE_GET_PROPERTY_DATA_REPLY);
        message.data.push(0x80);
        assert!(message.to_midi1().is_err());
        message.data.pop();
        message.header.push('\u{80}');
        assert!(message.to_midi1().is_err());
    }

    #[test]
    fn pe_rejects_non_pe_messages() {
        let other = sysex::identity_request(0x7F);
        assert!(PropertyExchangeMessage::from_midi1(&other).is_err());
    }

    #[test]
    fn header_field_extracts_strings_and_numbers() {
        let header = r#"{"resource":"/cap/device","chunkCount":3,"enc":"utf-8"}"#;
        assert_eq!(
            header_field(header, "resource").as_deref(),
            Some("/cap/device")
        );
        assert_eq!(header_field_num(header, "chunkCount"), Some(3));
        assert_eq!(header_field(header, "missing"), None);
        assert_eq!(header_field_num(header, "resource"), None);
    }

    #[test]
    fn chunked_dataset_reassembles_in_order() {
        let mut asm = PropertyDataSetAssembler::default();
        let mk = |chunk: usize, bytes: Vec<u8>| PropertyExchangeMessage {
            device_id: 0x7F,
            version: 1,
            source_muid: 7,
            destination_muid: 9,
            sub_id2: PE_GET_PROPERTY_DATA_REPLY,
            request_id: 5,
            status: 0,
            header: format!(r#"{{"resource":"/big","chunkCount":3,"chunkNumber":{chunk}}}"#),
            data: bytes,
        };

        assert_eq!(asm.feed(&mk(2, vec![3, 3])).unwrap(), None);
        assert_eq!(asm.feed(&mk(0, vec![1, 1])).unwrap(), None);
        let (header, data) = asm.feed(&mk(1, vec![2, 2])).unwrap().unwrap();
        assert!(header.contains("\"resource\":\"/big\""));
        assert_eq!(data, vec![1, 1, 2, 2, 3, 3]);
        // Assembler is ready for the next stream.
        let single = PropertyExchangeMessage {
            header: r#"{"resource":"/small"}"#.into(),
            data: vec![9],
            ..pe(PE_GET_PROPERTY_DATA_REPLY)
        };
        let (_, data) = asm.feed(&single).unwrap().unwrap();
        assert_eq!(data, vec![9]);
    }

    #[test]
    fn chunk_assembler_caps_stream() {
        let mut asm = PropertyDataSetAssembler::default();
        // Stream of MAX_CHUNKS+1 un-terminated chunks must error, not grow.
        for i in 0..=PropertyDataSetAssembler::MAX_CHUNKS {
            let message = PropertyExchangeMessage {
                header: format!(r#"{{"chunkNumber":{i},"chunkCount":99999}}"#),
                data: vec![0xAA; 1024],
                ..pe(PE_GET_PROPERTY_DATA_REPLY)
            };
            let result = asm.feed(&message);
            if i >= PropertyDataSetAssembler::MAX_CHUNKS {
                assert!(result.is_err(), "cap should trigger at chunk {i}");
                return;
            }
            assert!(result.unwrap().is_none());
        }
    }

    #[test]
    fn profile_config_roundtrips() {
        let inquiry = ProfileConfigMessage {
            device_id: 0x7F,
            version: 1,
            source_muid: 100,
            destination_muid: 200,
            sub_id2: profile::INQUIRY,
            profiles: vec![[0x00, 0x10, 0x00], [0x01, 0x00, 0x01]],
            profile: [0, 0],
            addresses: vec![],
        };
        let encoded = inquiry.to_midi1().unwrap();
        let parsed = parse_midi1(&encoded.to_bytes()).unwrap();
        assert_eq!(ProfileConfigMessage::from_midi1(&parsed).unwrap(), inquiry);

        let on = ProfileConfigMessage {
            sub_id2: profile::SET_ON,
            profiles: vec![],
            profile: [0x00, 0x10],
            addresses: vec![1, 0x40 | 3, 127],
            ..inquiry
        };
        let encoded = on.to_midi1().unwrap();
        let parsed = parse_midi1(&encoded.to_bytes()).unwrap();
        assert_eq!(ProfileConfigMessage::from_midi1(&parsed).unwrap(), on);
    }

    #[test]
    fn profile_rejects_non_profile() {
        let other = sysex::gm_system_on(0x7F);
        assert!(ProfileConfigMessage::from_midi1(&other).is_err());
    }
}
