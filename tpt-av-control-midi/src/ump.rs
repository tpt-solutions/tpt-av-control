//! Universal MIDI Packet (UMP): 32/64/128-bit packet parse and encode,
//! plus the MIDI 1.0 ↔ MIDI 2.0 translation layer.

use crate::messages::{
    DataFormat, DataMessage, FlexDataMessage, Midi1ChannelVoice, Midi2ChannelVoice, Midi2Message,
    SysExMessage, SystemCommonMessage, UtilityMessage,
};
use crate::midi1::{self, Midi1Message};
use tpt_av_control_utils::ControlError;

/// A Universal MIDI Packet: up to four 32-bit words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Ump {
    /// Raw packet words, big-endian message semantics.
    pub words: [u32; 4],
}

impl Ump {
    /// An empty (zero) packet.
    pub const ZERO: Ump = Ump { words: [0; 4] };

    /// Builds a packet from words (padding with zeros).
    pub const fn from_words(words: &[u32]) -> Self {
        let mut data = [0u32; 4];
        let mut i = 0;
        while i < words.len() && i < 4 {
            data[i] = words[i];
            i += 1;
        }
        Ump { words: data }
    }

    /// The message type nibble (bits 31-28 of the first word).
    pub fn message_type(&self) -> u8 {
        (self.words[0] >> 28) as u8
    }

    /// The group nibble (bits 27-24 of the first word).
    pub fn group(&self) -> u8 {
        ((self.words[0] >> 24) & 0xF) as u8
    }

    /// The number of words this packet occupies per its message type.
    pub fn num_words(&self) -> usize {
        match self.message_type() {
            0x0..=0x2 => 1,
            0x3 | 0x4 | 0xD => 2,
            0x5 | 0xE | 0xF => 4,
            _ => 1,
        }
    }

    /// Parses the packet into a typed MIDI 2.0 message.
    pub fn parse(&self) -> Result<Midi2Message, ControlError> {
        let n = self.num_words();
        for word in self.words.iter().skip(n) {
            if *word != 0 {
                return Err(ControlError::InvalidData(
                    "trailing garbage after UMP packet".into(),
                ));
            }
        }
        parse_words(&self.words[..n])
    }

