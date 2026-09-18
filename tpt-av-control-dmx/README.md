# tpt-av-control-dmx

**DMX512, Art-Net 4, and sACN (ANSI E1.31) — universes, streaming, and fixture definitions.**

Part of [`tpt-av-control`](../README.md) — the pure-Rust, real-time safe studio
protocol suite (OSC, MIDI 2.0, DMX, hardware control).

**License:** MIT OR Apache-2.0 · **MSRV:** 1.75 · **Status:** pre-1.0

---

## Overview

- **`DmxUniverse`** — 512-channel universe data with zero-based
  `set_channel`/`get_channel` (out-of-range reads return 0, writes are
  fallible), bulk `set_channels` with truncation, and blackout helpers.
- **Art-Net 4** (`artnet`) — ArtDmx packet build/parse (opcode `0x5000`,
  protocol version 14, big-endian length, SubUni/Net universe encoding)
  with byte-level conformance vectors; a minimal ArtPoll / ArtPollReply
  implementation so the crate can act as a discoverable node.
- **sACN / E1.31** (`sacn`) — standard 638-byte data packets: root layer
  with the `ASC-E1.17` identifier and CID, framing layer with 64-byte
  source name, priority (default 100), per-universe sequence, and the DMP
  layer with the DMX512-A start code; full build/parse round-trips.
- **Auto-detecting server** (`server`) — `DmxServer` accepts Art-Net *and*
  sACN on one socket, tags each received universe with its
  [`DmxProtocol`], and merges the latest state into a
  [`UniverseManager`].
- **Client** (`client`) — `DmxClient` streams universes to a fixed target
  over either protocol, with per-protocol sequence numbers.
- **Fixtures** (`fixture`) — `FixtureDefinition` profiles (dimmer, RGB,
  RGBW, or any channel list), `Fixture::patch` with address validation,
  and high-level `set_intensity` / `set_color` / `set_position` writers
  that only touch channels the profile actually has.
- **Universe management** (`universe`) — `UniverseManager` with on-demand
  creation, merge, and ordered iteration.

[`DmxProtocol`]: https://docs.rs/tpt-av-control-dmx/latest/tpt_av_control_dmx/server/enum.DmxProtocol.html
[`UniverseManager`]: https://docs.rs/tpt-av-control-dmx/latest/tpt_av_control_dmx/universe/struct.UniverseManager.html

## Installation

```toml
[dependencies]
tpt-av-control-dmx = "0.1"
```

## Usage

### Patch a rig and run a color chase (sACN)

```rust
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;
use tpt_av_control_dmx::{DmxClient, DmxProtocol, DmxUniverse, Fixture, FixtureDefinition};

let target: SocketAddr = ("console.example", 5568).to_socket_addrs()?.next().unwrap();
let mut client = DmxClient::new(target, DmxProtocol::Sacn)?;

// Four RGBW PARs at consecutive addresses on universe 1.
let mut rig = Vec::new();
for i in 0..4u16 {
    rig.push(Fixture::patch(FixtureDefinition::rgbw(), format!("par {i}"), 1, i * 4)?);
}

let palette = [[255u8, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0]];
let mut universe = DmxUniverse::new(1);
loop {
    for step in 0..4 {
        universe.clear();
        for (i, fixture) in rig.iter().enumerate() {
            let c = palette[(i + step) % 4];
            fixture.set_intensity(&mut universe, 255);
            fixture.set_color(&mut universe, c[0], c[1], c[2], 0);
        }
        client.send_universe(&universe)?;
        std::thread::sleep(Duration::from_millis(500));
    }
}
```

(For Art-Net, use `DmxProtocol::ArtNet` and port
[`artnet::ARTNET_PORT`](https://docs.rs/tpt-av-control-dmx/latest/tpt_av_control_dmx/artnet/constant.ARTNET_PORT.html) = 6454.)

### Receive either protocol on one socket

```rust
use tpt_av_control_dmx::{DmxServer, DmxProtocol};

let mut server = DmxServer::new(5568)?;
loop {
    let (universe, src) = server.recv_universe()?;
    println!(
        "{} from {src}: ch1 = {} ({:?})",
        universe.universe,
        universe.get_channel(0),
        server.last_protocol().unwrap(),
    );
    // server.manager() holds the merged latest state of every universe.
}
```

### Raw protocol nodes

```rust
use tpt_av_control_dmx::{ArtNet, Sacn, DmxUniverse};

let mut artnet = ArtNet::new(tpt_av_control_dmx::artnet::ARTNET_PORT)?;
let mut universe = DmxUniverse::new(0);
universe.set_channel(0, 255);
// artnet.recv_universe()? also answers ArtPoll with an ArtPollReply.

let mut sacn = Sacn::new(tpt_av_control_dmx::sacn::SACN_PORT)?;
sacn.set_source(tpt_av_control_dmx::Cid::from_bytes(b"my-console"), "my console");
sacn.send_universe_to("127.0.0.1:5568".parse()?, &universe)?;
```

## Conformance & testing

- ArtDmx layout vectors (header, opcode endianness, protocol version,
  sequence, SubUni/Net, BE length, slot data), ArtPoll parsing, and
  universe-encoding checks.
- sACN 638-byte layout vectors (preamble, ACN identifier, root/framing
  vectors, CID, priority/sequence/universe offsets, DMP counters) and
  round-trip parse, plus truncation rejection.
- `tests/loopback.rs` — end-to-end show tests: patched fixtures →
  `DmxClient` → real UDP → `DmxServer` → verified universe state, for both
  protocols, plus sequence-advance stress.
- `tests/hardware.rs` — a visual color chase to real hardware, **skipped
  unless** `TPT_DMX_TARGET` (an Art-Net node or sACN listener) and
  `TPT_DMX_PROTOCOL` (`artnet`|`sacn`) are set.

## Protocol notes

- Universes are addressed **zero-based** in this API; the wire carries
  1-based slot numbers — the conversion is handled internally.
- Art-Net universe = `SubUni` (low byte) + `Net` (high 7 bits).
- sACN sequence numbers increment per client; the packet layout follows
  ANSI E1.31 with the short (512-slot) form only.
- Priority defaults to 100 (the E1.31 recommendation).

## License

Dual-licensed under MIT OR Apache-2.0 — see [LICENSE-MIT](../LICENSE-MIT) and
[LICENSE-APACHE](../LICENSE-APACHE). See also the per-crate
[CHANGELOG](CHANGELOG.md).
