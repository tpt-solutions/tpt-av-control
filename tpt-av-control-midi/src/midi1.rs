//! MIDI 1.0 messages: parsing and encoding.
//!
//! [`parse_midi1`] parses a single message from a raw byte slice without
//! allocating for channel and system messages (System Exclusive returns an
//! owned buffer, since its length is unbounded). [`encode_midi1`] and
//! [`Midi1Message::write_bytes`] encode back to the wire format.

use tpt_av_control_utils::ControlError;

/// A MIDI 1.0 message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Midi1Message {
    /// `8n` — note off.
    NoteOff { channel: u8, note: u8, velocity: u8 },
    /// `9n` — note on (velocity 0 is conventionally a note off).
    NoteOn { channel: u8, note: u8, velocity: u8 },
    /// `An` — polyphonic key pressure.
    PolyphonicKeyPressure { channel: u8, note: u8, pressure: u8 },
    /// `Bn` — control change.
    ControlChange { channel: u8, controller: u8, value: u8 },
    /// `Cn` — program change.
    ProgramChange { channel: u8, program: u8 },
    /// `Dn` — channel pressure.
    ChannelPressure { channel: u8, pressure: u8 },
    /// `En` — pitch bend; `value` is the 14-bit centered value (0..16384,
    /// center 8192).
    PitchBendChange { channel: u8, value: u16 },
    /// `F0 .. F7` — system exclusive (bytes include F0 and F7).
    SystemExclusive(Vec<u8>),
    /// `F1` — MIDI time code quarter frame.
    TimeCodeQuarterFrame(u8),
    /// `F2` — song position pointer (14-bit).
    SongPositionPointer(u16),
    /// `F3` — song select.
    SongSelect(u8),
    /// `F6` — tune request.
    TuneRequest,
    /// `F8` — timing clock.
    TimingClock,
    /// `FA` — start.
    Start,
    /// `FB` — continue.
    Continue,
    /// `FC` — stop.
    Stop,
    /// `FE` — active sensing.
    ActiveSensing,
    /// `FF` — system reset.
    SystemReset,
}

impl Midi1Message {
    /// The status byte for this message, or `None` for SysEx (whose
    /// status is part of the payload).
    pub fn status(&self) -> Option<u8> {
        Some(match self {
            Midi1Message::NoteOff { channel, .. } => 0x80 | channel,
            Midi1Message::NoteOn { channel, .. } => 0x90 | channel,
            Midi1Message::PolyphonicKeyPressure { channel, .. } => 0xA0 | channel,
            Midi1Message::ControlChange { channel, .. } => 0xB0 | channel,
            Midi1Message::ProgramChange { channel, .. } => 0xC0 | channel,
            Midi1Message::ChannelPressure { channel, .. } => 0xD0 | channel,
            Midi1Message::PitchBendChange { channel, .. } => 0xE0 | channel,
            Midi1Message::TimeCodeQuarterFrame(_) => 0xF1,
            Midi1Message::SongPositionPointer(_) => 0xF2,
            Midi1Message::SongSelect(_) => 0xF3,
            Midi1Message::TuneRequest => 0xF6,
            Midi1Message::TimingClock => 0xF8,
            Midi1Message::Start => 0xFA,
            Midi1Message::Continue => 0xFB,
            Midi1Message::Stop => 0xFC,
            Midi1Message::ActiveSensing => 0xFE,
            Midi1Message::SystemReset => 0xFF,
            Midi1Message::SystemExclusive(_) => return None,
        })
    }

    /// For channel messages, the channel (0-15).
    pub fn channel(&self) -> Option<u8> {
        match self {
            Midi1Message::NoteOff { channel, .. }
            | Midi1Message::NoteOn { channel, .. }
            | Midi1Message::PolyphonicKeyPressure { channel, .. }
            | Midi1Message::ControlChange { channel, .. }
            | Midi1Message::ProgramChange { channel, .. }
            | Midi1Message::ChannelPressure { channel, .. }
            | Midi1Message::PitchBendChange { channel, .. } => Some(*channel),
            _ => None,
        }
    }

