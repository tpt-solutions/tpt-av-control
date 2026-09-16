//! Mapping MIDI messages onto stack parameters.

use tpt_av_control_utils::parameter::{Mapping, ParameterId, ParameterValue};

/// A bound CC: incoming control changes on `controller` map onto
#[derive(Debug, Clone, PartialEq)]
pub struct CcBinding {
    /// MIDI CC number (0-127).
    pub controller: u8,
    /// Target parameter.
    pub parameter: ParameterId,
    /// Transfer from 0..127 onto the parameter range.
    pub mapping: Mapping,
}

#[derive(Debug, Clone, PartialEq)]
/// A bound CC: incoming control changes on `controller` map onto
pub struct NoteBinding {
    /// MIDI note number (0-127).
    pub note: u8,
    /// Target parameter.
    pub parameter: ParameterId,
    /// Value sent on trigger.
    pub value: ParameterValue,
    /// Whether the parameter reverts to `off_value` on note-off.
    pub momentary: bool,
    /// Value used when momentary and the note is released.
    pub off_value: ParameterValue,
}

#[derive(Debug, Clone, PartialEq, Default)]
/// A bound note: note-on events on `note` trigger `parameter` = `value`.
pub struct MidiParameterMapper {
    cc_bindings: Vec<CcBinding>,
    note_bindings: Vec<NoteBinding>,
}

#[derive(Debug, Clone, PartialEq)]
/// A MIDI-to-parameter routing table.
pub struct MappedParameter {
    /// The parameter that changed.
    pub parameter: ParameterId,
    /// Its new value.
    pub value: ParameterValue,
}

impl MidiParameterMapper {
    /// An empty mapper.
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds a CC number to a parameter range.
    pub fn bind_cc(&mut self, binding: CcBinding) {
        self.cc_bindings
            .retain(|b| b.controller != binding.controller);
        self.cc_bindings.push(binding);
    }

    /// Binds a note to a parameter trigger.
    pub fn bind_note(&mut self, binding: NoteBinding) {
        self.note_bindings.retain(|b| b.note != binding.note);
        self.note_bindings.push(binding);
    }

    /// Registered CC bindings.
    pub fn cc_bindings(&self) -> &[CcBinding] {
        &self.cc_bindings
    }

    /// Registered note bindings.
    pub fn note_bindings(&self) -> &[NoteBinding] {
        &self.note_bindings
    }

    /// Maps an inbound MIDI message onto parameter changes. Returns all
    /// bindings the message triggers (usually zero or one).
    pub fn apply(&self, message: &crate::midi1::Midi1Message) -> Vec<MappedParameter> {
        use crate::midi1::Midi1Message;
        match message {
            Midi1Message::ControlChange {
                controller, value, ..
            } => self
                .cc_bindings
                .iter()
                .filter(|b| b.controller == *controller)
                .map(|b| MappedParameter {
                    parameter: b.parameter.clone(),
                    value: ParameterValue::Float(b.mapping.map(f32::from(*value))),
                })
                .collect(),
            Midi1Message::NoteOn { note, velocity, .. } if *velocity > 0 => self
                .note_bindings
                .iter()
                .filter(|b| b.note == *note)
                .map(|b| MappedParameter {
                    parameter: b.parameter.clone(),
                    value: b.value.clone(),
                })
                .collect(),
            Midi1Message::NoteOff { note, .. }
            | Midi1Message::NoteOn {
                note, velocity: 0, ..
            } => self
                .note_bindings
                .iter()
                .filter(|b| b.note == *note && b.momentary)
                .map(|b| MappedParameter {
                    parameter: b.parameter.clone(),
                    value: b.off_value.clone(),
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi1::Midi1Message;
    use tpt_av_control_utils::parameter::Curve;

    #[test]
    fn cc_maps_through_range() {
        let mut mapper = MidiParameterMapper::new();
        mapper.bind_cc(CcBinding {
            controller: 74,
            parameter: ParameterId::new("track.1.filter"),
            mapping: Mapping {
                input_range: (0.0, 127.0),
                output_range: (20.0, 20_000.0),
                curve: Curve::Exponential { rate: 3.0 },
            },
        });
        let low = mapper.apply(&Midi1Message::ControlChange {
            channel: 0,
            controller: 74,
            value: 0,
        });
        assert_eq!(low.len(), 1);
        assert!((low[0].value.as_f32().unwrap() - 20.0).abs() < 0.01);

        let high = mapper.apply(&Midi1Message::ControlChange {
            channel: 0,
            controller: 74,
            value: 127,
        });
        assert!((high[0].value.as_f32().unwrap() - 20_000.0).abs() < 0.1);
    }

    #[test]
    fn note_triggers_and_releases() {
        let mut mapper = MidiParameterMapper::new();
        mapper.bind_note(NoteBinding {
            note: 60,
            parameter: ParameterId::new("transport.play"),
            value: ParameterValue::Bool(true),
            momentary: true,
            off_value: ParameterValue::Bool(false),
        });
        let on = mapper.apply(&Midi1Message::NoteOn {
            channel: 0,
            note: 60,
            velocity: 100,
        });
        assert_eq!(on[0].value, ParameterValue::Bool(true));
        let off = mapper.apply(&Midi1Message::NoteOff {
            channel: 0,
            note: 60,
            velocity: 0,
        });
        assert_eq!(off[0].value, ParameterValue::Bool(false));
        // Velocity-0 note-on acts as note-off.
        let off = mapper.apply(&Midi1Message::NoteOn {
            channel: 0,
            note: 60,
            velocity: 0,
        });
        assert_eq!(off[0].value, ParameterValue::Bool(false));
    }

    #[test]
    fn rebinding_replaces() {
        let mut mapper = MidiParameterMapper::new();
        mapper.bind_cc(CcBinding {
            controller: 1,
            parameter: ParameterId::new("a"),
            mapping: Mapping::default(),
        });
        mapper.bind_cc(CcBinding {
            controller: 1,
            parameter: ParameterId::new("b"),
            mapping: Mapping::default(),
        });
        assert_eq!(mapper.cc_bindings().len(), 1);
        assert_eq!(mapper.cc_bindings()[0].parameter.as_str(), "b");
    }
}
