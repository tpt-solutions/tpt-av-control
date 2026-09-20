//! MIDI quickstart: opens the first available MIDI input and prints every
//! incoming note in plain English until Ctrl+C. Needs a real MIDI device
//! (or virtual port) connected — run `cargo run -p tpt-av-control-examples
//! --bin midi_controller` instead for a hardware-free CC-mapping demo.
//!
//! ```sh
//! cargo run -p tpt-av-control-examples --bin hello_midi
//! ```

use tpt_av_control_midi::{enumerate_devices, open_input, Midi1Message};

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

fn note_name(note: u8) -> String {
    let octave = i32::from(note) / 12 - 1;
    format!("{}{octave}", NOTE_NAMES[usize::from(note) % 12])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = enumerate_devices()?;
    let device = devices
        .iter()
        .find(|d| !d.inputs.is_empty())
        .ok_or("no MIDI input devices found — connect a controller (or a virtual port) first")?;
    let port = &device.inputs[0];
    println!(
        "listening on {:?} ({}); Ctrl-C to quit",
        device.name, port.name
    );

    let mut input = open_input(port)?;
    loop {
        let inbound = input.recv()?;
        match inbound.message {
            Some(Midi1Message::NoteOn {
                channel,
                note,
                velocity,
            }) if velocity > 0 => {
                println!(
                    "Note On:  {} (ch {channel}, velocity {velocity})",
                    note_name(note)
                );
            }
            Some(Midi1Message::NoteOn {
                channel,
                note,
                velocity: 0,
            })
            | Some(Midi1Message::NoteOff { channel, note, .. }) => {
                println!("Note Off: {} (ch {channel})", note_name(note));
            }
            Some(other) => println!("{other:?}"),
            None => println!("(unparsed) {:02x?}", inbound.raw),
        }
    }
}