    /// Encodes into `buf`, returning the number of bytes written. Does not
    /// allocate — safe for real-time threads.
    pub fn write_bytes(&self, buf: &mut [u8]) -> Result<usize, ControlError> {
        let needed = self.encoded_len();
        if buf.len() < needed {
            return Err(ControlError::BufferTooSmall { needed, available: buf.len() });
        }
        macro_rules! three {
            ($status:expr, $d1:expr, $d2:expr) => {{
                buf[0] = $status;
                buf[1] = $d1;
                buf[2] = $d2;
                3
            }};
        }
        Ok(match self {
            Midi1Message::NoteOff { channel, note, velocity } => {
                three!(0x80 | channel, *note, *velocity)
            }
            Midi1Message::NoteOn { channel, note, velocity } => {
                three!(0x90 | channel, *note, *velocity)
            }
            Midi1Message::PolyphonicKeyPressure { channel, note, pressure } => {
                three!(0xA0 | channel, *note, *pressure)
            }
            Midi1Message::ControlChange { channel, controller, value } => {
                three!(0xB0 | channel, *controller, *value)
            }
            Midi1Message::ProgramChange { channel, program } => {
                buf[0] = 0xC0 | channel;
                buf[1] = *program;
                2
            }
            Midi1Message::ChannelPressure { channel, pressure } => {
                buf[0] = 0xD0 | channel;
                buf[1] = *pressure;
                2
            }
            Midi1Message::PitchBendChange { channel, value } => {
                let lsb = (value & 0x7F) as u8;
                let msb = ((value >> 7) & 0x7F) as u8;
                three!(0xE0 | channel, lsb, msb)
            }
            Midi1Message::SystemExclusive(bytes) => {
                buf[..bytes.len()].copy_from_slice(bytes);
                bytes.len()
            }
            Midi1Message::TimeCodeQuarterFrame(q) => {
                buf[0] = 0xF1;
                buf[1] = q & 0x7F;
                2
            }
            Midi1Message::SongPositionPointer(v) => {
                buf[0] = 0xF2;
                buf[1] = (v & 0x7F) as u8;
                buf[2] = ((v >> 7) & 0x7F) as u8;
                3
            }
            Midi1Message::SongSelect(s) => {
                buf[0] = 0xF3;
                buf[1] = s & 0x7F;
                2
            }
            Midi1Message::TuneRequest
            | Midi1Message::TimingClock
            | Midi1Message::Start
            | Midi1Message::Continue
            | Midi1Message::Stop
            | Midi1Message::ActiveSensing
            | Midi1Message::SystemReset => {
                buf[0] = self.status().expect("system status");
                1
            }
        })
    }

    /// Encoded length in bytes.
    pub fn encoded_len(&self) -> usize {
        match self {
            Midi1Message::SystemExclusive(bytes) => bytes.len(),
            Midi1Message::NoteOff { .. }
            | Midi1Message::NoteOn { .. }
            | Midi1Message::PolyphonicKeyPressure { .. }
            | Midi1Message::ControlChange { .. }
            | Midi1Message::PitchBendChange { .. }
            | Midi1Message::SongPositionPointer(_) => 3,
            Midi1Message::ProgramChange { .. }
            | Midi1Message::ChannelPressure { .. }
            | Midi1Message::TimeCodeQuarterFrame(_)
            | Midi1Message::SongSelect(_) => 2,
            _ => 1,
        }
    }

    /// Encodes into a new buffer (allocates).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![0u8; self.encoded_len()];
        let len = self.write_bytes(&mut buf).expect("buffer sized");
        buf.truncate(len);
        buf
    }
}

