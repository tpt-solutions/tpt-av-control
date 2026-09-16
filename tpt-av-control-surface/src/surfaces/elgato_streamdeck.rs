//! Elgato Stream Deck support: the USB HID packet protocol implemented in
//! pure Rust over a pluggable [`HidDevice`] transport.
//!
//! No C bindings are required to *speak* the protocol; an application
//! supplies the transport by implementing [`HidDevice`] (e.g. over
//! `hidapi`, WebHID, or a test double). Packet framing below follows the
//! publicly documented Stream Deck protocol:
//!
//! - Key states arrive as input reports `01 <key states...>` (one byte
//!   per key, `01` = down) on Mini/V2-family devices.
//! - Images are sent as a header packet followed by fixed-size payload
//!   chunks; V2 family uses reports `0x07`/`0x05`, the original uses
//!   `0x02`/`0x02` with BMP images.
//! - Brightness is a HID feature/class report `0x03` (original) or
//!   `0x05` (V2 family).

use crate::feedback::Feedback;
use crate::surface::{ControlEvent, ControlSurface};
use std::time::Duration;
use tpt_av_control_utils::ControlError;

/// A bidirectional HID transport for one Stream Deck device.
pub trait HidDevice: Send {
    /// Reads an input report (blocking) into `buf` (including the report
    /// id as the first byte). Returns the number of bytes read.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;

    /// Reads an input report with a timeout. `Ok(0)` = timed out.
    fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> std::io::Result<usize>;

    /// Writes an output report (`data` includes the report id).
    fn write(&mut self, data: &[u8]) -> std::io::Result<()>;

    /// Writes a feature report (includes the report id).
    fn send_feature_report(&mut self, data: &[u8]) -> std::io::Result<()>;
}

/// A Stream Deck hardware generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckModel {
    /// Original Stream Deck (5×3, 72×72 BMP images).
    Original,
    /// Stream Deck Mini (3×2, 80×80 JPEG images, V2 framing).
    Mini,
    /// Stream Deck V2 / Mk.2 (5×3, 72×72 JPEG images, V2 framing).
    V2,
    /// Stream Deck XL (8×4, 96×96 JPEG images, V2 framing).
    Xl,
}

impl DeckModel {
    /// Key grid rows.
    pub fn rows(self) -> usize {
        match self {
            DeckModel::Original | DeckModel::V2 => 3,
            DeckModel::Mini => 2,
            DeckModel::Xl => 4,
        }
    }

    /// Key grid columns.
    pub fn cols(self) -> usize {
        match self {
            DeckModel::Original | DeckModel::V2 => 5,
            DeckModel::Mini => 3,
            DeckModel::Xl => 8,
        }
    }

    /// Total key count.
    pub fn key_count(self) -> usize {
        self.rows() * self.cols()
    }

    /// Per-key image pixel size (images are square).
    pub fn image_size(self) -> usize {
        match self {
            DeckModel::Original => 72,
            DeckModel::Mini => 80,
            DeckModel::V2 => 72,
            DeckModel::Xl => 96,
        }
    }

    /// Payload chunk size for image data.
    fn chunk_size(self) -> usize {
        match self {
            DeckModel::Original => 8191,
            DeckModel::Mini | DeckModel::V2 => 102,
            DeckModel::Xl => 102,
        }
    }

    /// Whether the device uses V2-style packet framing.
    fn is_v2(self) -> bool {
        !matches!(self, DeckModel::Original)
    }
}

/// A Stream Deck control surface.
///
/// Key images must already be encoded in the model's image format (the
/// protocol framing is handled here; JPEG/BMP encoding is the caller's
/// choice).
pub struct StreamDeckSurface {
    model: DeckModel,
    device: Box<dyn HidDevice>,
    /// Last-known key states, for edge detection.
    states: Vec<bool>,
    name: String,
}

impl StreamDeckSurface {
    /// Wraps a connected HID device of the given model.
    pub fn new(model: DeckModel, device: Box<dyn HidDevice>) -> Self {
        Self {
            model,
            states: vec![false; model.key_count()],
            device,
            name: format!("Stream Deck {model:?}"),
        }
    }

    /// The device model.
    pub fn model(&self) -> DeckModel {
        self.model
    }

    /// Sets panel brightness (0-100%).
    pub fn set_brightness(&mut self, percent: u8) -> Result<(), ControlError> {
        let percent = percent.min(100);
        let report: &[u8] = if self.model.is_v2() {
            &[0x05, percent]
        } else {
            &[0x03, percent]
        };
        self.device
            .send_feature_report(report)
            .map_err(hid_error("set brightness"))?;
        Ok(())
    }

