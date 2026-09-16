# tpt-av-control — Project Todo

Tracking checklist for the whole project, organized by phase. License: dual MIT OR Apache-2.0.

## Phase 0 — Project & Repo Setup

- [ ] Create GitHub repo `tpt-solutions/tpt-av-control` *(requires GitHub access; code and CI config are ready to push)*
- [x] Initialize Cargo workspace (`Cargo.toml`: `[workspace]`, `resolver = "2"`, `[workspace.package]`, `[workspace.dependencies]`)
- [x] Switch licensing to dual MIT/Apache-2.0
  - [x] Add `LICENSE-MIT`
  - [x] Add `LICENSE-APACHE`
  - [x] Set `license = "MIT OR Apache-2.0"` in `[workspace.package]`
  - [x] Update spec/README wording away from "Strict/Pure MIT licensing" to dual-license phrasing
- [x] Add `deny.toml` (cargo-deny license allow/deny lists; already permits MIT + Apache-2.0)
- [x] Add root `README.md`
- [x] Add `DESIGN.md` (spec content)
- [x] Add `CONTRIBUTING.md` (dual-license contribution terms)
- [x] Set up CI (GitHub Actions)
  - [x] Build
  - [x] Test
  - [x] `cargo fmt --check`
  - [x] `cargo clippy`
  - [x] `cargo deny check`
- [x] Scaffold empty crates (each with own `Cargo.toml` dual-licensed + `src/lib.rs`)
  - [x] `tpt-av-control-utils`
  - [x] `tpt-av-control-osc`
  - [x] `tpt-av-control-midi`
  - [x] `tpt-av-control-dmx`
  - [x] `tpt-av-control-surface`
  - [x] `tpt-av-control-webrtc`
- [x] Add `examples/` directory skeleton (workspace member `examples` with four binaries)

## Phase 1 — Foundation & OSC

- [x] `tpt-av-control-utils`
  - [x] `ControlError` enum
  - [x] Generic message type (`message.rs`)
  - [x] Timecode/timing types (`time.rs`)
  - [x] Parameter mapping types (`parameter.rs`)
- [x] `tpt-av-control-osc`
  - [x] `OscMessage`, `OscArg`, `OscBundle` types
  - [x] Type-tag encode/decode (`types.rs`)
  - [x] `OscAddressMatcher` (address pattern matching)
  - [x] Real-time-safe zero-allocation OSC parser (`parse_osc_message`)
  - [x] `OscServer` (blocking `run` + async `run_async`, UDP)
  - [x] `OscClient` (`send`, `send_bundle`, UDP)
  - [x] Message routing/dispatch (`dispatch.rs`)
  - [x] Conformance unit tests against OSC 1.0/1.1 spec
- [x] `examples/osc_server.rs`

## Phase 2 — MIDI 1.0

- [x] `Midi1Message` enum
- [x] Real-time-safe raw-byte parser (`parse_midi1`)
- [x] SysEx handling (`sysex.rs`)
- [x] Device enumeration (`enumerate_devices`) wrapping `midir`
- [x] Port management (`port.rs`)
- [x] `MidiInput` (blocking `recv`, non-blocking `try_recv`)
- [x] `MidiOutput` (`send_midi1`)
- [x] MIDI clock/transport (`clock.rs`)
- [x] Integration tests with real MIDI hardware *(implemented in `tests/hardware.rs`; runs when `TPT_MIDI_IN_PORT`/`TPT_MIDI_OUT_PORT` point at hardware)*
- [x] `examples/midi_controller.rs`

## Phase 3 — MIDI 2.0 / UMP

- [x] `Ump` struct (`parse`, `from_message`)
- [x] `Midi2Message` variants (Utility, SystemCommon, SysEx, Midi1ChannelVoice, DataMessage, FlexData)
- [x] `Midi2ChannelVoice` variants
  - [x] NoteOn/NoteOff (16-bit velocity)
  - [x] PolyphonicKeyPressure, ChannelPressure (32-bit)
  - [x] ControlChange (32-bit)
  - [x] ProgramChange (with bank select)
  - [x] PitchBend (32-bit)
  - [x] PerNoteRcc / PerNoteAcc
  - [x] Rpn / Nrpn / RelativeRpn / RelativeNrpn
  - [x] PerNoteManagement
- [x] `send_ump` on `MidiOutput`
- [x] MIDI-CI property exchange and capability negotiation *(discovery/reply, endpoint inquiry/info, NAK/ACK; property exchange sub-IDs return `Unsupported` pending M2-115 data-set messages)*
- [x] MIDI 1.0 ↔ 2.0 backward-compatibility translation layer
- [x] Conformance tests against UMP/MIDI 2.0 spec

## Phase 4 — DMX / Lighting

- [x] `DmxUniverse` (512-channel `set_channel`/`get_channel`)
- [x] Art-Net protocol (`ArtNet`: `send_universe`, `recv_universe`, UDP)
- [x] sACN/E1.31 protocol (`Sacn`: `send_universe`, `recv_universe`, UDP)
- [x] Universe management (`universe.rs`)
- [x] Lighting fixture definitions (`fixture.rs`)
- [x] DMX server/client wiring (`server.rs`, `client.rs`)
- [x] Integration tests with real DMX/Art-Net/sACN hardware or simulator *(UDP loopback simulator tests always run; hardware chase test gated on `TPT_DMX_TARGET`/`TPT_DMX_PROTOCOL`)*

## Phase 5 — Control Surfaces

- [x] `ControlSurface` trait
- [x] `ControlEvent` enum (FaderMove, ButtonPress/Release, EncoderRotate, TouchStrip)
- [x] `Feedback` enum (Led, LedColor, DisplayText, FaderPosition)
- [x] `GenericMidiSurface` (`init`, `read_event`, `send_feedback`)
- [x] `SurfaceMapping` / `FaderMapping` / `ButtonMapping` / `EncoderMapping` config types
- [x] Hardware-specific surfaces
  - [x] Behringer X32/M32 (`behringer_x32.rs`, over OSC)
  - [x] Elgato Stream Deck (`elgato_streamdeck.rs`, pure-Rust protocol over a pluggable HID transport)
  - [x] Custom surface support (`custom.rs`)
- [x] LED/button/display feedback implementation
- [x] `examples/control_surface.rs`
- [x] `examples/dmx_lighting.rs`

## Phase 6 — Advanced Features

- [x] `tpt-av-control-webrtc`: WebRTC data channels for networked control *(envelope codec + `DataChannelTransport` trait with loopback pair; full ICE/DTLS/SCTP plugs in via the trait)*
- [x] MIDI Time Code (MTC) *(quarter frames + full-frame SysEx, `MtcDecoder`)*
- [x] MIDI Show Control (MSC)
- [x] Advanced parameter mapping/automation curves *(utils `Curve`/`Automation` + midi `MidiParameterMapper`)*

## Release & Ecosystem

- [ ] Tag v0.1.0 *(requires push access; `CHANGELOG.md` entry is ready)*
- [ ] Publish all crates to crates.io in dependency order *(requires crates.io token)*
- [x] Verify `cargo-deny` license CI gate is green *(verified locally: `bans ok, licenses ok`; CI runs the same check)*
- [x] Maintain `CHANGELOG.md` per release
- [ ] Cross-check integration points with `tpt-audio` and `tpt-visual` *(those repositories are not in this workspace)*
