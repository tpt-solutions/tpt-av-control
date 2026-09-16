//! MIDI 2.0 message types (as carried in Universal MIDI Packets).
//!
//! Bit layouts follow M2-101-U; the channel-voice opcodes are:
//! Per-Note RCC `0x0`, Per-Note ACC `0x1`, RPN `0x2`, NRPN `0x3`,
//! Relative RPN `0x4`, Relative NRPN `0x5`, Per-Note Pitch Bend `0x6`,
//! Note Off `0x8`, Note On `0x9`, Poly Pressure `0xA`, Control Change
//! `0xB`, Program Change `0xC`, Channel Pressure `0xD`, Pitch Bend `0xE`,
//! Per-Note Management `0xF`.

use crate::midi1::Midi1Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// UMP Utility messages (type `0x0`).
pub enum UtilityMessage {
    /// No operation.
    NoOp,
    /// Transport clock tick (16-bit counter).
    /// Transport clock tick counter.
    /// Clock — see the variant name.
    Clock {
        /// Transport clock tick counter.
        clock: u16,
    },
    /// Timestamp in milliseconds (16-bit).
    /// Timestamp value in milliseconds.
    /// Timestamp — see the variant name.
    Timestamp {
        /// Timestamp value in milliseconds.
        timestamp: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// UMP System Common / Real-Time messages (type `0x1`).
pub enum SystemCommonMessage {
    /// MIDI time code quarter frame.
    /// UMP group (0-15).
    /// Quarter-frame payload (piece index in the high nibble).
    /// MIDI time code quarter frame.
    TimeCodeQuarterFrame {
        /// UMP group (0-15).
        group: u8,
        /// Quarter-frame payload (piece index in the high nibble).
        quarter_frame: u8,
    },
    /// Song position pointer (14-bit).
    /// UMP group (0-15).
    /// Song position in 16th notes (14-bit).
    /// Song position pointer (14-bit).
    SongPositionPointer {
        /// UMP group (0-15).
        group: u8,
        /// Song position in 16th notes (14-bit).
        position: u16,
    },
    /// Song select.
    /// UMP group (0-15).
    /// Song number (0-127).
    /// Song select.
    SongSelect {
        /// UMP group (0-15).
        group: u8,
        /// Song number (0-127).
        song: u8,
    },
    /// Tune request.
    /// UMP group (0-15).
    /// Tune request.
    TuneRequest {
        /// UMP group (0-15).
        group: u8,
    },
    /// Timing clock.
    /// UMP group (0-15).
    /// Timing clock (24 per quarter note).
    TimingClock {
        /// UMP group (0-15).
        group: u8,
    },
    /// Start transport.
    /// UMP group (0-15).
    /// First chunk of a multi-packet SysEx (status `0x1`).
    Start {
        /// UMP group (0-15).
        group: u8,
    },
    /// Continue transport.
    /// UMP group (0-15).
    /// Middle chunk (status `0x2`).
    Continue {
        /// UMP group (0-15).
        group: u8,
    },
    /// Stop transport.
    /// UMP group (0-15).
    /// STOP — stop the (optional) cue.
    Stop {
        /// UMP group (0-15).
        group: u8,
    },
    /// Active sensing.
    /// UMP group (0-15).
    /// Active sensing keep-alive.
    ActiveSensing {
        /// UMP group (0-15).
        group: u8,
    },
    /// System reset.
    /// UMP group (0-15).
    /// System reset.
    SystemReset {
        /// UMP group (0-15).
        group: u8,
    },
}

impl SystemCommonMessage {
    /// The UMP group.
    pub fn group(&self) -> u8 {
        match self {
            SystemCommonMessage::TimeCodeQuarterFrame { group, .. }
            | SystemCommonMessage::SongPositionPointer { group, .. }
            | SystemCommonMessage::SongSelect { group, .. }
            | SystemCommonMessage::TuneRequest { group }
            | SystemCommonMessage::TimingClock { group }
            | SystemCommonMessage::Start { group }
            | SystemCommonMessage::Continue { group }
            | SystemCommonMessage::Stop { group }
            | SystemCommonMessage::ActiveSensing { group }
            | SystemCommonMessage::SystemReset { group } => *group,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// UMP Data messages: System Exclusive carried as 6-byte 7-bit chunks
pub enum SysExMessage {
    /// A complete SysEx payload in one UMP (status `0x0`).
    /// UMP group (0-15).
    /// Chunk payload bytes (up to 6 per packet).
    /// A complete SysEx payload in one UMP (status `0x0`).
    Complete {
        /// UMP group (0-15).
        group: u8,
        /// Chunk payload bytes (up to 6 per packet).
        data: Vec<u8>,
    },
    /// First chunk of a multi-packet SysEx (status `0x1`).
    /// UMP group (0-15).
    /// Chunk payload bytes (up to 6 per packet).
    /// First chunk of a multi-packet SysEx (status `0x1`).
    Start {
        /// UMP group (0-15).
        group: u8,
        /// Chunk payload bytes (up to 6 per packet).
        data: Vec<u8>,
    },
    /// Middle chunk (status `0x2`).
    /// UMP group (0-15).
    /// Chunk payload bytes (up to 6 per packet).
    /// Middle chunk (status `0x2`).
    Continue {
        /// UMP group (0-15).
        group: u8,
        /// Chunk payload bytes (up to 6 per packet).
        data: Vec<u8>,
    },
    /// Final chunk (status `0x3`).
    /// UMP group (0-15).
    /// Chunk payload bytes (up to 6 per packet).
    /// Final chunk (status `0x3`).
    End {
        /// UMP group (0-15).
        group: u8,
        /// Chunk payload bytes (up to 6 per packet).
        data: Vec<u8>,
    },
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

    /// The UMP group the message belongs to.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Midi1ChannelVoice {
    /// UMP group.
    pub group: u8,
    /// The MIDI 1.0 message.
    pub message: Midi1Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// MIDI 1.0 channel voice wrapped in a UMP (type `0x2`).
pub enum Midi2ChannelVoice {
    /// Registered per-note controller (opcode `0x0`).
    PerNoteRcc {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// Note number (0-127).
        note: u8,
        /// Enumeration-order index.
        index: u8,
        /// Value, full scale for the protocol version.
        value: u32,
    },
    /// Assignable per-note controller (opcode `0x1`).
    PerNoteAcc {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// Note number (0-127).
        note: u8,
        /// Enumeration-order index.
        index: u8,
        /// Value, full scale for the protocol version.
        value: u32,
    },
    /// Registered parameter number (opcode `0x2`).
    Rpn {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// RPN/NRPN bank number (7-bit).
        bank: u8,
        /// Enumeration-order index.
        index: u8,
        /// Value, full scale for the protocol version.
        value: u32,
    },
    /// Non-registered parameter number (opcode `0x3`).
    Nrpn {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// RPN/NRPN bank number (7-bit).
        bank: u8,
        /// Enumeration-order index.
        index: u8,
        /// Value, full scale for the protocol version.
        value: u32,
    },
    /// Relative registered parameter number (opcode `0x4`); `value` is a
    /// 32-bit two's-complement delta.
    RelativeRpn {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// RPN/NRPN bank number (7-bit).
        bank: u8,
        /// Enumeration-order index.
        index: u8,
        /// Value, full scale for the protocol version.
        value: u32,
    },
    /// Relative non-registered parameter number (opcode `0x5`).
    RelativeNrpn {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// RPN/NRPN bank number (7-bit).
        bank: u8,
        /// Enumeration-order index.
        index: u8,
        /// Value, full scale for the protocol version.
        value: u32,
    },
    /// Per-note pitch bend (opcode `0x6`).
    PerNotePitchBend {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// Note number (0-127).
        note: u8,
        /// Value, full scale for the protocol version.
        value: u32,
    },
    /// Note off with 16-bit velocity (opcode `0x8`).
    NoteOff {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// Note number (0-127).
        note: u8,
        /// Attribute type (0 = none).
        attribute_type: u8,
        /// Attribute data.
        attribute: u16,
        /// Velocity, full scale for the protocol version.
        velocity: u16,
    },
    /// Note on with 16-bit velocity (opcode `0x9`).
    NoteOn {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// Note number (0-127).
        note: u8,
        /// Attribute type (0 = none).
        attribute_type: u8,
        /// Attribute data.
        attribute: u16,
        /// Velocity, full scale for the protocol version.
        velocity: u16,
    },
    /// Polyphonic key pressure, 32-bit (opcode `0xA`).
    PolyphonicKeyPressure {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// Note number (0-127).
        note: u8,
        /// Pressure, full scale for the protocol version.
        pressure: u32,
    },
    /// Control change, 32-bit (opcode `0xB`).
    ControlChange {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// Enumeration-order index.
        index: u8,
        /// Value, full scale for the protocol version.
        value: u32,
    },
    /// Program change with optional bank select (opcode `0xC`).
    ProgramChange {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// Vendor-defined option flags.
        option_flags: u8,
        /// Program (patch) number.
        program: u8,
        /// Whether the bank select fields are meaningful.
        bank_valid: bool,
        /// Bank select MSB (7-bit).
        bank_msb: u8,
        /// Bank select LSB (7-bit).
        bank_lsb: u8,
    },
    /// Channel pressure, 32-bit (opcode `0xD`).
    ChannelPressure {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// Pressure, full scale for the protocol version.
        pressure: u32,
    },
    /// Pitch bend, 32-bit centered at `0x8000_0000` (opcode `0xE`).
    PitchBend {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// Value, full scale for the protocol version.
        value: u32,
    },
    /// Per-note management (opcode `0xF`): note-on/off attributes and
    /// controller reset flags.
    PerNoteManagement {
        /// UMP group (0-15).
        group: u8,
        /// Channel (0-15).
        channel: u8,
        /// Note number (0-127).
        note: u8,
        /// Vendor-defined option flags.
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

    /// The channel (0-15).
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

#[derive(Debug, Clone, PartialEq, Eq)]
/// Flex Data messages (type `0xD`): studio metadata carried in UMPs.
pub enum FlexDataMessage {
    /// Set Tempo (status `0x00`): quarter note duration in 10 ns units.
    SetTempo {
        /// UMP group (0-15).
        group: u8,
        /// Multi-packet form (complete/start/continue/end).
        format: DataFormat,
        /// Quarter note duration in 10 ns units.
        ten_nanosecond_units_per_quarter_note: u32,
    },
    /// Time Signature (status `0x01`).
    TimeSignature {
        /// UMP group (0-15).
        group: u8,
        /// Multi-packet form (complete/start/continue/end).
        format: DataFormat,
        /// Time signature numerator.
        numerator: u8,
        /// Denominator as a power-of-two exponent (e.g. 2 = quarter note).
        denominator_exponent: u8,
        /// Number of 32nd notes per quarter note.
        thirty_second_notes_per_quarter: u8,
    },
    /// Metadata / text messages (statuses `0x04`-`0x0C` performance text,
    /// `0x10`+ project text); the text is 7-bit ASCII.
    Text {
        /// UMP group (0-15).
        group: u8,
        /// Multi-packet form (complete/start/continue/end).
        format: DataFormat,
        /// RPN/NRPN bank number (7-bit).
        bank: u8,
        /// Message status number.
        status: u8,
        /// Channel (0-15).
        channel: u8,
        /// The text payload (7-bit characters).
        text: String,
    },
    /// Any other flex-data message, preserved losslessly.
    Other {
        /// UMP group (0-15).
        group: u8,
        /// Multi-packet form (complete/start/continue/end).
        format: DataFormat,
        /// RPN/NRPN bank number (7-bit).
        bank: u8,
        /// Message status number.
        status: u8,
        /// Channel (0-15).
        channel: u8,
        /// Chunk payload bytes (up to 6 per packet).
        data: [u32; 4],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Form of a multi-packet data message (2-bit form field).
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataMessage {
    /// UMP group.
    pub group: u8,
    /// Number of meaningful words (2-4).
    pub word_count: usize,
    /// Raw words including the header.
    pub words: [u32; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A MIDI 2.0 message as carried in a Universal MIDI Packet.
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
