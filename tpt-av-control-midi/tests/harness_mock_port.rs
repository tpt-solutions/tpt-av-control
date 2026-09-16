//! Hardware-independent MIDI pipelines wired to the shared `tpt-av-test`
//! mock crate.
//!
//! `MockMidiDevice` plays the role of the physical controller: messages are
//! injected into the virtual port and drained by the code under test, so
//! the parse layer is exercised in CI without a single piece of hardware.

use tpt_av_control_midi::midi1::{parse_midi1, Midi1Message};
use tpt_av_test_mock::midi_port::{MidiMessage, MockMidiDevice};

/// Feeds raw wire bytes through the virtual port and parses whatever the
/// port delivers.
fn parse_next(device: &MockMidiDevice) -> Result<Midi1Message, String> {
    let received = device
        .try_recv()
        .map_err(|err| format!("port error: {err}"))?
        .expect("a message was queued");
    parse_midi1(&received.bytes).map_err(|err| err.to_string())
}

#[test]
fn injected_note_on_round_trips_through_the_virtual_port() {
    let device = MockMidiDevice::new();

    // Build the canonical message, serialize it to wire bytes, and play it
    // "on the cable".
    let played = Midi1Message::NoteOn {
        channel: 3,
        note: 60,
        velocity: 100,
    };
    device.inject(MidiMessage::new(played.to_bytes()));

    assert_eq!(parse_next(&device), Ok(played));
    assert_eq!(device.pending_count(), 0, "port fully drained");
}

#[test]
fn injected_control_change_and_note_off_parse_in_order() {
    let device = MockMidiDevice::new();
    let messages = [
        Midi1Message::ControlChange {
            channel: 0,
            controller: 7,
            value: 127,
        },
        Midi1Message::NoteOff {
            channel: 3,
            note: 60,
            velocity: 64,
        },
    ];
    device.inject_all(messages.iter().map(|message| MidiMessage::new(message.to_bytes())));

    // FIFO order is preserved by the virtual port.
    assert_eq!(parse_next(&device), Ok(messages[0].clone()));
    assert_eq!(parse_next(&device), Ok(messages[1].clone()));
    assert_eq!(device.try_recv().unwrap(), None);
}

#[test]
fn real_world_gesture_stream_parses_without_hardware() {
    let device = MockMidiDevice::new();

    // A small performance gesture: sustain pedal down, chord, pedal up.
    let gesture = [
        Midi1Message::ControlChange { channel: 0, controller: 64, value: 127 },
        Midi1Message::NoteOn { channel: 0, note: 60, velocity: 96 },
        Midi1Message::NoteOn { channel: 0, note: 64, velocity: 96 },
        Midi1Message::NoteOn { channel: 0, note: 67, velocity: 96 },
        Midi1Message::ControlChange { channel: 0, controller: 64, value: 0 },
    ];
    device.inject_all(gesture.iter().map(|message| MidiMessage::new(message.to_bytes())));

    for expected in &gesture {
        assert_eq!(
            parse_next(&device).map_err(|err| err.clone()),
            Ok(expected.clone())
        );
    }
}