/// Parses one MIDI 1.0 message from the front of `data`.
///
/// Real-time safe except for [`Midi1Message::SystemExclusive`], which
/// allocates its payload. Extra trailing bytes are ignored.
pub fn parse_midi1(data: &[u8]) -> Result<Midi1Message, ControlError> {
    if data.is_empty() {
        return Err(ControlError::InvalidData("empty MIDI data".into()));
    }
    let status = data[0];
    if status < 0x80 {
        return Err(ControlError::InvalidData(
            "MIDI message must start with a status byte".into(),
        ));
    }
    let need = |n: usize| -> Result<(), ControlError> {
        if data.len() < n + 1 {
            Err(ControlError::InvalidData(format!(
                "truncated MIDI message: status {status:#04x} needs {n} data byte(s)"
            )))
        } else {
            Ok(())
        }
    };
    let d7 = |i: usize| -> Result<u8, ControlError> {
        let b = data[i + 1];
        if b >= 0x80 {
            return Err(ControlError::InvalidData(format!(
                "data byte {b:#04x} has the high bit set"
            )));
        }
        Ok(b)
    };

    match status {
        0x80..=0x9F => {
            need(2)?;
            let channel = status & 0x0F;
            let note = d7(0)?;
            let velocity = d7(1)?;
            if status < 0x90 {
                Ok(Midi1Message::NoteOff { channel, note, velocity })
            } else {
                Ok(Midi1Message::NoteOn { channel, note, velocity })
            }
        }
        0xA0..=0xEF => {
            need(2)?;
            let channel = status & 0x0F;
            let d1 = d7(0)?;
            let d2 = d7(1)?;
            match status & 0xF0 {
                0xA0 => Ok(Midi1Message::PolyphonicKeyPressure {
                    channel,
                    note: d1,
                    pressure: d2,
                }),
                0xB0 => Ok(Midi1Message::ControlChange {
                    channel,
                    controller: d1,
                    value: d2,
                }),
                _ => Ok(Midi1Message::PitchBendChange {
                    channel,
                    value: u16::from(d2) << 7 | u16::from(d1),
                }),
            }
        }
        0xC0..=0xDF => {
            need(1)?;
            let channel = status & 0x0F;
            let d1 = d7(0)?;
            if status < 0xD0 {
                Ok(Midi1Message::ProgramChange { channel, program: d1 })
            } else {
                Ok(Midi1Message::ChannelPressure { channel, pressure: d1 })
            }
        }
        0xF0 => {
            // SysEx: scan to EOX; require it to be present.
            let end = data[1..]
                .iter()
                .position(|&b| b == 0xF7)
                .map(|p| p + 1)
                .ok_or_else(|| {
                    ControlError::InvalidData("SysEx missing F7 terminator".into())
                })?;
            // Validate all payload bytes are 7-bit (except F0 and F7).
            for &b in &data[1..end] {
                if b >= 0x80 {
                    return Err(ControlError::InvalidData(format!(
                        "SysEx data byte {b:#04x} has the high bit set"
                    )));
                }
            }
            Ok(Midi1Message::SystemExclusive(data[..=end].to_vec()))
        }
        0xF1 | 0xF3 => {
            need(1)?;
            let d = d7(0)?;
            if status == 0xF1 {
                Ok(Midi1Message::TimeCodeQuarterFrame(d))
            } else {
                Ok(Midi1Message::SongSelect(d))
            }
        }
        0xF2 => {
            need(2)?;
            let lsb = d7(0)?;
            let msb = d7(1)?;
            Ok(Midi1Message::SongPositionPointer(
                u16::from(msb) << 7 | u16::from(lsb),
            ))
        }
        0xF6 => Ok(Midi1Message::TuneRequest),
        0xF8 => Ok(Midi1Message::TimingClock),
        0xFA => Ok(Midi1Message::Start),
        0xFB => Ok(Midi1Message::Continue),
        0xFC => Ok(Midi1Message::Stop),
        0xFE => Ok(Midi1Message::ActiveSensing),
        0xFF => Ok(Midi1Message::SystemReset),
        _ => Err(ControlError::InvalidData(format!(
            "undefined MIDI status {status:#04x}"
        ))),
    }
}

