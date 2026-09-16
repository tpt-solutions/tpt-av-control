//! # tpt-av-control-midi
//!
//! MIDI 1.0 and MIDI 2.0 (Universal MIDI Packet) for the TPT AV control
//! suite.
//!
//! - Real-time-safe MIDI 1.0 byte-stream parser and encoder.
//! - Full UMP support: Utility, System Common, SysEx (7-bit data chunks),
//!   MIDI 1.0 and MIDI 2.0 channel voice, extended data, and Flex Data
//!   messages, with a MIDI 1.0 ↔ 2.0 translation layer.
//! - MIDI-CI discovery and capability negotiation message layer.
//! - Device enumeration and I/O wrapping `midir`, MIDI clock/transport,
//!   MIDI Time Code, MIDI Show Control, and parameter mapping.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod ci;
pub mod clock;
pub mod device;
pub mod mapping;
pub mod messages;
pub mod midi1;
pub mod msc;
pub mod mtc;
pub mod port;
pub mod sysex;
pub mod ump;

pub use ci::{CiDiscoverySession, CiMessage};
pub use clock::{MidiClockReceiver, MidiClockSender, TransportState};
pub use device::{
    enumerate_devices, open_input, open_output, InboundMidi, MidiDevice, MidiInput, MidiOutput,
    MidiSink, MidiSource, VirtualMidiPair, VirtualSink, VirtualSource,
};
pub use mapping::{CcBinding, MappedParameter, MidiParameterMapper, NoteBinding};
pub use messages::{
    DataFormat, DataMessage, FlexDataMessage, Midi1ChannelVoice, Midi2ChannelVoice, Midi2Message,
    SysExMessage, SystemCommonMessage, UtilityMessage,
};
pub use midi1::{encode_midi1, parse_midi1, Midi1Message};
pub use msc::{MscCommand, MscMessage};
pub use mtc::{MtcDecoder, MtcFrameRate};
pub use port::{MidiPort, PortDirection, PortId};
pub use ump::{SysexReassembler, Ump};

pub use tpt_av_control_utils::ControlError;
