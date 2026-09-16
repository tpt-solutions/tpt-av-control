//! Feedback rendering: LEDs, displays, and motor faders.

/// Feedback to send to a control surface.
#[derive(Debug, Clone, PartialEq)]
pub enum Feedback {
    /// Set LED state (on/off).
    Led {
        /// LED index.
        led: u8,
        /// Lit or dark.
        on: bool,
    },
    /// Set LED color (RGB); used by RGB pads (Stream Deck, Push, etc.).
    LedColor {
        /// LED index.
        led: u8,
        /// Red 0-255.
        r: u8,
        /// Green 0-255.
        g: u8,
        /// Blue 0-255.
        b: u8,
    },
    /// Set display text.
    DisplayText {
        /// Display index.
        display: u8,
        /// The text to show.
        text: String,
    },
    /// Set fader position (motorized faders); `value` is 0.0-1.0.
    FaderPosition {
        /// Fader index.
        channel: u8,
        /// Normalized position.
        value: f32,
    },
}

impl Feedback {
    /// Normalized scalar payloads (fader/strip positions) render to 0-255.
    pub fn normalized(&self) -> Option<f32> {
        match self {
            Feedback::FaderPosition { value, .. } => Some(value.clamp(0.0, 1.0)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fader_feedback_normalizes() {
        let f = Feedback::FaderPosition {
            channel: 1,
            value: 1.5,
        };
        assert!((f.normalized().unwrap() - 1.0).abs() < f32::EPSILON);
        let f = Feedback::FaderPosition {
            channel: 1,
            value: -0.2,
        };
        assert!((f.normalized().unwrap() - 0.0).abs() < f32::EPSILON);
        assert!(Feedback::Led { led: 0, on: true }.normalized().is_none());
    }
}
