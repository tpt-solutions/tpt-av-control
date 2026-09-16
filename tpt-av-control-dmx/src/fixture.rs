//! Lighting fixture definitions and addressing.

use crate::dmx::DmxUniverse;
use tpt_av_control_utils::ControlError;

/// The functional role of one fixture channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FixtureChannel {
    /// Master dimmer.
    Dimmer,
    /// Red component.
    Red,
    /// Green component.
    Green,
    /// Blue component.
    Blue,
    /// White component.
    White,
    /// Pan (coarse).
    Pan,
    /// Tilt (coarse).
    Tilt,
    /// Color wheel / generic function slot.
    Generic,
}

/// A fixture profile: which channels the fixture exposes, in the order
/// they occupy its DMX footprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureDefinition {
    /// Profile name (e.g. "Generic RGBW PAR").
    pub name: String,
    /// Channels in DMX-offset order (offset = index).
    pub channels: Vec<FixtureChannel>,
}

impl FixtureDefinition {
    /// Creates a profile, validating the channel count.
    pub fn new(
        name: impl Into<String>,
        channels: Vec<FixtureChannel>,
    ) -> Result<Self, ControlError> {
        if channels.is_empty() || channels.len() > crate::dmx::DMX_CHANNELS {
            return Err(ControlError::OutOfRange {
                value: channels.len() as i64,
                min: 1,
                max: crate::dmx::DMX_CHANNELS as i64,
            });
        }
        Ok(Self {
            name: name.into(),
            channels,
        })
    }

    /// A 1-channel dimmer profile.
    pub fn dimmer() -> Self {
        Self {
            name: "Dimmer".into(),
            channels: vec![FixtureChannel::Dimmer],
        }
    }

    /// A 3-channel RGB profile.
    pub fn rgb() -> Self {
        Self {
            name: "RGB".into(),
            channels: vec![
                FixtureChannel::Red,
                FixtureChannel::Green,
                FixtureChannel::Blue,
            ],
        }
    }

    /// A 4-channel RGBW profile.
    pub fn rgbw() -> Self {
        Self {
            name: "RGBW".into(),
            channels: vec![
                FixtureChannel::Red,
                FixtureChannel::Green,
                FixtureChannel::Blue,
                FixtureChannel::White,
            ],
        }
    }

    /// Offset of a channel type within the fixture, if present.
    pub fn offset(&self, channel: FixtureChannel) -> Option<usize> {
        self.channels.iter().position(|c| *c == channel)
    }

    /// DMX footprint size in channels.
    pub fn footprint(&self) -> usize {
        self.channels.len()
    }
}

/// One addressed fixture instance: a profile patched onto a universe at a
/// (zero-based) start address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixture {
    /// The fixture profile.
    pub definition: FixtureDefinition,
    /// Human label (e.g. "upstage wash 2").
    pub label: String,
    /// Universe the fixture lives on.
    pub universe: u16,
    /// Zero-based start address within the universe.
    pub start_address: u16,
}

impl Fixture {
    /// Patches a fixture, validating the address fits the universe.
    pub fn patch(
        definition: FixtureDefinition,
        label: impl Into<String>,
        universe: u16,
        start_address: u16,
    ) -> Result<Self, ControlError> {
        let end = usize::from(start_address) + definition.footprint();
        if end > crate::dmx::DMX_CHANNELS {
            return Err(ControlError::OutOfRange {
                value: end as i64,
                min: 1,
                max: crate::dmx::DMX_CHANNELS as i64,
            });
        }
        Ok(Self {
            definition,
            label: label.into(),
            universe,
            start_address,
        })
    }

    /// The first channel address AFTER this fixture's footprint.
    pub fn footprint_end(&self) -> usize {
        usize::from(self.start_address) + self.definition.footprint()
    }

    fn write(&self, universe: &mut DmxUniverse, offset: usize, value: u8) {
        universe.set_channel(self.start_address + offset as u16, value);
    }

    /// Sets master intensity (0-255); a no-op without a Dimmer channel.
    pub fn set_intensity(&self, universe: &mut DmxUniverse, intensity: u8) {
        if let Some(o) = self.definition.offset(FixtureChannel::Dimmer) {
            self.write(universe, o, intensity);
        }
    }

    /// Sets RGB(W) color at full intensity for present components;
    /// components the fixture lacks are skipped.
    pub fn set_color(&self, universe: &mut DmxUniverse, r: u8, g: u8, b: u8, w: u8) {
        for (channel, value) in [
            (FixtureChannel::Red, r),
            (FixtureChannel::Green, g),
            (FixtureChannel::Blue, b),
            (FixtureChannel::White, w),
        ] {
            if let Some(o) = self.definition.offset(channel) {
                self.write(universe, o, value);
            }
        }
    }

    /// Sets pan/tilt (coarse, 0-255, 128 = center); channels the fixture
    /// lacks are skipped.
    pub fn set_position(&self, universe: &mut DmxUniverse, pan: u8, tilt: u8) {
        if let Some(o) = self.definition.offset(FixtureChannel::Pan) {
            self.write(universe, o, pan);
        }
        if let Some(o) = self.definition.offset(FixtureChannel::Tilt) {
            self.write(universe, o, tilt);
        }
    }

    /// Reads a channel value back from the universe.
    pub fn get_channel(&self, universe: &DmxUniverse, channel: FixtureChannel) -> Option<u8> {
        self.definition
            .offset(channel)
            .map(|o| universe.get_channel(self.start_address + o as u16))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_have_expected_footprints() {
        assert_eq!(FixtureDefinition::dimmer().footprint(), 1);
        assert_eq!(FixtureDefinition::rgb().footprint(), 3);
        assert_eq!(FixtureDefinition::rgbw().footprint(), 4);
        assert!(FixtureDefinition::new("empty", vec![]).is_err());
    }

    #[test]
    fn patching_validates_address() {
        // 509 + 4 = 513 overruns the 512-channel universe.
        assert!(Fixture::patch(FixtureDefinition::rgbw(), "par 1", 1, 509).is_err());
        // 508 + 4 = 512 fits exactly.
        let fixture = Fixture::patch(FixtureDefinition::rgbw(), "par 1", 1, 508).unwrap();
        assert_eq!(fixture.footprint_end(), 512);
    }

    #[test]
    fn color_and_intensity_writes() {
        let mut u = DmxUniverse::new(1);
        let dimmer = Fixture::patch(FixtureDefinition::dimmer(), "d1", 1, 0).unwrap();
        let par = Fixture::patch(FixtureDefinition::rgbw(), "par 1", 1, 10).unwrap();
        dimmer.set_intensity(&mut u, 200);
        par.set_color(&mut u, 255, 0, 10, 0);
        assert_eq!(u.get_channel(0), 200);
        assert_eq!(u.get_channel(10), 255);
        assert_eq!(u.get_channel(11), 0);
        assert_eq!(u.get_channel(12), 10);
        assert_eq!(u.get_channel(13), 0);
        // Dimmer fixture ignores color (no RGB channels).
        dimmer.set_color(&mut u, 1, 2, 3, 4);
        assert_eq!(u.get_channel(0), 200);
        // Read-back.
        assert_eq!(par.get_channel(&u, FixtureChannel::Green), Some(0));
        assert_eq!(par.get_channel(&u, FixtureChannel::Pan), None);
    }
}
