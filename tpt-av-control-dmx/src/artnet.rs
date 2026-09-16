//! Art-Net 4 protocol (UDP port 6454): ArtDmx data packets plus a minimal
//! ArtPoll / ArtPollReply implementation.

use crate::dmx::{DmxUniverse, DMX_CHANNELS};
use std::net::{SocketAddr, UdpSocket};
use tpt_av_control_utils::ControlError;

/// The default Art-Net UDP port.
pub const ARTNET_PORT: u16 = 6454;

/// ArtDmx opcode (OpOutput / OpDmx).
pub const OP_DMX: u16 = 0x5000;
/// ArtPoll opcode.
pub const OP_POLL: u16 = 0x2000;
/// ArtPollReply opcode.
pub const OP_POLL_REPLY: u16 = 0x2100;
/// Art-Net protocol version 14.
pub const PROTOCOL_VERSION: u8 = 14;

/// An incoming Art-Net packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtNetPacket {
    /// An ArtDmx data packet.
    Dmx {
        /// The DMX universe number.
        universe: u16,
        /// The slot data (1-512 entries as sent).
        slots: Vec<u8>,
    },
    /// An ArtPoll requesting node discovery.
    Poll,
    /// Anything else (opcode preserved).
    Other {
        /// The Art-Net opcode.
        opcode: u16,
    },
}

/// Builds an ArtDmx packet: header + 512-slot data.
pub fn build_artdmx(universe: u16, sequence: u8, data: &[u8; DMX_CHANNELS]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(18 + DMX_CHANNELS);
    packet.extend_from_slice(b"Art-Net\0");
    packet.push((OP_DMX & 0xFF) as u8); // opcode low byte first
    packet.push((OP_DMX >> 8) as u8);
    packet.push(0x00); // ProtVer hi
    packet.push(PROTOCOL_VERSION);
    packet.push(sequence);
    packet.push(0x00); // physical
    packet.push((universe & 0xFF) as u8); // SubUni
    packet.push(((universe >> 8) & 0x7F) as u8); // Net
    let len = DMX_CHANNELS as u16;
    packet.push((len >> 8) as u8); // length big-endian
    packet.push((len & 0xFF) as u8);
    packet.extend_from_slice(data);
    packet
}

/// Builds a minimal ArtPollReply announcing one input port on `universe`.
pub fn build_art_poll_reply(source_ip: [u8; 4], short_name: &str, universe: u16) -> Vec<u8> {
    let mut r = Vec::with_capacity(239);
    r.extend_from_slice(b"Art-Net\0");
    r.push((OP_POLL_REPLY & 0xFF) as u8);
    r.push((OP_POLL_REPLY >> 8) as u8);
    r.extend_from_slice(&source_ip);
    r.extend_from_slice(&ARTNET_PORT.to_be_bytes());
    r.push(1); // VersInfo
    r.push(0); // VersInfo (sub)
    r.push((universe & 0xFF) as u8); // NetSwitch
    r.push(((universe >> 8) & 0x7F) as u8); // SubSwitch
    r.extend_from_slice(&[0xFF, 0xFF]); // OEM
    r.push(0x00); // Ubea
    r.push(0x00); // Status1
    r.extend_from_slice(&[0x00, 0x00]); // ESTA manufacturer
    push_fixed(&mut r, short_name, 18);
    push_fixed(&mut r, short_name, 64);
    push_fixed(&mut r, "#0000 [0000] tpt-av-control ready", 64);
    r.push(0x00); // NumPortsHi
    r.push(0x01); // NumPorts: 1
    r.push(0x80); // PortTypes: port 1 output DMX
    r.extend_from_slice(&[0x00, 0x00, 0x00]); // remaining PortTypes
    r.extend_from_slice(&[0x80, 0x00, 0x00, 0x00]); // GoodInput
    r.extend_from_slice(&[0x80, 0x00, 0x00, 0x00]); // GoodOutput
    let sub = (universe & 0x0F) as u8;
    r.push(sub); // SwIn
    r.push(sub); // SwOut
    r.extend_from_slice(&[0x00; 4]); // SwIn video
    r.extend_from_slice(&[0x00; 4]); // SwOut video
    r.push(0x00); // AcnPriority
    r.extend(std::iter::repeat(0x80).take(23));
    r
}

/// Parses an Art-Net packet.
pub fn parse_packet(data: &[u8]) -> Result<ArtNetPacket, ControlError> {
    if data.len() < 12 || &data[0..8] != b"Art-Net\0" {
        return Err(ControlError::InvalidData(
            "not an Art-Net packet (missing 'Art-Net\\0' header)".into(),
        ));
    }
    let opcode = u16::from(data[8]) | u16::from(data[9]) << 8;
    match opcode {
        OP_DMX => {
            if data.len() < 18 {
                return Err(ControlError::InvalidData("ArtDmx packet truncated".into()));
            }
            let len = (u16::from(data[16]) << 8) | u16::from(data[17]);
            let end = 18usize + usize::from(len);
            if end > data.len() {
                return Err(ControlError::InvalidData(
                    "ArtDmx length overruns packet".into(),
                ));
            }
            let universe = u16::from(data[14]) | u16::from(data[15] & 0x7F) << 8;
            Ok(ArtNetPacket::Dmx {
                universe,
                slots: data[18..end].to_vec(),
            })
        }
        OP_POLL => Ok(ArtNetPacket::Poll),
        other => Ok(ArtNetPacket::Other { opcode: other }),
    }
}

