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
- [x] Add `examples/` directory skeleton (workspace member `examples` with binaries)

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
- [x] MIDI-CI property exchange and capability negotiation *(discovery/reply, endpoint inquiry/info, NAK/ACK; full Property Exchange in Phase 8)*
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

## Phase 7 — Security & Hardening

- [x] Fix `Sacn::recv_universe` (`tpt-av-control-dmx/src/sacn.rs`) to parse `&buf[..len]` instead of the full fixed buffer
- [x] Add a chunk-count/total-size cap to `SysexReassembler` (`tpt-av-control-midi/src/ump.rs`, `feed`) to bound memory growth from unterminated `Continue` streams *(1024 chunks / 1 MiB)*
- [x] Fix CI's `cargo-deny-action` step to run `advisories` (and `sources`), not just `licenses bans` (`.github/workflows/ci.yml` now runs the full `check`; verified locally: `advisories ok, bans ok, licenses ok, sources ok`)
- [x] Add `cargo-fuzz` targets for `parse_osc_message`, `OscBundle::decode`, `parse_midi1`, `Ump::from_bytes`, `artnet::parse_packet`, `sacn::parse_packet` *(plus `ControlEnvelope::decode`; `fuzz/` is workspace-excluded — run with `cargo +nightly fuzz run <target>`)*
- [x] Cap wildcard/alternation count (or memoize) in `OscAddressMatcher` (`tpt-av-control-osc/src/address.rs`) to bound worst-case match cost *(10,000-step budget; hostile inputs degrade to no-match)*
- [x] Make `Fixture::start_address`/`definition` private with a re-validating setter (`tpt-av-control-dmx/src/fixture.rs`), or re-check bounds in `write()` *(took the `write()` re-check option)*
- [x] Use `checked_add` for the `cursor + 4 + len` arithmetic in `ControlEnvelope::decode` (`tpt-av-control-webrtc/src/envelope.rs`) for 32-bit-target safety
- [x] Add `SECURITY.md` with a vulnerability-disclosure policy

## Phase 8 — MIDI-CI Property Exchange

- [x] Implement MIDI-CI Property Exchange sub-IDs (M2-115 data-set messages) in `tpt-av-control-midi/src/property.rs`: Get/Set Property Data + Replies, Subscribe/Subscription Data + Subscribe Reply, Notify (0x30-0x37), request-id correlation, 14-bit header sizing, chunked data-set reassembly (`PropertyDataSetAssembler` with caps), dependency-free JSON header field parsing (`header_field`/`header_field_num`)
- [x] Implement MIDI-CI Profile Configuration messages alongside property exchange (`ProfileConfigMessage`: Inquiry/Reply, Set Profile On/Off + Replies)

## Phase 9 — Adoption & Tooling

- [x] Fix README/example quick-start commands: `cargo run --example X` → `--bin X` (root `README.md`, and doc comments in `examples/src/bin/*.rs`); removed the reference to the nonexistent `osc_sender` example
- [x] Add `keywords`, `categories`, `documentation`, `homepage` fields to every publishable crate's `Cargo.toml` *(the `examples` package is `publish = false` and carries only `readme`)*
- [x] Add per-crate `README.md` files and set the `readme` field
- [x] Add doctested (`cargo test --doc`) usage examples to public APIs across crates *(14 doctests workspace-wide, run in CI)*
- [x] Add `cargo test --doc --workspace` and `cargo doc --workspace --no-deps` as CI steps
- [x] Add a `justfile` mirroring CI steps (fmt, clippy, test, doctest, doc, deny, package dry-run, run examples)
- [x] Add `.github/ISSUE_TEMPLATE/*`, `.github/PULL_REQUEST_TEMPLATE.md`, `CODEOWNERS`
- [x] Add release automation (tag-triggered `cargo publish` in dependency order via `.github/workflows/release.yml`; needs the `CARGO_REGISTRY_TOKEN` secret)
- [x] Wire in the sibling `tpt-av-test-benchmark` allocation-tracking harness to validate the "real-time safe / zero-allocation" claim *(`tests/real_time.rs` in `tpt-av-control-osc`/`-midi`/`-dmx`, using `assert_real_time_safe!` around `parse_osc_message`, `parse_midi1`, `Ump::from_bytes`, and `DmxUniverse::set_channel`/`get_channel`; runs under plain `cargo test`, no separate criterion `[[bench]]` harness added since the allocation-tracking macro already gives a pass/fail RT-safety gate)*
- [x] Wire in `tpt-av-test-fuzz` proptest harness *(`tests/fuzz_never_panics.rs` in `tpt-av-control-osc`/`-midi`/`-dmx`/`-webrtc`, using `fuzz_parser_never_panics!` against `parse_osc_message`, `parse_midi1`, `Ump::from_bytes`+`parse`, `artnet::parse_packet`, `sacn::parse_packet`, `DmxUniverse::try_set_channel`/`get_channel`, and `ControlEnvelope::decode`; runs under plain `cargo test`, complements rather than replaces the in-repo `fuzz/` libfuzzer targets)*
- [x] Add an end-to-end example demonstrating the full network-thread → SPSC-ring → RT-thread pipeline described in `DESIGN.md` §5.3 (`examples/src/bin/spsc_pipeline.rs`)

