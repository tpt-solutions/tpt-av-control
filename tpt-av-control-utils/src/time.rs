//! Timecode and timing types.

use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Offset in seconds between the NTP epoch (1900-01-01) and the Unix epoch
/// (1970-01-01).
pub const NTP_UNIX_EPOCH_OFFSET_SECS: u64 = 2_208_988_800;

/// Frame rates used by SMPTE-style timecode and MIDI Time Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameRate {
    /// 24 fps (film).
    Fps24,
    /// 25 fps (PAL).
    Fps25,
    /// 30 fps non-drop (NTSC without drop-frame).
    Fps30Ndf,
    /// 29.97 fps drop-frame.
    Fps2997Df,
}

impl FrameRate {
    /// Nominal integer frame rate (29.97 drop-frame reports 30).
    pub fn fps(self) -> u8 {
        match self {
            FrameRate::Fps24 => 24,
            FrameRate::Fps25 => 25,
            FrameRate::Fps30Ndf | FrameRate::Fps2997Df => 30,
        }
    }

    /// Whether this rate uses drop-frame counting.
    pub fn is_drop_frame(self) -> bool {
        matches!(self, FrameRate::Fps2997Df)
    }

    /// All supported frame rates.
    pub const ALL: [FrameRate; 4] = [
        FrameRate::Fps24,
        FrameRate::Fps25,
        FrameRate::Fps30Ndf,
        FrameRate::Fps2997Df,
    ];
}

/// An SMPTE-style timecode value.
///
/// Frames are zero-based within the second (0 ..= fps-1), matching MTC.
/// For drop-frame time, the two "virtual" labels `;00` and `;01` of every
/// dropped minute normalize to the count of the next real frame (`;02`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timecode {
    /// Hours (0-23).
    pub hours: u8,
    /// Minutes (0-59).
    pub minutes: u8,
    /// Seconds (0-59).
    pub seconds: u8,
    /// Frame within the second (0 ..= fps-1).
    pub frames: u8,
    /// The frame rate this timecode is expressed in.
    pub rate: FrameRate,
}

impl Timecode {
    /// Creates a timecode, returning `None` if any field is out of range.
    pub fn new(hours: u8, minutes: u8, seconds: u8, frames: u8, rate: FrameRate) -> Option<Self> {
        if hours > 23 || minutes > 59 || seconds > 59 || frames >= rate.fps() {
            return None;
        }
        Some(Self {
            hours,
            minutes,
            seconds,
            frames,
            rate,
        })
    }

    /// Splits a nominal-frame count into components at the given integer fps.
    fn split_nominal(nominal: u64, fps: u64) -> (u64, u64, u64, u64) {
        (
            nominal / (3600 * fps),
            (nominal / (60 * fps)) % 60,
            (nominal / fps) % 60,
            nominal % fps,
        )
    }

    // Drop-frame (30 fps nominal) counting layout, per SMPTE 12M:
    // - every minute whose number is not a multiple of 10 loses its two
    //   nominal frames `;00` and `;01`;
    // - one hour holds 107,892 real frames, one 10-minute block 17,982;
    // - within a block, minute 0 spans counts [0, 1800) and minute k
    //   (1..=9) spans [1798*k, 1798*(k+1)).

    /// Number of frames dropped at minute boundaries strictly before this
    /// label. For virtual labels (`;00`/`;01` of a dropped minute) the
    /// current minute's boundary is not counted.
    fn drops_passed(&self) -> u64 {
        let h = u64::from(self.hours);
        let m = u64::from(self.minutes);
        let mut dropped = 2 * (54 * h + m - m / 10);
        if m % 10 != 0 && self.seconds == 0 && u32::from(self.frames) < 2 {
            // The current minute's boundary has not been crossed yet.
            dropped = dropped.saturating_sub(2);
        }
        dropped
    }

    /// True for the two "virtual" labels of a dropped minute (`;00`/`;01`),
    /// which never occur as real frames.
    fn is_virtual_label(&self) -> bool {
        self.rate.is_drop_frame()
            && u64::from(self.minutes) % 10 != 0
            && self.seconds == 0
            && u32::from(self.frames) < 2
    }

