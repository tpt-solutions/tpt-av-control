//! The [`ControlSurface`] trait and the protocol-neutral event model.

use crate::feedback::Feedback;
use tpt_av_control_utils::ControlError;

/// A hardware control surface: faders, buttons, encoders, displays.
pub trait ControlSurface: Send {
    /// The surface name.
    fn name(&self) -> &str;

    /// Initializes the surface (subscribe, reset LEDs, etc.).
    fn init(&mut self) -> Result<(), ControlError>;

    /// Reads the next control event. Returns `Ok(None)` when nothing is
    /// pending (non-blocking).
    fn read_event(&mut self) -> Result<Option<ControlEvent>, ControlError>;

    /// Sends feedback to the surface (LEDs, displays, motor faders).
    fn send_feedback(&mut self, feedback: &Feedback) -> Result<(), ControlError>;
}

/// A control event from a surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlEvent {
    /// Fader/knob movement; `value` is normalized 0.0-1.0.
    FaderMove {
        /// Fader index.
        channel: u8,
        /// Normalized position.
        value: f32,
    },
    /// Button press.
    ButtonPress {
        /// Button index.
        button: u8,
    },
    /// Button release.
    ButtonRelease {
        /// Button index.
        button: u8,
    },
    /// Encoder rotation; `delta` is the detent count (+/-).
    EncoderRotate {
        /// Encoder index.
        encoder: u8,
        /// Detents moved (positive = clockwise).
        delta: i8,
    },
    /// Touch strip movement; `value` is normalized 0.0-1.0.
    TouchStrip {
        /// Strip index.
        strip: u8,
        /// Normalized position.
        value: f32,
    },
}

impl ControlEvent {
    /// The control index this event refers to (channel/button/encoder/strip).
    pub fn control(&self) -> u8 {
        match *self {
            ControlEvent::FaderMove { channel, .. } => channel,
            ControlEvent::ButtonPress { button } => button,
            ControlEvent::ButtonRelease { button } => button,
            ControlEvent::EncoderRotate { encoder, .. } => encoder,
            ControlEvent::TouchStrip { strip, .. } => strip,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_index_extraction() {
        assert_eq!(
            ControlEvent::FaderMove {
                channel: 3,
                value: 0.5
            }
            .control(),
            3
        );
        assert_eq!(ControlEvent::ButtonPress { button: 9 }.control(), 9);
        assert_eq!(
            ControlEvent::EncoderRotate {
                encoder: 2,
                delta: -1
            }
            .control(),
            2
        );
        assert_eq!(
            ControlEvent::TouchStrip {
                strip: 5,
                value: 0.1
            }
            .control(),
            5
        );
    }
}
