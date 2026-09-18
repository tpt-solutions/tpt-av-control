//! Surface-to-parameter mapping configuration.

/// Maps a MIDI CC (or OSC path) fader onto a parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct FaderMapping {
    /// MIDI CC number (or control index for non-MIDI surfaces).
    pub cc: u8,
    /// Parameter path (e.g. "track.1.volume").
    pub parameter: String,
    /// Output value range (min, max).
    pub range: (f32, f32),
}

/// Maps a note/button onto an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonMapping {
    /// MIDI note number (or button index).
    pub note: u8,
    /// Action path (e.g. "transport.play").
    pub action: String,
    /// Whether the action fires on press and release (momentary) or only
    /// on press (toggle).
    pub momentary: bool,
}

/// Maps a relative CC encoder onto a parameter increment.
#[derive(Debug, Clone, PartialEq)]
pub struct EncoderMapping {
    /// MIDI CC number (or encoder index).
    pub cc: u8,
    /// Parameter path.
    pub parameter: String,
    /// Parameter delta per detent.
    pub step: f32,
    /// Whether the controller emits two's-complement relative values
    /// (0x41 = +1 … 0x3F = -1) rather than absolute values.
    pub relative: bool,
}

/// A declarative mapping configuration for a control surface.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SurfaceMapping {
    /// Fader mappings (MIDI CC → parameter).
    pub faders: Vec<FaderMapping>,
    /// Button mappings (MIDI note → action).
    pub buttons: Vec<ButtonMapping>,
    /// Encoder mappings (MIDI CC → parameter step).
    pub encoders: Vec<EncoderMapping>,
}

impl SurfaceMapping {
    /// An empty mapping.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a fader mapping (replacing any with the same CC).
    /// # Examples
    ///
    /// ```
    /// use tpt_av_control_surface::{FaderMapping, SurfaceMapping};
    /// let mut m = SurfaceMapping::new();
    /// m.add_fader(FaderMapping {
    ///     cc: 7,
    ///     parameter: "master.volume".into(),
    ///     range: (0.0, 1.0),
    /// });
    /// assert_eq!(m.fader(7).unwrap().parameter, "master.volume");
    /// ```
    pub fn add_fader(&mut self, fader: FaderMapping) {
        self.faders.retain(|f| f.cc != fader.cc);
        self.faders.push(fader);
    }

    /// Adds a button mapping (replacing any with the same note).
    pub fn add_button(&mut self, button: ButtonMapping) {
        self.buttons.retain(|b| b.note != button.note);
        self.buttons.push(button);
    }

    /// Adds an encoder mapping (replacing any with the same CC).
    pub fn add_encoder(&mut self, encoder: EncoderMapping) {
        self.encoders.retain(|e| e.cc != encoder.cc);
        self.encoders.push(encoder);
    }

    /// Looks up the fader bound to `cc`.
    pub fn fader(&self, cc: u8) -> Option<&FaderMapping> {
        self.faders.iter().find(|f| f.cc == cc)
    }

    /// Looks up the button bound to `note`.
    pub fn button(&self, note: u8) -> Option<&ButtonMapping> {
        self.buttons.iter().find(|b| b.note == note)
    }

    /// Looks up the encoder bound to `cc`.
    pub fn encoder(&self, cc: u8) -> Option<&EncoderMapping> {
        self.encoders.iter().find(|e| e.cc == cc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_replaces_same_control() {
        let mut m = SurfaceMapping::new();
        m.add_fader(FaderMapping {
            cc: 7,
            parameter: "a".into(),
            range: (0.0, 1.0),
        });
        m.add_fader(FaderMapping {
            cc: 7,
            parameter: "b".into(),
            range: (0.0, 100.0),
        });
        assert_eq!(m.faders.len(), 1);
        assert_eq!(m.fader(7).unwrap().parameter, "b");
        assert!(m.fader(8).is_none());
    }

    #[test]
    fn buttons_and_encoders_lookup() {
        let mut m = SurfaceMapping::new();
        m.add_button(ButtonMapping {
            note: 60,
            action: "play".into(),
            momentary: true,
        });
        m.add_encoder(EncoderMapping {
            cc: 16,
            parameter: "tempo".into(),
            step: 0.5,
            relative: true,
        });
        assert_eq!(m.button(60).unwrap().action, "play");
        assert_eq!(m.encoder(16).unwrap().step, 0.5);
    }
}
