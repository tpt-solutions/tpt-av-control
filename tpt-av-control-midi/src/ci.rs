//! MIDI-CI (Capability Inquiry): discovery and capability negotiation
//! message layer (per MIDI-CI 1.2 / M2-115).
//!
//! MIDI-CI travels in Universal System Exclusive messages:
//! `F0 7E <device> 0D <sub_id2> <version> [payload] F7` where sub-ID 1
//! `0x0D` selects MIDI-CI.

use crate::midi1::Midi1Message;
use crate::sysex::{self, UNIVERSAL_NON_REALTIME};
use tpt_av_control_utils::ControlError;

/// Universal System Exclusive sub-ID 1 for MIDI-CI.
pub const MIDI_CI_SUB_ID1: u8 = 0x0D;

/// MIDI-CI message types (sub-ID 2).
pub mod message_type {
    /// Discovery (sub-ID 2 = 0x70).
    pub const DISCOVERY: u8 = 0x70;
    /// Discovery Reply.
    pub const DISCOVERY_REPLY: u8 = 0x71;
    /// Invalid MIDICI Message / NAK.
    pub const INVALID_MESSAGE_NAK: u8 = 0x7E;
    /// Acknowledgement / End of Data Set.
    pub const ACK: u8 = 0x7F;
    /// Endpoint Inquiry.
    pub const ENDPOINT_INQUIRY: u8 = 0x60;
    /// Endpoint Info Notification.
    pub const ENDPOINT_INFO: u8 = 0x61;
}

/// The protocol version implemented here: MIDI-CI 1.2.
pub const CI_VERSION: u8 = 0x01;

#[derive(Debug, Clone, PartialEq, Eq)]
/// A parsed MIDI-CI message.
pub enum CiMessage {
    /// Discovery (0x70): asks peers to identify themselves.
    Discovery {
        /// Target device ID (0x7F = broadcast).
        device_id: u8,
        /// CI version supported by the initiator.
        version: u8,
        /// MUID of the initiator.
        source_muid: u32,
        /// Supported profiles plus Capability bytes.
        profiles: Vec<u8>,
        /// Protocol type the initiator prefers (0x00 = MIDI 2.0).
        protocol_type: u8,
        /// Extensions bitmap (protocol negotiation data).
        extensions: u8,
    },
    /// Discovery Reply (0x71).
    DiscoveryReply {
        /// Target device ID.
        device_id: u8,
        /// CI version supported by the replier.
        version: u8,
        /// MUID of the replier.
        source_muid: u32,
        /// MUID the reply is addressed to.
        destination_muid: u32,
        /// Supported profiles plus Capability bytes.
        profiles: Vec<u8>,
        /// Protocol type the replier prefers.
        protocol_type: u8,
        /// Extensions bitmap.
        extensions: u8,
    },
    /// Endpoint Inquiry (0x60).
    EndpointInquiry {
        /// Target device ID.
        device_id: u8,
        /// CI version.
        version: u8,
        /// MUID of the initiator.
        source_muid: u32,
        /// Bitmap of requested info (UMP version, protocol, function blocks).
        filter_bitmap: u8,
    },
    /// Endpoint Info Notification (0x61).
    EndpointInfo {
        /// Target device ID.
        device_id: u8,
        /// CI version.
        version: u8,
        /// MUID of the reporter.
        source_muid: u32,
        /// UMP protocol major version.
        protocol_major: u8,
        /// UMP protocol minor version.
        protocol_minor: u8,
        /// Number of UMP function blocks (0-31; 0x7F = dynamic).
        function_blocks: u8,
        /// Whether the endpoint supports the MIDI 2.0 protocol.
        midi2_protocol: bool,
    },
    /// NAK (0x7E): the request is not understood or declined.
    Nak {
        /// Target device ID.
        device_id: u8,
        /// CI version.
        version: u8,
        /// MUID of the sender.
        source_muid: u32,
        /// MUID the NAK is addressed to.
        destination_muid: u32,
    },
    /// ACK (0x7F).
    Ack {
        /// Target device ID.
        device_id: u8,
        /// CI version.
        version: u8,
        /// MUID of the sender.
        source_muid: u32,
        /// MUID the ACK is addressed to.
        destination_muid: u32,
    },
}

