//! System Exclusive helpers: universal message builders and chunking.

use crate::midi1::Midi1Message;
use tpt_av_control_utils::ControlError;

/// Universal System Exclusive sub-ID 1: Sample Dump.
pub const SAMPLE_DUMP: u8 = 0x01;
/// Universal System Exclusive sub-ID 1: Sample Dump Request.
pub const SAMPLE_DUMP_REQUEST: u8 = 0x02;
/// Universal System Exclusive sub-ID 1: MIDI Time Code.
pub const MIDI_TIME_CODE: u8 = 0x04;
/// Universal System Exclusive sub-ID 1: Show Control (MSC).
pub const SHOW_CONTROL: u8 = 0x02;
/// Universal System Exclusive sub-ID 1: Notation Information.
pub const NOTATION_INFORMATION: u8 = 0x03;
/// Universal System Exclusive sub-ID 1: Device Control.
pub const DEVICE_CONTROL: u8 = 0x04;
/// Universal System Exclusive sub-ID 1: Real-Time MTC Cueing.
pub const REALTIME_MTC_CUEING: u8 = 0x05;
/// Universal System Exclusive sub-ID 1: MIDI Machine Control.
pub const MMC_COMMAND: u8 = 0x06;
/// Universal System Exclusive sub-ID 1: MIDI Machine Control reply.
pub const MMC_RESPONSE: u8 = 0x07;
/// Universal System Exclusive sub-ID 1: MIDI Tuning Standard.
pub const MIDI_TUNING_STANDARD: u8 = 0x08;
/// Universal System Exclusive sub-ID 1: Controller Destination Setting.
pub const CONTROLLER_DESTINATION: u8 = 0x09;
/// Universal System Exclusive sub-ID 1: Key-Based Instrument Control.
pub const KEY_BASED_INSTRUMENT_CONTROL: u8 = 0x0A;
/// Universal System Exclusive sub-ID 1: Scale/Octave Tuning.
pub const SCALE_OCTAVE_TUNING: u8 = 0x0B;

/// Manufacturer ID for Universal Non-Real-Time messages.
pub const UNIVERSAL_NON_REALTIME: u8 = 0x7E;
/// Manufacturer ID for Universal Real-Time messages.
pub const UNIVERSAL_REALTIME: u8 = 0x7F;
/// Non-commercial (educational) manufacturer ID.
pub const NON_COMMERCIAL: u8 = 0x7D;

/// General MIDI System On (turn on GM mode).
pub fn gm_system_on(device_id: u8) -> Midi1Message {
    sysex_universal(UNIVERSAL_NON_REALTIME, device_id, 0x09, 0x01, &[])
}

/// General MIDI System Off.
pub fn gm_system_off(device_id: u8) -> Midi1Message {
    sysex_universal(UNIVERSAL_NON_REALTIME, device_id, 0x09, 0x02, &[])
}

/// MIDI Identity Request (asks the device for its Identity Reply).
pub fn identity_request(device_id: u8) -> Midi1Message {
    sysex_universal(UNIVERSAL_NON_REALTIME, device_id, 0x06, 0x01, &[])
}

