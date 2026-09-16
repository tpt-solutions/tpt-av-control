//! DMX client: sends universes via Art-Net or sACN to a fixed target.

use crate::artnet;
use crate::dmx::DmxUniverse;
use crate::sacn::{self, Cid};
use crate::server::DmxProtocol;
use std::net::{SocketAddr, UdpSocket};
use tpt_av_control_utils::ControlError;

/// Sends DMX universes over UDP using the selected protocol.
pub struct DmxClient {
    socket: UdpSocket,
    target: SocketAddr,
    protocol: DmxProtocol,
    artnet_sequence: u8,
    sacn_cid: Cid,
    sacn_name: String,
    sacn_sequence: u8,
}

impl DmxClient {
    /// Creates a client targeting `target` over `protocol`, bound to an
    /// ephemeral local port.
    pub fn new(target: SocketAddr, protocol: DmxProtocol) -> Result<Self, ControlError> {
        let socket = UdpSocket::bind(("0.0.0.0", 0))?;
        Ok(Self {
            socket,
            target,
            protocol,
            artnet_sequence: 0,
            sacn_cid: Cid::from_bytes(b"tpt-av-control-dmx-client"),
            sacn_name: "tpt-av-control".to_string(),
            sacn_sequence: 0,
        })
    }

    /// Overrides the sACN source identity (ignored for Art-Net).
    pub fn set_sacn_source(&mut self, cid: Cid, name: impl Into<String>) {
        self.sacn_cid = cid;
        self.sacn_name = name.into();
    }

    /// The send target.
    pub fn target(&self) -> SocketAddr {
        self.target
    }

    /// The selected protocol.
    pub fn protocol(&self) -> DmxProtocol {
        self.protocol
    }

    /// Sends one universe.
    pub fn send_universe(&mut self, universe: &DmxUniverse) -> Result<(), ControlError> {
        match self.protocol {
            DmxProtocol::ArtNet => {
                self.artnet_sequence = self.artnet_sequence.wrapping_add(1);
                let packet = artnet::build_artdmx(
                    universe.universe,
                    self.artnet_sequence,
                    &universe.channels,
                );
                self.socket.send_to(&packet, self.target)?;
            }
            DmxProtocol::Sacn => {
                self.sacn_sequence = self.sacn_sequence.wrapping_add(1);
                let packet = sacn::build_data_packet(
                    &self.sacn_cid,
                    &self.sacn_name,
                    universe.universe,
                    sacn::DEFAULT_PRIORITY,
                    self.sacn_sequence,
                    &universe.channels,
                );
                self.socket.send_to(&packet, self.target)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::DmxServer;

    fn loopback() -> SocketAddr {
        "127.0.0.1:0".parse().unwrap()
    }

    #[test]
    fn sends_artnet_to_server() {
        let mut server = DmxServer::bind(loopback()).unwrap();
        let addr = server.local_addr().unwrap();
        let mut client = DmxClient::new(addr, DmxProtocol::ArtNet).unwrap();
        assert_eq!(client.protocol(), DmxProtocol::ArtNet);

        let t = std::thread::spawn(move || server.recv_universe().unwrap());
        std::thread::sleep(std::time::Duration::from_millis(50));

        let mut universe = DmxUniverse::new(1);
        universe.set_channel(0, 200);
        client.send_universe(&universe).unwrap();

        let (received, _) = t.join().unwrap();
        assert_eq!(received.universe, 1);
        assert_eq!(received.get_channel(0), 200);
    }

    #[test]
    fn sends_sacn_to_server() {
        let mut server = DmxServer::bind(loopback()).unwrap();
        let addr = server.local_addr().unwrap();
        let mut client = DmxClient::new(addr, DmxProtocol::Sacn).unwrap();

        let t = std::thread::spawn(move || server.recv_universe().unwrap());
        std::thread::sleep(std::time::Duration::from_millis(50));

        let mut universe = DmxUniverse::new(7);
        universe.set_channel(511, 30);
        client.send_universe(&universe).unwrap();

        let (received, _) = t.join().unwrap();
        assert_eq!(received.universe, 7);
        assert_eq!(received.get_channel(511), 30);
    }
}
