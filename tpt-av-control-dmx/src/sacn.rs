//! sACN / ANSI E1.31 protocol (UDP port 5568): streaming DMX512 over ACN.

use crate::dmx::{DmxUniverse, DMX_CHANNELS};
use std::net::{SocketAddr, UdpSocket};
use tpt_av_control_utils::ControlError;

/// The default sACN UDP port.
pub const SACN_PORT: u16 = 5568;

/// Root layer vector for E1.31 data packets.
pub const VECTOR_ROOT_E131_DATA: u32 = 0x0000_0004;
/// Framing layer vector for data packets.
pub const VECTOR_E131_DATA_PACKET: u32 = 0x0000_0002;
/// DMP vector for set-property packets.
pub const VECTOR_DMP_SET_PROPERTY: u8 = 0x02;

/// The 12-byte ACN packet identifier: "ASC-E1.17\0\0\0".
pub const ACN_IDENTIFIER: &[u8; 12] = b"ASC-E1.17\0\0\0";

/// Default sACN priority (0-200; 100 per the E1.31 recommendation).
pub const DEFAULT_PRIORITY: u8 = 100;

fn flglen(flags: u8, length: u16) -> [u8; 2] {
    [
        (flags << 4) | ((length >> 8) & 0x0F) as u8,
        (length & 0xFF) as u8,
    ]
}

/// A 16-byte Component Identifier (CID) for sACN sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cid(pub [u8; 16]);

impl Cid {
    /// A deterministic CID derived from an arbitrary byte string (test and
    /// quick-start helper; real deployments should use a UUID).
    pub fn from_bytes(seed: &[u8]) -> Self {
        let mut cid = [0u8; 16];
        for (i, &b) in seed.iter().cycle().take(16).enumerate() {
            cid[i] = b ^ (((i as u32) * 0x2B) & 0xFF) as u8;
        }
        Cid(cid)
    }
}

/// Builds an E1.31 data packet for one universe (start code 0x00).
pub fn build_data_packet(
    cid: &Cid,
    source_name: &str,
    universe: u16,
    priority: u8,
    sequence: u8,
    slots: &[u8; DMX_CHANNELS],
) -> Vec<u8> {
    let total_len: u16 = 638; // 16 root hdr + 77 framing + 10 dmp + 513 slots
    let framing_len: u16 = total_len - 16; // framing + DMP + slots
    let dmp_len: u16 = 10 + (1 + DMX_CHANNELS as u16); // start code + 512 slots

    let mut p = Vec::with_capacity(usize::from(total_len));
    // Root layer.
    p.extend_from_slice(&0x0010u16.to_be_bytes()); // preamble size
    p.extend_from_slice(&0x0000u16.to_be_bytes()); // post-amble
    p.extend_from_slice(ACN_IDENTIFIER);
    p.extend_from_slice(&flglen(0x7, total_len));
    p.extend_from_slice(&VECTOR_ROOT_E131_DATA.to_be_bytes());
    p.extend_from_slice(&cid.0);
    // Framing layer.
    p.extend_from_slice(&flglen(0x7, framing_len));
    p.extend_from_slice(&VECTOR_E131_DATA_PACKET.to_be_bytes());
    let mut name = [0u8; 64];
    let name_bytes = source_name.as_bytes();
    name[..name_bytes.len().min(64)].copy_from_slice(&name_bytes[..name_bytes.len().min(64)]);
    p.extend_from_slice(&name);
    p.push(priority);
    p.extend_from_slice(&[0x00, 0x00]); // sync address
    p.push(sequence);
    p.push(0x00); // options
    p.extend_from_slice(&universe.to_be_bytes());
    // DMP layer.
    p.extend_from_slice(&flglen(0x7, dmp_len));
    p.push(VECTOR_DMP_SET_PROPERTY);
    p.push(0xA1); // address & data type
    p.extend_from_slice(&[0x00, 0x00]); // first property address
    p.extend_from_slice(&[0x00, 0x01]); // address increment
    let count = 1 + DMX_CHANNELS as u16; // start code + 512 slots
    p.extend_from_slice(&count.to_be_bytes());
    p.push(0x00); // DMX512-A start code
    p.extend_from_slice(slots);
    p
}

/// A parsed E1.31 data packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SacnDataPacket {
    /// Source name from the framing layer.
    pub source_name: String,
    /// Source priority (0-200).
    pub priority: u8,
    /// Per-universe sequence number.
    pub sequence: u8,
    /// The DMX universe number.
    pub universe: u16,
    /// Slot data (after the start code).
    pub slots: Vec<u8>,
}