/// Parses one MIDI 1.0 message and returns it with the number of bytes
/// consumed, suitable for walking a byte stream.
pub fn parse_midi1_prefix(data: &[u8]) -> Result<(Midi1Message, usize), ControlError> {
    let message = parse_midi1(data)?;
    let len = message.encoded_len();
    Ok((message, len))
}

/// Encodes a message into a fresh buffer (allocates).
pub fn encode_midi1(message: &Midi1Message) -> Vec<u8> {
    message.to_bytes()
}

/// Scales a 7-bit value to 16 bits per the MIDI 2.0 recommended mapping
/// (full scale 0x7F maps to 0xFFFF).
pub const fn scale_7_to_16(v: u8) -> u16 {
    (((v as u32) * 0xFFFF + 63) / 0x7F) as u16
}

/// Scales a 16-bit value to 7 bits (rounded).
pub const fn scale_16_to_7(v: u16) -> u8 {
    (((v as u32) * 0x7F + 0x7FFF) / 0xFFFF) as u8
}

/// Scales a 7-bit value to 32 bits (full scale).
pub const fn scale_7_to_32(v: u8) -> u32 {
    (((v as u64) * 0xFFFF_FFFF + 63) / 0x7F) as u32
}

/// Scales a 32-bit value to 7 bits (rounded).
pub const fn scale_32_to_7(v: u32) -> u8 {
    (((v as u64) * 0x7F + 0x7FFF_FFFF) / 0xFFFF_FFFF) as u8
}

/// Scales a 16-bit value to 32 bits (full scale).
pub const fn scale_16_to_32(v: u16) -> u32 {
    (((v as u64) * 0xFFFF_FFFF + 0x7FFF) / 0xFFFF) as u32
}