fn push_fixed(buf: &mut Vec<u8>, text: &str, width: usize) {
    let bytes = text.as_bytes();
    let n = bytes.len().min(width);
    buf.extend_from_slice(&bytes[..n]);
    buf.extend(std::iter::repeat(0u8).take(width - n));
}

/// An Art-Net server: sends and receives DMX universes over UDP.
pub struct ArtNet {
    socket: UdpSocket,
    /// The address replies are sent to (set from the last received poll).
    last_poller: Option<SocketAddr>,
    sequence: u8,
}

impl ArtNet {
    /// Creates a server bound to `0.0.0.0:port` (use [`ARTNET_PORT`] for a
    /// standard node).
    pub fn new(port: u16) -> Result<Self, ControlError> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        Ok(Self {
            socket,
            last_poller: None,
            sequence: 0,
        })
    }

    /// The bound local address.
    pub fn local_addr(&self) -> Result<SocketAddr, ControlError> {
        Ok(self.socket.local_addr()?)
    }

    /// Sends a DMX universe as an ArtDmx packet to `target`.
    pub fn send_universe_to(
        &mut self,
        target: SocketAddr,
        universe: &DmxUniverse,
    ) -> Result<(), ControlError> {
        self.sequence = self.sequence.wrapping_add(1);
        let packet = build_artdmx(universe.universe, self.sequence, &universe.channels);
        self.socket.send_to(&packet, target)?;
        Ok(())
    }

    /// Receives the next Art-Net packet, blocking. ArtDmx packets are
    /// returned as universes; a poll is answered with an ArtPollReply.
    pub fn recv_universe(&mut self) -> Result<DmxUniverse, ControlError> {
        let mut buf = vec![0u8; 2048];
        loop {
            let (len, src) = self.socket.recv_from(&mut buf)?;
            match parse_packet(&buf[..len])? {
                ArtNetPacket::Dmx { universe, slots } => {
                    let mut u = DmxUniverse::new(universe);
                    u.set_channels(0, &slots);
                    return Ok(u);
                }
                ArtNetPacket::Poll => {
                    self.last_poller = Some(src);
                    let ip = match src {
                        SocketAddr::V4(v4) => v4.ip().octets(),
                        SocketAddr::V6(_) => [127, 0, 0, 1],
                    };
                    let reply = build_art_poll_reply(ip, "tpt-av-control", 0);
                    let _ = self.socket.send_to(&reply, src);
                }
                ArtNetPacket::Other { .. } => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artdmx_packet_layout() {
        let mut data = [0u8; DMX_CHANNELS];
        data[0] = 0xFF;
        data[511] = 0x7F;
        let packet = build_artdmx(0, 1, &data);
        assert_eq!(&packet[0..8], b"Art-Net\0");
        assert_eq!(&packet[8..10], &[0x00, 0x50]); // opcode 0x5000 LE
        assert_eq!(&packet[10..12], &[0x00, 14]); // protocol 14
        assert_eq!(packet[12], 1); // sequence
        assert_eq!(packet[14], 0); // SubUni
        assert_eq!(packet[15], 0); // Net
        assert_eq!(&packet[16..18], &[0x02, 0x00]); // length 512 BE
        assert_eq!(packet.len(), 18 + 512);
        assert_eq!(packet[18], 0xFF);
        assert_eq!(packet[18 + 511], 0x7F);
    }

    #[test]
    fn artdmx_universe_encoding() {
        let packet = build_artdmx(0x0102, 0, &[0u8; DMX_CHANNELS]);
        assert_eq!(packet[14], 0x02); // SubUni (low byte)
        assert_eq!(packet[15], 0x01); // Net (high bits)
    }

    #[test]
    fn parse_roundtrip() {
        let packet = build_artdmx(3, 9, &[7u8; DMX_CHANNELS]);
        match parse_packet(&packet).unwrap() {
            ArtNetPacket::Dmx { universe, slots } => {
                assert_eq!(universe, 3);
                assert_eq!(slots.len(), 512);
                assert_eq!(slots[7], 7);
            }
            other => panic!("expected ArtDmx, got {other:?}"),
        }
        // Poll.
        let mut poll = vec![
            b'A', b'r', b't', b'-', b'N', b'e', b't', 0, 0x00, 0x20, 0x00, 0x0E, 0x00, 0x00,
        ];
        let _ = &mut poll;
        match parse_packet(&poll).unwrap() {
            ArtNetPacket::Poll => {}
            other => panic!("expected poll, got {other:?}"),
        }
        // Non-Art-Net data rejected.
        assert!(parse_packet(b"not artnet").is_err());
    }
}
