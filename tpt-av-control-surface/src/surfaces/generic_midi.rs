//! Generic MIDI control surface: any MIDI controller with a declarative
//! [`SurfaceMapping`].

use crate::feedback::Feedback;
use crate::mapping::SurfaceMapping;
use crate::surface::{ControlEvent, ControlSurface};
use std::time::Duration;
use tpt_av_control_midi::{ControlError, Midi1Message, MidiSink, MidiSource};

/// A generic MIDI control surface driven by MIDI note/CC events.
///
/// Convention: note-on/off become button press/release, absolute CCs
/// become fader moves (0-127 normalized), relative CCs (per mapping)
/// become encoder rotations, and pitch bend becomes a touch strip.
pub struct GenericMidiSurface {
    name: String,
    input: Box<dyn MidiSource>,
    output: Option<Box<dyn MidiSink>>,
    mapping: SurfaceMapping,
}

impl GenericMidiSurface {
    /// Creates a surface over a MIDI source (and optional sink for
    /// feedback), with the mapping that describes the controller.
    pub fn new(
        name: impl Into<String>,
        input: Box<dyn MidiSource>,
        output: Option<Box<dyn MidiSink>>,
        mapping: SurfaceMapping,
    ) -> Self {
        Self {
            name: name.into(),
            input,
            output,
            mapping,
        }
    }

    /// The active mapping.
    pub fn mapping(&self) -> &SurfaceMapping {
        &self.mapping
    }

    /// Converts an inbound MIDI message into a control event.
    fn to_event(&self, message: &Midi1Message) -> Option<ControlEvent> {
        match message {
            Midi1Message::NoteOn { note, velocity, .. } if *velocity > 0 => {
                Some(ControlEvent::ButtonPress { button: *note })
            }
            Midi1Message::NoteOff { note, .. }
            | Midi1Message::NoteOn {
                note, velocity: 0, ..
            } => Some(ControlEvent::ButtonRelease { button: *note }),
            Midi1Message::ControlChange {
                controller, value, ..
            } => {
                if self.mapping.encoder(*controller).map(|e| e.relative) == Some(true) {
                    // Two's-complement relative encoding per the MIDI
                    // relative convention (0x41..=0x7F positive,
                    // 0x01..=0x3F negative).
                    // Two's-complement relative decode: 0x41..=0x7F is
                    // +1..=+63, 0x00..=0x3F is -64..=-1, 0x40 is a no-op.
                    let delta = match *value {
                        0x41..=0x7F => (*value - 0x40) as i8,
                        0x00..=0x3F => (*value as i16 - 0x40) as i8,
                        _ => 0,
                    };
                    Some(ControlEvent::EncoderRotate {
                        encoder: *controller,
                        delta,
                    })
                } else {
                    Some(ControlEvent::FaderMove {
                        channel: *controller,
                        value: f32::from(*value) / 127.0,
                    })
                }
            }
            Midi1Message::PitchBendChange { value, .. } => Some(ControlEvent::TouchStrip {
                strip: 0,
                value: f32::from(*value) / 16383.0,
            }),
            _ => None,
        }
    }
}

impl ControlSurface for GenericMidiSurface {
    fn name(&self) -> &str {
        &self.name
    }

    fn init(&mut self) -> Result<(), ControlError> {
        // Drain anything pending from before we connected.
        while self.input.try_recv()?.is_some() {}
        Ok(())
    }

    fn read_event(&mut self) -> Result<Option<ControlEvent>, ControlError> {
        while let Some(inbound) = self.input.try_recv()? {
            if let Some(event) = inbound.message.as_ref().and_then(|m| self.to_event(m)) {
                return Ok(Some(event));
            }
        }
        Ok(None)
    }

    fn send_feedback(&mut self, feedback: &Feedback) -> Result<(), ControlError> {
        let Some(output) = self.output.as_mut() else {
            return Err(ControlError::Unsupported(
                "surface has no feedback output".into(),
            ));
        };
        match feedback {
            Feedback::Led { led, on } => output.send_midi1(&Midi1Message::NoteOn {
                channel: 0,
                note: *led,
                velocity: if *on { 127 } else { 0 },
            }),
            Feedback::FaderPosition { channel, value } => {
                let raw = (value.clamp(0.0, 1.0) * 127.0).round() as u8;
                output.send_midi1(&Midi1Message::ControlChange {
                    channel: 0,
                    controller: *channel,
                    value: raw,
                })
            }
            Feedback::LedColor { .. } => Err(ControlError::Unsupported(
                "generic MIDI LEDs are monochrome; use Led".into(),
            )),
            Feedback::DisplayText { .. } => Err(ControlError::Unsupported(
                "generic MIDI controllers have no displays".into(),
            )),
        }
    }
}