/// Parses an E1.31 data packet; non-data packets return `Ok(None)`.
pub fn parse_packet(data: &[u8]) -> Result<Option<SacnDataPacket>, ControlError> {
    if data.len() < 38 || data[4..16] != ACN_IDENTIFIER[..] {
        return Err(ControlError::InvalidData(
            "not an sACN packet (missing ACN identifier)".into(),
        ));
    }
    let root_vector = u32::from_be_bytes([data[18], data[19], data[20], data[21]]);
    if root_vector != VECTOR_ROOT_E131_DATA {
        return Ok(None); // discovery/_sync packets are ignored
    }
    if data.len() < 126 {
        return Err(ControlError::InvalidData("sACN packet truncated".into()));
    }
    let framing_vector = u32::from_be_bytes([data[40], data[41], data[42], data[43]]);
    if framing_vector != VECTOR_E131_DATA_PACKET {
        return Ok(None);
    }
    let source_name = String::from_utf8_lossy(&data[44..108])
        .trim_end_matches('\0')
        .to_string();
    // Framing layer: priority at 108, sync 109..111, sequence 111, options
    // 112, universe 113..115. DMP layer: flglen 115..117, vector 117, type
    // 118, first 119..121, increment 121..123, count 123..125, start code
    // 125, slots from 126.
    let priority = data[108];
    let sequence = data[111];
    let universe = u16::from_be_bytes([data[113], data[114]]);
    let count = u16::from_be_bytes([data[123], data[124]]) as usize;
    let end = (125 + count).min(data.len());
    if end < 126 {
        return Err(ControlError::InvalidData("sACN slot data missing".into()));
    }
    Ok(Some(SacnDataPacket {
        source_name,
        priority,
        sequence,
        universe,
        slots: data[126..end].to_vec(),
    }))
}

/// An sACN server: sends and receives DMX universes over UDP.
pub struct Sacn {
    socket: UdpSocket,
    cid: Cid,
    source_name: String,
    priority: u8,
    sequence: u8,
}

impl Sacn {
    /// Creates a server bound to `0.0.0.0:port` (use [`SACN_PORT`] for a
    /// standard node).
    pub fn new(port: u16) -> Result<Self, ControlError> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        Ok(Self {
            socket,
            cid: Cid::from_bytes(b"tpt-av-control-sacn"),
            source_name: "tpt-av-control".to_string(),
            priority: DEFAULT_PRIORITY,
            sequence: 0,
        })
    }

    /// Overrides the source CID and name.
    pub fn set_source(&mut self, cid: Cid, source_name: impl Into<String>) {
        self.cid = cid;
        self.source_name = source_name.into();
    }

    /// The bound local address.
    pub fn local_addr(&self) -> Result<SocketAddr, ControlError> {
        Ok(self.socket.local_addr()?)
    }

    /// Sends a DMX universe as an E1.31 data packet to `target`.
    pub fn send_universe_to(
        &mut self,
        target: SocketAddr,
        universe: &DmxUniverse,
    ) -> Result<(), ControlError> {
        self.sequence = self.sequence.wrapping_add(1);
        let packet = build_data_packet(
            &self.cid,
            &self.source_name,
            universe.universe,
            self.priority,
            self.sequence,
            &universe.channels,
        );
        self.socket.send_to(&packet, target)?;
        Ok(())
    }

    /// Receives the next E1.31 data packet as a universe (blocking).
    /// Non-data packets (discovery, sync) are skipped.
    pub fn recv_universe(&mut self) -> Result<DmxUniverse, ControlError> {
        let mut buf = vec![0u8; 2048];
        loop {
            let (len, _src) = self.socket.recv_from(&mut buf)?;
            if let Some(packet) = parse_packet(&buf[..len])? {
                let mut u = DmxUniverse::new(packet.universe);
                u.set_channels(0, &packet.slots);
                return Ok(u);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_packet_layout() {
        let mut slots = [0u8; DMX_CHANNELS];
        slots[0] = 0xAB;
        let cid = Cid::from_bytes(b"test");
        let packet = build_data_packet(&cid, "src", 1, 100, 42, &slots);
        assert_eq!(packet.len(), 638);
        assert_eq!(&packet[0..2], &0x0010u16.to_be_bytes());
        assert_eq!(&packet[4..16], &ACN_IDENTIFIER[..]);
        assert_eq!(&packet[16..18], &[0x72, 0x7E]); // flglen 0x7<<12 | 638
        assert_eq!(&packet[18..22], &VECTOR_ROOT_E131_DATA.to_be_bytes());
        assert_eq!(&packet[22..38], &cid.0);
        assert_eq!(&packet[40..44], &VECTOR_E131_DATA_PACKET.to_be_bytes());
        assert_eq!(packet[108], 100); // priority
        assert_eq!(packet[111], 42); // sequence
        assert_eq!(&packet[113..115], &1u16.to_be_bytes());
        assert_eq!(packet[125], 0x00); // start code
        assert_eq!(packet[126], 0xAB);
    }

    #[test]
    fn parse_roundtrip() {
        let mut slots = [0u8; DMX_CHANNELS];
        slots[100] = 9;
        let cid = Cid::from_bytes(b"roundtrip");
        let packet = build_data_packet(&cid, "my console", 42, 90, 7, &slots);
        let parsed = parse_packet(&packet).unwrap().unwrap();
        assert_eq!(parsed.source_name, "my console");
        assert_eq!(parsed.priority, 90);
        assert_eq!(parsed.sequence, 7);
        assert_eq!(parsed.universe, 42);
        assert_eq!(parsed.slots.len(), 512);
        assert_eq!(parsed.slots[100], 9);

        // Not an sACN packet.
        assert!(parse_packet(b"short").is_err());
        // Truncated after the header.
        assert!(parse_packet(&packet[..50]).is_err());
    }

    #[test]
    fn cid_is_deterministic() {
        let a = Cid::from_bytes(b"seed");
        let b = Cid::from_bytes(b"seed");
        let c = Cid::from_bytes(b"other");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
