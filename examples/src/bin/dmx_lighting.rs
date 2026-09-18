//! DMX lighting demo: patches a small rig and runs a color chase,
//! sending it as sACN (or Art-Net) to a console/interface.
//!
//! ```sh
//! cargo run -p tpt-av-control-examples --bin dmx_lighting [target-ip]
//! # target defaults to 127.0.0.1; protocol via TPT_PROTOCOL=sacn|artnet
//! ```

use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;
use tpt_av_control_dmx::{
    DmxClient, DmxProtocol, DmxUniverse, Fixture, FixtureChannel, FixtureDefinition,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let host = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1".into());
    let protocol = match std::env::var("TPT_PROTOCOL").as_deref() {
        Ok("artnet") => DmxProtocol::ArtNet,
        _ => DmxProtocol::Sacn,
    };
    let port = match protocol {
        DmxProtocol::Sacn => tpt_av_control_dmx::sacn::SACN_PORT,
        DmxProtocol::ArtNet => tpt_av_control_dmx::artnet::ARTNET_PORT,
    };
    let target: SocketAddr = (host.as_str(), port)
        .to_socket_addrs()?
        .next()
        .ok_or("cannot resolve target")?;
    println!("streaming {protocol:?} to {target}");

    let mut client = DmxClient::new(target, protocol)?;

    // Patch four RGBW PARs on universe 1 at consecutive addresses.
    let mut rig = Vec::new();
    for i in 0..4u16 {
        rig.push(Fixture::patch(
            FixtureDefinition::rgbw(),
            format!("par {}", i + 1),
            1,
            i * 4,
        )?);
    }

    let mut universe = DmxUniverse::new(1);
    let palette: [[u8; 3]; 4] = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0]];
    println!("running chase; Ctrl-C to quit");
    loop {
        for step in 0..4 {
            universe.clear();
            for (i, fixture) in rig.iter().enumerate() {
                let color = palette[(i + step) % 4];
                fixture.set_intensity(&mut universe, 255);
                fixture.set_color(&mut universe, color[0], color[1], color[2], 0);
                // Tilt-capable fixtures get a gentle sweep via their
                // generic channel when present.
                if fixture.definition.offset(FixtureChannel::Pan).is_some() {
                    fixture.set_position(&mut universe, 96, 128);
                }
            }
            client.send_universe(&universe)?;
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}
