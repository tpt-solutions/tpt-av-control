//! MIDI clock and transport (24 ticks per quarter note).

use crate::device::{MidiSink, MidiSource};
use crate::midi1::Midi1Message;
use std::time::Duration;
use tpt_av_control_utils::ControlError;

/// MIDI clock resolution: 24 timing-clock ticks per quarter note.
pub const TICKS_PER_QUARTER_NOTE: u32 = 24;

/// Sends MIDI clock and transport commands at a configured tempo.
pub struct MidiClockSender {
    sink: Box<dyn MidiSink>,
    /// Beats per minute (0 = stopped / manual ticking).
    pub bpm: f64,
}

impl MidiClockSender {
    /// Wraps any [`MidiSink`].
    pub fn new(sink: Box<dyn MidiSink>, bpm: f64) -> Self {
        Self { sink, bpm }
    }

    /// The interval between timing-clock ticks at the current tempo.
    pub fn tick_interval(&self) -> Duration {
        let beats_per_second = self.bpm.max(f64::EPSILON) / 60.0;
        Duration::from_secs_f64(1.0 / (beats_per_second * f64::from(TICKS_PER_QUARTER_NOTE)))
    }

    /// Sends a single timing-clock tick.
    pub fn tick(&mut self) -> Result<(), ControlError> {
        self.sink.send_midi1(&Midi1Message::TimingClock)
    }

    /// Sends Start and positions at zero (song position pointer 0).
    pub fn start(&mut self) -> Result<(), ControlError> {
        self.set_song_position(0)?;
        self.sink.send_midi1(&Midi1Message::Start)
    }

    /// Sends Continue (resume from current position).
    pub fn continue_playback(&mut self) -> Result<(), ControlError> {
        self.sink.send_midi1(&Midi1Message::Continue)
    }

    /// Sends Stop.
    pub fn stop(&mut self) -> Result<(), ControlError> {
        self.sink.send_midi1(&Midi1Message::Stop)
    }

    /// Sends a song position pointer (in 16th notes).
    pub fn set_song_position(&mut self, sixteenth_notes: u16) -> Result<(), ControlError> {
        self.sink.send_midi1(&Midi1Message::SongPositionPointer(
            sixteenth_notes.min(0x3FFF),
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Transport state observed by [`MidiClockReceiver`].
pub enum TransportState {
    /// No transport messages seen yet.
    #[default]
    Unknown,
    /// Start received; playing.
    Playing,
    /// Continue received; playing.
    Continuing,
    /// Stop received; halted.
    Stopped,
}

/// Watches a MIDI stream for clock ticks and transport commands, and
#[derive(Debug, Default)]
pub struct MidiClockReceiver {
    state: TransportState,
    last_tick: Option<std::time::Instant>,
    /// Rolling estimate of beats per minute (None until two ticks arrive).
    bpm_estimate: Option<f64>,
    ticks_seen: u64,
}

impl MidiClockReceiver {
    /// A fresh receiver in [`TransportState::Unknown`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Current transport state.
    pub fn state(&self) -> TransportState {
        self.state
    }

    /// Rolling BPM estimate from observed ticks.
    pub fn bpm_estimate(&self) -> Option<f64> {
        self.bpm_estimate
    }

    /// Total timing-clock ticks observed since creation.
    pub fn ticks_seen(&self) -> u64 {
        self.ticks_seen
    }

    /// Feeds an inbound message, updating transport state and tempo
    pub fn observe(&mut self, message: &Midi1Message) {
        match message {
            Midi1Message::Start => self.state = TransportState::Playing,
            Midi1Message::Continue => self.state = TransportState::Continuing,
            Midi1Message::Stop => self.state = TransportState::Stopped,
            Midi1Message::TimingClock => {
                self.ticks_seen += 1;
                let now = std::time::Instant::now();
                if let Some(last) = self.last_tick {
                    let interval = now.duration_since(last);
                    if interval > Duration::ZERO {
                        let instant_bpm =
                            60.0 / (interval.as_secs_f64() * f64::from(TICKS_PER_QUARTER_NOTE));
                        // Exponential smoothing keeps the estimate stable
                        // against jitter while tracking tempo changes.
                        self.bpm_estimate = Some(match self.bpm_estimate {
                            Some(prev) => prev * 0.75 + instant_bpm * 0.25,
                            None => instant_bpm,
                        });
                    }
                }
                self.last_tick = Some(now);
            }
            _ => {}
        }
    }

    /// Drains all pending messages from `source`, observing each.
    pub fn observe_all(&mut self, source: &mut dyn MidiSource) -> Result<usize, ControlError> {
        let mut count = 0;
        while let Some(inbound) = source.try_recv()? {
            if let Some(message) = inbound.message {
                self.observe(&message);
            }
            count += 1;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::VirtualMidiPair;

    #[test]
    fn tick_interval_matches_bpm() {
        let pair = VirtualMidiPair::new();
        let (_source, sink) = pair.split();
        let sender = MidiClockSender::new(Box::new(sink), 120.0);
        // 120 BPM → 0.5 s per quarter → 1/48 s per tick.
        assert!((sender.tick_interval().as_secs_f64() - 1.0 / 48.0).abs() < 1e-9);

        let pair = VirtualMidiPair::new();
        let (_source, sink) = pair.split();
        let sender = MidiClockSender::new(Box::new(sink), 60.0);
        assert!((sender.tick_interval().as_secs_f64() - 1.0 / 24.0).abs() < 1e-9);
    }

    #[test]
    fn transport_commands_are_sent() {
        let pair = VirtualMidiPair::new();
        let (mut source, sink) = pair.split();
        let mut sender = MidiClockSender::new(Box::new(sink), 100.0);
        sender.start().unwrap();
        sender.tick().unwrap();
        sender.stop().unwrap();

        assert_eq!(
            source.recv().unwrap().message,
            Some(Midi1Message::SongPositionPointer(0))
        );
        assert_eq!(source.recv().unwrap().message, Some(Midi1Message::Start));
        assert_eq!(
            source.recv().unwrap().message,
            Some(Midi1Message::TimingClock)
        );
        assert_eq!(source.recv().unwrap().message, Some(Midi1Message::Stop));
    }

    #[test]
    fn receiver_tracks_state_and_tempo() {
        let mut rx = MidiClockReceiver::new();
        assert_eq!(rx.state(), TransportState::Unknown);
        rx.observe(&Midi1Message::Start);
        assert_eq!(rx.state(), TransportState::Playing);

        // Feed ticks at a fixed spacing by observing with injected instants:
        // the receiver uses Instant::now(), so we assert only monotonic
        // behavior here; exact tempo tests would need clock injection.
        rx.observe(&Midi1Message::TimingClock);
        std::thread::sleep(Duration::from_millis(8));
        rx.observe(&Midi1Message::TimingClock);
        assert_eq!(rx.ticks_seen(), 2);
        assert!(rx.bpm_estimate().is_some());

        rx.observe(&Midi1Message::Stop);
        assert_eq!(rx.state(), TransportState::Stopped);
        // Non-clock messages are ignored.
        rx.observe(&Midi1Message::NoteOn {
            channel: 0,
            note: 60,
            velocity: 1,
        });
        assert_eq!(rx.state(), TransportState::Stopped);
    }
}
