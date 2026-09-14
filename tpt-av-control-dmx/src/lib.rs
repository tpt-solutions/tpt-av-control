//! # tpt-av-control-dmx
//!
//! DMX512, Art-Net 4, and sACN (ANSI E1.31) for the TPT AV control suite.
//!
//! - [`DmxUniverse`]: 512-channel universe data with fast set/get.
//! - [`ArtNet`]: ArtDmx send/receive over UDP (port 6454) plus ArtPoll.
//! - [`Sacn`]: E1.31 data packets over UDP (port 5568).
//! - Universe management and lighting fixture definitions.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod artnet;
pub mod client;
pub mod dmx;
pub mod fixture;
pub mod sacn;
pub mod server;
pub mod universe;

// TODO(phase): pub use artnet::ArtNet;
// TODO(phase): pub use client::DmxClient;
// TODO(phase): pub use dmx::DmxUniverse;
// TODO(phase): pub use fixture::{Fixture, FixtureChannel, FixtureDefinition, FixtureType};
// TODO(phase): pub use sacn::Sacn;
// TODO(phase): pub use server::DmxServer;
// TODO(phase): pub use universe::UniverseManager;

// TODO(phase): pub use tpt_av_control_utils::ControlError;
