//! DMX512 universe data.

use tpt_av_control_utils::ControlError;

/// Number of channels in a DMX512 universe.
pub const DMX_CHANNELS: usize = 512;

/// A DMX512 universe (512 channels, values 0-255).
///
/// Channels are addressed zero-based (`0..512`); the on-wire slot number
/// is the channel plus one (slots are 1-based in the DMX512 standard).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DmxUniverse {
    /// Universe number (protocol-dependent range).
    pub universe: u16,
    /// Channel values (0-255), indexed by channel minus one.
    pub channels: [u8; DMX_CHANNELS],
}

impl DmxUniverse {
    /// Creates a new all-black universe.
    pub fn new(universe: u16) -> Self {
        Self {
            universe,
            channels: [0u8; DMX_CHANNELS],
        }
    }

    /// Sets a channel value (zero-based index).
    ///
    /// # Panics
    /// Panics if `channel >= 512`. For fallible use, see
    /// [`DmxUniverse::try_set_channel`].
    /// # Examples
    ///
    /// ```
    /// use tpt_av_control_dmx::DmxUniverse;
    /// let mut u = DmxUniverse::new(1);
    /// u.set_channel(0, 255);
    /// assert_eq!(u.get_channel(0), 255);
    /// assert_eq!(u.get_channel(512), 0, "out of range reads zero");
    /// ```
    pub fn set_channel(&mut self, channel: u16, value: u8) {
        self.channels[channel as usize] = value;
    }

    /// Sets a channel value, returning an error if out of range.
    pub fn try_set_channel(&mut self, channel: u16, value: u8) -> Result<(), ControlError> {
        let index = usize::from(channel);
        if index >= DMX_CHANNELS {
            return Err(ControlError::OutOfRange {
                value: i64::from(channel),
                min: 0,
                max: (DMX_CHANNELS - 1) as i64,
            });
        }
        self.channels[index] = value;
        Ok(())
    }

    /// Gets a channel value (zero-based index). Out-of-range reads 0.
    pub fn get_channel(&self, channel: u16) -> u8 {
        self.channels
            .get(usize::from(channel))
            .copied()
            .unwrap_or(0)
    }

    /// Copies `values` into consecutive channels starting at `start`.
    /// Truncates at the end of the universe.
    pub fn set_channels(&mut self, start: u16, values: &[u8]) {
        let start = usize::from(start);
        if start >= DMX_CHANNELS {
            return;
        }
        let end = (start + values.len()).min(DMX_CHANNELS);
        self.channels[start..end].copy_from_slice(&values[..end - start]);
    }

    /// Whether every channel is at zero.
    pub fn is_blackout(&self) -> bool {
        self.channels.iter().all(|&v| v == 0)
    }

    /// Sets every channel to zero.
    pub fn clear(&mut self) {
        self.channels = [0u8; DMX_CHANNELS];
    }
}

impl Default for DmxUniverse {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_channels() {
        let mut u = DmxUniverse::new(1);
        u.set_channel(0, 255);
        u.set_channel(511, 128);
        assert_eq!(u.get_channel(0), 255);
        assert_eq!(u.get_channel(511), 128);
        assert_eq!(u.get_channel(512), 0, "out-of-range reads 0");
        assert!(u.try_set_channel(512, 1).is_err());
    }

    #[test]
    fn bulk_set_truncates() {
        let mut u = DmxUniverse::new(0);
        u.set_channels(510, &[1, 2, 3, 4, 5]);
        assert_eq!(u.get_channel(509), 0);
        assert_eq!(u.get_channel(510), 1);
        assert_eq!(u.get_channel(511), 2);
    }

    #[test]
    fn blackout_helpers() {
        let mut u = DmxUniverse::new(7);
        assert!(u.is_blackout());
        u.set_channel(10, 5);
        assert!(!u.is_blackout());
        u.clear();
        assert!(u.is_blackout());
        assert_eq!(u.universe, 7);
    }
}
