//! # tpt-av-control-midi
//!
//! MIDI 1.0 and MIDI 2.0 (Universal MIDI Packet) for the TPT AV control
//! suite.
//!
//! - Real-time-safe MIDI 1.0 byte-stream parser and encoder.
//! - Full UMP support: Utility, System Common, SysEx (8-bit data),
//!   MIDI 1.0 and MIDI 2.0 channel voice, Data, and Flex Data messages.
//! - MIDI-CI discovery and capability negotiation message layer.
//! - Device enumeration and I/O wrapping `midir`, MIDI clock/transport,
//!   MIDI Time Code, and MIDI Show Control.

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