    /// Builds the packet sequence that draws `image` (already encoded
    /// JPEG/BMP bytes) on `key`, and writes it to the device.
    pub fn set_key_image(&mut self, key: u8, image: &[u8]) -> Result<(), ControlError> {
        if usize::from(key) >= self.model.key_count() {
            return Err(ControlError::OutOfRange {
                value: i64::from(key),
                min: 0,
                max: (self.model.key_count() - 1) as i64,
            });
        }
        let packets = build_image_packets(self.model, key, image)?;
        for packet in &packets {
            self.device
                .write(packet)
                .map_err(hid_error("write image packet"))?;
        }
        Ok(())
    }

    /// Polls for one button-state report and returns press/release events.
    /// `Ok(None)` when no report arrived within `timeout`.
    pub fn poll_key_states(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<ControlEvent>, ControlError> {
        let mut buf = vec![0u8; 1 + self.model.key_count() + 8];
        let len = self
            .device
            .read_timeout(&mut buf, timeout)
            .map_err(hid_error("read key states"))?;
        if len == 0 {
            return Ok(None);
        }
        if buf[0] != 0x01 {
            return Ok(None); // not a key-state report
        }
        let states = &buf[1..1 + self.model.key_count()];
        for (key, (&state, prev)) in states.iter().zip(self.states.iter_mut()).enumerate() {
            let down = state == 0x01;
            if down != *prev {
                *prev = down;
                let event = if down {
                    ControlEvent::ButtonPress { button: key as u8 }
                } else {
                    ControlEvent::ButtonRelease { button: key as u8 }
                };
                return Ok(Some(event));
            }
        }
        Ok(None)
    }
}

/// Builds the framed packets for one key image.
pub fn build_image_packets(
    model: DeckModel,
    key: u8,
    image: &[u8],
) -> Result<Vec<Vec<u8>>, ControlError> {
    if usize::from(key) >= model.key_count() {
        return Err(ControlError::OutOfRange {
            value: i64::from(key),
            min: 0,
            max: (model.key_count() - 1) as i64,
        });
    }
    let chunk = model.chunk_size();
    let mut packets = Vec::new();
    if model.is_v2() {
        let total_chunks = image.len().div_ceil(chunk).max(1);
        // Header: 07 01 03 00 <key> <remaining bytes hi> <lo> ...
        let remaining = image.len() as u16;
        packets.push(vec![
            0x07,
            0x01,
            0x03,
            0x00,
            key,
            (remaining >> 8) as u8,
            (remaining & 0xFF) as u8,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ]);
        for (index, chunk_data) in image.chunks(chunk).enumerate() {
            let last = index + 1 == total_chunks;
            let mut packet = vec![0x05, 0x01, 0x03, 0x00, key, u8::from(last), 0x00, 0x00];
            packet.extend_from_slice(chunk_data);
            packets.push(packet);
        }
    } else {
        // Original: a single header packet, then payload packets of the
        // remaining image bytes each (report 0x02).
        let mut header = vec![0x02, 0x08, key & 0x1F, 0x00, 0x00, 0x00];
        header.extend_from_slice(&(image.len() as u16 + 6).to_le_bytes());
        header.extend_from_slice(&0x0001u16.to_le_bytes());
        packets.push(header);
        for (index, chunk_data) in image.chunks(chunk).enumerate() {
            let last = index + 1 == image.len().div_ceil(chunk).max(1);
            let mut packet = vec![0x02, 0x01, u8::from(last), 0x00];
            packet.extend_from_slice(chunk_data);
            packets.push(packet);
        }
    }
    Ok(packets)
}

fn hid_error(action: &str) -> impl Fn(std::io::Error) -> ControlError + '_ {
    move |e| ControlError::Io(std::io::Error::other(format!("{action}: {e}")))
}

impl ControlSurface for StreamDeckSurface {
    fn name(&self) -> &str {
        &self.name
    }

    fn init(&mut self) -> Result<(), ControlError> {
        self.set_brightness(80)
    }

    fn read_event(&mut self) -> Result<Option<ControlEvent>, ControlError> {
        self.poll_key_states(Duration::from_millis(0))
    }

