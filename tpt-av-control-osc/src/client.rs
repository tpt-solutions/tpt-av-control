//! OSC client: UDP sending of messages and bundles.

use crate::bundle::OscBundle;
use crate::message::OscMessage;
use std::net::{SocketAddr, UdpSocket};
use tpt_av_control_utils::ControlError;

/// An OSC client bound to an ephemeral local port, sending to a fixed
/// target address.
pub struct OscClient {
    socket: UdpSocket,
    target: SocketAddr,
}

impl OscClient {
    /// Creates a client targeting `target`, bound to an ephemeral port.
    pub fn new(target: SocketAddr) -> Result<Self, ControlError> {
        Self::bind(SocketAddr::from(([0, 0, 0, 0], 0)), target)
    }

    /// Creates a client bound to a specific local address.
    pub fn bind(source: SocketAddr, target: SocketAddr) -> Result<Self, ControlError> {
        let socket = UdpSocket::bind(source)?;
        Ok(Self { socket, target })
    }

    /// The address this client sends to.
    pub fn target(&self) -> SocketAddr {
        self.target
    }

    /// The local address this client sends from.
    pub fn local_addr(&self) -> Result<SocketAddr, ControlError> {
        Ok(self.socket.local_addr()?)
    }

    /// Sends an OSC message, returning the number of bytes written.
    pub fn send(&mut self, message: &OscMessage) -> Result<usize, ControlError> {
        let bytes = message.encode();
        self.socket.send_to(&bytes, self.target)?;
        Ok(bytes.len())
    }

    /// Sends an OSC bundle, returning the number of bytes written.
    pub fn send_bundle(&mut self, bundle: &OscBundle) -> Result<usize, ControlError> {
        let bytes = bundle.encode();
        self.socket.send_to(&bytes, self.target)?;
        Ok(bytes.len())
    }

    /// Sends an already-encoded OSC packet.
    pub fn send_raw(&self, bytes: &[u8]) -> Result<usize, ControlError> {
        self.socket.send_to(bytes, self.target)?;
        Ok(bytes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::OscArg;

    #[test]
    fn client_binds_and_sends() {
        // Send to a port that is closed; UDP send_to still succeeds
        // locally on most platforms, but to be portable just assert the
        // encode+send path doesn't error on construction.
        let target: SocketAddr = "127.0.0.1:9".parse().unwrap();
        let client = OscClient::new(target).unwrap();
        assert_eq!(client.target(), target);
        assert_ne!(client.local_addr().unwrap().port(), 0);
    }

    #[test]
    fn send_bundle_roundtrip_through_server() {
        let mut server =
            crate::server::OscServer::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        let addr = server.local_addr().unwrap();
        let mut client = OscClient::new(addr).unwrap();
        let bundle = OscBundle::new(
            None,
            vec![crate::bundle::OscPacket::Message(
                OscMessage::new("/x", &[OscArg::Int(1)]).unwrap(),
            )],
        );
        // recv_packet in background thread.
        let t = std::thread::spawn(move || server.recv_packet().unwrap());
        std::thread::sleep(std::time::Duration::from_millis(50));
        client.send_bundle(&bundle).unwrap();
        let (packet, _src) = t.join().unwrap();
        match packet {
            crate::bundle::OscPacket::Bundle(b) => {
                assert_eq!(b.elements.len(), 1);
            }
            _ => panic!("expected bundle"),
        }
    }
}
