# tpt-av-control

**A pure-Rust, real-time safe studio protocol suite. OSC, MIDI 2.0, DMX, and hardware control. The integration layer of the TPT AV stack.**

**License:** MIT OR Apache-2.0
**Status:** Early-stage / Pre-1.0
**Ecosystem:** [TPT Solutions Open Source](https://opensource.tptsolutions.co.nz/)

---

## Overview

`tpt-av-control` is the **protocol and hardware integration layer** of the TPT AV Stack. It provides pure-Rust, real-time safe implementations of the studio control protocols used in audio, video, lighting, and live performance.

- **Unified protocol suite** — OSC, MIDI 2.0 (UMP), DMX512, Art-Net, and sACN in one cohesive library.
- **Real-time safe** — protocol parsing is allocation-free and lock-free where it matters; safe to feed results to audio callbacks.
- **MIDI 2.0 / UMP native** — full support for Universal MIDI Packet with MIDI 1.0 backward compatibility.
- **Hardware control surfaces** — a unified API for physical controllers (mixers, Stream Decks, generic MIDI).
- **Permissive licensing** — dual MIT OR Apache-2.0, zero copyleft contamination, enforced via `cargo-deny` in CI.

## Crates

Every crate ships its own README (usage, protocol notes, testing) and
CHANGELOG alongside its sources.

| Crate | Purpose |
| :--- | :--- |
| [`tpt-av-control-utils`](tpt-av-control-utils/README.md) | Shared types, errors, lock-free RT queue, timecode, parameter mapping |
| [`tpt-av-control-osc`](tpt-av-control-osc/README.md) | Open Sound Control 1.0/1.1: parse, encode, UDP server/client |
| [`tpt-av-control-midi`](tpt-av-control-midi/README.md) | MIDI 1.0 and MIDI 2.0 (UMP), device I/O, clock, MTC/MSC, MIDI-CI |
| [`tpt-av-control-dmx`](tpt-av-control-dmx/README.md) | DMX512 universes, Art-Net, sACN/E1.31, fixture definitions |
| [`tpt-av-control-surface`](tpt-av-control-surface/README.md) | Hardware control surfaces (generic MIDI, X32, Stream Deck, custom) |
| [`tpt-av-control-webrtc`](tpt-av-control-webrtc/README.md) | WebRTC data channel transport for networked control |

Engines integrating with this suite: see [INTEGRATION.md](INTEGRATION.md)
for the `Message`/`MessageQueue` contract consumed by `tpt-audio` and
`tpt-visual`.

## Quick start

```toml
[dependencies]
tpt-av-control-osc = "0.1"
```

```rust
use tpt_av_control_osc::{OscClient, OscMessage, OscArg};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = "127.0.0.1:8000".parse()?;
    let mut client = OscClient::new(target)?;
    let msg = OscMessage::new("/track/1/volume", &[OscArg::Float(0.75)])?;
    client.send(&msg)?;
    Ok(())
}
```

Each crate is independently useful — use just OSC, just MIDI, or just DMX.

## Examples

Runnable demos live in the [`examples`](examples) workspace member:

```sh
cargo run -p tpt-av-control-examples --bin osc_server
cargo run -p tpt-av-control-examples --bin midi_controller
cargo run -p tpt-av-control-examples --bin dmx_lighting
cargo run -p tpt-av-control-examples --bin control_surface
```

## Real-time safety

Protocol parsing (`parse_osc_message`, `parse_midi1`, UMP handling) performs **no heap allocation** for fixed-size messages, and network I/O is kept off the real-time thread:

```text
Network Thread (async, can allocate)   →  lock-free SPSC queue  →  Audio/Video Thread (zero allocation)
MIDI Thread (blocking, can allocate)   →  lock-free SPSC queue  →
```

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Dependencies are restricted to permissively licensed crates (MIT, Apache-2.0, BSD, ISC, Zlib); GPL/LGPL/AGPL/MPL are banned and enforced by [`deny.toml`](deny.toml) in CI.

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution terms.
