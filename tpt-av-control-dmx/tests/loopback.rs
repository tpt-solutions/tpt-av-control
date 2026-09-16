//! End-to-end lighting rig tests: fixtures → client → server over real
//! UDP loopback, for both Art-Net and sACN.

use std::net::SocketAddr;
use std::time::Duration;
use tpt_av_control_dmx::{
    DmxClient, DmxProtocol, DmxServer, DmxUniverse, Fixture, FixtureChannel, FixtureDefinition,
};

fn loopback() -> SocketAddr {
    "127.0.0.1:0".parse().unwrap()
}

/// Patches a small rig, sets a look, sends it over sACN, and verifies the
/// server's universe state.
#[test]
fn sacn_show_roundtrip() {
    let mut server = DmxServer::bind(loopback()).unwrap();
    let addr = server.local_addr().unwrap();
    let mut client = DmxClient::new(addr, DmxProtocol::Sacn).unwrap();

    let handle = std::thread::spawn(move || server.recv_universe().unwrap());
    std::thread::sleep(Duration::from_millis(50));

    let mut universe = DmxUniverse::new(1);
    let par1 = Fixture::patch(FixtureDefinition::rgbw(), "par 1", 1, 0).unwrap();
    let par2 = Fixture::patch(FixtureDefinition::rgb(), "par 2", 1, 4).unwrap();
    par1.set_intensity(&mut universe, 255);
    par1.set_color(&mut universe, 255, 30, 0, 0);
    par2.set_color(&mut universe, 0, 0, 255, 0);
    client.send_universe(&universe).unwrap();

    let (received, _src) = handle.join().unwrap();
    assert_eq!(received.universe, 1);
    // par1: dimmer@0, r@1, g@2, b@3, w@4? No — RGBW has r,g,b,w at 0..4;
    // dimmer is a separate fixture.
    assert_eq!(received.get_channel(0), 255); // par1 red
    assert_eq!(received.get_channel(1), 30); // par1 green
    assert_eq!(received.get_channel(3), 0); // par1 white
    assert_eq!(received.get_channel(6), 255); // par2 blue
}

/// The same rig over Art-Net.
#[test]
fn artnet_show_roundtrip() {
    let mut server = DmxServer::bind(loopback()).unwrap();
    let addr = server.local_addr().unwrap();
    let mut client = DmxClient::new(addr, DmxProtocol::ArtNet).unwrap();

    let handle = std::thread::spawn(move || server.recv_universe().unwrap());
    std::thread::sleep(Duration::from_millis(50));

    let mut universe = DmxUniverse::new(0);
    let mover = Fixture::patch(
        FixtureDefinition::new(
            "Moving head",
            vec![
                FixtureChannel::Pan,
                FixtureChannel::Tilt,
                FixtureChannel::Dimmer,
            ],
        )
        .unwrap(),
        "mover 1",
        0,
        0,
    )
    .unwrap();
    mover.set_position(&mut universe, 30, 220);
    mover.set_intensity(&mut universe, 128);
    client.send_universe(&universe).unwrap();

    let (received, _) = handle.join().unwrap();
    assert_eq!(received.get_channel(0), 30); // pan
    assert_eq!(received.get_channel(1), 220); // tilt
    assert_eq!(received.get_channel(2), 128); // dimmer
}

/// Two consecutive sends advance the sequence numbers (both protocols).
#[test]
fn sequence_numbers_advance() {
    let mut sacn_client = DmxClient::new(loopback(), DmxProtocol::Sacn).unwrap();
    let mut artnet_client = DmxClient::new(loopback(), DmxProtocol::ArtNet).unwrap();
    let universe = DmxUniverse::new(0);
    // No assertion possible through the public API; exercise the path for
    // panics/overflow in debug mode.
    for _ in 0..300 {
        sacn_client.send_universe(&universe).unwrap();
        artnet_client.send_universe(&universe).unwrap();
    }
}
