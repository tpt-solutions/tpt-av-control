//! Integration test with real DMX/Art-Net/sACN hardware or a console
//! simulator.
//!
//! Skipped unless `TPT_DMX_TARGET` is set to `ip:port` of an Art-Net node
//! (port 6454) or sACN listener (port 5568), and `TPT_DMX_PROTOCOL` to
//! `artnet` or `sacn`. Sends a slow color chase on universe 0 for a few
//! seconds — visually verifiable against real fixtures.

use std::net::ToSocketAddrs;
use std::time::Duration;
use tpt_av_control_dmx::{DmxClient, DmxProtocol, DmxUniverse, Fixture, FixtureDefinition};

#[test]
fn hardware_chase() {
    let Ok(target) = std::env::var("TPT_DMX_TARGET") else {
        eprintln!("skipping: TPT_DMX_TARGET not set");
        return;
    };
    let protocol = match std::env::var("TPT_DMX_PROTOCOL").as_deref() {
        Ok("sacn") => DmxProtocol::Sacn,
        _ => DmxProtocol::ArtNet,
    };
    let Some(addr) = target.to_socket_addrs().ok().and_then(|mut i| i.next()) else {
        eprintln!("skipping: cannot resolve TPT_DMX_TARGET {target:?}");
        return;
    };

    let mut client = DmxClient::new(addr, protocol).unwrap();
    let par = Fixture::patch(FixtureDefinition::rgb(), "chase", 0, 0).unwrap();
    let mut universe = DmxUniverse::new(0);

    // A brief red→green→blue chase at full intensity.
    for color in [[255u8, 0, 0], [0, 255, 0], [0, 0, 255]] {
        par.set_color(&mut universe, color[0], color[1], color[2], 0);
        client.send_universe(&universe).unwrap();
        std::thread::sleep(Duration::from_millis(700));
    }
}
