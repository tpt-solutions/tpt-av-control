//! # tpt-av-control-surface
//!
//! Hardware control surface integration for the TPT AV control suite.
//!
//! - The [`ControlSurface`](surface::ControlSurface) trait: a uniform
//!   `init` / `read_event` / `send_feedback` API over physical controllers.
//! - [`ControlEvent`] and [`Feedback`] as the protocol-neutral event model.
//! - Built-in surfaces: generic MIDI controllers, Behringer X32/M32 (OSC),
//!   Elgato Stream Deck (pure-Rust HID protocol), and a config-driven
//!   custom surface.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod feedback;
pub mod mapping;
pub mod surface;
pub mod surfaces;

// TODO(phase): pub use feedback::Feedback;
// TODO(phase): pub use mapping::{ButtonMapping, EncoderMapping, FaderMapping, SurfaceMapping};
// TODO(phase): pub use surface::{ControlEvent, ControlSurface};
// TODO(phase): pub use surfaces::behringer_x32::BehringerX32Surface;
// TODO(phase): pub use surfaces::custom::CustomSurface;
// TODO(phase): pub use surfaces::elgato_streamdeck::StreamDeckSurface;
// TODO(phase): pub use surfaces::generic_midi::GenericMidiSurface;

// TODO(phase): pub use tpt_av_control_utils::ControlError;
