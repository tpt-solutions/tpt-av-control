//! # tpt-av-control-webrtc
//!
//! WebRTC data channels for networked AV control.
//!
//! This crate defines the control-plane message envelope and the
//! [`transport::DataChannelTransport`] abstraction used to carry them over
//! WebRTC data channels. It ships with a reliable in-process loopback
//! transport for testing and local IPC; a full ICE/DTLS/SCTP stack can be
//! plugged in by implementing the transport trait.
//!
//! # Security
//!
//! [`ControlEnvelope`] is a bare binary format with no authentication or
//! encryption of its own — that's expected to come from whatever transport
//! you plug into [`DataChannelTransport`] (WebRTC's DTLS/SCTP gives you
//! this "for free" if you wire up a real ICE stack; [`LoopbackTransport`]
//! does not, since it's for local/testing use only). See
//! [`SECURITY.md`](https://github.com/tpt-solutions/tpt-av-control/blob/master/SECURITY.md)
//! for the full policy.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod envelope;
pub mod transport;

pub use envelope::{ControlEnvelope, ParameterChangeRequest};
pub use transport::{DataChannelTransport, LoopbackTransport};

pub use tpt_av_control_utils::{ControlError, TransportCommand};
