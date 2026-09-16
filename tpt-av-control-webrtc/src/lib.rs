//! # tpt-av-control-webrtc
//!
//! WebRTC data channels for networked AV control.
//!
//! This crate defines the control-plane message envelope and the
//! [`transport::DataChannelTransport`] abstraction used to carry them over
//! WebRTC data channels. It ships with a reliable in-process loopback
//! transport for testing and local IPC; a full ICE/DTLS/SCTP stack can be
//! plugged in by implementing the transport trait.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod envelope;
pub mod transport;

pub use envelope::{ControlEnvelope, ParameterChangeRequest};
pub use transport::{DataChannelTransport, LoopbackTransport};

pub use tpt_av_control_utils::{ControlError, TransportCommand};
