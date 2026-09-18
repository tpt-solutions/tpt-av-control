# Changelog

All notable changes to `tpt-av-control-midi` are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning:
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **MIDI-CI Property Exchange** (`property` module, M2-115 message layer):
  `PropertyExchangeMessage` for Get/Set Property Data and Replies,
  Subscribe/Subscription Data, Subscribe Reply, and Notify (sub-ID 2
  `0x30`-`0x37`) with request-id correlation, 14-bit header sizing, and
  7-bit payload validation; `PropertyDataSetAssembler` reassembles chunked
  data sets (`chunkCount`/`chunkNumber`) with 128-chunk / 1 MiB caps;
  dependency-free JSON header field extraction (`header_field`,
  `header_field_num`).
- **MIDI-CI Profile Configuration** (`ProfileConfigMessage`): Profile
  Inquiry / Reply and Set Profile On/Off (+ Replies) with profile lists
  and per-address targeting.
- Doc-tested usage examples for `parse_midi1`, `Ump::from_message`, and
  `header_field`.
- Crates.io metadata: `readme`, `keywords`, `categories`, `documentation`,
  `homepage`.
- Fuzz targets for `parse_midi1` and `Ump::from_bytes` (`../fuzz`).

### Changed

- **Hardening:** `SysexReassembler` caps unterminated chunk streams at
  1024 chunks / 1 MiB, returning `ControlError::InvalidData` instead of
  growing unbounded.

## [0.1.0] — initial release

### Added

- **MIDI 1.0** (`midi1`): `Midi1Message` for all channel and system
  messages; real-time-safe `parse_midi1` with strict validation (leading
  status byte, 7-bit data, defined statuses, F0…F7 SysEx); allocation-free
  `write_bytes`/`encoded_len`/`to_bytes`; `parse_midi1_prefix` for walking
  byte streams; recommended-scaling constants (`scale_7_to_16`,
  `scale_16_to_7`, `scale_7_to_32`, `scale_32_to_7`, `scale_16_to_32`,
  `scale_32_to_14`, `scale_14_to_32`).
- **SysEx** (`sysex`): universal-message builder (`sysex_universal`), GM
  System On/Off, MIDI Identity Request, sub-ID constants, `parse_sysex`
  views (manufacturer, device id, sub-ids, payload, universal flag),
  payload chunking, 7-bit validation.
- **UMP / MIDI 2.0** (`ump`, `messages`): `Ump` parse/encode for message
  types 0x0 (Utility: NoOp/Clock/Timestamp), 0x1 (System: MTC quarter
  frame, song position/select, tune request, clock/transport,
  active sensing, reset), 0x2 (MIDI 1.0 CV), 0x3 (sysex7 chunks with
  status/byte-count encoding and exact 7-bit packing), 0x4 (all 15
  MIDI 2.0 CV opcodes: Per-Note RCC/ACC, RPN/NRPN and Relative variants,
  Per-Note Pitch Bend, Note On/Off with attributes + 16-bit velocity,
  Poly Pressure, CC 32-bit, Program Change with bank-valid bit and
  bank MSB/LSB, Channel Pressure, Pitch Bend 32-bit, Per-Note
  Management), 0x5/0xE/0xF (extended data, lossless), and 0xD (Flex Data:
  Set Tempo, Time Signature, text, lossless other).
  Word-count validation, trailing-garbage rejection, big-endian byte
  conversion (`from_bytes`/`to_bytes`), `message_to_umps` chunking,
  `SysexReassembler`.
- **Translation layer**: `midi1_to_midi2` (velocity-0 note-on becomes
  NoteOff; pitch bend uses the 14-bit range; SysEx payload chunked into
  sysex7) and `midi2_to_midi1` with recommended downscaling; stable
  round-trips for all channel messages.
- **MIDI-CI** (`ci`): `CiMessage` Discovery / Discovery Reply /
  Endpoint Inquiry / Endpoint Info / NAK / ACK with 28-bit MUID packing,
  plus `CiDiscoverySession` (broadcast request, peer collection, endpoint
  inquiry). Property-exchange sub-IDs surface `ControlError::Unsupported`.
- **Device I/O** (`device`, `port`): `enumerate_devices` (input/output
  ports merged by name into `MidiDevice`s), `open_input`/`open_output`
  over `midir`, `MidiInput` blocking/non-blocking/timed receive with
  `InboundMidi` (raw bytes + parsed message), `MidiOutput::send_midi1` /
  `send_ump` / `send_raw`, `MidiSource`/`MidiSink` traits, and
  `VirtualMidiPair` in-memory loopback.
- **Clock/transport** (`clock`): `MidiClockSender` (24 PPQ, tempo-derived
  tick interval, start/stop/continue, song position), `MidiClockReceiver`
  (transport state machine, exponentially-smoothed BPM estimate,
  `observe_all` drain).
- **MIDI Time Code** (`mtc`): quarter-frame encode/decode with piece-index
  nibbles and rate bits, `MtcDecoder` assembly into `Timecode`, Universal
  Real-Time full-frame SysEx build/parse.
- **MIDI Show Control** (`msc`): GO/GO OFF/STOP/FIRE/RESET/GO JAM with cue
  numbers and cue lists, command-format constants, encode/parse
  round-trips, cue-character validation.
- **Parameter mapping** (`mapping`): `MidiParameterMapper` with CC
  (range+curve) and momentary note bindings; velocity-0 note-on treated
  as note-off.
- Conformance suite (`tests/conformance.rs`) and hardware integration
  test (`tests/hardware.rs`, gated on `TPT_MIDI_IN_PORT` /
  `TPT_MIDI_OUT_PORT`).

[0.1.0]: https://github.com/tpt-solutions/tpt-av-control/releases/tag/v0.1.0
