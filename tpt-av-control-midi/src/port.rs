//! MIDI port types.

/// Identifies a MIDI port within one enumeration.
///
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Whether a port carries MIDI into or out of the host.
pub enum PortDirection {
    /// Receives MIDI from the device.
    Input,
    /// Sends MIDI to the device.
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// A single MIDI input or output port on a device.
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
        Self {
            id,
            name: name.into(),
            direction,
        }
    }
}
