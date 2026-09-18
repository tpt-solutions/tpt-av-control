//! Control surface demo: a virtual surface over an in-memory MIDI
//! loopback, plus a live generic-MIDI surface when hardware is present.
//!
//! ```sh
//! cargo run -p tpt-av-control-examples --bin control_surface
//! ```

use std::time::Duration;
use tpt_av_control_midi::{Midi1Message, MidiSink, MidiSource, VirtualMidiPair};
use tpt_av_control_surface::{
    ControlEvent, ControlSurface, CustomSurface, FaderMapping, Feedback, SurfaceMapping,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // 1. A custom surface built from closures, driven by a virtual MIDI
    //    loopback (works with no hardware at all).
    let pair = VirtualMidiPair::new();
    let (mut source, mut sink) = pair.split();

    let mut surface = CustomSurface::builder("virtual panel")
        .on_read(move || {
            Ok(source.try_recv()?.map(|inbound| match inbound.message {
                Some(Midi1Message::NoteOn { note, .. }) => {
                    ControlEvent::ButtonPress { button: note }
                }
                Some(Midi1Message::ControlChange {
                    controller, value, ..
                }) => ControlEvent::FaderMove {
                    channel: controller,
                    value: f32::from(value) / 127.0,
                },
                _ => ControlEvent::ButtonRelease { button: 0 },
            }))
        })
        .on_feedback(|feedback| {
            println!("  feedback: {feedback:?}");
            Ok(())
        })
        .build();

    surface.init()?;
    sink.send_midi1(&Midi1Message::NoteOn {
        channel: 0,
        note: 60,
        velocity: 100,
    })?;
    sink.send_midi1(&Midi1Message::ControlChange {
        channel: 0,
        controller: 7,
        value: 90,
    })?;
    while let Some(event) = surface.read_event()? {
        println!("event: {event:?}");
        surface.send_feedback(&Feedback::Led {
            led: event.control(),
            on: true,
        })?;
    }

    // 2. If real MIDI hardware is present, attach a generic surface.
    let devices = tpt_av_control_midi::enumerate_devices()?;
    if let Some(device) = devices
        .iter()
        .find(|d| !d.inputs.is_empty() && !d.outputs.is_empty())
    {
        println!("attaching live surface: {}", device.name);
        let input = tpt_av_control_midi::open_input(&device.inputs[0])?;
        let output = tpt_av_control_midi::open_output(&device.outputs[0])?;
        let mut mapping = SurfaceMapping::new();
        mapping.add_fader(FaderMapping {
            cc: 7,
            parameter: "master.volume".into(),
            range: (0.0, 1.0),
        });
        let mut live = tpt_av_control_surface::GenericMidiSurface::new(
            device.name.clone(),
            Box::new(input),
            Some(Box::new(output)),
            mapping,
        );
        live.init()?;
        // Poll for a few seconds, then exit.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if let Some(event) = live.read_event()? {
                println!("live event: {event:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    } else {
        println!("no bidirectional MIDI hardware found; virtual demo only");
    }

    Ok(())
}