    /// Total frame count since timecode zero, skipping dropped frames.
    ///
    /// Dropped (virtual) labels normalize forward to the count of the next
    /// real frame (`;02` of the same minute).
    pub fn to_frames(&self) -> u64 {
        let fps = u64::from(self.rate.fps());
        let nominal =
            (u64::from(self.hours) * 3600 + u64::from(self.minutes) * 60 + u64::from(self.seconds))
                * fps
                + u64::from(self.frames);
        if self.rate.is_drop_frame() {
            if self.is_virtual_label() {
                // Normalize to the count of the next real frame, ;02.
                let h = u64::from(self.hours);
                let m = u64::from(self.minutes);
                return h * 107_892 + 1798 * m + 2;
            }
            nominal - self.drops_passed()
        } else {
            nominal
        }
    }

    /// Builds a timecode from a total frame count.
    ///
    /// For drop-frame rates the result is always a *real* (existing) frame
    /// label; dropped labels are never produced.
    pub fn from_frames(frames: u64, rate: FrameRate) -> Self {
        let fps = u64::from(rate.fps());
        if !rate.is_drop_frame() {
            let (h, m, s, f) = Self::split_nominal(frames, fps);
            return Self {
                hours: h.min(23) as u8,
                minutes: m as u8,
                seconds: s as u8,
                frames: f as u8,
                rate,
            };
        }
        let hours = frames / 107_892;
        let block_and_rest = frames % 107_892;
        let block = block_and_rest / 17_982;
        let rem = block_and_rest % 17_982;
        // Within a 10-minute block: minute 0 keeps all frames; minutes
        // 1..=9 hold 1798 real frames each, starting at nominal index 2.
        let (minute, nominal_index) = if rem < 1800 {
            (0u64, rem)
        } else {
            let k = (rem - 1800) / 1798;
            let within = (rem - 1800) % 1798;
            (k + 1, within + 2)
        };
        Self {
            hours: hours.min(23) as u8,
            minutes: (block * 10 + minute) as u8,
            seconds: (nominal_index / fps) as u8,
            frames: (nominal_index % fps) as u8,
            rate,
        }
    }

    /// Duration elapsed since timecode zero (at the nominal rate).
    pub fn to_duration(&self) -> Duration {
        let micros = self.to_frames() as f64 * 1_000_000.0 / f64::from(self.rate.fps());
        Duration::from_micros(micros as u64)
    }
}

impl fmt::Display for Timecode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sep = if self.rate.is_drop_frame() { ';' } else { ':' };
        write!(
            f,
            "{:02}:{:02}:{:02}{}{:02}",
            self.hours, self.minutes, self.seconds, sep, self.frames
        )
    }
}

/// A timestamp in nanoseconds since the Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Timestamp(u64);

impl Timestamp {
    /// A timestamp of zero (Unix epoch).
    pub const ZERO: Timestamp = Timestamp(0);

    /// The special OSC "immediate" NTP time tag value.
    pub const NTP_IMMEDIATE: u64 = 1;

    /// Current system time.
    pub fn now() -> Self {
        let d = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        Timestamp(d.as_nanos() as u64)
    }

    /// From nanoseconds since the Unix epoch.
    pub const fn from_nanos(nanos: u64) -> Self {
        Timestamp(nanos)
    }

    /// From a `SystemTime`.
    pub fn from_system_time(t: SystemTime) -> Self {
        t.duration_since(UNIX_EPOCH)
            .map(|d| Timestamp(d.as_nanos() as u64))
            .unwrap_or(Timestamp::ZERO)
    }

    /// Nanoseconds since the Unix epoch.
    pub const fn as_nanos(self) -> u64 {
        self.0
    }

    /// Seconds (fractional) since the Unix epoch.
    pub fn as_secs_f64(self) -> f64 {
        self.0 as f64 / 1e9
    }

    /// Converts to a 64-bit NTP timestamp (seconds:fraction), as used by
    /// OSC time tags.
    pub fn to_ntp(self) -> u64 {
        let secs = self.0 / 1_000_000_000 + NTP_UNIX_EPOCH_OFFSET_SECS;
        let nanos = self.0 % 1_000_000_000;
        let frac = ((nanos as u128) << 32) / 1_000_000_000u128;
        (secs << 32) | (frac as u64 & 0xFFFF_FFFF)
    }

