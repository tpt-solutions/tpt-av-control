//! Integration test against real MIDI hardware.
//!
//! Skipped unless both `TPT_MIDI_IN_PORT` and `TPT_MIDI_OUT_PORT` are set
//! to port indices (as listed by `enumerate_devices`). With a hardware
//! loopback cable (or a single interface that echoes), this round-trips a
//! note-on through the physical device.

use std::time::Duration;
use tpt_av_control_midi::{
    enumerate_devices, open_input, open_output, Midi1Message, MidiPort, MidiSource, PortDirection,
};

#[test]
fn hardware_roundtrip() {
    let Ok(in_index) = std::env::var("TPT_MIDI_IN_PORT") else {
        eprintln!("skipping: TPT_MIDI_IN_PORT not set");
        return;
    };
    let Ok(out_index) = std::env::var("TPT_MIDI_OUT_PORT") else {
        eprintln!("skipping: TPT_MIDI_OUT_PORT not set");
        return;
    };
    let in_index: usize = in_index.parse().expect("TPT_MIDI_IN_PORT must be an index");
    let out_index: usize = out_index
        .parse()
        .expect("TPT_MIDI_OUT_PORT must be an index");

    let devices = enumerate_devices().expect("device enumeration should not fail");
    for device in &devices {
        println!(
            "device {:?}: {} input(s), {} output(s)",
            device.name,
            device.inputs.len(),
            device.outputs.len()
        );
    }

    let in_port = MidiPort::new(
        tpt_av_control_midi::PortId::new(in_index),
        "hw in".to_string(),
        PortDirection::Input,
    );
    let out_port = MidiPort::new(
        tpt_av_control_midi::PortId::new(out_index),
        "hw out".to_string(),
        PortDirection::Output,
    );

    let mut input = open_input(&in_port).expect("open input");
    let mut output = open_output(&out_port).expect("open output");

    let note = Midi1Message::NoteOn {
        channel: 0,
        note: 60,
        velocity: 100,
    };
    output.send_midi1(&note).expect("send note on");

    let received = input
        .recv_timeout(Duration::from_secs(5))
        .expect("recv")
        .expect("timed out waiting for echo");
    assert_eq!(
        received.message,
        Some(Midi1Message::NoteOn {
            channel: 0,
            note: 60,
            velocity: 100
        })
    );
}
