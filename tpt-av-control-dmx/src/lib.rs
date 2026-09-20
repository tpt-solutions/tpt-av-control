//! # tpt-av-control-dmx
//!
//! DMX512, Art-Net 4, and sACN (ANSI E1.31) for the TPT AV control suite.
//!
//! - [`DmxUniverse`]: 512-channel universe data with fast set/get.
//! - [`ArtNet`]: ArtDmx send/receive over UDP (port 6454) with a minimal
//!   ArtPoll / ArtPollReply implementation.
//! - [`Sacn`]: E1.31 data packets over UDP (port 5568).
//! - [`DmxServer`]/[`DmxClient`]: auto-detecting server and protocol
//!   multiplexing.
//! - [`Fixture`] definitions for addressing real lighting rigs.
//!
//! # Security
//!
//! Art-Net and sACN have no authentication or encryption built into their
//! specs — anyone who can reach [`ArtNet`]'s or [`Sacn`]'s bound UDP port
//! can send it universe data. On untrusted networks, tunnel these protocols
//! over a VPN rather than exposing them directly. See
//! [`SECURITY.md`](https://github.com/tpt-solutions/tpt-av-control/blob/master/SECURITY.md)
//! for the full policy.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod artnet;
pub mod client;
pub mod dmx;
pub mod fixture;
pub mod sacn;
pub mod server;
pub mod universe;

pub use artnet::{ArtNet, ArtNetPacket};
pub use client::DmxClient;
pub use dmx::{DmxUniverse, DMX_CHANNELS};
#[cfg(feature = "serde")]
pub use fixture::FixturePatch;
pub use fixture::{Fixture, FixtureChannel, FixtureDefinition};
pub use sacn::{Cid, Sacn, SacnDataPacket};
pub use server::{DmxProtocol, DmxServer};
pub use universe::UniverseManager;

pub use tpt_av_control_utils::ControlError;
