# Changelog

All notable changes to `tpt-av-control` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Per-crate `README.md` (usage, protocol notes, testing) and `CHANGELOG.md`
  for all crates, plus crates.io metadata (`readme`, `keywords`,
  `categories`, `documentation`, `homepage`).
- **MIDI-CI Property Exchange and Profile Configuration** in
  `tpt-av-control-midi` (`property` module; see its CHANGELOG).
- `fuzz/` cargo-fuzz targets for every network-facing parser
  (`parse_osc_message`, `OscBundle::decode`, `parse_midi1`,
  `Ump::from_bytes`, `artnet::parse_packet`, `sacn::parse_packet`,
  `ControlEnvelope::decode`); run with `cargo +nightly fuzz run <target>`.
- `SECURITY.md` vulnerability-disclosure policy; `.github` issue/PR
  templates and `CODEOWNERS`; tag-triggered release workflow publishing
  crates in dependency order; `justfile` mirroring CI steps.
- CI: `cargo test --doc --workspace` and `cargo doc --workspace
  --no-deps` (warnings denied) steps; `cargo-deny` now runs the full
  check (licenses + bans + advisories + sources).
- `INTEGRATION.md`: the `Message`/`MessageQueue` contract consumed by
  `tpt-audio`/`tpt-visual`.
- `examples/spsc_pipeline`: end-to-end network-thread → SPSC ring →
  real-time-thread demo.
- Doctested usage examples across all crates' public APIs (14 total).

### Changed

- **Hardening** (details in per-crate CHANGELOGs): bounded OSC address
  matching, capped `SysexReassembler` streams, `Sacn::recv_universe`
  datagram slicing, defensive `Fixture` write bounds, checked arithmetic
  in `ControlEnvelope::decode`.

### Fixed

- Example quick-start commands use `--bin` (the demos are binaries, not
  `--example` targets); removed the reference to a nonexistent
  `osc_sender` example.

## [0.1.0] — initial implementation

### Added

- **Workspace**: six crates under the `tpt-av-control-` prefix, dual-licensed
  `MIT OR Apache-2.0`, with per-crate READMEs and CHANGELOGs plus crates.io
  metadata (keywords/categories), CI (build/test on Linux, macOS, Windows;
  `cargo fmt`; `cargo clippy`; `cargo-deny` license/ban checks), `deny.toml`
  enforcing permissive-only dependencies (GPL/LGPL/AGPL/MPL and
  `rtmidi`/`portmidi` banned), README, DESIGN, CONTRIBUTING, and
  INTEGRATION docs.

- **tpt-av-control-utils**
  - `ControlError` error enum shared across the suite.
  - `Message` envelope with lock-free bounded `SpscRing` (`MessageQueue`)
    bridging network/MIDI threads to real-time threads; the ring contains
    the workspace's only `unsafe`, confined to a textbook SPSC algorithm.
  - SMPTE-style `Timecode` with correct 29.97 drop-frame counting and
    inverse, `FrameRate`, and `Timestamp` with NTP (OSC time tag) conversion.
  - Parameter mapping: `ParameterId`, `ParameterValue`, `Mapping`, transfer
    `Curve`s (linear, ease, exponential, logarithmic), and keyframe
    `Automation` timelines with segment curves.

- **tpt-av-control-osc**
  - OSC 1.0/1.1 wire format: all standard type tags (`i f s b h t d S c r m
    T F N I`), 4-byte padding, bundles with nested bundles and the
    "immediate" time tag.
  - Real-time-safe zero-copy parser `parse_osc_message` returning
    `OscMessageRef` that borrows addresses and argument data.
  - UDP `OscServer` (blocking and tokio `run_async`) with bundle expansion,
    `OscClient` (`send`, `send_bundle`), pattern-based `OscDispatcher`
    (`?`, `*`, `[...]` with `!`/ranges, `{a,b}`, and OSC 1.1 `//`), and
    conformance tests against the OSC 1.0 spec.

