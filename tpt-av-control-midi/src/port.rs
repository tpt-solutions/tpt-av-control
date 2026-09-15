//! MIDI port types.

/// Identifies a MIDI port within one enumeration.
///
/// The index is an enumeration-order handle; re-enumerate before reusing
/// stale ids (devices may come and go).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PortId {
    /// Enumeration-order index.
    pub index: usize,
}

impl PortId {
    /// Creates a port id from an enumeration index.
    pub const fn new(index: usize) -> Self {
        Self { index }
    }
}

/// Whether a port carries MIDI into or out of the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortDirection {
    /// Receives MIDI from the device.
    Input,
    /// Sends MIDI to the device.
    Output,
}

/// A single MIDI input or output port on a device.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MidiPort {
    /// Port identifier.
    pub id: PortId,
    /// Human-readable port name.
    pub name: String,
    /// Direction.
    pub direction: PortDirection,
}

impl MidiPort {
    /// Creates a port description.
    pub fn new(id: PortId, name: impl Into<String>, direction: PortDirection) -> Self {
        Self { id, name: name.into(), direction }
    }
}
