# tpt-av-control-surface

**Hardware control surfaces — one uniform API over MIDI controllers, mixing consoles, and Stream Decks.**

Part of [`tpt-av-control`](../README.md) — the pure-Rust, real-time safe studio
protocol suite (OSC, MIDI 2.0, DMX, hardware control).

**License:** MIT OR Apache-2.0 · **MSRV:** 1.75 · **Status:** pre-1.0

---

## Overview

- **The [`ControlSurface`] trait** — `name` / `init` / `read_event` /
  `send_feedback` covers every physical controller in the suite.
- **Protocol-neutral events** — [`ControlEvent`]: `FaderMove` (normalized
  0.0-1.0), `ButtonPress`/`ButtonRelease`, `EncoderRotate` (detent delta),
  `TouchStrip`.
- **Protocol-neutral feedback** — [`Feedback`]: `Led`, `LedColor` (RGB),
  `DisplayText`, `FaderPosition` (motorized).
- **Declarative mapping** — [`SurfaceMapping`] binds CC numbers to
  parameters (`FaderMapping`), notes to actions (`ButtonMapping`,
  momentary or toggle), and encoders to parameter steps
  (`EncoderMapping`, absolute or relative).
- **Built-in surfaces**
  - [`GenericMidiSurface`] — any MIDI controller: notes → buttons,
    absolute CC → faders, relative CC (two's-complement `0x41=+1 …
    0x3F=−1`) → encoders, pitch bend → touch strip; LED/fader feedback
    over the MIDI output.
  - [`BehringerX32Surface`] — Behringer X32/M32 consoles over OSC
    (`/ch/NN/mix/fader`, `/auxin/`, `/fxrtn/`, `/bus/`,
    `/main/mix/fader`; `/xremote` subscription; mute-lamp feedback).
  - [`StreamDeckSurface`] — Elgato Stream Deck Mini/V2/XL/Original: the
    USB HID packet protocol (key-state edge detection, JPEG/BMP image
    chunking, brightness reports) implemented in pure Rust over a
    pluggable [`HidDevice`] transport — no C bindings required to speak
    the protocol.
  - [`CustomSurface`] — closure-built panels for anything else.

[`ControlSurface`]: https://docs.rs/tpt-av-control-surface/latest/tpt_av_control_surface/surface/trait.ControlSurface.html
[`ControlEvent`]: https://docs.rs/tpt-av-control-surface/latest/tpt_av_control_surface/surface/enum.ControlEvent.html
[`Feedback`]: https://docs.rs/tpt-av-control-surface/latest/tpt_av_control_surface/feedback/enum.Feedback.html
[`SurfaceMapping`]: https://docs.rs/tpt-av-control-surface/latest/tpt_av_control_surface/mapping/struct.SurfaceMapping.html
[`GenericMidiSurface`]: https://docs.rs/tpt-av-control-surface/latest/tpt_av_control_surface/surfaces/generic_midi/struct.GenericMidiSurface.html
[`BehringerX32Surface`]: https://docs.rs/tpt-av-control-surface/latest/tpt_av_control_surface/surfaces/behringer_x32/struct.BehringerX32Surface.html
[`StreamDeckSurface`]: https://docs.rs/tpt-av-control-surface/latest/tpt_av_control_surface/surfaces/elgato_streamdeck/struct.StreamDeckSurface.html
[`HidDevice`]: https://docs.rs/tpt-av-control-surface/latest/tpt_av_control_surface/surfaces/elgato_streamdeck/trait.HidDevice.html
[`CustomSurface`]: https://docs.rs/tpt-av-control-surface/latest/tpt_av_control_surface/surfaces/custom/struct.CustomSurface.html

## Installation

```toml
[dependencies]
tpt-av-control-surface = "0.1"
```

Optional feature: `serde` — serializes the mapping configuration types.

## Usage

### Any MIDI controller

```rust
use tpt_av_control_surface::{
    ControlSurface, Feedback, FaderMapping, SurfaceMapping,
};
use tpt_av_control_midi::{
    enumerate_devices, open_input, open_output, MidiSink, MidiSource,
};

let devices = enumerate_devices()?;
let device = &devices[0]; // pick your controller
let input = open_input(&device.inputs[0])?;
let output = open_output(&device.outputs[0])?;

let mut mapping = SurfaceMapping::new();
mapping.add_fader(FaderMapping {
    cc: 7,
    parameter: "master.volume".into(),
    range: (0.0, 1.0),
});

let mut surface = tpt_av_control_surface::GenericMidiSurface::new(
    device.name.clone(),
    Box::new(input),
    Some(Box::new(output)),
    mapping,
);
surface.init()?;

loop {
    match surface.read_event()? {
        Some(event) => {
            println!("{event:?}");
            surface.send_feedback(&Feedback::Led { led: event.control(), on: true })?;
        }
        None => std::thread::sleep(std::time::Duration::from_millis(10)),
    }
}
```

### Behringer X32/M32 (OSC)

```rust
use tpt_av_control_surface::{BehringerX32Surface, ControlSurface};
use tpt_av_control_osc::OscArg;

let console = "192.168.1.50:10023".parse()?;
let mut x32 = BehringerX32Surface::new(console)?;
x32.init()?; // subscribes with /xremote
println!("receiving on {}", x32.local_addr()?); // point the console's OSC output here

// Fader moves arrive as normalized FaderMove events (main = index 32).
// Feedback writes the same paths back:
use tpt_av_control_surface::Feedback;
x32.send_feedback(&Feedback::FaderPosition { channel: 0, value: 0.8 })?;
```

### Elgato Stream Deck

```rust
use tpt_av_control_surface::{ControlSurface, DeckModel, HidDevice, StreamDeckSurface};

// Implement HidDevice over your HID backend (hidapi, WebHID, ...);
// the protocol itself is handled here.
struct MyHid; // implements HidDevice

let mut deck = StreamDeckSurface::new(DeckModel::Mini, Box::new(MyHid));
deck.init()?; // brightness to 80%
// Key edges arrive as ButtonPress/ButtonRelease; draw JPEG/BMP key
// images through send_feedback(Feedback::LedColor { .. }) framing.
```

### Custom surfaces from closures

```rust
use tpt_av_control_surface::{ControlEvent, ControlSurface, CustomSurface, Feedback};

let mut panel = CustomSurface::builder("my-panel")
    .on_read(|| Ok(Some(ControlEvent::ButtonPress { button: 1 })))
    .on_feedback(|feedback| {
        match feedback {
            Feedback::Led { .. } => println!("led!"),
            _ => {}
        }
        Ok(())
    })
    .build();
assert_eq!(
    panel.read_event().unwrap(),
    Some(ControlEvent::ButtonPress { button: 1 })
);
```

## Testing

```sh
cargo test -p tpt-av-control-surface
```

Runs hardware-free: the generic-MIDI tests drive the surface through the
`VirtualMidiPair` loopback (including the relative-encoder two's-complement
table and feedback messages observed on the wire), the Stream Deck tests
frame image packets byte-exactly and script key-state reports through a
fake HID transport, the X32 tests verify the OSC path ↔ event mapping, and
the custom-surface doc-test is executed by CI.

## Notes

- The Stream Deck's per-model geometry (`rows`/`cols`/`key_count`/
  `image_size`) and packet framing are exposed for applications that
  render key images; JPEG/BMP encoding is deliberately left to the caller
  (pick your own codec crate — the suite adds none).
- X32 fader channels are 1-based on the wire and 0-based in
  `ControlEvent`; the main fader is index 32.

## License

Dual-licensed under MIT OR Apache-2.0 — see [LICENSE-MIT](../LICENSE-MIT) and
[LICENSE-APACHE](../LICENSE-APACHE). See also the per-crate
[CHANGELOG](CHANGELOG.md).