    /// Builds a single-packet UMP from a MIDI 2.0 message.
    ///
    /// Note: a full MIDI-CI or chunked SysEx exchange spans several
    /// packets; use [`crate::ump::message_to_umps`] for those.
    /// # Examples
    ///
    /// ```
    /// use tpt_av_control_midi::{Midi2ChannelVoice, Midi2Message, Ump};
    /// let ump = Ump::from_message(&Midi2Message::Midi2ChannelVoice(
    ///     Midi2ChannelVoice::ControlChange {
    ///         group: 0, channel: 1, index: 7, value: 0xFFFF_FFFF,
    ///     },
    /// ));
    /// assert_eq!(ump.to_bytes(), [0x40, 0xB1, 0x07, 0x00, 0xFF, 0xFF, 0xFF, 0xFF]);
    /// ```
    pub fn from_message(message: &Midi2Message) -> Self {
        let mut words = [0u32; 4];
        match message {
            Midi2Message::Utility(u) => {
                words[0] = match u {
                    UtilityMessage::NoOp => 0x0000_0000,
                    UtilityMessage::Clock { clock } => 0x0010_0000 | u32::from(*clock),
                    UtilityMessage::Timestamp { timestamp } => 0x0020_0000 | u32::from(*timestamp),
                };
            }
            Midi2Message::SystemCommon(s) => {
                words[0] = 0x1000_0000 | u32::from(s.group()) << 24;
                words[0] |= match s {
                    SystemCommonMessage::TimeCodeQuarterFrame { quarter_frame, .. } => {
                        0x00F1_0000 | u32::from(quarter_frame & 0x7F) << 8
                    }
                    SystemCommonMessage::SongPositionPointer { position, .. } => {
                        0x00F2_0000
                            | u32::from(position & 0x7F) << 8
                            | u32::from((position >> 7) & 0x7F)
                    }
                    SystemCommonMessage::SongSelect { song, .. } => {
                        0x00F3_0000 | u32::from(song & 0x7F) << 8
                    }
                    SystemCommonMessage::TuneRequest { .. } => 0x00F6_0000,
                    SystemCommonMessage::TimingClock { .. } => 0x00F8_0000,
                    SystemCommonMessage::Start { .. } => 0x00FA_0000,
                    SystemCommonMessage::Continue { .. } => 0x00FB_0000,
                    SystemCommonMessage::Stop { .. } => 0x00FC_0000,
                    SystemCommonMessage::ActiveSensing { .. } => 0x00FE_0000,
                    SystemCommonMessage::SystemReset { .. } => 0x00FF_0000,
                };
            }
            Midi2Message::Midi1ChannelVoice(cv) => {
                let status = cv.message.status().unwrap_or(0);
                words[0] = 0x2000_0000 | u32::from(cv.group) << 24 | u32::from(status) << 16;
                match &cv.message {
                    Midi1Message::NoteOff { note, velocity, .. } => {
                        words[0] |= u32::from(*note) << 8 | u32::from(*velocity);
                    }
                    Midi1Message::NoteOn { note, velocity, .. } => {
                        words[0] |= u32::from(*note) << 8 | u32::from(*velocity);
                    }
                    Midi1Message::PolyphonicKeyPressure { note, pressure, .. } => {
                        words[0] |= u32::from(*note) << 8 | u32::from(*pressure);
                    }
                    Midi1Message::ControlChange {
                        controller, value, ..
                    } => {
                        words[0] |= u32::from(*controller) << 8 | u32::from(*value);
                    }
                    Midi1Message::ProgramChange { program, .. } => {
                        words[0] |= u32::from(*program) << 8;
                    }
                    Midi1Message::ChannelPressure { pressure, .. } => {
                        words[0] |= u32::from(*pressure) << 8;
                    }
                    Midi1Message::PitchBendChange { value, .. } => {
                        words[0] |= u32::from(value & 0x7F) << 8 | u32::from((value >> 7) & 0x7F);
                    }
                    _ => {}
                }
            }
            Midi2Message::SystemExclusive(s) => {
                let data = s.data();
                words[0] = 0x3000_0000
                    | u32::from(s.group()) << 24
                    | u32::from(s.status_code()) << 20
                    | ((data.len().min(6) as u32) << 16);
                // Six 7-bit bytes: two in word0, four in word1.
                let mut offset: i32 = 8;
                for &byte in data.iter().take(6) {
                    words[0] |= u32::from(byte & 0x7F) << offset;
                    if offset == 0 {
                        break;
                    }
                    offset = offset.saturating_sub(8);
                }
                let mut offset: i32 = 24;
                for &byte in data.iter().skip(2).take(4) {
                    words[1] |= u32::from(byte & 0x7F) << offset;
                    offset = offset.saturating_sub(8);
                }
            }
            Midi2Message::Midi2ChannelVoice(cv) => {
                words[0] = 0x4000_0000
                    | u32::from(cv.group()) << 24
                    | u32::from(cv.opcode()) << 20
                    | u32::from(cv.channel()) << 16;
                match *cv {
                    Midi2ChannelVoice::NoteOff {
                        note,
                        attribute_type,
                        attribute,
                        velocity,
                        ..
                    }
                    | Midi2ChannelVoice::NoteOn {
                        note,
                        attribute_type,
                        attribute,
                        velocity,
                        ..
                    } => {
                        words[0] |= u32::from(note) << 8 | u32::from(attribute_type);
                        words[1] = u32::from(velocity) << 16 | u32::from(attribute);
                    }
                    Midi2ChannelVoice::PolyphonicKeyPressure { note, pressure, .. } => {
                        words[0] |= u32::from(note) << 8;
                        words[1] = pressure;
                    }
                    Midi2ChannelVoice::ControlChange { index, value, .. } => {
                        words[0] |= u32::from(index) << 8;
                        words[1] = value;
                    }
                    Midi2ChannelVoice::PerNoteRcc {
                        note, index, value, ..
                    }
                    | Midi2ChannelVoice::PerNoteAcc {
                        note, index, value, ..
                    } => {
                        words[0] |= u32::from(note) << 8 | u32::from(index);
                        words[1] = value;
                    }
                    Midi2ChannelVoice::Rpn {
                        bank, index, value, ..
                    }
                    | Midi2ChannelVoice::Nrpn {
                        bank, index, value, ..
                    }
                    | Midi2ChannelVoice::RelativeRpn {
                        bank, index, value, ..
                    }
                    | Midi2ChannelVoice::RelativeNrpn {
                        bank, index, value, ..
                    } => {
                        words[0] |= u32::from(bank & 0x7F) << 8 | u32::from(index & 0x7F);
                        words[1] = value;
                    }
                    Midi2ChannelVoice::PerNotePitchBend { note, value, .. } => {
                        words[0] |= u32::from(note) << 8;
                        words[1] = value;
                    }
                    Midi2ChannelVoice::ProgramChange {
                        option_flags,
                        program,
                        bank_valid,
                        bank_msb,
                        bank_lsb,
                        ..
                    } => {
                        let valid_bit = u32::from(bank_valid) << 15;
                        words[0] |= valid_bit | u32::from(option_flags);
                        words[1] = u32::from(program) << 24
                            | u32::from(bank_msb & 0x7F) << 8
                            | u32::from(bank_lsb & 0x7F);
                    }
                    Midi2ChannelVoice::ChannelPressure { pressure, .. }
                    | Midi2ChannelVoice::PitchBend {
                        value: pressure, ..
                    } => {
                        words[1] = pressure;
                    }
                    Midi2ChannelVoice::PerNoteManagement {
                        note, option_flags, ..
                    } => {
                        words[0] |= u32::from(note) << 8 | u32::from(option_flags);
                    }
                }
            }
            Midi2Message::DataMessage(d) => {
                let n = d.word_count.clamp(2, 4);
                words[..n].copy_from_slice(&d.words[..n]);
            }
            Midi2Message::FlexData(f) => encode_flex(f, &mut words),
        }
        Ump { words }
    }

    /// Serializes the meaningful words as big-endian bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let n = self.num_words();
        let mut out = Vec::with_capacity(n * 4);
        for word in &self.words[..n] {
            out.extend_from_slice(&word.to_be_bytes());
        }
        out
    }

    /// Parses a packet from big-endian bytes (4/8/12/16 bytes).
    pub fn from_bytes(data: &[u8]) -> Result<Self, ControlError> {
        if data.is_empty() || data.len() % 4 != 0 || data.len() > 16 {
            return Err(ControlError::InvalidData(format!(
                "UMP byte length must be a multiple of 4 (4..16), got {}",
                data.len()
            )));
        }
        let mut words = [0u32; 4];
        for (i, word) in words.iter_mut().enumerate().take(data.len() / 4) {
            *word = u32::from_be_bytes([
                data[i * 4],
                data[i * 4 + 1],
                data[i * 4 + 2],
                data[i * 4 + 3],
            ]);
        }
        let ump = Ump { words };
        if data.len() / 4 != ump.num_words() {
            return Err(ControlError::InvalidData(format!(
                "UMP type {:#x} needs {} words, got {}",
                ump.message_type(),
                ump.num_words(),
                data.len() / 4
            )));
        }
        Ok(ump)
    }
}

fn flex_word0(group: u8, format: DataFormat, bank: u8, status: u8, channel: u8) -> u32 {
    (0xDu32 << 28)
        | u32::from(group) << 24
        | u32::from(format.to_bits()) << 22
        | u32::from(bank & 0x3) << 20
        | u32::from(channel & 0xF) << 8
        | u32::from(status & 0x7F)
}

