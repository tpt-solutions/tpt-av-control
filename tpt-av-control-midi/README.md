# tpt-av-control-midi

**MIDI 1.0 and MIDI 2.0 (Universal MIDI Packet) — parsing, translation, device I/O, clock, MTC, MSC, MIDI-CI.**

Part of [`tpt-av-control`](../README.md) — the pure-Rust, real-time safe studio
protocol suite (OSC, MIDI 2.0, DMX, hardware control).

**License:** MIT OR Apache-2.0 · **MSRV:** 1.75 · **Status:** pre-1.0

---

## Overview

- **MIDI 1.0** — `parse_midi1` / `Midi1Message` for every channel and system
  message, with allocation-free encoding into caller buffers
  (`write_bytes`); strict validation (7-bit data bytes, defined statuses,
  SysEx terminators). SysEx helpers build Universal messages, GM on/off,
  identity requests, and chunk long dumps.
- **MIDI 2.0 / UMP (M2-101-U)** — full `Ump` parse/encode for Utility,
  System Common/Real-Time, MIDI 1.0 CV-in-UMP, sysex7 data chunks, and
  **all 15 MIDI 2.0 channel-voice opcodes**: Per-Note RCC/ACC,
  RPN/NRPN and the Relative variants, Per-Note Pitch Bend, Note On/Off with
  attributes and 16-bit velocity, Poly Pressure, CC (32-bit), Program Change
  with bank select, Channel Pressure, 32-bit Pitch Bend, Per-Note
  Management — plus extended data and Flex Data (Set Tempo, Time Signature,
  text).
- **MIDI 1.0 ↔ 2.0 translation** — `midi1_to_midi2` / `midi2_to_midi1` with
  the recommended scalings (7↔16↔32-bit full scale, 14-bit pitch bend),
  velocity-0 note-on → note-off handling, and a `SysexReassembler` that
  reassembles chunked sysex7 packets.
- **MIDI-CI (capability negotiation)** — `CiMessage` for Discovery /
  Discovery Reply, Endpoint Inquiry / Endpoint Info, NAK/ACK, and a
  `CiDiscoverySession` that runs the discovery handshake.
- **Device I/O** — `enumerate_devices` / `open_input` / `open_output`
  wrapping `midir` (MIT), with blocking, non-blocking, and timed receive,
  plus a `VirtualMidiPair` in-memory loopback so applications and tests run
  hardware-free. `MidiSource`/`MidiSink` traits accept any transport.
- **Clock & transport** — `MidiClockSender` (tempo-aware 24-PPQ ticks,
  start/stop/continue, song position) and `MidiClockReceiver` (transport
  state tracking with an exponentially-smoothed BPM estimate).
- **MIDI Time Code** — quarter-frame encoding/decoding (`MtcDecoder`
  assembles the eight nibbles into a `Timecode`) and Universal Real-Time
  full-frame SysEx round-trips.
- **MIDI Show Control** — `MscMessage` (GO, GO OFF, STOP, FIRE, RESET,
  GO JAM; cue numbers and cue lists) with encode/parse round-trips.
- **Parameter mapping** — `MidiParameterMapper` binds CCs and notes onto
  stack parameters through `tpt-av-control-utils` mappings.

## Installation

```toml
[dependencies]
tpt-av-control-midi = "0.1"
```

## Usage

### Parsing a byte stream (real-time safe)

```rust
use tpt_av_control_midi::{parse_midi1, Midi1Message};

let bytes = [0x90, 0x3C, 0x40]; // Note On, channel 1, note 60, velocity 64
match parse_midi1(&bytes)? {
    Midi1Message::NoteOn { channel, note, velocity } => { /* ... */ }
    other => { /* ... */ }
}
```

### MIDI 2.0: build and parse UMPs

```rust
use tpt_av_control_midi::{
    Midi2ChannelVoice, Midi2Message, Ump,
};

let message = Midi2Message::Midi2ChannelVoice(Midi2ChannelVoice::NoteOn {
    group: 0, channel: 3, note: 60,
    attribute_type: 0, attribute: 0,
    velocity: 0x8000, // 16-bit, half scale
});
let ump = Ump::from_message(&message);
assert_eq!(ump.to_bytes(), [0x40, 0x93, 0x3C, 0x00, 0x80, 0x00, 0x00, 0x00]);
let back = ump.parse()?; // → Midi2Message
```

### Translating MIDI 1.0 to MIDI 2.0 (and back)

```rust
use tpt_av_control_midi::{midi1_to_midi2, midi2_to_midi1, Midi1Message};

let note_on = Midi1Message::NoteOn { channel: 0, note: 60, velocity: 64 };
let ump_messages = midi1_to_midi2(&note_on, /* group */ 0);
let back_to_midi1 = midi2_to_midi1(&ump_messages[0]);
assert_eq!(back_to_midi1[0], note_on);
```