impl CiMessage {
    /// Encodes to a MIDI 1.0 SysEx message.
    pub fn to_midi1(&self) -> Result<Midi1Message, ControlError> {
        let mut body: Vec<u8> = Vec::new();
        let device_id;
        match self {
            CiMessage::Discovery {
                device_id: dev,
                version,
                source_muid,
                profiles,
                protocol_type,
                extensions,
            } => {
                device_id = *dev;
                body.push(*version & 0x7F);
                push_muid(&mut body, *source_muid);
                body.extend(profiles.iter().map(|b| b & 0x7F));
                body.push(protocol_type & 0x7F);
                body.push(extensions & 0x7F);
            }
            CiMessage::DiscoveryReply {
                device_id: dev,
                version,
                source_muid,
                destination_muid,
                profiles,
                protocol_type,
                extensions,
            } => {
                device_id = *dev;
                body.push(*version & 0x7F);
                push_muid(&mut body, *source_muid);
                push_muid(&mut body, *destination_muid);
                body.extend(profiles.iter().map(|b| b & 0x7F));
                body.push(*protocol_type & 0x7F);
                body.push(*extensions & 0x7F);
            }
            CiMessage::Nak {
                device_id: dev,
                version,
                source_muid,
                destination_muid,
            }
            | CiMessage::Ack {
                device_id: dev,
                version,
                source_muid,
                destination_muid,
            } => {
                device_id = *dev;
                body.push(*version & 0x7F);
                push_muid(&mut body, *source_muid);
                push_muid(&mut body, *destination_muid);
            }
            CiMessage::EndpointInquiry {
                device_id: dev,
                version,
                source_muid,
                filter_bitmap,
            } => {
                device_id = *dev;
                body.push(*version & 0x7F);
                push_muid(&mut body, *source_muid);
                body.push(filter_bitmap & 0x7F);
            }
            CiMessage::EndpointInfo {
                device_id: dev,
                version,
                source_muid,
                protocol_major,
                protocol_minor,
                function_blocks,
                midi2_protocol,
            } => {
                device_id = *dev;
                body.push(*version & 0x7F);
                push_muid(&mut body, *source_muid);
                body.push(protocol_major & 0x7F);
                body.push(protocol_minor & 0x7F);
                let fb = (*function_blocks).min(0x7F);
                body.push(if *midi2_protocol {
                    (fb & 0x3F) | 0x40
                } else {
                    fb & 0x7F
                });
            }
        }
        let sub_id2 = match self {
            CiMessage::Discovery { .. } => message_type::DISCOVERY,
            CiMessage::DiscoveryReply { .. } => message_type::DISCOVERY_REPLY,
            CiMessage::EndpointInquiry { .. } => message_type::ENDPOINT_INQUIRY,
            CiMessage::EndpointInfo { .. } => message_type::ENDPOINT_INFO,
            CiMessage::Nak { .. } => message_type::INVALID_MESSAGE_NAK,
            CiMessage::Ack { .. } => message_type::ACK,
        };
        Ok(sysex::sysex_universal(
            UNIVERSAL_NON_REALTIME,
            device_id,
            MIDI_CI_SUB_ID1,
            sub_id2,
            &body,
        ))
    }

