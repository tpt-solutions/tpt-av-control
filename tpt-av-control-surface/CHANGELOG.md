# Changelog

All notable changes to `tpt-av-control-surface` are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning:
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Doc-tested usage example for `SurfaceMapping::add_fader`.
- Crates.io metadata: `readme`, `keywords`, `categories`, `documentation`,
  `homepage`.

## [0.1.0] — initial release

### Added

- `ControlSurface` trait (`name`/`init`/`read_event`/`send_feedback`) with
  `ControlEvent` (FaderMove normalized 0.0-1.0, ButtonPress/Release,
  EncoderRotate with detent delta, TouchStrip) and `Feedback` (Led,
  LedColor RGB, DisplayText, FaderPosition) as the protocol-neutral event
  model; `ControlEvent::control()` extracts the control index.
- Declarative mapping (`mapping`): `SurfaceMapping` with
  `add_fader`/`add_button`/`add_encoder` (replace-on-same-control),
  `FaderMapping` (CC → parameter range), `ButtonMapping` (note → action,
  momentary flag), `EncoderMapping` (CC → parameter step, relative flag);
  lookup accessors.
- `GenericMidiSurface`: notes → button events (velocity-0 note-on is a
  release), absolute CC → fader moves, relative two's-complement CC
  (`0x41..=0x7F` positive, `0x00..=0x3F` negative, `0x40` no-op) → encoder
  rotations, pitch bend → touch strip; LED (note on/off) and fader
  (CC) feedback; `read_event_timeout` poll helper; init drains stale
  input; unsupported feedback kinds (color/display) return
  `ControlError::Unsupported`.
- `BehringerX32Surface`: OSC client + listener pair aimed at a console
  (default port 10023), `/xremote` + `/info` subscription on init,
  `/ch/NN/mix/fader`, `/auxin/`, `/fxrtn/`, `/bus/`, and
  `/main/mix/fader` mapped to/from normalized fader events (main = index
  32), mute-lamp feedback via `/mix/on`, non-blocking event pump.
- `StreamDeckSurface` (`surfaces::elgato_streamdeck`): pure-Rust Stream
  Deck protocol over the pluggable `HidDevice` trait — `DeckModel`
  geometry (Original/Mini/V2/XL rows, columns, image sizes, chunk sizes),
  V2 (`0x07`/`0x05`) and Original (`0x02`) image framing with byte-exact
  packets, key-state report edge detection (press/release, repeats
  suppressed), brightness feature reports per generation, key-range
  validation; testable through scripted in-memory HID transports.
- `CustomSurface` / `CustomSurfaceBuilder`: closure-based surfaces with
  optional `on_init`/`on_read`/`on_feedback` (missing closures become
  no-ops); documented with an executable doc-test.
- Optional `serde` feature for the mapping configuration types.
- Hardware-free test coverage: virtual MIDI loopback, fake HID scripting,
  and OSC path-mapping vectors.

[0.1.0]: https://github.com/tpt-solutions/tpt-av-control/releases/tag/v0.1.0
