//! # tpt-av-control-surface
//!
//! Hardware control surface integration for the TPT AV control suite.
//!
//! - The [`ControlSurface`] trait: a uniform `init` / `read_event` /
//!   `send_feedback` API over physical controllers.
//! - [`ControlEvent`] and [`Feedback`] as the protocol-neutral event model.
//! - [`SurfaceMapping`]: declarative controller descriptions.
//! - Built-in surfaces: [`GenericMidiSurface`] (any MIDI controller),
//!   [`BehringerX32Surface`] (X32/M32 over OSC), [`StreamDeckSurface`]
//!   (Elgato Stream Deck over a pluggable HID transport), and
//!   [`CustomSurface`] (closure-based).

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod feedback;
pub mod mapping;
pub mod surface;
pub mod surfaces;

pub use feedback::Feedback;
pub use mapping::{ButtonMapping, EncoderMapping, FaderMapping, SurfaceMapping};
pub use surface::{ControlEvent, ControlSurface};
pub use surfaces::behringer_x32::{BehringerX32Surface, X32_PORT};
pub use surfaces::custom::{CustomSurface, CustomSurfaceBuilder};
pub use surfaces::elgato_streamdeck::{DeckModel, HidDevice, StreamDeckSurface};
pub use surfaces::generic_midi::GenericMidiSurface;

pub use tpt_av_control_utils::ControlError;
