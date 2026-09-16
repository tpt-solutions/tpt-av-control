//! The data-channel transport abstraction.
//!
//! Full ICE/DTLS/SCTP is intentionally out of scope: applications plug in
//! a WebRTC stack (or any reliable ordered channel) by implementing
//! [`DataChannelTransport`]. [`LoopbackTransport`] provides an in-process
//! reliable pair for tests and local IPC.

use crate::envelope::ControlEnvelope;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;
use tpt_av_control_utils::ControlError;

/// A reliable, ordered data channel carrying control envelopes.
pub trait DataChannelTransport: Send {
    /// Sends one envelope.
    fn send(&mut self, envelope: &ControlEnvelope) -> Result<(), ControlError>;

    /// Receives one envelope (blocking).
    fn recv(&mut self) -> Result<ControlEnvelope, ControlError>;

    /// Receives one envelope with a timeout. `Ok(None)` on timeout.
    fn recv_timeout(&mut self, timeout: Duration) -> Result<Option<ControlEnvelope>, ControlError>;
}

/// A connected pair of in-process transports: what one side sends, the
/// other receives, in order.
#[derive(Debug)]
pub struct LoopbackTransport {
    sender: mpsc::Sender<ControlEnvelope>,
    receiver: Receiver<ControlEnvelope>,
}

impl LoopbackTransport {
    /// Creates a connected pair.
    pub fn pair() -> (LoopbackTransport, LoopbackTransport) {
        let (a_tx, b_rx) = mpsc::channel();
        let (b_tx, a_rx) = mpsc::channel();
        (
            LoopbackTransport {
                sender: a_tx,
                receiver: a_rx,
            },
            LoopbackTransport {
                sender: b_tx,
                receiver: b_rx,
            },
        )
    }
}

impl DataChannelTransport for LoopbackTransport {
    fn send(&mut self, envelope: &ControlEnvelope) -> Result<(), ControlError> {
        self.sender
            .send(envelope.clone())
            .map_err(|_| ControlError::Closed)
    }

    fn recv(&mut self) -> Result<ControlEnvelope, ControlError> {
        self.receiver.recv().map_err(|_| ControlError::Closed)
    }

    fn recv_timeout(&mut self, timeout: Duration) -> Result<Option<ControlEnvelope>, ControlError> {
        match self.receiver.recv_timeout(timeout) {
            Ok(envelope) => Ok(Some(envelope)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(ControlError::Closed),
        }
    }
}

/// Receives all currently-pending envelopes (drain helper).
pub fn try_drain(
    transport: &mut dyn DataChannelTransport,
) -> Result<Vec<ControlEnvelope>, ControlError> {
    let mut out = Vec::new();
    loop {
        // Loopback drains exactly; generic transports would need a
        // non-blocking recv, so this helper is loopback-flavored.
        match transport.recv_timeout(Duration::ZERO) {
            Ok(Some(envelope)) => out.push(envelope),
            Ok(None) => return Ok(out),
            Err(ControlError::Closed) => return Ok(out),
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_av_control_utils::parameter::{ParameterId, ParameterValue};
    use tpt_av_control_utils::TransportCommand;

    #[test]
    fn loopback_pair_is_reliable_and_ordered() {
        let (mut a, mut b) = LoopbackTransport::pair();
        let envelopes = vec![
            ControlEnvelope::Command(TransportCommand::Play),
            ControlEnvelope::Parameter(crate::envelope::ParameterChangeRequest {
                parameter: ParameterId::new("master.gain"),
                value: ParameterValue::Float(0.5),
            }),
            ControlEnvelope::Text("one".into()),
            ControlEnvelope::Text("two".into()),
        ];
        for e in &envelopes {
            a.send(e).unwrap();
        }
        for expected in &envelopes {
            assert_eq!(&b.recv().unwrap(), expected);
        }
        assert!(b.recv_timeout(Duration::from_millis(10)).unwrap().is_none());
    }

    #[test]
    fn drain_collects_pending() {
        let (mut a, mut b) = LoopbackTransport::pair();
        a.send(&ControlEnvelope::Text("x".into())).unwrap();
        a.send(&ControlEnvelope::Text("y".into())).unwrap();
        let drained = try_drain(&mut b).unwrap();
        assert_eq!(drained.len(), 2);
        assert!(try_drain(&mut b).unwrap().is_empty());
        // Dropping `a` closes the channel.
        drop(a);
        assert!(matches!(b.recv(), Err(ControlError::Closed)));
    }

    #[test]
    fn timeout_maps_to_none() {
        let (mut a, mut b) = LoopbackTransport::pair();
        a.send(&ControlEnvelope::Text("z".into())).unwrap();
        assert_eq!(b.recv().unwrap(), ControlEnvelope::Text("z".into()));
        assert!(matches!(
            b.recv_timeout(Duration::ZERO),
            Ok::<Option<ControlEnvelope>, ControlError>(None)
        ));
    }
}