    /// Parses a MIDI-CI message from a MIDI 1.0 SysEx.
    pub fn from_midi1(message: &Midi1Message) -> Result<Self, ControlError> {
        let view = sysex::parse_sysex(message)?;
        if view.manufacturer != UNIVERSAL_NON_REALTIME || view.sub_id1 != Some(MIDI_CI_SUB_ID1) {
            return Err(ControlError::InvalidData("not a MIDI-CI message".into()));
        }
        let device_id = view.device_id.unwrap_or(0x7F);
        let payload = view.payload;
        let sub_id2 = view.sub_id2.unwrap();
        if payload.is_empty() {
            return Err(ControlError::InvalidData("MIDI-CI body too short".into()));
        }
        let version = payload[0] & 0x7F;
        let mut cursor = &payload[1..];
        let take_muid = |cursor: &mut &[u8]| -> u32 {
            if cursor.len() < 4 {
                *cursor = &[];
                return 0;
            }
            let muid = u32::from(cursor[0])
                | u32::from(cursor[1]) << 7
                | u32::from(cursor[2]) << 14
                | u32::from(cursor[3]) << 21;
            *cursor = &cursor[4..];
            muid
        };
        let out = match sub_id2 {
            message_type::DISCOVERY => {
                let source_muid = take_muid(&mut cursor);
                let profiles = cursor
                    .get(..cursor.len().saturating_sub(2))
                    .unwrap_or(&[])
                    .to_vec();
                let protocol_type = cursor.last().copied().unwrap_or(0);
                let extensions = 0;
                CiMessage::Discovery {
                    device_id,
                    version,
                    source_muid,
                    profiles,
                    protocol_type,
                    extensions,
                }
            }
            message_type::DISCOVERY_REPLY => {
                let source_muid = take_muid(&mut cursor);
                let destination_muid = take_muid(&mut cursor);
                let profiles = cursor
                    .get(..cursor.len().saturating_sub(2))
                    .unwrap_or(&[])
                    .to_vec();
                let protocol_type = cursor.last().copied().unwrap_or(0);
                let extensions = 0;
                CiMessage::DiscoveryReply {
                    device_id,
                    version,
                    source_muid,
                    destination_muid,
                    profiles,
                    protocol_type,
                    extensions,
                }
            }
            message_type::ENDPOINT_INQUIRY => {
                let source_muid = take_muid(&mut cursor);
                let filter_bitmap = cursor.first().copied().unwrap_or(0);
                CiMessage::EndpointInquiry {
                    device_id,
                    version,
                    source_muid,
                    filter_bitmap,
                }
            }
            message_type::ENDPOINT_INFO => {
                let source_muid = take_muid(&mut cursor);
                let protocol_major = cursor.first().copied().unwrap_or(0);
                let protocol_minor = cursor.get(1).copied().unwrap_or(0);
                let fb_byte = cursor.get(2).copied().unwrap_or(0);
                CiMessage::EndpointInfo {
                    device_id,
                    version,
                    source_muid,
                    protocol_major,
                    protocol_minor,
                    function_blocks: fb_byte & 0x3F,
                    midi2_protocol: fb_byte & 0x40 != 0,
                }
            }
            message_type::INVALID_MESSAGE_NAK => {
                let source_muid = take_muid(&mut cursor);
                let destination_muid = take_muid(&mut cursor);
                CiMessage::Nak {
                    device_id,
                    version,
                    source_muid,
                    destination_muid,
                }
            }
            message_type::ACK => {
                let source_muid = take_muid(&mut cursor);
                let destination_muid = take_muid(&mut cursor);
                CiMessage::Ack {
                    device_id,
                    version,
                    source_muid,
                    destination_muid,
                }
            }
            other => {
                return Err(ControlError::Unsupported(format!(
                    "MIDI-CI sub-id2 {other:#04x} (property exchange not implemented)"
                )))
            }
        };
        Ok(out)
    }
}

/// A helper state machine that runs the discovery handshake: send
/// Discovery, collect peers from Discovery Replies, then interrogate
#[derive(Debug, Default)]
pub struct CiDiscoverySession {
    /// Peers that answered our discovery, with their reported MUIDs.
    pub peers: Vec<u32>,
    /// Our own MUID (assigned once per session).
    pub local_muid: u32,
}

impl CiDiscoverySession {
    /// Starts a session with a locally-chosen MUID (7- or 28-bit value,
    pub fn new(local_muid: u32) -> Self {
        Self {
            local_muid: local_muid & 0x0FFF_FFFF,
            peers: Vec::new(),
        }
    }

    /// The Discovery message to broadcast (device 0x7F = all).
    pub fn discovery_request(&self) -> Result<Midi1Message, ControlError> {
        CiMessage::Discovery {
            device_id: 0x7F,
            version: CI_VERSION,
            source_muid: self.local_muid,
            profiles: Vec::new(),
            protocol_type: 0x00, // prefer MIDI 2.0
            extensions: 0x7F,
        }
        .to_midi1()
    }