/// Scales a 32-bit value to 16 bits (rounded).
pub const fn scale_32_to_16(v: u32) -> u16 {
    (((v as u64) * 0xFFFF + 0x7FFF_FFFF) / 0xFFFF_FFFF) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_channel_messages() {
        let (m, n) = parse_midi1_prefix(&[0x90, 0x3C, 0x40]).unwrap();
        assert_eq!(m, Midi1Message::NoteOn { channel: 0, note: 60, velocity: 64 });
        assert_eq!(n, 3);

        let (m, _) = parse_midi1_prefix(&[0x8F, 0x3C, 0x7F]).unwrap();
        assert_eq!(m, Midi1Message::NoteOff { channel: 15, note: 60, velocity: 127 });

        let (m, _) = parse_midi1_prefix(&[0xB2, 0x07, 0x64]).unwrap();
        assert_eq!(
            m,
            Midi1Message::ControlChange { channel: 2, controller: 7, value: 100 }
        );

        let (m, _) = parse_midi1_prefix(&[0xE0, 0x00, 0x40]).unwrap();
        assert_eq!(m, Midi1Message::PitchBendChange { channel: 0, value: 8192 });

        let (m, _) = parse_midi1_prefix(&[0xC5, 0x10]).unwrap();
        assert_eq!(m, Midi1Message::ProgramChange { channel: 5, program: 16 });
    }

    #[test]
    fn parses_system_messages() {
        assert_eq!(parse_midi1(&[0xF8]).unwrap(), Midi1Message::TimingClock);
        assert_eq!(parse_midi1(&[0xFA]).unwrap(), Midi1Message::Start);
        assert_eq!(
            parse_midi1(&[0xF2, 0x00, 0x40]).unwrap(),
            Midi1Message::SongPositionPointer(8192)
        );
        assert_eq!(
            parse_midi1(&[0xF1, 0x23]).unwrap(),
            Midi1Message::TimeCodeQuarterFrame(0x23)
        );
    }

    #[test]
    fn parses_sysex() {
        let m = parse_midi1(&[0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]).unwrap();
        match m {
            Midi1Message::SystemExclusive(bytes) => {
                assert_eq!(bytes, vec![0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]);
            }
            other => panic!("expected sysex, got {other:?}"),
        }
        // Missing terminator.
        assert!(parse_midi1(&[0xF0, 0x7E, 0x01]).is_err());
        // High bit in payload.
        assert!(parse_midi1(&[0xF0, 0x90, 0xF7]).is_err());
    }

    #[test]
    fn rejects_malformed() {
        assert!(parse_midi1(&[]).is_err());
        assert!(parse_midi1(&[0x40]).is_err(), "data byte first");
        assert!(parse_midi1(&[0x90, 0x3C]).is_err(), "truncated");
        assert!(parse_midi1(&[0x90, 0x3C, 0xFF]).is_err(), "high data bit");
        assert!(parse_midi1(&[0xF4]).is_err(), "undefined status");
    }

    #[test]
    fn encode_roundtrips() {
        let messages = vec![
            Midi1Message::NoteOn { channel: 3, note: 42, velocity: 100 },
            Midi1Message::NoteOff { channel: 9, note: 42, velocity: 0 },
            Midi1Message::ControlChange { channel: 0, controller: 74, value: 1 },
            Midi1Message::PitchBendChange { channel: 7, value: 12345 },
            Midi1Message::ProgramChange { channel: 15, program: 127 },
            Midi1Message::ChannelPressure { channel: 1, pressure: 55 },
            Midi1Message::PolyphonicKeyPressure { channel: 1, note: 9, pressure: 88 },
            Midi1Message::SongPositionPointer(16000),
            Midi1Message::SongSelect(3),
            Midi1Message::TimeCodeQuarterFrame(0x7F),
            Midi1Message::TimingClock,
            Midi1Message::SystemReset,
            Midi1Message::SystemExclusive(vec![0xF0, 0x7D, 1, 2, 3, 0xF7]),
        ];
        for m in messages {
            let bytes = m.to_bytes();
            let (back, n) = parse_midi1_prefix(&bytes).unwrap();
            assert_eq!(back, m, "roundtrip {m:?}");
            assert_eq!(n, bytes.len());
        }
    }

    #[test]
    fn write_bytes_into_small_buffer() {
        let m = Midi1Message::NoteOn { channel: 0, note: 60, velocity: 64 };
        let mut buf = [0u8; 2];
        assert!(matches!(
            m.write_bytes(&mut buf),
            Err(ControlError::BufferTooSmall { needed: 3, available: 2 })
        ));
        let mut buf = [0u8; 3];
        assert_eq!(m.write_bytes(&mut buf).unwrap(), 3);
    }

    #[test]
    fn scaling_formulas() {
        assert_eq!(scale_7_to_16(0), 0);
        assert_eq!(scale_7_to_16(0x7F), 0xFFFF);
        assert_eq!(scale_16_to_7(0xFFFF), 0x7F);
        assert_eq!(scale_7_to_32(0x7F), 0xFFFF_FFFF);
        assert_eq!(scale_32_to_7(0xFFFF_FFFF), 0x7F);
        assert_eq!(scale_16_to_32(0xFFFF), 0xFFFF_FFFF);
        assert_eq!(scale_32_to_16(0xFFFF_FFFF), 0xFFFF);
        // Roundtrips are stable for extremes and midpoints.
        for v in [0u8, 1, 0x40, 0x7E, 0x7F] {
            let wide = scale_7_to_16(v);
            assert_eq!(scale_16_to_7(wide), v, "7->16->7 for {v}");
            let wide32 = scale_7_to_32(v);
            assert_eq!(scale_32_to_7(wide32), v, "7->32->7 for {v}");
        }
    }
}
