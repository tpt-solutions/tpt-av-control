//! # tpt-av-control-utils
//!
//! Shared types for the TPT AV control suite: the [`ControlError`] error
//! type, a generic [`Message`] envelope with a real-time-safe SPSC ring
//! queue, timecode/timing types, and parameter mapping primitives.
//!
//! This crate is the dependency-free foundation every other
//! `tpt-av-control-*` crate builds on.

#![warn(missing_docs)]
// `ring.rs` is the one sanctioned `unsafe` module (the lock-free SPSC
// queue); everything else must stay unsafe-free.
#![deny(unsafe_code)]

pub mod error;
pub mod message;
pub mod parameter;
#[allow(unsafe_code)]
pub mod ring;
pub mod time;

pub use error::ControlError;
pub use message::{Message, MessageBody, MessageQueue, MessageSource};
pub use parameter::{
    Automation, AutomationPoint, Curve, Mapping, ParameterId, ParameterValue,
};
pub use ring::SpscRing;
pub use time::{FrameRate, Timecode, Timestamp};
