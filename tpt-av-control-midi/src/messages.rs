//! MIDI 2.0 message types (as carried in Universal MIDI Packets).
//!
//! Bit layouts follow M2-101-U; the channel-voice opcodes are:
//! Per-Note RCC `0x0`, Per-Note ACC `0x1`, RPN `0x2`, NRPN `0x3`,
//! Relative RPN `0x4`, Relative NRPN `0x5`, Per-Note Pitch Bend `0x6`,
//! Note Off `0x8`, Note On `0x9`, Poly Pressure `0xA`, Control Change
//! `0xB`, Program Change `0xC`, Channel Pressure `0xD`, Pitch Bend `0xE`,
//! Per-Note Management `0xF`.

use crate::midi1::Midi1Message;

/// UMP Utility messages (type `0x0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UtilityMessage {
    /// No operation.
    NoOp,
    /// Transport clock tick (16-bit counter).
    Clock { clock: u16 },
    /// Timestamp in milliseconds (16-bit).
    Timestamp { timestamp: u16 },
}

/// UMP System Common / Real-Time messages (type `0x1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemCommonMessage {
    /// MIDI time code quarter frame.
    TimeCodeQuarterFrame { group: u8, quarter_frame: u8 },
    /// Song position pointer (14-bit).
    SongPositionPointer { group: u8, position: u16 },
    /// Song select.
    SongSelect { group: u8, song: u8 },
    /// Tune request.
    TuneRequest { group: u8 },
    /// Timing clock.
    TimingClock { group: u8 },
    /// Start transport.
    Start { group: u8 },
    /// Continue transport.
    Continue { group: u8 },
    /// Stop transport.
    Stop { group: u8 },
    /// Active sensing.
    ActiveSensing { group: u8 },
    /// System reset.
    SystemReset { group: u8 },
}

/// UMP Data messages: System Exclusive carried as 6-byte 7-bit chunks
/// (type `0x3`), or unknown/extended data preserved losslessly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SysExMessage {
    /// A complete SysEx payload in one UMP (status `0x0`).
    Complete { group: u8, data: Vec<u8> },
    /// First chunk of a multi-packet SysEx (status `0x1`).
    Start { group: u8, data: Vec<u8> },
    /// Middle chunk (status `0x2`).
    Continue { group: u8, data: Vec<u8> },
    /// Final chunk (status `0x3`).
    End { group: u8, data: Vec<u8> },
}

impl SysExMessage {
    /// The chunk's data bytes.
    pub fn data(&self) -> &[u8] {
        match self {
            SysExMessage::Complete { data, .. }
            | SysExMessage::Start { data, .. }
            | SysExMessage::Continue { data, .. }
            | SysExMessage::End { data, .. } => data,
        }
    }

    /// The group the message belongs to.
    pub fn group(&self) -> u8 {
        match self {
            SysExMessage::Complete { group, .. }
            | SysExMessage::Start { group, .. }
            | SysExMessage::Continue { group, .. }
            | SysExMessage::End { group, .. } => *group,
        }
    }

    /// The chunk status code (`0x0`..`0x3`).
    pub fn status_code(&self) -> u8 {
        match self {
            SysExMessage::Complete { .. } => 0x0,
            SysExMessage::Start { .. } => 0x1,
            SysExMessage::Continue { .. } => 0x2,
            SysExMessage::End { .. } => 0x3,
        }
    }
}

/// MIDI 1.0 channel voice wrapped in a UMP (type `0x2`). The UMP group is
/// kept alongside the message because `Midi1Message` carries only the
/// channel nibble.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Midi1ChannelVoice {
    /// UMP group.
    pub group: u8,
    /// The MIDI 1.0 message.
    pub message: Midi1Message,
}