## Phase 10 — Cleanup

- [x] Delete stray root-level `fdmod.rs` *(and other stray exploration files)*
- [x] Revisit the dev-dependency on `tpt-av-test-*` crates now that `tpt-av-test` is published *(confirmed via `git -C ../tpt-av-test remote -v` + `git ls-remote`: `origin` is `https://github.com/tpt-solutions/tpt-av-test.git`, pushed and public — `GET https://api.github.com/repos/tpt-solutions/tpt-av-test` returns `"private": false`. Switched `tpt-av-test-mock`, `tpt-av-test-benchmark`, and `tpt-av-test-fuzz` from `path = "../../tpt-av-test/..."` to `git = "https://github.com/tpt-solutions/tpt-av-test", rev = "823cd17fe4fd99d3190f51386cc997a294afc7b5"` in the root `Cargo.toml`'s `[workspace.dependencies]`, referenced via `{ workspace = true }` from each consuming crate. Verified this resolves and builds/tests cleanly with no sibling checkout on disk — `cargo build --workspace --all-targets` and `cargo test --workspace` both pass from a clean `Cargo.lock`. This removes the pre-push CI blocker entirely. Note: pinned by commit `rev`, not a branch, so bumping to a newer `tpt-av-test` commit is a deliberate one-line edit in `[workspace.dependencies]`, not automatic.)*

## Release & Ecosystem

- [ ] Tag v0.1.0 *(requires push access; root + per-crate `CHANGELOG.md` entries are ready and `cargo package` dry-runs pass for the dependency root)*
- [ ] Publish all crates to crates.io in dependency order *(requires crates.io token; `release.yml` automates it once the secret is set — utils must land first, then the rest follow)*
- [x] Verify `cargo-deny` license CI gate is green *(verified locally with the full check: `advisories ok, bans ok, licenses ok, sources ok`)*
- [x] Maintain `CHANGELOG.md` per release *(root plus per-crate)*
- [x] Cross-check integration points with `tpt-audio` and `tpt-visual` *(the message/queue contract they consume is specified in [INTEGRATION.md](INTEGRATION.md); those repos live outside this workspace. Findings: neither repo currently consumes this contract — no `tpt-av-control-*` dependency, `SpscRing`, `ParameterId`/`ParameterValue`, `MessageBody`, or `ControlEnvelope` reference anywhere in either tree; this is greenfield integration work, not yet started, on both sides. `tpt-visual` has no related types at all. `tpt-audio` independently defines its own same-named-but-incompatible types: `tpt-av-audio-core::ring::SpscRing` (an f32 interleaved audio-sample ring, unrelated generic shape to `tpt-av-control-utils::SpscRing<Message>`), and `tpt-av-audio-plugin::parameter::ParameterId(pub u32)` (a numeric plugin-parameter index paired with raw `f32` values, no `ParameterValue` enum or dotted-string namespace) versus this suite's dotted-string `ParameterId` + `ParameterValue::Float/Int/Bool/String`. Bridging tpt-audio's numeric parameter IDs to this suite's string namespace will need an explicit translation layer, not a drop-in match. No changes made to INTEGRATION.md — the mismatches are on the consumer side, not something this document promises incorrectly. Not actionable further from inside this workspace, and no `tpt-av-control` changes are warranted by these findings.)*