    fn send_feedback(&mut self, feedback: &Feedback) -> Result<(), ControlError> {
        match feedback {
            Feedback::LedColor { led, r, g, b } => {
                // A solid color image: one pixel of the requested RGB per
                // key at the model's resolution (JPEG encoding is the
                // caller's job; here we accept raw pixel bytes and frame
                // them). For a raw 1×1 pixel the deck tiles it.
                let mut image = Vec::with_capacity(3);
                image.extend_from_slice(&[*r, *g, *b]);
                self.set_key_image(*led, &image)
            }
            Feedback::Led { led, on } => {
                let pixel = if *on { [255u8, 255, 255] } else { [0u8, 0, 0] };
                self.set_key_image(*led, &pixel)
            }
            Feedback::DisplayText { .. } => Err(ControlError::Unsupported(
                "draw text into a key image and use LedColor instead".into(),
            )),
            Feedback::FaderPosition { .. } => Err(ControlError::Unsupported(
                "Stream Decks have no motorized faders".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// In-memory HID transport capturing writes and replaying scripted
    /// reads — keeps the protocol tests hardware-free.
    #[derive(Default, Clone)]
    struct FakeHid {
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
        features: Arc<Mutex<Vec<Vec<u8>>>>,
        script: Arc<Mutex<VecDeque<Vec<u8>>>>,
    }
    use std::collections::VecDeque;

    impl HidDevice for FakeHid {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.read_timeout(buf, Duration::ZERO)
        }

        fn read_timeout(&mut self, buf: &mut [u8], _t: Duration) -> std::io::Result<usize> {
            let mut script = self.script.lock().unwrap();
            if let Some(report) = script.pop_front() {
                let n = report.len().min(buf.len());
                buf[..n].copy_from_slice(&report[..n]);
                Ok(n)
            } else {
                Ok(0)
            }
        }

        fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
            self.writes.lock().unwrap().push(data.to_vec());
            Ok(())
        }

        fn send_feature_report(&mut self, data: &[u8]) -> std::io::Result<()> {
            self.features.lock().unwrap().push(data.to_vec());
            Ok(())
        }
    }

    #[test]
    fn image_framing_v2() {
        let model = DeckModel::V2;
        let image = vec![0xABu8; 250]; // three chunks: 102, 102, 46
        let packets = build_image_packets(model, 4, &image).unwrap();
        assert_eq!(packets.len(), 4);
        // Header.
        assert_eq!(packets[0][0], 0x07);
        assert_eq!(packets[0][4], 4); // key
        assert_eq!(&packets[0][5..7], &[0x00, 250]); // total length BE
                                                     // First payload chunk.
        assert_eq!(packets[1][0], 0x05);
        assert_eq!(packets[1][4], 4);
        assert_eq!(packets[1][5], 0); // not last
        assert_eq!(&packets[1][8..9], &[0xAB]);
        // Last chunk flags "last".
        assert_eq!(packets[3][5], 1);
    }

    #[test]
    fn image_framing_original() {
        let model = DeckModel::Original;
        let image = vec![1u8; 100];
        let packets = build_image_packets(model, 14, &image).unwrap();
        assert_eq!(packets[0][0], 0x02);
        assert_eq!(packets[0][2], 14); // key
    }

    #[test]
    fn key_range_validated() {
        let model = DeckModel::Mini; // 6 keys
        assert!(build_image_packets(model, 6, &[1, 2, 3]).is_err());
        assert!(build_image_packets(model, 5, &[1, 2, 3]).is_ok());
    }

    #[test]
    fn button_press_and_release_edges() {
        let deck = FakeHid::default();
        // Script: all-up, then key 2 down, then a repeat of key 2 down
        // (no edge), then key 2 up.
        let mut down = vec![0x01u8, 0, 0, 0, 0, 0, 0];
        down[3] = 0x01; // key 2 (report id at 0, keys from 1)
        let up = vec![0x01u8, 0, 0, 0, 0, 0, 0];
        deck.script.lock().unwrap().push_back(down.clone());
        deck.script.lock().unwrap().push_back(down);
        deck.script.lock().unwrap().push_back(up);

        let mut surface = StreamDeckSurface::new(DeckModel::Mini, Box::new(deck));
        assert_eq!(
            surface.read_event().unwrap(),
            Some(ControlEvent::ButtonPress { button: 2 })
        );
        // Repeat report produces no edge; poll returns None.
        assert_eq!(surface.read_event().unwrap(), None);
        assert_eq!(
            surface.read_event().unwrap(),
            Some(ControlEvent::ButtonRelease { button: 2 })
        );
    }

    #[test]
    fn brightness_report_selects_generation() {
        let deck = FakeHid::default();
        let mut surface = StreamDeckSurface::new(DeckModel::V2, Box::new(deck.clone()));
        surface.set_brightness(50).unwrap();
        assert_eq!(
            deck.features.lock().unwrap().last().unwrap(),
            &vec![0x05, 50]
        );

        let deck = FakeHid::default();
        let mut surface = StreamDeckSurface::new(DeckModel::Original, Box::new(deck.clone()));
        surface.set_brightness(100).unwrap();
        assert_eq!(
            deck.features.lock().unwrap().last().unwrap(),
            &vec![0x03, 100]
        );
    }
}
