//! MIDI Time Code (MTC): quarter-frame packing and full-frame assembly.

use crate::midi1::Midi1Message;
use tpt_av_control_utils::time::{FrameRate, Timecode};

/// Universal Real-Time MTC sub-ID: Full/Frame message.
pub const MTC_FULL_FRAME: u8 = 0x01;
/// Universal Real-Time MTC sub-ID: User bits.
pub const MTC_USER_BITS: u8 = 0x02;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The MTC frame-rate encoding (2 bits, per the SMPTE/MIDI spec).
pub enum MtcFrameRate {
    /// 24 fps (code 0).
    Fps24,
    /// 25 fps (code 1).
    Fps25,
    /// 30 fps drop-frame (code 2).
    Fps2997Df,
    /// 30 fps non-drop (code 3).
    Fps30Ndf,
}

impl MtcFrameRate {
    /// From the 2-bit MTC rate field.
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0x3 {
            0 => MtcFrameRate::Fps24,
            1 => MtcFrameRate::Fps25,
            2 => MtcFrameRate::Fps2997Df,
            _ => MtcFrameRate::Fps30Ndf,
        }
    }

    /// To the 2-bit MTC rate field.
    pub fn to_bits(self) -> u8 {
        match self {
            MtcFrameRate::Fps24 => 0,
            MtcFrameRate::Fps25 => 1,
            MtcFrameRate::Fps2997Df => 2,
            MtcFrameRate::Fps30Ndf => 3,
        }
    }

    /// The corresponding [`FrameRate`].
    pub fn frame_rate(self) -> FrameRate {
        match self {
            MtcFrameRate::Fps24 => FrameRate::Fps24,
            MtcFrameRate::Fps25 => FrameRate::Fps25,
            MtcFrameRate::Fps2997Df => FrameRate::Fps2997Df,
            MtcFrameRate::Fps30Ndf => FrameRate::Fps30Ndf,
        }
    }
}

/// Encodes a timecode into the eight MTC quarter-frame messages.
///
/// Quarter frames carry one nibble each, in order:
pub fn quarter_frames(timecode: &Timecode) -> Vec<Midi1Message> {
    let rate_bits = match timecode.rate {
        FrameRate::Fps24 => MtcFrameRate::Fps24.to_bits(),
        FrameRate::Fps25 => MtcFrameRate::Fps25.to_bits(),
        FrameRate::Fps2997Df => MtcFrameRate::Fps2997Df.to_bits(),
        FrameRate::Fps30Ndf => MtcFrameRate::Fps30Ndf.to_bits(),
    };
    let h = u16::from(timecode.hours);
    let nibbles: [u8; 8] = [
        timecode.frames & 0x0F,
        (timecode.frames >> 4) & 0x0F,
        timecode.seconds & 0x0F,
        (timecode.seconds >> 4) & 0x0F,
        timecode.minutes & 0x0F,
        (timecode.minutes >> 4) & 0x0F,
        (h & 0x0F) as u8,
        (((h >> 4) & 0x1) as u8) | (rate_bits << 1),
    ];
    nibbles
        .into_iter()
        .enumerate()
        .map(|(i, n)| Midi1Message::TimeCodeQuarterFrame(((i as u8) << 4) | (n & 0x0F)))
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq)]
/// Assembles quarter frames into a timecode.
pub struct MtcDecoder {
    nibbles: [Option<u8>; 8],
    /// The last fully-assembled timecode.
    pub timecode: Option<Timecode>,
}

impl MtcDecoder {
    /// A fresh decoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one quarter-frame message (`piece` 0-7 given by the message's
    /// high nibble, data in the low nibble). When all eight pieces have
    /// arrived, the assembled timecode is stored in
    pub fn feed_quarter_frame(&mut self, piece: u8) {
        let index = ((piece >> 4) & 0x7) as usize;
        let data = piece & 0x0F;
        self.nibbles[index] = Some(data);
        if self.nibbles.iter().all(|n| n.is_some()) {
            let n = self.nibbles.map(|n| n.unwrap_or(0));
            let frames = n[0] | (n[1] << 4);
            let seconds = n[2] | (n[3] << 4);
            let minutes = n[4] | (n[5] << 4);
            let hours = n[6] | ((n[7] & 0x1) << 4);
            let rate = MtcFrameRate::from_bits(n[7] >> 1);
            if let Some(tc) = Timecode::new(hours, minutes, seconds, frames, rate.frame_rate()) {
                self.timecode = Some(tc);
            }
            self.nibbles = [None; 8];
        }
    }