/// MIDI 2.0 high-resolution channel voice messages (type `0x4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Midi2ChannelVoice {
    /// Registered per-note controller (opcode `0x0`).
    PerNoteRcc {
        group: u8,
        channel: u8,
        note: u8,
        index: u8,
        value: u32,
    },
    /// Assignable per-note controller (opcode `0x1`).
    PerNoteAcc {
        group: u8,
        channel: u8,
        note: u8,
        index: u8,
        value: u32,
    },
    /// Registered parameter number (opcode `0x2`).
    Rpn {
        group: u8,
        channel: u8,
        bank: u8,
        index: u8,
        value: u32,
    },
    /// Non-registered parameter number (opcode `0x3`).
    Nrpn {
        group: u8,
        channel: u8,
        bank: u8,
        index: u8,
        value: u32,
    },
    /// Relative registered parameter number (opcode `0x4`); `value` is a
    /// 32-bit two's-complement delta.
    RelativeRpn {
        group: u8,
        channel: u8,
        bank: u8,
        index: u8,
        value: u32,
    },
    /// Relative non-registered parameter number (opcode `0x5`).
    RelativeNrpn {
        group: u8,
        channel: u8,
        bank: u8,
        index: u8,
        value: u32,
    },
    /// Per-note pitch bend (opcode `0x6`).
    PerNotePitchBend {
        group: u8,
        channel: u8,
        note: u8,
        value: u32,
    },
    /// Note off with 16-bit velocity (opcode `0x8`).
    NoteOff {
        group: u8,
        channel: u8,
        note: u8,
        attribute_type: u8,
        attribute: u16,
        velocity: u16,
    },
    /// Note on with 16-bit velocity (opcode `0x9`).
    NoteOn {
        group: u8,
        channel: u8,
        note: u8,
        attribute_type: u8,
        attribute: u16,
        velocity: u16,
    },
    /// Polyphonic key pressure, 32-bit (opcode `0xA`).
    PolyphonicKeyPressure {
        group: u8,
        channel: u8,
        note: u8,
        pressure: u32,
    },
    /// Control change, 32-bit (opcode `0xB`).
    ControlChange {
        group: u8,
        channel: u8,
        index: u8,
        value: u32,
    },
    /// Program change with optional bank select (opcode `0xC`).
    ProgramChange {
        group: u8,
        channel: u8,
        option_flags: u8,
        program: u8,
        bank_valid: bool,
        bank_msb: u8,
        bank_lsb: u8,
    },
    /// Channel pressure, 32-bit (opcode `0xD`).
    ChannelPressure {
        group: u8,
        channel: u8,
        pressure: u32,
    },
    /// Pitch bend, 32-bit centered at `0x8000_0000` (opcode `0xE`).
    PitchBend {
        group: u8,
        channel: u8,
        value: u32,
    },
    /// Per-note management (opcode `0xF`): note-on/off attributes and
    /// controller reset flags.
    PerNoteManagement {
        group: u8,
        channel: u8,
        note: u8,
        option_flags: u8,
    },
}

impl Midi2ChannelVoice {
    /// The UMP opcode for this message.
    pub fn opcode(&self) -> u8 {
        match self {
            Midi2ChannelVoice::PerNoteRcc { .. } => 0x0,
            Midi2ChannelVoice::PerNoteAcc { .. } => 0x1,
            Midi2ChannelVoice::Rpn { .. } => 0x2,
            Midi2ChannelVoice::Nrpn { .. } => 0x3,
            Midi2ChannelVoice::RelativeRpn { .. } => 0x4,
            Midi2ChannelVoice::RelativeNrpn { .. } => 0x5,
            Midi2ChannelVoice::PerNotePitchBend { .. } => 0x6,
            Midi2ChannelVoice::NoteOff { .. } => 0x8,
            Midi2ChannelVoice::NoteOn { .. } => 0x9,
            Midi2ChannelVoice::PolyphonicKeyPressure { .. } => 0xA,
            Midi2ChannelVoice::ControlChange { .. } => 0xB,
            Midi2ChannelVoice::ProgramChange { .. } => 0xC,
            Midi2ChannelVoice::ChannelPressure { .. } => 0xD,
            Midi2ChannelVoice::PitchBend { .. } => 0xE,
            Midi2ChannelVoice::PerNoteManagement { .. } => 0xF,
        }
    }

    /// The channel (0-15).
    pub fn channel(&self) -> u8 {
        match self {
            Midi2ChannelVoice::PerNoteRcc { channel, .. }
            | Midi2ChannelVoice::PerNoteAcc { channel, .. }
            | Midi2ChannelVoice::Rpn { channel, .. }
            | Midi2ChannelVoice::Nrpn { channel, .. }
            | Midi2ChannelVoice::RelativeRpn { channel, .. }
            | Midi2ChannelVoice::RelativeNrpn { channel, .. }
            | Midi2ChannelVoice::PerNotePitchBend { channel, .. }
            | Midi2ChannelVoice::NoteOff { channel, .. }
            | Midi2ChannelVoice::NoteOn { channel, .. }
            | Midi2ChannelVoice::PolyphonicKeyPressure { channel, .. }
            | Midi2ChannelVoice::ControlChange { channel, .. }
            | Midi2ChannelVoice::ProgramChange { channel, .. }
            | Midi2ChannelVoice::ChannelPressure { channel, .. }
            | Midi2ChannelVoice::PitchBend { channel, .. }
            | Midi2ChannelVoice::PerNoteManagement { channel, .. } => *channel,
        }
    }

