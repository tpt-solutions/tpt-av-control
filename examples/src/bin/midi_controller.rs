//! MIDI controller demo: lists devices, opens the first one (or the port
//! index given as argv[1]), and prints mapped control events.
//!
//! ```sh
//! cargo run -p tpt-av-control-examples --example midi_controller [in-port-index]
//! ```

use std::time::Duration;
use tpt_av_control_midi::{
    enumerate_devices, open_input, open_output, MidiParameterMapper, MidiPort, PortDirection,
    PortId,
};
use tpt_av_control_utils::parameter::{Mapping, ParameterId};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let devices = enumerate_devices()?;
    println!("MIDI devices:");
    for (i, device) in devices.iter().enumerate() {
        println!(
            "  [{i}] {:?} ({} in / {} out)",
            device.name,
            device.inputs.len(),
            device.outputs.len()
        );
    }

    let in_index: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(0);

    let input = open_input(&MidiPort::new(
        PortId::new(in_index),
        format!("input {in_index}"),
        PortDirection::Input,
    ))?;
    let mut output = open_output(&MidiPort::new(
        PortId::new(in_index),
        format!("output {in_index}"),
        PortDirection::Output,
    ))
    .ok();

    println!("listening on input {in_index}; Ctrl-C to quit");

    // Map CC 7 to a master volume parameter and CC 1 to vibrato depth.
    let mut mapper = MidiParameterMapper::new();
    mapper.bind_cc(tpt_av_control_midi::CcBinding {
        controller: 7,
        parameter: ParameterId::new("master.volume"),
        mapping: Mapping::linear((0.0, 127.0), (0.0, 1.0)),
    });
    mapper.bind_cc(tpt_av_control_midi::CcBinding {
        controller: 1,
        parameter: ParameterId::new("synth.vibrato"),
        mapping: Mapping::linear((0.0, 127.0), (0.0, 1.0)),
    });

    let mut input = input;
    loop {
        match input.recv() {
            Ok(inbound) => {
                if let Some(message) = &inbound.message {
                    for mapped in mapper.apply(message) {
                        println!("  → {} = {:?}", mapped.parameter, mapped.value);
                    }
                }
                // Echo a note-off LED cue back when a note arrives.
                if let Some(out) = output.as_mut() {
                    if let Some(message) = &inbound.message {
                        let _ = out.send_midi1(message);
                    }
                }
            }
            Err(e) => {
                eprintln!("input closed: {e}");
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }
}