/// Builds a Universal System Exclusive message:
pub fn sysex_universal(
    manufacturer: u8,
    device_id: u8,
    sub_id1: u8,
    sub_id2: u8,
    payload: &[u8],
) -> Midi1Message {
    let mut bytes = Vec::with_capacity(payload.len() + 7);
    bytes.push(0xF0);
    bytes.push(manufacturer);
    bytes.push(device_id & 0x7F);
    bytes.push(sub_id1 & 0x7F);
    bytes.push(sub_id2 & 0x7F);
    for &b in payload {
        bytes.push(b & 0x7F);
    }
    bytes.push(0xF7);
    Midi1Message::SystemExclusive(bytes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Parsed view of a System Exclusive message.
pub struct SysExView<'a> {
    /// Manufacturer ID: 1 byte, or the first byte of a 3-byte ID when
    /// (per the MMA 3-byte ID scheme).
    pub manufacturer: u8,
    /// For universal messages: device ID.
    pub device_id: Option<u8>,
    /// For universal messages: sub-ID 1.
    pub sub_id1: Option<u8>,
    /// For universal messages: sub-ID 2.
    pub sub_id2: Option<u8>,
    /// Remaining payload bytes.
    pub payload: &'a [u8],
    /// Whether this is a universal message (0x7E / 0x7F manufacturer).
    pub universal: bool,
}

/// Splits a `SystemExclusive` message into its header fields and payload.
pub fn parse_sysex(message: &Midi1Message) -> Result<SysExView<'_>, ControlError> {
    let bytes = match message {
        Midi1Message::SystemExclusive(bytes) => bytes,
        other => {
            return Err(ControlError::InvalidData(format!(
                "not a SysEx message: {other:?}"
            )))
        }
    };
    if bytes.len() < 2 || bytes[0] != 0xF0 || *bytes.last().unwrap() != 0xF7 {
        return Err(ControlError::InvalidData(
            "SysEx must start with F0 and end with F7".into(),
        ));
    }
    let manufacturer = bytes[1];
    let universal = manufacturer == UNIVERSAL_NON_REALTIME || manufacturer == UNIVERSAL_REALTIME;
    let mut view = SysExView {
        manufacturer,
        device_id: None,
        sub_id1: None,
        sub_id2: None,
        payload: &bytes[2..bytes.len() - 1],
        universal,
    };
    if universal && view.payload.len() >= 3 {
        view.device_id = Some(view.payload[0]);
        view.sub_id1 = Some(view.payload[1]);
        view.sub_id2 = Some(view.payload[2]);
        view.payload = &view.payload[3..];
    }
    Ok(view)
}

/// each wrapped as a complete `F0 .. F7` message (as required by some
pub fn chunk_sysex(data: &[u8], chunk_size: usize) -> Vec<Midi1Message> {
    let chunk_size = chunk_size.max(1);
    data.chunks(chunk_size)
        .map(|chunk| {
            let mut bytes = Vec::with_capacity(chunk.len() + 2);
            bytes.push(0xF0);
            bytes.extend_from_slice(chunk);
            bytes.push(0xF7);
            Midi1Message::SystemExclusive(bytes)
        })
        .collect()
}

/// Validates that every byte is a legal 7-bit data byte.
pub fn validate_7bit(data: &[u8]) -> Result<(), ControlError> {
    for &b in data {
        if b >= 0x80 {
            return Err(ControlError::InvalidData(format!(
                "byte {b:#04x} is not a 7-bit data byte"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi1::parse_midi1;

    #[test]
    fn identity_request_encoding() {
        let msg = identity_request(0x7F);
        match msg {
            Midi1Message::SystemExclusive(bytes) => {
                assert_eq!(bytes, vec![0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]);
            }
            other => panic!("expected sysex: {other:?}"),
        }
    }

    #[test]
    fn universal_view() {
        let msg = sysex_universal(UNIVERSAL_REALTIME, 0x10, MIDI_TIME_CODE, 0x01, &[3]);
        let view = parse_sysex(&msg).unwrap();
        assert!(view.universal);
        assert_eq!(view.device_id, Some(0x10));
        assert_eq!(view.sub_id1, Some(MIDI_TIME_CODE));
        assert_eq!(view.sub_id2, Some(0x01));
        assert_eq!(view.payload, &[3]);
    }

    #[test]
    fn non_universal_view() {
        let raw = [0xF0, 0x41, 0x10, 0xF7];
        let msg = parse_midi1(&raw).unwrap();
        let view = parse_sysex(&msg).unwrap();
        assert!(!view.universal);
        assert_eq!(view.device_id, None);
        assert_eq!(view.payload, &[0x10]);
    }

    #[test]
    fn chunking() {
        let chunks = chunk_sysex(&[1, 2, 3, 4, 5], 2);
        assert_eq!(chunks.len(), 3);
        for chunk in &chunks {
            assert!(parse_sysex(chunk).is_ok());
        }
    }

    #[test]
    fn rejects_bad_sysex() {
        assert!(parse_sysex(&Midi1Message::TimingClock).is_err());
        let bad = Midi1Message::SystemExclusive(vec![0xF0, 0x01]);
        assert!(parse_sysex(&bad).is_err());
    }
}