    /// Feeds an inbound message; returns `true` when it was a Discovery
    pub fn observe(&mut self, message: &Midi1Message) -> Result<bool, ControlError> {
        if let Ok(CiMessage::DiscoveryReply { source_muid, .. }) = CiMessage::from_midi1(message) {
            if !self.peers.contains(&source_muid) {
                self.peers.push(source_muid);
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Builds an Endpoint Info request aimed at a specific peer.
    pub fn endpoint_inquiry(&self, peer_muid: u32) -> Result<Midi1Message, ControlError> {
        CiMessage::EndpointInquiry {
            device_id: 0x7F,
            version: CI_VERSION,
            source_muid: self.local_muid,
            filter_bitmap: 0x7F,
        }
        .to_midi1()
        .map(|mut m| {
            // Address the peer by swapping in its device id slot.
            if let Midi1Message::SystemExclusive(bytes) = &mut m {
                bytes[2] = peer_muid as u8 & 0x7F;
            }
            m
        })
    }
}

fn push_muid(buf: &mut Vec<u8>, muid: u32) {
    let muid = muid & 0x0FFF_FFFF;
    buf.push((muid & 0x7F) as u8);
    buf.push(((muid >> 7) & 0x7F) as u8);
    buf.push(((muid >> 14) & 0x7F) as u8);
    buf.push(((muid >> 21) & 0x7F) as u8);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi1::parse_midi1;

    #[test]
    fn discovery_roundtrip() {
        let message = CiMessage::Discovery {
            device_id: 0x7F,
            version: CI_VERSION,
            source_muid: 0x0F1E_2D3C,
            profiles: vec![0x01, 0x02],
            protocol_type: 0x01,
            extensions: 0x7F,
        }
        .to_midi1()
        .unwrap();
        let parsed = parse_midi1(&message.to_bytes()).unwrap();
        match CiMessage::from_midi1(&parsed).unwrap() {
            CiMessage::Discovery {
                version,
                source_muid,
                profiles,
                ..
            } => {
                assert_eq!(version, CI_VERSION);
                assert_eq!(source_muid, 0x0F1E_2D3C);
                assert_eq!(profiles, vec![0x01, 0x02]);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn discovery_reply_roundtrip() {
        let message = CiMessage::DiscoveryReply {
            device_id: 0x7F,
            version: CI_VERSION,
            source_muid: 0x1234,
            destination_muid: 0x5678,
            profiles: vec![],
            protocol_type: 0x00,
            extensions: 0x7F,
        }
        .to_midi1()
        .unwrap();
        let parsed = parse_midi1(&message.to_bytes()).unwrap();
        match CiMessage::from_midi1(&parsed).unwrap() {
            CiMessage::DiscoveryReply {
                source_muid,
                destination_muid,
                ..
            } => {
                assert_eq!(source_muid, 0x1234);
                assert_eq!(destination_muid, 0x5678);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn endpoint_info_roundtrip() {
        let message = CiMessage::EndpointInfo {
            device_id: 0x7F,
            version: CI_VERSION,
            source_muid: 42,
            protocol_major: 1,
            protocol_minor: 1,
            function_blocks: 3,
            midi2_protocol: true,
        }
        .to_midi1()
        .unwrap();
        let parsed = parse_midi1(&message.to_bytes()).unwrap();
        match CiMessage::from_midi1(&parsed).unwrap() {
            CiMessage::EndpointInfo {
                protocol_major,
                protocol_minor,
                function_blocks,
                midi2_protocol,
                ..
            } => {
                assert_eq!((protocol_major, protocol_minor), (1, 1));
                assert_eq!(function_blocks, 3);
                assert!(midi2_protocol);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn discovery_session_collects_peers() {
        let mut session = CiDiscoverySession::new(0x00AB_CDEF);
        let request = session.discovery_request().unwrap();
        // The request parses back as a Discovery aimed at 0x7F.
        assert!(matches!(
            CiMessage::from_midi1(&parse_midi1(&request.to_bytes()).unwrap()).unwrap(),
            CiMessage::Discovery { .. }
        ));

        let reply = CiMessage::DiscoveryReply {
            device_id: 0x7F,
            version: CI_VERSION,
            source_muid: 0x777,
            destination_muid: session.local_muid,
            profiles: vec![],
            protocol_type: 0,
            extensions: 0,
        }
        .to_midi1()
        .unwrap();
        assert!(session.observe(&reply).unwrap());
        assert!(!session.observe(&reply).unwrap(), "duplicate peer ignored");
        assert_eq!(session.peers, vec![0x777]);
    }

    #[test]
    fn rejects_non_ci() {
        let other = sysex::gm_system_on(0x7F);
        assert!(CiMessage::from_midi1(&other).is_err());
    }
}