- **tpt-av-control-midi**
  - MIDI 1.0: real-time-safe `parse_midi1` / `write_bytes` for all channel
    and system messages; SysEx helpers (universal builders, GM on/off,
    identity request, chunking).
  - MIDI 2.0 / UMP (M2-101-U): Utility, System, MIDI 1.0 CV, sysex7 Data,
    MIDI 2.0 CV (all 15 opcodes including Per-Note RCC/ACC, RPN/NRPN and
    relative variants, Per-Note Management), extended data, and Flex Data
    (Set Tempo, Time Signature, text) with reference-vector conformance
    tests.
  - MIDI 1.0 ↔ 2.0 translation with the recommended scaling (7↔16↔32 bit,
    14-bit pitch bend), velocity-0 note-on handling, and a `SysexReassembler`
    for chunked sysex7.
  - MIDI-CI message layer: Discovery / Discovery Reply, Endpoint
    Inquiry / Info, NAK/ACK, and a `CiDiscoverySession` handshake helper.
  - Device I/O wrapping `midir`: `enumerate_devices`, `open_input`,
    `open_output`, blocking/non-blocking receive, plus `VirtualMidiPair`
    loopback for hardware-free testing; hardware round-trip test gated on
    `TPT_MIDI_IN_PORT` / `TPT_MIDI_OUT_PORT`.
  - MIDI clock/transport (`MidiClockSender`, `MidiClockReceiver` with BPM
    estimation), MIDI Time Code (quarter frames + full-frame SysEx with an
    assembling `MtcDecoder`), MIDI Show Control (GO/STOP/FIRE/RESET/GO JAM/
    GO OFF with cue lists), and `MidiParameterMapper` for CC/note binding.

- **tpt-av-control-dmx**
  - `DmxUniverse` (512 channels), `UniverseManager` with on-demand creation.
  - Art-Net 4: ArtDmx build/parse with conformance vectors, minimal
    ArtPoll/ArtPollReply.
  - sACN / ANSI E1.31: 638-byte data packets with CID, priority, sequence;
    round-trip conformance tests.
  - Auto-detecting `DmxServer` and protocol-multiplexing `DmxClient`,
    UDP loopback integration tests, and an env-gated hardware chase test
    (`TPT_DMX_TARGET` / `TPT_DMX_PROTOCOL`).
  - `FixtureDefinition` profiles (dimmer, RGB, RGBW, moving head) and
    `Fixture` patching with color/intensity/position helpers.

- **tpt-av-control-surface**
  - `ControlSurface` trait with `ControlEvent` (fader/button/encoder/touch
    strip) and `Feedback` (LED, LED color, display text, fader position).
  - `SurfaceMapping` / `FaderMapping` / `ButtonMapping` / `EncoderMapping`
    declarative configuration.
  - `GenericMidiSurface` (notes→buttons, CC→faders or relative encoders,
    pitch bend→touch strip, LED/fader feedback).
  - `BehringerX32Surface` over OSC (`/ch/NN/mix/fader`, `/main/mix/fader`,
    `/xremote` subscribe, mute-lamp feedback).
  - `StreamDeckSurface`: pure-Rust Stream Deck packet protocol (Mini/V2/XL/
    Original framing, key-state edge detection, image chunking, brightness)
    over a pluggable `HidDevice` transport — no C bindings required to
    speak the protocol.
  - `CustomSurface`: closure-based custom panels.

- **tpt-av-control-webrtc**
  - `ControlEnvelope` (parameter change / transport command / text) with a
    compact binary codec and optional JSON rendering (`json` feature).
  - `DataChannelTransport` trait with an in-process `LoopbackTransport`
    pair; a full ICE/DTLS/SCTP stack plugs in via the trait.

- **examples**: `osc_server`, `midi_controller`, `dmx_lighting`,
  `control_surface`.

[Unreleased]: https://github.com/tpt-solutions/tpt-av-control/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/tpt-solutions/tpt-av-control/releases/tag/v0.1.0
