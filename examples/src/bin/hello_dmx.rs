//! Zero-hardware DMX quickstart: patches one RGB fixture, cross-fades it
//! red → green → blue over a loopback Art-Net link (both ends run in this
//! process), and prints the channel values as they're received. No real
//! lighting hardware or network configuration required.
//!
//! ```sh
//! cargo run -p tpt-av-control-examples --bin hello_dmx
//! ```

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;
use tpt_av_control_dmx::{ArtNet, DmxUniverse, Fixture, FixtureDefinition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bind two ephemeral Art-Net sockets on loopback: one plays the part of
    // the sending console, the other the receiving fixture/interface.
    let mut receiver = ArtNet::new(0)?;
    let receiver_port = receiver.local_addr()?.port();
    let target: SocketAddr = ([127, 0, 0, 1], receiver_port).into();
    let mut sender = ArtNet::new(0)?;

    println!("looping back Art-Net over 127.0.0.1:{receiver_port}");

    // The "receiving" side: read universes and print channel 0..3 (the
    // fixture's R/G/B) as they arrive. This thread runs for the life of the
    // process; `recv_universe` has no timeout, so we don't join it — the
    // process exit at the end of `main` tears it down.
    thread::spawn(move || loop {
        match receiver.recv_universe() {
            Ok(universe) => println!(
                "  received: R={} G={} B={}",
                universe.get_channel(0),
                universe.get_channel(1),
                universe.get_channel(2)
            ),
            Err(e) => {
                eprintln!("receiver stopped: {e}");
                break;
            }
        }
    });

    // The "sending" side: one RGB fixture at address 0 on universe 1,
    // cross-faded red -> green -> blue.
    let fixture = Fixture::patch(FixtureDefinition::rgb(), "demo par", 1, 0)?;
    let mut universe = DmxUniverse::new(1);
    let steps = [
        (255u8, 0u8, 0u8),
        (0, 255, 0),
        (0, 0, 255),
        (255, 0, 0), // back to red so the loop is visibly cyclic
    ];
    println!("sending cross-fade; Ctrl-C to quit");
    for (r, g, b) in steps.into_iter().cycle().take(12) {
        fixture.set_color(&mut universe, r, g, b, 0);
        sender.send_universe_to(target, &universe)?;
        thread::sleep(Duration::from_millis(400));
    }

    // Give the last frame a moment to arrive and print before we exit.
    thread::sleep(Duration::from_millis(200));
    Ok(())
}
