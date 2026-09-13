# tpt-av-control — Project Todo

Tracking checklist for the whole project, organized by phase. License: dual MIT OR Apache-2.0.

## Phase 0 — Project & Repo Setup

- [ ] Create GitHub repo `tpt-solutions/tpt-av-control`
- [ ] Initialize Cargo workspace (`Cargo.toml`: `[workspace]`, `resolver = "2"`, `[workspace.package]`, `[workspace.dependencies]`)
- [ ] Switch licensing to dual MIT/Apache-2.0
  - [ ] Add `LICENSE-MIT`
  - [ ] Add `LICENSE-APACHE`
  - [ ] Set `license = "MIT OR Apache-2.0"` in `[workspace.package]`
  - [ ] Update spec/README wording away from "Strict/Pure MIT licensing" to dual-license phrasing
- [ ] Add `deny.toml` (cargo-deny license allow/deny lists; already permits MIT + Apache-2.0)
- [ ] Add root `README.md`
- [ ] Add `DESIGN.md` (spec content)
- [ ] Add `CONTRIBUTING.md` (dual-license contribution terms)
- [ ] Set up CI (GitHub Actions)
  - [ ] Build
  - [ ] Test
  - [ ] `cargo fmt --check`
  - [ ] `cargo clippy`
  - [ ] `cargo deny check`
- [ ] Scaffold empty crates (each with own `Cargo.toml` dual-licensed + `src/lib.rs`)
  - [ ] `tpt-av-control-utils`
  - [ ] `tpt-av-control-osc`
  - [ ] `tpt-av-control-midi`
  - [ ] `tpt-av-control-dmx`
  - [ ] `tpt-av-control-surface`
  - [ ] `tpt-av-control-webrtc`
- [ ] Add `examples/` directory skeleton

## Phase 1 — Foundation & OSC

- [ ] `tpt-av-control-utils`
  - [ ] `ControlError` enum
  - [ ] Generic message type (`message.rs`)
  - [ ] Timecode/timing types (`time.rs`)
  - [ ] Parameter mapping types (`parameter.rs`)
- [ ] `tpt-av-control-osc`
  - [ ] `OscMessage`, `OscArg`, `OscBundle` types
  - [ ] Type-tag encode/decode (`types.rs`)
  - [ ] `OscAddressMatcher` (address pattern matching)
  - [ ] Real-time-safe zero-allocation OSC parser (`parse_osc_message`)
  - [ ] `OscServer` (blocking `run` + async `run_async`, UDP)
  - [ ] `OscClient` (`send`, `send_bundle`, UDP)
  - [ ] Message routing/dispatch (`dispatch.rs`)
  - [ ] Conformance unit tests against OSC 1.0/1.1 spec
- [ ] `examples/osc_server.rs`

## Phase 2 — MIDI 1.0

- [ ] `Midi1Message` enum
- [ ] Real-time-safe raw-byte parser (`parse_midi1`)
- [ ] SysEx handling (`sysex.rs`)
- [ ] Device enumeration (`enumerate_devices`) wrapping `midir`
- [ ] Port management (`port.rs`)
- [ ] `MidiInput` (blocking `recv`, non-blocking `try_recv`)
- [ ] `MidiOutput` (`send_midi1`)
- [ ] MIDI clock/transport (`clock.rs`)
- [ ] Integration tests with real MIDI hardware
- [ ] `examples/midi_controller.rs`

## Phase 3 — MIDI 2.0 / UMP

- [ ] `Ump` struct (`parse`, `from_message`)
- [ ] `Midi2Message` variants (Utility, SystemCommon, SysEx, Midi1ChannelVoice, DataMessage, FlexData)
- [ ] `Midi2ChannelVoice` variants
  - [ ] NoteOn/NoteOff (16-bit velocity)
  - [ ] PolyphonicKeyPressure, ChannelPressure (32-bit)
  - [ ] ControlChange (32-bit)
  - [ ] ProgramChange (with bank select)
  - [ ] PitchBend (32-bit)
  - [ ] PerNoteRcc / PerNoteAcc
  - [ ] Rpn / Nrpn / RelativeRpn / RelativeNrpn
  - [ ] PerNoteManagement
- [ ] `send_ump` on `MidiOutput`
- [ ] MIDI-CI property exchange and capability negotiation
- [ ] MIDI 1.0 ↔ 2.0 backward-compatibility translation layer
- [ ] Conformance tests against UMP/MIDI 2.0 spec

## Phase 4 — DMX / Lighting

- [ ] `DmxUniverse` (512-channel `set_channel`/`get_channel`)
- [ ] Art-Net protocol (`ArtNet`: `send_universe`, `recv_universe`, UDP)
- [ ] sACN/E1.31 protocol (`Sacn`: `send_universe`, `recv_universe`, UDP)
- [ ] Universe management (`universe.rs`)
- [ ] Lighting fixture definitions (`fixture.rs`)
- [ ] DMX server/client wiring (`server.rs`, `client.rs`)
- [ ] Integration tests with real DMX/Art-Net/sACN hardware or simulator

## Phase 5 — Control Surfaces

- [ ] `ControlSurface` trait
- [ ] `ControlEvent` enum (FaderMove, ButtonPress/Release, EncoderRotate, TouchStrip)
- [ ] `Feedback` enum (Led, LedColor, DisplayText, FaderPosition)
- [ ] `GenericMidiSurface` (`init`, `read_event`, `send_feedback`)
- [ ] `SurfaceMapping` / `FaderMapping` / `ButtonMapping` / `EncoderMapping` config types
- [ ] Hardware-specific surfaces
  - [ ] Behringer X32/M32 (`behringer_x32.rs`)
  - [ ] Elgato Stream Deck (`elgato_streamdeck.rs`)
  - [ ] Custom surface support (`custom.rs`)
- [ ] LED/button/display feedback implementation
- [ ] `examples/control_surface.rs`
- [ ] `examples/dmx_lighting.rs`

## Phase 6 — Advanced Features

- [ ] `tpt-av-control-webrtc`: WebRTC data channels for networked control
- [ ] MIDI Time Code (MTC)
- [ ] MIDI Show Control (MSC)
- [ ] Advanced parameter mapping/automation curves

## Release & Ecosystem

- [ ] Tag v0.1.0
- [ ] Publish all crates to crates.io in dependency order
- [ ] Verify `cargo-deny` license CI gate is green
- [ ] Maintain `CHANGELOG.md` per release
- [ ] Cross-check integration points with `tpt-audio` and `tpt-visual`