fn encode_flex(f: &FlexDataMessage, words: &mut [u32; 4]) {
    let (group, format, bank, status, channel) = match f {
        FlexDataMessage::SetTempo { group, format, .. } => (*group, *format, 0x1, 0x00, 0x0),
        FlexDataMessage::TimeSignature { group, format, .. } => (*group, *format, 0x1, 0x01, 0x0),
        FlexDataMessage::Text {
            group,
            format,
            bank,
            status,
            channel,
            ..
        } => (*group, *format, *bank, *status, *channel),
        FlexDataMessage::Other {
            group,
            format,
            bank,
            status,
            channel,
            data,
        } => {
            words[0] = flex_word0(*group, *format, *bank, *status, *channel);
            *words = *data;
            // Restore the header over the stored copy.
            words[0] = flex_word0(*group, *format, *bank, *status, *channel);
            return;
        }
    };
    words[0] = flex_word0(group, format, bank, status, channel);
    match f {
        FlexDataMessage::SetTempo {
            ten_nanosecond_units_per_quarter_note: tempo,
            ..
        } => {
            words[1] = *tempo;
        }
        FlexDataMessage::TimeSignature {
            numerator,
            denominator_exponent,
            thirty_second_notes_per_quarter,
            ..
        } => {
            words[1] = u32::from(*numerator) << 24
                | u32::from(*denominator_exponent & 0x7F) << 16
                | u32::from(*thirty_second_notes_per_quarter & 0x7F) << 8;
        }
        FlexDataMessage::Text { text, .. } => {
            // Up to twelve 7-bit characters across words 1-3.
            let mut offset = 24;
            let mut word = 1;
            for byte in text.bytes().take(12) {
                words[word] |= u32::from(byte & 0x7F) << offset;
                if offset == 0 {
                    word += 1;
                    offset = 24;
                } else {
                    offset -= 8;
                }
            }
        }
        FlexDataMessage::Other { .. } => unreachable!("handled above"),
    }
}