    /// The UMP group.
    pub fn group(&self) -> u8 {
        match self {
            Midi2ChannelVoice::PerNoteRcc { group, .. }
            | Midi2ChannelVoice::PerNoteAcc { group, .. }
            | Midi2ChannelVoice::Rpn { group, .. }
            | Midi2ChannelVoice::Nrpn { group, .. }
            | Midi2ChannelVoice::RelativeRpn { group, .. }
            | Midi2ChannelVoice::RelativeNrpn { group, .. }
            | Midi2ChannelVoice::PerNotePitchBend { group, .. }
            | Midi2ChannelVoice::NoteOff { group, .. }
            | Midi2ChannelVoice::NoteOn { group, .. }
            | Midi2ChannelVoice::PolyphonicKeyPressure { group, .. }
            | Midi2ChannelVoice::ControlChange { group, .. }
            | Midi2ChannelVoice::ProgramChange { group, .. }
            | Midi2ChannelVoice::ChannelPressure { group, .. }
            | Midi2ChannelVoice::PitchBend { group, .. }
            | Midi2ChannelVoice::PerNoteManagement { group, .. } => *group,
        }
    }
}

/// Flex Data messages (type `0xD`): studio metadata carried in UMPs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlexDataMessage {
    /// Set Tempo (status `0x00`): quarter note duration in 10 ns units.
    SetTempo {
        group: u8,
        format: DataFormat,
        ten_nanosecond_units_per_quarter_note: u32,
    },
    /// Time Signature (status `0x01`).
    TimeSignature {
        group: u8,
        format: DataFormat,
        numerator: u8,
        /// Denominator as a power-of-two exponent (e.g. 2 = quarter note).
        denominator_exponent: u8,
        /// Number of 32nd notes per quarter note.
        thirty_second_notes_per_quarter: u8,
    },
    /// Metadata / text messages (statuses `0x04`-`0x0C` performance text,
    /// `0x10`+ project text); the text is 7-bit ASCII.
    Text {
        group: u8,
        format: DataFormat,
        bank: u8,
        status: u8,
        channel: u8,
        text: String,
    },
    /// Any other flex-data message, preserved losslessly.
    Other {
        group: u8,
        format: DataFormat,
        bank: u8,
        status: u8,
        channel: u8,
        data: [u32; 4],
    },
}

/// Form of a multi-packet data message (2-bit form field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataFormat {
    /// The whole payload in one packet.
    Complete,
    /// First packet.
    Start,
    /// Continuation packet.
    Continue,
    /// Final packet.
    End,
}

impl DataFormat {
    /// From the 2-bit form field.
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0x3 {
            0 => DataFormat::Complete,
            1 => DataFormat::Start,
            2 => DataFormat::Continue,
            _ => DataFormat::End,
        }
    }

    /// To the 2-bit form field.
    pub fn to_bits(self) -> u8 {
        match self {
            DataFormat::Complete => 0,
            DataFormat::Start => 1,
            DataFormat::Continue => 2,
            DataFormat::End => 3,
        }
    }
}

/// Extended / unknown data messages (type `0x5`), preserved losslessly as
/// raw words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataMessage {
    /// UMP group.
    pub group: u8,
    /// Number of meaningful words (2-4).
    pub word_count: usize,
    /// Raw words including the header.
    pub words: [u32; 4],
}

/// A MIDI 2.0 message as carried in a Universal MIDI Packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Midi2Message {
    /// Utility (type `0x0`).
    Utility(UtilityMessage),
    /// System common / real-time (type `0x1`).
    SystemCommon(SystemCommonMessage),
    /// System Exclusive data (type `0x3`).
    SystemExclusive(SysExMessage),
    /// MIDI 1.0 channel voice in UMP form (type `0x2`).
    Midi1ChannelVoice(Midi1ChannelVoice),
    /// MIDI 2.0 channel voice (type `0x4`).
    Midi2ChannelVoice(Midi2ChannelVoice),
    /// Extended/unknown data (type `0x5`).
    DataMessage(DataMessage),
    /// Flex data (type `0xD`).
    FlexData(FlexDataMessage),
}

impl Midi2Message {
    /// The UMP message type nibble for this message.
    pub fn message_type(&self) -> u8 {
        match self {
            Midi2Message::Utility(_) => 0x0,
            Midi2Message::SystemCommon(_) => 0x1,
            Midi2Message::Midi1ChannelVoice(_) => 0x2,
            Midi2Message::SystemExclusive(_) => 0x3,
            Midi2Message::Midi2ChannelVoice(_) => 0x4,
            Midi2Message::DataMessage(_) => 0x5,
            Midi2Message::FlexData(_) => 0xD,
        }
    }
}
