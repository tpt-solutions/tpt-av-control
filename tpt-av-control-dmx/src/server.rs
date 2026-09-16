//! DMX server: receives Art-Net and sACN universes into a shared
//! [`UniverseManager`].

use crate::artnet;
use crate::dmx::DmxUniverse;
use crate::sacn;
use crate::universe::UniverseManager;
use std::net::{SocketAddr, UdpSocket};
use tpt_av_control_utils::ControlError;

/// Which wire protocol a received universe arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmxProtocol {
    /// Art-Net (ArtDmx).
    ArtNet,
    /// sACN / E1.31.
    Sacn,
}

/// A DMX server that accepts both Art-Net and sACN on one socket and
/// keeps the latest state per universe.
pub struct DmxServer {
    socket: UdpSocket,
    manager: UniverseManager,
    /// The last protocol seen (updated per datagram).
    last_protocol: Option<DmxProtocol>,
}

impl DmxServer {
    /// Binds a server on `0.0.0.0:port`.
    pub fn new(port: u16) -> Result<Self, ControlError> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        Ok(Self {
            socket,
            manager: UniverseManager::new(),
            last_protocol: None,
        })
    }

    /// Binds a server on a specific address (tests use loopback).
    pub fn bind(addr: SocketAddr) -> Result<Self, ControlError> {
        let socket = UdpSocket::bind(addr)?;
        Ok(Self {
            socket,
            manager: UniverseManager::new(),
            last_protocol: None,
        })
    }

    /// The bound local address.
    pub fn local_addr(&self) -> Result<SocketAddr, ControlError> {
        Ok(self.socket.local_addr()?)
    }

    /// Read-only view of the managed universes.
    pub fn manager(&self) -> &UniverseManager {
        &self.manager
    }

    /// The protocol of the most recently received datagram.
    pub fn last_protocol(&self) -> Option<DmxProtocol> {
        self.last_protocol
    }

    /// Receives one datagram (blocking), auto-detects Art-Net vs sACN,
    /// merges the universe, and returns it with its source address.
    pub fn recv_universe(&mut self) -> Result<(DmxUniverse, SocketAddr), ControlError> {
        let mut buf = vec![0u8; 2048];
        loop {
            let (len, src) = self.socket.recv_from(&mut buf)?;
            let data = &buf[..len];
            let parsed = if data.starts_with(b"Art-Net\0") {
                match artnet::parse_packet(data)? {
                    artnet::ArtNetPacket::Dmx { universe, slots } => {
                        let mut u = DmxUniverse::new(universe);
                        u.set_channels(0, &slots);
                        Some((DmxProtocol::ArtNet, u))
                    }
                    _ => None,
                }
            } else if data.len() > 16 && data[4..16] == sacn::ACN_IDENTIFIER[..] {
                match sacn::parse_packet(data)? {
                    Some(packet) => {
                        let mut u = DmxUniverse::new(packet.universe);
                        u.set_channels(0, &packet.slots);
                        Some((DmxProtocol::Sacn, u))
                    }
                    None => None,
                }
            } else {
                None // unrelated UDP chatter
            };
            let (protocol, universe) = match parsed {
                Some(found) => found,
                None => continue,
            };
            self.manager.merge(&universe);
            self.last_protocol = Some(protocol);
            return Ok((universe, src));
        }
    }

    /// Runs the receive loop forever, passing each universe to `handler`.
    pub fn run(
        &mut self,
        mut handler: impl FnMut(DmxUniverse, SocketAddr),
    ) -> Result<(), ControlError> {
        loop {
            let (universe, src) = self.recv_universe()?;
            handler(universe, src);
        }
    }
}