    /// Builds from a 64-bit NTP timestamp.
    pub fn from_ntp(ntp: u64) -> Self {
        let secs = (ntp >> 32).wrapping_sub(NTP_UNIX_EPOCH_OFFSET_SECS);
        let frac = ntp & 0xFFFF_FFFF;
        let nanos = (u128::from(frac) * 1_000_000_000u128) >> 32;
        Timestamp((u128::from(secs) * 1_000_000_000u128 + nanos) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timecode_validation() {
        assert!(Timecode::new(0, 0, 0, 24, FrameRate::Fps24).is_none());
        assert!(Timecode::new(24, 0, 0, 0, FrameRate::Fps24).is_none());
        assert!(Timecode::new(23, 59, 59, 23, FrameRate::Fps24).is_some());
    }

    #[test]
    fn timecode_frame_roundtrip() {
        for rate in FrameRate::ALL {
            let tc = Timecode::new(1, 2, 3, 4, rate).unwrap();
            let frames = tc.to_frames();
            let back = Timecode::from_frames(frames, rate);
            assert_eq!(back, tc, "roundtrip failed for {rate:?}");
        }
    }

    #[test]
    fn drop_frame_counting_matches_smpte() {
        let rate = FrameRate::Fps2997Df;
        // First minute holds 1798 real frames: 00:00:59;29 is count 1799,
        // and 00:01:00;02 (the first real frame of minute 1) is 1800.
        assert_eq!(Timecode::new(0, 0, 59, 29, rate).unwrap().to_frames(), 1799);
        assert_eq!(Timecode::new(0, 1, 0, 2, rate).unwrap().to_frames(), 1800);
        // Virtual labels normalize to the next real frame.
        assert_eq!(Timecode::new(0, 1, 0, 0, rate).unwrap().to_frames(), 1800);
        assert_eq!(Timecode::new(0, 1, 0, 1, rate).unwrap().to_frames(), 1800);
        // Minute ten keeps its frames: 00:10:00;00 = 17,982.
        assert_eq!(
            Timecode::new(0, 10, 0, 0, rate).unwrap().to_frames(),
            17_982
        );
        // One hour = 107,892 real frames.
        assert_eq!(
            Timecode::new(1, 0, 0, 0, rate).unwrap().to_frames(),
            107_892
        );
    }

    #[test]
    fn drop_frame_inverse_is_total() {
        let rate = FrameRate::Fps2997Df;
        // Every real frame count maps back onto itself.
        for frames in (0..250_000).step_by(7) {
            let tc = Timecode::from_frames(frames, rate);
            assert_eq!(tc.to_frames(), frames, "inverse failed at {frames}");
        }
        // Checkpoints.
        for frames in [
            0u64, 1797, 1798, 1799, 1800, 3597, 3598, 17_981, 17_982, 107_891,
        ] {
            let tc = Timecode::from_frames(frames, rate);
            assert_eq!(tc.to_frames(), frames, "inverse failed at {frames}");
        }
        // Dropped labels are never produced.
        assert!(Timecode::from_frames(1800, rate) != Timecode::new(0, 1, 0, 0, rate).unwrap());
        assert_eq!(
            Timecode::from_frames(1800, rate),
            Timecode::new(0, 1, 0, 2, rate).unwrap()
        );
    }

    #[test]
    fn display_format() {
        let tc = Timecode::new(1, 2, 3, 4, FrameRate::Fps24).unwrap();
        assert_eq!(tc.to_string(), "01:02:03:04");
        let tc = Timecode::new(0, 0, 5, 20, FrameRate::Fps2997Df).unwrap();
        assert_eq!(tc.to_string(), "00:00:05;20");
    }

    #[test]
    fn ntp_roundtrip() {
        let t = Timestamp::from_nanos(1_700_000_123_456_789_012);
        let ntp = t.to_ntp();
        let back = Timestamp::from_ntp(ntp);
        // NTP has ~0.23 ns resolution; allow small rounding.
        assert!((t.as_nanos() as i64 - back.as_nanos() as i64).abs() < 2);
    }

    #[test]
    fn ntp_immediate_is_special() {
        assert_eq!(Timestamp::NTP_IMMEDIATE, 1);
    }
}