fn parse_words(words: &[u32]) -> Result<Midi2Message, ControlError> {
    if words.is_empty() {
        return Err(ControlError::InvalidData("empty UMP".into()));
    }
    let word0 = words[0];
    let group = ((word0 >> 24) & 0xF) as u8;
    match (word0 >> 28) as u8 {
        0x0 => {
            let status = (word0 >> 20) & 0xF;
            let value = (word0 & 0xFFFF) as u16;
            Ok(Midi2Message::Utility(match status {
                0x0 => UtilityMessage::NoOp,
                0x1 => UtilityMessage::Clock { clock: value },
                0x2 => UtilityMessage::Timestamp { timestamp: value },
                _ => {
                    return Err(ControlError::InvalidData(format!(
                        "unknown utility status {status:#x}"
                    )))
                }
            }))
        }
        0x1 => {
            let status = (word0 >> 16) & 0xFF;
            let p1 = ((word0 >> 8) & 0x7F) as u8;
            let p2 = (word0 & 0x7F) as u8;
            let common = match status {
                0xF1 => SystemCommonMessage::TimeCodeQuarterFrame {
                    group,
                    quarter_frame: p1,
                },
                0xF2 => SystemCommonMessage::SongPositionPointer {
                    group,
                    position: u16::from(p2) << 7 | u16::from(p1),
                },
                0xF3 => SystemCommonMessage::SongSelect { group, song: p1 },
                0xF6 => SystemCommonMessage::TuneRequest { group },
                0xF8 => SystemCommonMessage::TimingClock { group },
                0xFA => SystemCommonMessage::Start { group },
                0xFB => SystemCommonMessage::Continue { group },
                0xFC => SystemCommonMessage::Stop { group },
                0xFE => SystemCommonMessage::ActiveSensing { group },
                0xFF => SystemCommonMessage::SystemReset { group },
                _ => {
                    return Err(ControlError::InvalidData(format!(
                        "unknown system status {status:#04x}"
                    )))
                }
            };
            Ok(Midi2Message::SystemCommon(common))
        }
        0x2 => {
            let status = (word0 >> 20) & 0xF;
            let channel = ((word0 >> 16) & 0xF) as u8;
            let d1 = ((word0 >> 8) & 0x7F) as u8;
            let d2 = (word0 & 0x7F) as u8;
            let message = match status {
                0x8 => Midi1Message::NoteOff {
                    channel,
                    note: d1,
                    velocity: d2,
                },
                0x9 => Midi1Message::NoteOn {
                    channel,
                    note: d1,
                    velocity: d2,
                },
                0xA => Midi1Message::PolyphonicKeyPressure {
                    channel,
                    note: d1,
                    pressure: d2,
                },
                0xB => Midi1Message::ControlChange {
                    channel,
                    controller: d1,
                    value: d2,
                },
                0xC => Midi1Message::ProgramChange {
                    channel,
                    program: d1,
                },
                0xD => Midi1Message::ChannelPressure {
                    channel,
                    pressure: d1,
                },
                0xE => Midi1Message::PitchBendChange {
                    channel,
                    value: u16::from(d2) << 7 | u16::from(d1),
                },
                _ => {
                    return Err(ControlError::InvalidData(format!(
                        "unknown MIDI 1.0 UMP opcode {status:#x}"
                    )))
                }
            };
            Ok(Midi2Message::Midi1ChannelVoice(Midi1ChannelVoice {
                group,
                message,
            }))
        }
        0x3 => {
            let status = (word0 >> 20) & 0xF;
            let count = ((word0 >> 16) & 0xF) as usize;
            if count > 6 {
                return Err(ControlError::InvalidData(format!(
                    "sysex7 byte count {count} exceeds 6"
                )));
            }
            // Two 7-bit bytes in word0 ([15:8], [7:0]), four in word1.
            let mut data = Vec::with_capacity(count);
            for i in 0..count {
                let byte = if i < 2 {
                    (word0 >> (8 - i * 8)) & 0x7F
                } else {
                    (words[1] >> (24 - (i - 2) * 8)) & 0x7F
                };
                data.push(byte as u8);
            }
            let message = match status {
                0x0 => SysExMessage::Complete { group, data },
                0x1 => SysExMessage::Start { group, data },
                0x2 => SysExMessage::Continue { group, data },
                0x3 => SysExMessage::End { group, data },
                _ => {
                    return Err(ControlError::InvalidData(format!(
                        "unknown sysex status {status:#x}"
                    )))
                }
            };
            Ok(Midi2Message::SystemExclusive(message))
        }
        0x4 => {
            let opcode = (word0 >> 20) & 0xF;
            let channel = ((word0 >> 16) & 0xF) as u8;
            let note_or_index = ((word0 >> 8) & 0xFF) as u8;
            let attr_or_flags = (word0 & 0xFF) as u8;
            let word1 = words.get(1).copied().ok_or_else(|| {
                ControlError::InvalidData("MIDI 2.0 CV missing second word".into())
            })?;
            let cv = match opcode {
                0x0 => Midi2ChannelVoice::PerNoteRcc {
                    group,
                    channel,
                    note: note_or_index,
                    index: attr_or_flags,
                    value: word1,
                },
                0x1 => Midi2ChannelVoice::PerNoteAcc {
                    group,
                    channel,
                    note: note_or_index,
                    index: attr_or_flags,
                    value: word1,
                },
                0x2 => Midi2ChannelVoice::Rpn {
                    group,
                    channel,
                    bank: note_or_index & 0x7F,
                    index: attr_or_flags & 0x7F,
                    value: word1,
                },
                0x3 => Midi2ChannelVoice::Nrpn {
                    group,
                    channel,
                    bank: note_or_index & 0x7F,
                    index: attr_or_flags & 0x7F,
                    value: word1,
                },
                0x4 => Midi2ChannelVoice::RelativeRpn {
                    group,
                    channel,
                    bank: note_or_index & 0x7F,
                    index: attr_or_flags & 0x7F,
                    value: word1,
                },
                0x5 => Midi2ChannelVoice::RelativeNrpn {
                    group,
                    channel,
                    bank: note_or_index & 0x7F,
                    index: attr_or_flags & 0x7F,
                    value: word1,
                },
                0x6 => Midi2ChannelVoice::PerNotePitchBend {
                    group,
                    channel,
                    note: note_or_index,
                    value: word1,
                },
                0x8 => Midi2ChannelVoice::NoteOff {
                    group,
                    channel,
                    note: note_or_index,
                    attribute_type: attr_or_flags,
                    attribute: (word1 & 0xFFFF) as u16,
                    velocity: (word1 >> 16) as u16,
                },
                0x9 => Midi2ChannelVoice::NoteOn {
                    group,
                    channel,
                    note: note_or_index,
                    attribute_type: attr_or_flags,
                    attribute: (word1 & 0xFFFF) as u16,
                    velocity: (word1 >> 16) as u16,
                },
                0xA => Midi2ChannelVoice::PolyphonicKeyPressure {
                    group,
                    channel,
                    note: note_or_index,
                    pressure: word1,
                },
                0xB => Midi2ChannelVoice::ControlChange {
                    group,
                    channel,
                    index: note_or_index,
                    value: word1,
                },
                0xC => Midi2ChannelVoice::ProgramChange {
                    group,
                    channel,
                    option_flags: attr_or_flags,
                    program: (word1 >> 24) as u8,
                    bank_valid: (word0 >> 15) & 1 == 1,
                    bank_msb: ((word1 >> 8) & 0x7F) as u8,
                    bank_lsb: (word1 & 0x7F) as u8,
                },
                0xD => Midi2ChannelVoice::ChannelPressure {
                    group,
                    channel,
                    pressure: word1,
                },
                0xE => Midi2ChannelVoice::PitchBend {
                    group,
                    channel,
                    value: word1,
                },
                0xF => Midi2ChannelVoice::PerNoteManagement {
                    group,
                    channel,
                    note: note_or_index,
                    option_flags: attr_or_flags,
                },
                _ => {
                    return Err(ControlError::InvalidData(format!(
                        "reserved MIDI 2.0 opcode {opcode:#x}"
                    )))
                }
            };
            Ok(Midi2Message::Midi2ChannelVoice(cv))
        }
        0x5 | 0xE | 0xF => {
            let word_count = match (word0 >> 28) as u8 {
                0x5 => 4,
                0xE => 4,
                _ => 2,
            };
            if words.len() < word_count {
                return Err(ControlError::InvalidData("extended UMP truncated".into()));
            }
            let mut data = [0u32; 4];
            data.copy_from_slice(words);
            Ok(Midi2Message::DataMessage(DataMessage {
                group,
                word_count,
                words: data,
            }))
        }
        0xD => {
            let format = DataFormat::from_bits(((word0 >> 22) & 0x3) as u8);
            let bank = ((word0 >> 20) & 0x3) as u8;
            let status = (word0 & 0x7F) as u8;
            let channel = ((word0 >> 8) & 0xF) as u8;
            let flex = match status {
                0x00 => FlexDataMessage::SetTempo {
                    group,
                    format,
                    ten_nanosecond_units_per_quarter_note: words[1],
                },
                0x01 => FlexDataMessage::TimeSignature {
                    group,
                    format,
                    numerator: (words[1] >> 24) as u8,
                    denominator_exponent: ((words[1] >> 16) & 0x7F) as u8,
                    thirty_second_notes_per_quarter: ((words[1] >> 8) & 0x7F) as u8,
                },
                0x04..=0x0C => {
                    let text = decode_text(&words[1..]);
                    FlexDataMessage::Text {
                        group,
                        format,
                        bank,
                        status,
                        channel,
                        text,
                    }
                }
                _ => {
                    let mut data = [0u32; 4];
                    data[..words.len()].copy_from_slice(words);
                    FlexDataMessage::Other {
                        group,
                        format,
                        bank,
                        status,
                        channel,
                        data,
                    }
                }
            };
            Ok(Midi2Message::FlexData(flex))
        }
        t => Err(ControlError::InvalidData(format!(
            "unknown UMP message type {t:#x}"
        ))),
    }
}

fn decode_text(words: &[u32]) -> String {
    let mut text = String::new();
    'outer: for word in words {
        for shift in [24i32, 16, 8, 0] {
            let byte = ((word >> shift) & 0x7F) as u8;
            if byte == 0 {
                break 'outer;
            }
            text.push(byte as char);
        }
    }
    text
}