### Device I/O

```rust
use tpt_av_control_midi::{
    enumerate_devices, open_input, open_output, MidiPort, PortDirection, PortId,
    MidiSink, MidiSource, Midi1Message,
};

for (i, device) in enumerate_devices()?.iter().enumerate() {
    println!("[{i}] {}", device.name);
}

let input = open_input(&MidiPort::new(PortId::new(0), "in".into(), PortDirection::Input))?;
let mut output = open_output(&MidiPort::new(PortId::new(0), "out".into(), PortDirection::Output))?;

output.send_midi1(&Midi1Message::NoteOn { channel: 0, note: 60, velocity: 100 })?;
// input.recv() / input.try_recv() / input.recv_timeout(...) deliver InboundMidi
```

No hardware? Use the in-memory loopback:

```rust
use tpt_av_control_midi::VirtualMidiPair;

let pair = VirtualMidiPair::new();
let (mut source, mut sink) = pair.split();
sink.send_midi1(&Midi1Message::TimingClock)?;
let inbound = source.recv()?; // echo of what was sent
```

### Clock, MTC, MSC

```rust
use tpt_av_control_midi::{MidiClockSender, MidiClockReceiver, MscCommand, MscMessage};
use tpt_av_control_utils::time::{FrameRate, Timecode};
use tpt_av_control_midi::mtc::{quarter_frames, MtcDecoder};

// Clock: 120 BPM ticks over any MidiSink.
// let mut clock = MidiClockSender::new(Box::new(sink), 120.0);
// clock.start()?; clock.tick()?;

// MTC: eight quarter frames assemble into a timecode.
let tc = Timecode::new(3, 24, 51, 17, FrameRate::Fps25).unwrap();
let mut decoder = MtcDecoder::new();
for piece in quarter_frames(&tc) {
    decoder.feed(&piece);
}
assert_eq!(decoder.timecode, Some(tc));

// MSC: fire cue 12.5 on list 3 to every device.
let msc = MscMessage {
    device_id: 0x7F,
    command_format: tpt_av_control_midi::msc::FORMAT_LIGHTING,
    command: MscCommand::Go {
        cue: Some("12.5".into()),
        cue_list: Some("3".into()),
    },
};
let sysex = msc.to_midi1()?; // send over any MIDI output
```

### MIDI-CI discovery

```rust
use tpt_av_control_midi::{CiDiscoverySession, CiMessage, MidiSource};

let mut session = CiDiscoverySession::new(0x00AB_CDEF);
let request = session.discovery_request()?; // broadcast Discovery
// feed inbound SysEx messages through session.observe(&message);
```

## Feature overview

| Module | Contents |
| :--- | :--- |
| `midi1` | `Midi1Message`, `parse_midi1`, `encode_midi1`, 7/14/16/32-bit scalers |
| `sysex` | Universal SysEx builders, parsing views, chunking |
| `ump` | `Ump`, `message_to_umps`, `midi1_to_midi2`, `midi2_to_midi1`, `SysexReassembler` |
| `messages` | `Midi2Message` and all UMP message families, `DataFormat` |
| `ci` | `CiMessage`, `CiDiscoverySession` |
| `clock` | `MidiClockSender`, `MidiClockReceiver`, `TransportState` |
| `mtc` | quarter frames, full-frame SysEx, `MtcDecoder`, `MtcFrameRate` |
| `msc` | `MscMessage`, `MscCommand`, format constants |
| `device` | enumeration, `open_input`/`open_output`, `MidiSource`/`MidiSink`, `VirtualMidiPair` |
| `mapping` | `MidiParameterMapper`, `CcBinding`, `NoteBinding` |

## Conformance & testing

- `tests/conformance.rs` — UMP reference vectors (e.g. Note On
  `4093 3C00 8000 0000`), all-opcode round-trips, translation stability,
  200-byte SysEx chunk/reassemble, truncation rejection.
- `tests/hardware.rs` — real-hardware loopback, **skipped unless**
  `TPT_MIDI_IN_PORT` / `TPT_MIDI_OUT_PORT` name port indices.
- Unit tests cover parser strictness, SysEx views, CI round-trips, MTC
  rate-bit survival, MSC command tables, clock tempo math, and the virtual
  loopback.

## Real-time safety

`parse_midi1` (channel/system messages), `Midi1Message::write_bytes`, and
`Ump` parse/encode allocate nothing; SysEx parsing allocates its payload by
nature. Device I/O and the clock run on their own threads — hand results to
the RT thread via `tpt_av_control_utils::MessageQueue`.

## License

Dual-licensed under MIT OR Apache-2.0 — see [LICENSE-MIT](../LICENSE-MIT) and
[LICENSE-APACHE](../LICENSE-APACHE). See also the per-crate
[CHANGELOG](CHANGELOG.md).