impl GenericMidiSurface {
    /// Blocks up to `timeout` waiting for an event (poll loop helper).
    pub fn read_event_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<ControlEvent>, ControlError> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if let Some(event) = self.read_event()? {
                return Ok(Some(event));
            }
            if std::time::Instant::now() >= deadline {
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_av_control_midi::VirtualMidiPair;

    type Parts = (
        GenericMidiSurface,
        tpt_av_control_midi::VirtualSink,
        Box<dyn MidiSource>,
    );

    fn surface() -> Parts {
        let pair = VirtualMidiPair::new();
        let (source, controller_side) = pair.split();
        // The surface reads what the "controller" sends; feedback goes
        // through a second pair so tests can observe it.
        let fb_pair = VirtualMidiPair::new();
        let (fb_source, feedback_sink) = fb_pair.split();
        let mut mapping = SurfaceMapping::new();
        mapping.add_fader(crate::mapping::FaderMapping {
            cc: 7,
            parameter: "volume".into(),
            range: (0.0, 1.0),
        });
        mapping.add_button(crate::mapping::ButtonMapping {
            note: 60,
            action: "play".into(),
            momentary: true,
        });
        mapping.add_encoder(crate::mapping::EncoderMapping {
            cc: 16,
            parameter: "tempo".into(),
            step: 0.5,
            relative: true,
        });
        (
            GenericMidiSurface::new(
                "test",
                Box::new(source),
                Some(Box::new(feedback_sink)),
                mapping,
            ),
            controller_side,
            Box::new(fb_source),
        )
    }

    #[test]
    fn note_on_becomes_button_press() {
        let (mut surface, mut controller, _fb_source) = surface();
        surface.init().unwrap();
        controller
            .send_midi1(&Midi1Message::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100,
            })
            .unwrap();
        let event = surface.read_event().unwrap().unwrap();
        assert_eq!(event, ControlEvent::ButtonPress { button: 60 });
        controller
            .send_midi1(&Midi1Message::NoteOff {
                channel: 0,
                note: 60,
                velocity: 0,
            })
            .unwrap();
        assert_eq!(
            surface.read_event().unwrap().unwrap(),
            ControlEvent::ButtonRelease { button: 60 }
        );
    }

    #[test]
    fn cc7_becomes_fader_move() {
        let (mut surface, mut controller, _fb_source) = surface();
        controller
            .send_midi1(&Midi1Message::ControlChange {
                channel: 0,
                controller: 7,
                value: 64,
            })
            .unwrap();
        match surface.read_event().unwrap().unwrap() {
            ControlEvent::FaderMove { channel, value } => {
                assert_eq!(channel, 7);
                assert!((value - 64.0 / 127.0).abs() < 1e-6);
            }
            other => panic!("expected fader move, got {other:?}"),
        }
    }

    #[test]
    fn relative_cc_becomes_encoder() {
        let (mut surface, mut controller, _fb_source) = surface();
        controller
            .send_midi1(&Midi1Message::ControlChange {
                channel: 0,
                controller: 16,
                value: 0x41, // +1
            })
            .unwrap();
        match surface.read_event().unwrap().unwrap() {
            ControlEvent::EncoderRotate { encoder, delta } => {
                assert_eq!(encoder, 16);
                assert_eq!(delta, 1);
            }
            other => panic!("expected encoder, got {other:?}"),
        }
        controller
            .send_midi1(&Midi1Message::ControlChange {
                channel: 0,
                controller: 16,
                value: 0x3F, // -1
            })
            .unwrap();
        match surface.read_event().unwrap().unwrap() {
            ControlEvent::EncoderRotate { delta, .. } => assert_eq!(delta, -1),
            other => panic!("expected encoder, got {other:?}"),
        }
    }

    #[test]
    fn led_feedback_sends_note() {
        let (mut surface, _controller, mut fb_source) = surface();
        surface
            .send_feedback(&Feedback::Led { led: 60, on: true })
            .unwrap();
        surface
            .send_feedback(&Feedback::FaderPosition {
                channel: 7,
                value: 0.5,
            })
            .unwrap();
        // Both feedback messages arrive on the feedback stream.
        assert_eq!(
            fb_source.recv().unwrap().message,
            Some(Midi1Message::NoteOn {
                channel: 0,
                note: 60,
                velocity: 127
            })
        );
        assert_eq!(
            fb_source.recv().unwrap().message,
            Some(Midi1Message::ControlChange {
                channel: 0,
                controller: 7,
                value: 64
            })
        );
        // Unsupported feedback types error.
        assert!(surface
            .send_feedback(&Feedback::LedColor {
                led: 0,
                r: 1,
                g: 2,
                b: 3
            })
            .is_err());
    }
}