/// Converts a MIDI 2.0 message into its UMP packet sequence.
///
pub fn message_to_umps(message: &Midi2Message) -> Vec<Ump> {
    match message {
        Midi2Message::SystemExclusive(s) => chunk_sysex7(s),
        _ => vec![Ump::from_message(message)],
    }
}

fn chunk_sysex7(s: &SysExMessage) -> Vec<Ump> {
    let data = s.data();
    let group = s.group();
    let mut chunks = Vec::new();
    if data.len() <= 6 {
        chunks.push(Ump::from_message(&Midi2Message::SystemExclusive(match s {
            SysExMessage::Start { .. } => SysExMessage::Start {
                group,
                data: data.to_vec(),
            },
            SysExMessage::End { .. } => SysExMessage::End {
                group,
                data: data.to_vec(),
            },
            SysExMessage::Continue { .. } => SysExMessage::Continue {
                group,
                data: data.to_vec(),
            },
            SysExMessage::Complete { .. } => SysExMessage::Complete {
                group,
                data: data.to_vec(),
            },
        })));
        return chunks;
    }
    for (i, chunk) in data.chunks(6).enumerate() {
        let last = i == data.len().div_ceil(6) - 1;
        let status = match (i, last) {
            (0, true) => 0x0,
            (0, false) => 0x1,
            (_, true) => 0x3,
            (_, false) => 0x2,
        };
        let message = match status {
            0x0 => SysExMessage::Complete {
                group,
                data: chunk.to_vec(),
            },
            0x1 => SysExMessage::Start {
                group,
                data: chunk.to_vec(),
            },
            0x2 => SysExMessage::Continue {
                group,
                data: chunk.to_vec(),
            },
            _ => SysExMessage::End {
                group,
                data: chunk.to_vec(),
            },
        };
        chunks.push(Ump::from_message(&Midi2Message::SystemExclusive(message)));
    }
    chunks
}

/// Translates a MIDI 1.0 message into its MIDI 2.0 (UMP) equivalent.
///
/// Channel voice messages gain full-scale high-resolution values;
/// system messages map to the UMP System type; SysEx is chunked into
pub fn midi1_to_midi2(message: &Midi1Message, group: u8) -> Vec<Midi2Message> {
    use crate::midi1::*;
    match message {
        Midi1Message::NoteOff {
            channel,
            note,
            velocity,
        } => {
            vec![Midi2Message::Midi2ChannelVoice(
                Midi2ChannelVoice::NoteOff {
                    group,
                    channel: *channel,
                    note: *note,
                    attribute_type: 0,
                    attribute: 0,
                    velocity: midi1::scale_7_to_16(*velocity),
                },
            )]
        }
        Midi1Message::NoteOn {
            channel,
            note,
            velocity,
        } => {
            if *velocity == 0 {
                // Velocity-0 note on is a note off per the MIDI 1.0 spec.
                return vec![Midi2Message::Midi2ChannelVoice(
                    Midi2ChannelVoice::NoteOff {
                        group,
                        channel: *channel,
                        note: *note,
                        attribute_type: 0,
                        attribute: 0,
                        velocity: 0,
                    },
                )];
            }
            vec![Midi2Message::Midi2ChannelVoice(Midi2ChannelVoice::NoteOn {
                group,
                channel: *channel,
                note: *note,
                attribute_type: 0,
                attribute: 0,
                velocity: midi1::scale_7_to_16(*velocity),
            })]
        }
        Midi1Message::PolyphonicKeyPressure {
            channel,
            note,
            pressure,
        } => vec![Midi2Message::Midi2ChannelVoice(
            Midi2ChannelVoice::PolyphonicKeyPressure {
                group,
                channel: *channel,
                note: *note,
                pressure: midi1::scale_7_to_32(*pressure),
            },
        )],
        Midi1Message::ControlChange {
            channel,
            controller,
            value,
        } => vec![Midi2Message::Midi2ChannelVoice(
            Midi2ChannelVoice::ControlChange {
                group,
                channel: *channel,
                index: *controller,
                value: midi1::scale_7_to_32(*value),
            },
        )],
        Midi1Message::ProgramChange { channel, program } => {
            vec![Midi2Message::Midi2ChannelVoice(
                Midi2ChannelVoice::ProgramChange {
                    group,
                    channel: *channel,
                    option_flags: 0,
                    program: *program,
                    bank_valid: false,
                    bank_msb: 0,
                    bank_lsb: 0,
                },
            )]
        }
        Midi1Message::ChannelPressure { channel, pressure } => {
            vec![Midi2Message::Midi2ChannelVoice(
                Midi2ChannelVoice::ChannelPressure {
                    group,
                    channel: *channel,
                    pressure: midi1::scale_7_to_32(*pressure),
                },
            )]
        }
        Midi1Message::PitchBendChange { channel, value } => {
            vec![Midi2Message::Midi2ChannelVoice(
                Midi2ChannelVoice::PitchBend {
                    group,
                    channel: *channel,
                    // Pitch bend is 14-bit in MIDI 1.0, not 16-bit.
                    value: midi1::scale_14_to_32(*value),
                },
            )]
        }
        Midi1Message::SystemExclusive(bytes) => {
            // Strip F0/F7 and chunk the payload into sysex7 packets.
            let payload: Vec<u8> = bytes
                .iter()
                .skip(1)
                .copied()
                .take_while(|&b| b != 0xF7)
                .collect();
            let mut out = Vec::new();
            if payload.len() <= 6 {
                out.push(Midi2Message::SystemExclusive(SysExMessage::Complete {
                    group,
                    data: payload,
                }));
            } else {
                for (i, chunk) in payload.chunks(6).enumerate() {
                    let last = i == payload.len().div_ceil(6) - 1;
                    let message = match (i, last) {
                        (0, false) => SysExMessage::Start {
                            group,
                            data: chunk.to_vec(),
                        },
                        (_, false) => SysExMessage::Continue {
                            group,
                            data: chunk.to_vec(),
                        },
                        _ => SysExMessage::End {
                            group,
                            data: chunk.to_vec(),
                        },
                    };
                    out.push(Midi2Message::SystemExclusive(message));
                }
            }
            out
        }
        Midi1Message::TimeCodeQuarterFrame(quarter) => {
            vec![Midi2Message::SystemCommon(
                SystemCommonMessage::TimeCodeQuarterFrame {
                    group,
                    quarter_frame: *quarter,
                },
            )]
        }
        Midi1Message::SongPositionPointer(position) => vec![Midi2Message::SystemCommon(
            SystemCommonMessage::SongPositionPointer {
                group,
                position: *position,
            },
        )],
        Midi1Message::SongSelect(song) => vec![Midi2Message::SystemCommon(
            SystemCommonMessage::SongSelect { group, song: *song },
        )],
        Midi1Message::TuneRequest => vec![Midi2Message::SystemCommon(
            SystemCommonMessage::TuneRequest { group },
        )],
        Midi1Message::TimingClock => vec![Midi2Message::SystemCommon(
            SystemCommonMessage::TimingClock { group },
        )],
        Midi1Message::Start => {
            vec![Midi2Message::SystemCommon(SystemCommonMessage::Start {
                group,
            })]
        }
        Midi1Message::Continue => vec![Midi2Message::SystemCommon(SystemCommonMessage::Continue {
            group,
        })],
        Midi1Message::Stop => {
            vec![Midi2Message::SystemCommon(SystemCommonMessage::Stop {
                group,
            })]
        }
        Midi1Message::ActiveSensing => vec![Midi2Message::SystemCommon(
            SystemCommonMessage::ActiveSensing { group },
        )],
        Midi1Message::SystemReset => vec![Midi2Message::SystemCommon(
            SystemCommonMessage::SystemReset { group },
        )],
    }
}