    /// Feeds a raw `TimeCodeQuarterFrame` message.
    pub fn feed(&mut self, message: &Midi1Message) {
        if let Midi1Message::TimeCodeQuarterFrame(piece) = message {
            self.feed_quarter_frame(*piece);
        }
    }
}

/// Builds a Universal Real-Time Full Frame SysEx message:
pub fn full_frame_sysex(timecode: &Timecode, device_id: u8) -> Midi1Message {
    let rate = match timecode.rate {
        FrameRate::Fps24 => MtcFrameRate::Fps24,
        FrameRate::Fps25 => MtcFrameRate::Fps25,
        FrameRate::Fps2997Df => MtcFrameRate::Fps2997Df,
        FrameRate::Fps30Ndf => MtcFrameRate::Fps30Ndf,
    };
    let mut payload = vec![(rate.to_bits() << 5) | (timecode.hours & 0x1F)];
    payload.push(timecode.minutes);
    payload.push(timecode.seconds);
    payload.push(timecode.frames);
    crate::sysex::sysex_universal(
        crate::sysex::UNIVERSAL_REALTIME,
        device_id,
        crate::sysex::MIDI_TIME_CODE,
        MTC_FULL_FRAME,
        &payload,
    )
}

/// Parses a Universal Real-Time Full Frame SysEx into a timecode.
pub fn parse_full_frame_sysex(message: &Midi1Message) -> Option<Timecode> {
    let view = crate::sysex::parse_sysex(message).ok()?;
    if view.manufacturer != crate::sysex::UNIVERSAL_REALTIME
        || view.sub_id1 != Some(crate::sysex::MIDI_TIME_CODE)
        || view.sub_id2 != Some(MTC_FULL_FRAME)
        || view.payload.len() < 4
    {
        return None;
    }
    let rate = MtcFrameRate::from_bits(view.payload[0] >> 5);
    Timecode::new(
        view.payload[0] & 0x1F,
        view.payload[1],
        view.payload[2],
        view.payload[3],
        rate.frame_rate(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi1::parse_midi1;

    #[test]
    fn quarter_frames_roundtrip() {
        let tc = Timecode::new(3, 24, 51, 17, FrameRate::Fps25).unwrap();
        let pieces = quarter_frames(&tc);
        assert_eq!(pieces.len(), 8);

        let mut decoder = MtcDecoder::new();
        for piece in &pieces {
            decoder.feed(piece);
        }
        assert_eq!(decoder.timecode, Some(tc));
    }

    #[test]
    fn rate_bits_survive() {
        for rate in FrameRate::ALL {
            let tc = Timecode::new(0x11, 0x22, 0x33, rate.fps() - 1, rate).unwrap();
            let pieces = quarter_frames(&tc);
            let mut decoder = MtcDecoder::new();
            for piece in pieces.iter() {
                decoder.feed(piece);
            }
            assert_eq!(decoder.timecode, Some(tc), "rate {rate:?}");
        }
    }

    #[test]
    fn decoder_restarts_after_complete() {
        let mut decoder = MtcDecoder::new();
        for piece in quarter_frames(&Timecode::new(0, 0, 0, 0, FrameRate::Fps24).unwrap()).iter() {
            decoder.feed(piece);
        }
        assert!(decoder.timecode.is_some());
        assert!(
            decoder.nibbles.iter().all(|n| n.is_none()),
            "nibble buffer resets for the next frame"
        );
    }

    #[test]
    fn full_frame_sysex_roundtrip() {
        let tc = Timecode::new(12, 34, 56, 23, FrameRate::Fps2997Df).unwrap();
        let message = full_frame_sysex(&tc, 0x7F);
        // Check the raw bytes: F0 7F 7F 04 01 <rate|hh> mm ss ff F7.
        match &message {
            Midi1Message::SystemExclusive(bytes) => {
                assert_eq!(
                    bytes,
                    &vec![
                        0xF0,
                        0x7F,
                        0x7F,
                        0x04,
                        0x01,
                        (2 << 5) | 12,
                        34,
                        56,
                        23,
                        0xF7
                    ]
                );
            }
            other => panic!("expected sysex, got {other:?}"),
        }
        let parsed = parse_midi1(&message.to_bytes()).unwrap();
        assert_eq!(parse_full_frame_sysex(&parsed), Some(tc));
    }
}
