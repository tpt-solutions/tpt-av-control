//! MIDI device enumeration and I/O.
//!
//! Real ports wrap `midir`; the [`MidiSource`] / [`MidiSink`] traits let
//! applications (and tests) inject their own transports, and
//! [`VirtualMidiPair`] provides an in-memory loopback.

use crate::midi1::{parse_midi1, Midi1Message};
use crate::port::{MidiPort, PortDirection, PortId};
use crate::ump::Ump;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::time::Duration;
use tpt_av_control_utils::ControlError;

/// Anything that can deliver inbound MIDI messages.
pub trait MidiSource: Send {
    /// Receives the next message (blocking).
    fn recv(&mut self) -> Result<InboundMidi, ControlError>;
    /// Receives the next message (non-blocking).
    fn try_recv(&mut self) -> Result<Option<InboundMidi>, ControlError>;
    /// Receives the next message with a timeout.
    fn recv_timeout(&mut self, timeout: Duration) -> Result<Option<InboundMidi>, ControlError>;
}

/// Anything that can send MIDI messages.
pub trait MidiSink: Send {
    /// Sends a MIDI 1.0 message.
    fn send_midi1(&mut self, message: &Midi1Message) -> Result<(), ControlError>;
    /// Sends a MIDI 2.0 UMP packet.
    fn send_ump(&mut self, ump: &Ump) -> Result<(), ControlError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A message received from a MIDI input.
pub struct InboundMidi {
    /// Microseconds since an unspecified fixed point in the past (as
    /// reported by midir; constant over the life of a connection).
    pub timestamp_us: u64,
    /// Raw bytes as delivered by the transport.
    pub raw: Vec<u8>,
    /// The parsed MIDI 1.0 message, when the bytes parse as one.
    pub message: Option<Midi1Message>,
}

impl InboundMidi {
    /// Builds an inbound message from raw bytes.
    pub fn from_raw(timestamp_us: u64, raw: &[u8]) -> Self {
        Self {
            timestamp_us,
            raw: raw.to_vec(),
            message: parse_midi1(raw).ok(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A MIDI device with its input and output ports.
pub struct MidiDevice {
    /// Device display name.
    pub name: String,
    /// Input ports exposed by the device.
    pub inputs: Vec<MidiPort>,
    /// Output ports exposed by the device.
    pub outputs: Vec<MidiPort>,
}

/// Enumerates available MIDI devices via midir.
///
/// Ports whose names match exactly between inputs and outputs are grouped
/// `MidiInput::ports()` / `MidiOutput::ports()`; [`open_input`] and
/// [`open_output`] re-enumerate to resolve them, so ids stay valid only
pub fn enumerate_devices() -> Result<Vec<MidiDevice>, ControlError> {
    let client_name = "tpt-av-control";
    let input = midir::MidiInput::new(client_name)
        .map_err(|e| ControlError::Io(std::io::Error::other(e.to_string())))?;
    let output = midir::MidiOutput::new(client_name)
        .map_err(|e| ControlError::Io(std::io::Error::other(e.to_string())))?;

    let mut devices: Vec<MidiDevice> = Vec::new();
    let mut next_id = 0usize;
    for (i, port) in input.ports().into_iter().enumerate() {
        let name = input
            .port_name(&port)
            .unwrap_or_else(|_| format!("input {i}"));
        let id = PortId::new(next_id);
        next_id += 1;
        devices.push(MidiDevice {
            name: name.clone(),
            inputs: vec![MidiPort::new(id, name, PortDirection::Input)],
            outputs: Vec::new(),
        });
    }
    for (i, port) in output.ports().into_iter().enumerate() {
        let name = output
            .port_name(&port)
            .unwrap_or_else(|_| format!("output {i}"));
        let id = PortId::new(next_id);
        next_id += 1;
        // Merge with an input-only device of the same name.
        if let Some(device) = devices
            .iter_mut()
            .find(|d| d.name == name && d.outputs.is_empty())
        {
            device
                .outputs
                .push(MidiPort::new(id, name, PortDirection::Output));
        } else {
            devices.push(MidiDevice {
                name: name.clone(),
                inputs: Vec::new(),
                outputs: vec![MidiPort::new(id, name, PortDirection::Output)],
            });
        }
    }
    Ok(devices)
}

/// Opens a MIDI input port for reading.
///
/// The port is resolved by re-enumerating and taking `port.id.index`
pub fn open_input(port: &MidiPort) -> Result<MidiInput, ControlError> {
    let input = midir::MidiInput::new("tpt-av-control")
        .map_err(|e| ControlError::Io(std::io::Error::other(e.to_string())))?;
    let midir_port = input
        .ports()
        .into_iter()
        .nth(port.id.index)
        .ok_or_else(|| ControlError::DeviceNotFound(port.name.clone()))?;
    let (tx, rx) = mpsc::channel::<InboundMidi>();
    let connection = input
        .connect(
            &midir_port,
            &port.name,
            move |timestamp_us, bytes, _| {
                let _ = tx.send(InboundMidi::from_raw(timestamp_us, bytes));
            },
            (),
        )
        .map_err(|e| ControlError::PortBusy(format!("{}: {}", port.name, e.kind())))?;
    Ok(MidiInput {
        connection,
        receiver: rx,
    })
}

/// Opens a MIDI output port for writing.
///
/// The port is resolved by re-enumerating and taking `port.id.index`
pub fn open_output(port: &MidiPort) -> Result<MidiOutput, ControlError> {
    let output = midir::MidiOutput::new("tpt-av-control")
        .map_err(|e| ControlError::Io(std::io::Error::other(e.to_string())))?;
    let midir_port = output
        .ports()
        .into_iter()
        .nth(port.id.index)
        .ok_or_else(|| ControlError::DeviceNotFound(port.name.clone()))?;
    let connection = output
        .connect(&midir_port, &port.name)
        .map_err(|e| ControlError::PortBusy(format!("{}: {}", port.name, e.kind())))?;
    Ok(MidiOutput { connection })
}

/// A live MIDI input stream (blocking `recv`, non-blocking `try_recv`).
pub struct MidiInput {
    // Holds the connection open; dropping this struct disconnects.
    #[allow(dead_code)]
    connection: midir::MidiInputConnection<()>,
    receiver: Receiver<InboundMidi>,
}

impl MidiInput {
    /// Receives the next MIDI message (blocking).
    pub fn recv(&mut self) -> Result<InboundMidi, ControlError> {
        match self.receiver.recv() {
            Ok(message) => Ok(message),
            Err(_) => Err(ControlError::Closed),
        }
    }

    /// Receives the next MIDI message (non-blocking).
    pub fn try_recv(&mut self) -> Result<Option<InboundMidi>, ControlError> {
        match self.receiver.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(ControlError::Closed),
        }
    }
}

impl MidiSource for MidiInput {
    fn recv(&mut self) -> Result<InboundMidi, ControlError> {
        MidiInput::recv(self)
    }

    fn try_recv(&mut self) -> Result<Option<InboundMidi>, ControlError> {
        MidiInput::try_recv(self)
    }

    fn recv_timeout(&mut self, timeout: Duration) -> Result<Option<InboundMidi>, ControlError> {
        match self.receiver.recv_timeout(timeout) {
            Ok(message) => Ok(Some(message)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(ControlError::Closed),
        }
    }
}

/// A live MIDI output stream.
pub struct MidiOutput {
    connection: midir::MidiOutputConnection,
}

impl MidiOutput {
    /// Sends a MIDI 1.0 message.
    pub fn send_midi1(&mut self, message: &Midi1Message) -> Result<(), ControlError> {
        let bytes = message.to_bytes();
        self.connection
            .send(&bytes)
            .map_err(|e| ControlError::Io(std::io::Error::other(e.to_string())))
    }

    /// Sends a MIDI 2.0 UMP packet.
    ///
    /// transport that carries UMPs natively (USB MIDI 2.0 or a BLE UMP
    /// endpoint). On MIDI 1.0-only endpoints, translate first with
    pub fn send_ump(&mut self, ump: &Ump) -> Result<(), ControlError> {
        let bytes = ump.to_bytes();
        self.connection
            .send(&bytes)
            .map_err(|e| ControlError::Io(std::io::Error::other(e.to_string())))
    }

    /// Sends raw bytes.
    pub fn send_raw(&mut self, bytes: &[u8]) -> Result<(), ControlError> {
        self.connection
            .send(bytes)
            .map_err(|e| ControlError::Io(std::io::Error::other(e.to_string())))
    }
}

impl MidiSink for MidiOutput {
    fn send_midi1(&mut self, message: &Midi1Message) -> Result<(), ControlError> {
        MidiOutput::send_midi1(self, message)
    }

    fn send_ump(&mut self, ump: &Ump) -> Result<(), ControlError> {
        MidiOutput::send_ump(self, ump)
    }
}

/// An in-memory MIDI loopback: sending into the sink makes the message
#[derive(Debug)]
pub struct VirtualMidiPair {
    sender: mpsc::Sender<InboundMidi>,
    receiver: mpsc::Receiver<InboundMidi>,
}

impl Default for VirtualMidiPair {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualMidiPair {
    /// Creates a loopback pair with an unbounded queue.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }

    /// Splits into a (source, sink) pair.
    pub fn split(self) -> (VirtualSource, VirtualSink) {
        let VirtualMidiPair { sender, receiver } = self;
        let source = VirtualSource {
            receiver,
            clock: std::time::Instant::now(),
        };
        let sink = VirtualSink { sender };
        (source, sink)
    }
}

/// Receiver half of [`VirtualMidiPair`].
pub struct VirtualSource {
    receiver: Receiver<InboundMidi>,
    /// Clock counter value.
    clock: std::time::Instant,
}

impl MidiSource for VirtualSource {
    fn recv(&mut self) -> Result<InboundMidi, ControlError> {
        self.receiver.recv().map_err(|_| ControlError::Closed)
    }

    fn try_recv(&mut self) -> Result<Option<InboundMidi>, ControlError> {
        match self.receiver.try_recv() {
            Ok(m) => Ok(Some(m)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(ControlError::Closed),
        }
    }

    fn recv_timeout(&mut self, timeout: Duration) -> Result<Option<InboundMidi>, ControlError> {
        match self.receiver.recv_timeout(timeout) {
            Ok(m) => Ok(Some(m)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(ControlError::Closed),
        }
    }
}

impl VirtualSource {
    /// Milliseconds elapsed since the pair was created (used as the
    pub fn elapsed_ms(&self) -> u64 {
        self.clock.elapsed().as_millis() as u64
    }
}

/// Sender half of [`VirtualMidiPair`].
pub struct VirtualSink {
    sender: mpsc::Sender<InboundMidi>,
}

impl MidiSink for VirtualSink {
    fn send_midi1(&mut self, message: &Midi1Message) -> Result<(), ControlError> {
        let raw = message.to_bytes();
        self.sender
            .send(InboundMidi {
                timestamp_us: 0,
                raw,
                message: Some(message.clone()),
            })
            .map_err(|_| ControlError::Closed)
    }

    fn send_ump(&mut self, ump: &Ump) -> Result<(), ControlError> {
        let raw = ump.to_bytes();
        self.sender
            .send(InboundMidi {
                timestamp_us: 0,
                raw,
                message: None,
            })
            .map_err(|_| ControlError::Closed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi1::Midi1Message;

    #[test]
    fn virtual_loopback() {
        let pair = VirtualMidiPair::new();
        let (mut source, mut sink) = pair.split();
        assert!(source.try_recv().unwrap().is_none());
        sink.send_midi1(&Midi1Message::NoteOn {
            channel: 0,
            note: 60,
            velocity: 100,
        })
        .unwrap();
        let got = source
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        assert_eq!(
            got.message,
            Some(Midi1Message::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100
            })
        );
        assert_eq!(got.raw, vec![0x90, 0x3C, 0x64]);
    }

    #[test]
    fn virtual_ump_loopback() {
        let pair = VirtualMidiPair::new();
        let (mut source, mut sink) = pair.split();
        sink.send_ump(&Ump::from_words(&[0x4090_3C00, 0x8000_0000]))
            .unwrap();
        let got = source.try_recv().unwrap().unwrap();
        // Raw UMP bytes don't parse as MIDI 1.0.
        assert!(got.message.is_none());
        assert_eq!(
            got.raw,
            vec![0x40, 0x90, 0x3C, 0x00, 0x80, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn enumeration_does_not_panic() {
        // Should succeed (possibly with zero devices) on any platform.
        let devices = enumerate_devices().unwrap();
        for device in &devices {
            assert!(!device.name.is_empty());
        }
    }
}
