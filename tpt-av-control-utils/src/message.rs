//! Generic message envelope shared across protocol crates.
//!
//! Protocol crates parse their wire formats (OSC, MIDI, Art-Net, …) into
//! their own typed messages; [`Message`] is the uniform envelope used to
//! carry control intent across thread boundaries — e.g. through a
//! [`MessageQueue`] from the network thread to the real-time thread.

use crate::parameter::{ParameterId, ParameterValue};
use crate::ring::SpscRing;
use crate::time::{Timecode, Timestamp};
use std::net::SocketAddr;

/// Bounded lock-free queue for [`Message`]s (see [`SpscRing`]).
pub type MessageQueue = SpscRing<Message>;

/// Which wire protocol a message arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// Open Sound Control.
    Osc,
    /// MIDI 1.0 byte stream.
    Midi1,
    /// MIDI 2.0 Universal MIDI Packet.
    Midi2,
    /// Art-Net.
    ArtNet,
    /// sACN / E1.31.
    Sacn,
    /// WebRTC data channel.
    WebRtc,
}

/// Where a message came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MessageSource {
    /// Unknown or internal origin.
    Internal,
    /// A UDP/TCP peer.
    Network(SocketAddr),
    /// A MIDI device/port name.
    MidiPort(String),
}

/// Transport-level commands shared by the stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportCommand {
    /// Start playback.
    Play,
    /// Stop playback (return to zero per convention).
    Stop,
    /// Pause playback (hold position).
    Pause,
    /// Continue playback from the current position.
    Continue,
    /// A single timing-clock tick (24 per quarter note).
    ClockTick,
}

/// The payload of a [`Message`].
#[derive(Debug, Clone, PartialEq)]
pub enum MessageBody {
    /// Raw protocol bytes (already validated by the receiving crate).
    Raw {
        /// The protocol the bytes belong to.
        protocol: Protocol,
        /// The raw payload.
        data: Vec<u8>,
    },
    /// A mapped parameter change.
    Parameter {
        /// The parameter to change.
        id: ParameterId,
        /// Its new value.
        value: ParameterValue,
    },
    /// A transport command.
    Transport(TransportCommand),
    /// A transport locate/goto request.
    Locate(Timecode),
    /// Free-form text (e.g. a display update).
    Text(String),
}

/// A control message: an envelope of timestamp, origin, and body.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// Monotonic sequence number assigned by the sender.
    pub id: u64,
    /// When the message was received/created.
    pub timestamp: Timestamp,
    /// Where the message came from.
    pub source: MessageSource,
    /// The message payload.
    pub body: MessageBody,
}

impl Message {
    /// Creates a message stamped with the current time.
    pub fn now(id: u64, source: MessageSource, body: MessageBody) -> Self {
        Self { id, timestamp: Timestamp::now(), source, body }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_queue_transfers_messages() {
        let queue: MessageQueue = SpscRing::new(8);
        let msg = Message::now(
            1,
            MessageSource::Internal,
            MessageBody::Parameter {
                id: ParameterId::new("track.1.volume"),
                value: ParameterValue::Float(0.5),
            },
        );
        queue.push(msg.clone()).unwrap();
        let got = queue.pop().unwrap();
        assert_eq!(got, msg);
        assert!(queue.pop().is_none());
    }

    #[test]
    fn body_variants_construct() {
        let bodies = [
            MessageBody::Raw { protocol: Protocol::Midi1, data: vec![0x90, 60, 100] },
            MessageBody::Transport(TransportCommand::Play),
            MessageBody::Locate(Timecode::new(0, 1, 0, 0, crate::time::FrameRate::Fps25).unwrap()),
            MessageBody::Text("hello".into()),
        ];
        for b in bodies {
            let msg = Message::now(0, MessageSource::Internal, b);
            assert_eq!(msg.id, 0);
        }
    }
}
