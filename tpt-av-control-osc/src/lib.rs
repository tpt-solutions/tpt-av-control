//! # tpt-av-control-osc
//!
//! Open Sound Control 1.0/1.1 for the TPT AV control suite.
//!
//! - Message, bundle, and type-tag encode/decode validated against the OSC spec.
//! - A real-time-safe, zero-copy parser ([`parse_osc_message`]) that borrows
//!   directly from the packet buffer.
//! - UDP [`OscServer`] (blocking and tokio async) and [`OscClient`].
//! - OSC address pattern matching ([`OscAddressMatcher`]) and a
//!   message [`dispatcher`](dispatch::OscDispatcher).

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod address;
pub mod bundle;
pub mod client;
pub mod dispatch;
pub mod message;
pub mod server;
pub mod types;

pub use address::OscAddressMatcher;
pub use bundle::{parse_bundle, OscBundle, OscPacket};
pub use client::OscClient;
pub use dispatch::OscDispatcher;
pub use message::{parse_osc_message, OscArg, OscMessage, OscMessageRef};
pub use server::OscServer;

pub use tpt_av_control_utils::ControlError;