/// Translates a MIDI 2.0 message into its MIDI 1.0 byte-stream equivalent
/// (downscaled per the MIDI 2.0 recommended scaling). Returns zero bytes
/// for messages with no MIDI 1.0 form (e.g. Per-Note Management).
pub fn midi2_to_midi1(message: &Midi2Message) -> Vec<Midi1Message> {
    use crate::midi1::*;
    match message {
        Midi2Message::Midi1ChannelVoice(cv) => vec![cv.message.clone()],
        Midi2Message::Midi2ChannelVoice(cv) => match *cv {
            Midi2ChannelVoice::NoteOff {
                channel,
                note,
                velocity,
                ..
            } => vec![Midi1Message::NoteOff {
                channel,
                note,
                velocity: scale_16_to_7(velocity),
            }],
            Midi2ChannelVoice::NoteOn {
                channel,
                note,
                velocity,
                ..
            } => vec![Midi1Message::NoteOn {
                channel,
                note,
                velocity: scale_16_to_7(velocity),
            }],
            Midi2ChannelVoice::PolyphonicKeyPressure {
                channel,
                note,
                pressure,
                ..
            } => vec![Midi1Message::PolyphonicKeyPressure {
                channel,
                note,
                pressure: scale_32_to_7(pressure),
            }],
            Midi2ChannelVoice::ControlChange {
                channel,
                index,
                value,
                ..
            } => vec![Midi1Message::ControlChange {
                channel,
                controller: index,
                value: scale_32_to_7(value),
            }],
            Midi2ChannelVoice::ProgramChange {
                channel, program, ..
            } => vec![Midi1Message::ProgramChange { channel, program }],
            Midi2ChannelVoice::ChannelPressure {
                channel, pressure, ..
            } => vec![Midi1Message::ChannelPressure {
                channel,
                pressure: scale_32_to_7(pressure),
            }],
            Midi2ChannelVoice::PitchBend { channel, value, .. } => {
                vec![Midi1Message::PitchBendChange {
                    channel,
                    value: scale_32_to_14(value),
                }]
            }
            // Per-note controllers, management, and pitch bend have no
            // MIDI 1.0 equivalent.
            _ => Vec::new(),
        },
        Midi2Message::SystemCommon(s) => vec![match *s {
            SystemCommonMessage::TimeCodeQuarterFrame { quarter_frame, .. } => {
                Midi1Message::TimeCodeQuarterFrame(quarter_frame)
            }
            SystemCommonMessage::SongPositionPointer { position, .. } => {
                Midi1Message::SongPositionPointer(position)
            }
            SystemCommonMessage::SongSelect { song, .. } => Midi1Message::SongSelect(song),
            SystemCommonMessage::TuneRequest { .. } => Midi1Message::TuneRequest,
            SystemCommonMessage::TimingClock { .. } => Midi1Message::TimingClock,
            SystemCommonMessage::Start { .. } => Midi1Message::Start,
            SystemCommonMessage::Continue { .. } => Midi1Message::Continue,
            SystemCommonMessage::Stop { .. } => Midi1Message::Stop,
            SystemCommonMessage::ActiveSensing { .. } => Midi1Message::ActiveSensing,
            SystemCommonMessage::SystemReset { .. } => Midi1Message::SystemReset,
        }],
        Midi2Message::SystemExclusive(s) => {
            // Reassemble chunks: complete/start/end map directly.
            let data = s.data();
            let mut bytes = Vec::with_capacity(data.len() + 2);
            bytes.push(0xF0);
            bytes.extend_from_slice(data);
            bytes.push(0xF7);
            match s {
                SysExMessage::Complete { .. } | SysExMessage::Start { .. } => {
                    vec![Midi1Message::SystemExclusive(bytes)]
                }
                SysExMessage::End { .. } => {
                    // Chunked messages are assembled by the caller with
                    // `sysex_reassembler`; a lone End chunk has no
                    // meaningful MIDI 1.0 form.
                    Vec::new()
                }
                SysExMessage::Continue { .. } => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

#[derive(Debug, Default)]
/// Accumulates chunked sysex7 packets back into whole SysEx messages.
///
/// Streams are capped: a peer that never terminates its chunks is cut off
/// at [`SysexReassembler::MAX_CHUNKS`] chunks or
/// [`SysexReassembler::MAX_BYTES`] bytes, so memory stays bounded under
/// hostile input.
pub struct SysexReassembler {
    /// UMP group (0-15).
    group: u8,
    chunks: Vec<Vec<u8>>,
}

impl SysexReassembler {
    /// Maximum number of chunks accepted in one stream.
    pub const MAX_CHUNKS: usize = 1024;
    /// Maximum total payload bytes accepted in one stream (1 MiB).
    pub const MAX_BYTES: usize = 1024 * 1024;

    fn total_len(&self) -> usize {
        self.chunks.iter().map(|c| c.len()).sum()
    }

    /// Feeds a SysEx chunk message. Returns the complete byte payload
    /// (without F0/F7) when the sequence finishes.
    pub fn feed(&mut self, message: &SysExMessage) -> Result<Option<Vec<u8>>, ControlError> {
        match message {
            SysExMessage::Complete { data, .. } => Ok(Some(data.clone())),
            SysExMessage::Start { group, data } => {
                self.group = *group;
                self.chunks = vec![data.clone()];
                Ok(None)
            }
            SysExMessage::Continue { data, .. } => {
                if self.chunks.is_empty() {
                    return Err(ControlError::InvalidData(
                        "sysex continue without start".into(),
                    ));
                }
                if self.chunks.len() >= Self::MAX_CHUNKS
                    || self.total_len().saturating_add(data.len()) > Self::MAX_BYTES
                {
                    self.chunks.clear();
                    return Err(ControlError::InvalidData(
                        "sysex chunk stream exceeded limits".into(),
                    ));
                }
                self.chunks.push(data.clone());
                Ok(None)
            }
            SysExMessage::End { data, group } => {
                if self.chunks.is_empty() {
                    return Err(ControlError::InvalidData("sysex end without start".into()));
                }
                if *group != self.group {
                    return Err(ControlError::InvalidData("sysex end group mismatch".into()));
                }
                if self.chunks.len() >= Self::MAX_CHUNKS
                    || self.total_len().saturating_add(data.len()) > Self::MAX_BYTES
                {
                    self.chunks.clear();
                    return Err(ControlError::InvalidData(
                        "sysex chunk stream exceeded limits".into(),
                    ));
                }
                self.chunks.push(data.clone());
                let all: Vec<u8> = self.chunks.concat();
                self.chunks.clear();
                Ok(Some(all))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi2_note_on_layout() {
        // Note On, group 0, channel 3, note 60, velocity 0x8000.
        let ump = Ump::from_message(&Midi2Message::Midi2ChannelVoice(
            Midi2ChannelVoice::NoteOn {
                group: 0,
                channel: 3,
                note: 60,
                attribute_type: 0,
                attribute: 0,
                velocity: 0x8000,
            },
        ));
        assert_eq!(ump.words[0], 0x4093_3C00);
        assert_eq!(ump.words[1], 0x8000_0000);
        assert_eq!(ump.num_words(), 2);
        let parsed = ump.parse().unwrap();
        match parsed {
            Midi2Message::Midi2ChannelVoice(Midi2ChannelVoice::NoteOn {
                channel,
                note,
                velocity,
                ..
            }) => {
                assert_eq!(channel, 3);
                assert_eq!(note, 60);
                assert_eq!(velocity, 0x8000);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn midi2_cc_layout() {
        let ump = Ump::from_message(&Midi2Message::Midi2ChannelVoice(
            Midi2ChannelVoice::ControlChange {
                group: 2,
                channel: 7,
                index: 74,
                value: 0xFFFF_FFFF,
            },
        ));
        assert_eq!(ump.words[0], 0x42B7_4A00);
        assert_eq!(ump.words[1], 0xFFFF_FFFF);
    }

    #[test]
    fn midi2_rpn_layout() {
        let ump = Ump::from_message(&Midi2Message::Midi2ChannelVoice(Midi2ChannelVoice::Rpn {
            group: 0,
            channel: 0,
            bank: 0,
            index: 0,
            value: 0x2000_0000,
        }));
        assert_eq!(ump.words[0], 0x4020_0000);
        assert_eq!(ump.words[1], 0x2000_0000);
        match ump.parse().unwrap() {
            Midi2Message::Midi2ChannelVoice(Midi2ChannelVoice::Rpn {
                bank, index, value, ..
            }) => {
                assert_eq!((bank, index, value), (0, 0, 0x2000_0000));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn midi2_program_change_bank() {
        let ump = Ump::from_message(&Midi2Message::Midi2ChannelVoice(
            Midi2ChannelVoice::ProgramChange {
                group: 0,
                channel: 1,
                option_flags: 0,
                program: 42,
                bank_valid: true,
                bank_msb: 3,
                bank_lsb: 7,
            },
        ));
        // bank_valid is bit 15 of word0.
        assert_eq!(ump.words[0], 0x40C1_8000);
        assert_eq!(ump.words[1], 0x2A00_0307);
        match ump.parse().unwrap() {
            Midi2Message::Midi2ChannelVoice(Midi2ChannelVoice::ProgramChange {
                program,
                bank_valid,
                bank_msb,
                bank_lsb,
                ..
            }) => {
                assert_eq!(program, 42);
                assert!(bank_valid);
                assert_eq!((bank_msb, bank_lsb), (3, 7));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn midi1_cv_in_ump() {
        let ump = Ump::from_message(&Midi2Message::Midi1ChannelVoice(Midi1ChannelVoice {
            group: 0,
            message: Midi1Message::NoteOn {
                channel: 5,
                note: 64,
                velocity: 100,
            },
        }));
        assert_eq!(ump.words[0], 0x2095_4064);
        assert_eq!(ump.num_words(), 1);
        match ump.parse().unwrap() {
            Midi2Message::Midi1ChannelVoice(cv) => {
                assert_eq!(
                    cv.message,
                    Midi1Message::NoteOn {
                        channel: 5,
                        note: 64,
                        velocity: 100
                    }
                );
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn sysex7_single_and_chunked() {
        // Short: complete in one packet.
        let ump = Ump::from_message(&Midi2Message::SystemExclusive(SysExMessage::Complete {
            group: 0,
            data: vec![0x7E, 0x7F, 0x06, 0x01],
        }));
        assert_eq!(ump.num_words(), 2);
        // 4 bytes: b0,b1 in word0 at [15:8],[7:0]; b2,b3 in word1.
        assert_eq!(ump.words[0], 0x3004_7E7F);
        assert_eq!(ump.words[1], 0x0601_0000);
        match ump.parse().unwrap() {
            Midi2Message::SystemExclusive(s) => assert_eq!(s.data(), &[0x7E, 0x7F, 0x06, 0x01]),
            other => panic!("unexpected {other:?}"),
        }

        // Long: chunked into 6-byte packets.
        let payload: Vec<u8> = (0..14).collect();
        let umps = chunk_sysex7(&SysExMessage::Start {
            group: 0,
            data: payload.clone(),
        });
        assert_eq!(umps.len(), 3);
        assert_eq!(umps[0].words[0] >> 20 & 0xF, 0x1, "start");
        assert_eq!(umps[1].words[0] >> 20 & 0xF, 0x2, "continue");
        assert_eq!(umps[2].words[0] >> 20 & 0xF, 0x3, "end");

        // Reassemble.
        let mut asm = SysexReassembler::default();
        for ump in &umps {
            if let Midi2Message::SystemExclusive(s) = ump.parse().unwrap() {
                if let Some(all) = asm.feed(&s).unwrap() {
                    assert_eq!(all, payload);
                    return;
                }
            }
        }
        panic!("never reassembled");
    }

    #[test]
    fn system_common_in_ump() {
        let ump = Ump::from_message(&Midi2Message::SystemCommon(
            SystemCommonMessage::SongPositionPointer {
                group: 0,
                position: 100,
            },
        ));
        // 100 = 0x64: lsb 0x64 at [15:8], msb 0 at [7:0].
        assert_eq!(ump.words[0], 0x10F2_6400);
    }

    #[test]
    fn utility_roundtrip() {
        let ump = Ump::from_message(&Midi2Message::Utility(UtilityMessage::Clock {
            clock: 0xBEEF,
        }));
        assert_eq!(ump.words[0], 0x0010_BEEF);
        match ump.parse().unwrap() {
            Midi2Message::Utility(UtilityMessage::Clock { clock }) => {
                assert_eq!(clock, 0xBEEF);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn flex_set_tempo_roundtrip() {
        let message = Midi2Message::FlexData(FlexDataMessage::SetTempo {
            group: 7,
            format: DataFormat::Complete,
            ten_nanosecond_units_per_quarter_note: 500_000, // 5 ms per qn? (test value)
        });
        let ump = Ump::from_message(&message);
        // 0xD710_0000 per the reference implementation test vector.
        assert_eq!(ump.words[0], 0xD710_0000);
        assert_eq!(ump.words[1], 500_000);
        match ump.parse().unwrap() {
            Midi2Message::FlexData(FlexDataMessage::SetTempo {
                group,
                ten_nanosecond_units_per_quarter_note: tempo,
                ..
            }) => {
                assert_eq!(group, 7);
                assert_eq!(tempo, 500_000);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn flex_time_signature_layout() {
        let message = Midi2Message::FlexData(FlexDataMessage::TimeSignature {
            group: 0xA,
            format: DataFormat::Complete,
            numerator: 4,
            denominator_exponent: 2,
            thirty_second_notes_per_quarter: 8,
        });
        let ump = Ump::from_message(&message);
        assert_eq!(ump.words[0], 0xDA10_0001);
        assert_eq!(ump.words[1], 0x0402_0800);
    }

    #[test]
    fn from_bytes_roundtrip() {
        let ump = Ump::from_message(&Midi2Message::Midi2ChannelVoice(
            Midi2ChannelVoice::PitchBend {
                group: 1,
                channel: 2,
                value: 0xC000_0000,
            },
        ));
        let bytes = ump.to_bytes();
        assert_eq!(bytes.len(), 8);
        assert_eq!(Ump::from_bytes(&bytes).unwrap(), ump);
        assert!(Ump::from_bytes(&[0u8; 3]).is_err());
        assert!(Ump::from_bytes(&[0u8; 20]).is_err());
    }

    #[test]
    fn translation_up_and_down() {
        // MIDI 1.0 note on velocity 64 → MIDI 2.0 16-bit → back to 7-bit.
        let m1 = Midi1Message::NoteOn {
            channel: 0,
            note: 60,
            velocity: 64,
        };
        let up = midi1_to_midi2(&m1, 0);
        assert_eq!(up.len(), 1);
        let down = midi2_to_midi1(&up[0]);
        assert_eq!(down.len(), 1);
        assert_eq!(down[0], m1);

        // Velocity-0 NoteOn becomes NoteOff.
        let m1 = Midi1Message::NoteOn {
            channel: 2,
            note: 40,
            velocity: 0,
        };
        let up = midi1_to_midi2(&m1, 0);
        match up[0] {
            Midi2Message::Midi2ChannelVoice(Midi2ChannelVoice::NoteOff { .. }) => {}
            ref other => panic!("expected note off, got {other:?}"),
        }

        // High-resolution CC downscale is stable at extremes.
        let cc = Midi2ChannelVoice::ControlChange {
            group: 0,
            channel: 0,
            index: 7,
            value: 0xFFFF_FFFF,
        };
        let down = midi2_to_midi1(&Midi2Message::Midi2ChannelVoice(cc));
        assert_eq!(
            down[0],
            Midi1Message::ControlChange {
                channel: 0,
                controller: 7,
                value: 127
            }
        );

        // Pitch bend center maps to center.
        let pb = Midi2Message::Midi2ChannelVoice(Midi2ChannelVoice::PitchBend {
            group: 0,
            channel: 0,
            value: 0x8000_0000,
        });
        let down = midi2_to_midi1(&pb);
        assert_eq!(
            down[0],
            Midi1Message::PitchBendChange {
                channel: 0,
                value: 8192
            }
        );
    }

    #[test]
    fn words_beyond_packet_rejected() {
        let mut ump = Ump::from_message(&Midi2Message::Utility(UtilityMessage::NoOp));
        ump.words[1] = 0xDEAD_BEEF;
        assert!(ump.parse().is_err());
    }
}
