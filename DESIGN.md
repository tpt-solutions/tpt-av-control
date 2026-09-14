# tpt-av-control — Design

**A pure-Rust, real-time safe studio protocol suite. OSC, MIDI 2.0, DMX, and hardware control. The integration layer of the TPT AV stack.**

**License:** MIT OR Apache-2.0
**Status:** Early-stage / Pre-1.0
**Ecosystem:** [TPT Solutions Open Source](https://opensource.tptsolutions.co.nz/)

---

## 1. Vision & Philosophy

`tpt-av-control` is the **protocol and hardware integration layer** of the TPT AV Stack. It provides pure-Rust, real-time safe implementations of studio control protocols used in audio, video, lighting, and live performance.

The current Rust ecosystem has **fragmented, incomplete, or blocking implementations** of these protocols:

- `rosc` (OSC) — Basic implementation, not optimized for real-time
- `midir` (MIDI) — Good I/O, but MIDI 1.0 only, no UMP support
- DMX/Art-Net crates — Minimal, not production-grade
- No unified protocol suite that works together in real-time audio/video applications

`tpt-av-control` fills this gap by providing:

1. **Unified protocol suite** — OSC, MIDI 2.0, DMX, Art-Net, sACN in one cohesive library
2. **Real-time safe** — Zero allocation, zero blocking in protocol processing
3. **MIDI 2.0 / UMP native** — Full support for Universal MIDI Packet, the next generation of MIDI
4. **Hardware control surfaces** — Integration with physical controllers (mixers, stream decks, etc.)
5. **Permissive dual licensing** — MIT OR Apache-2.0, zero copyleft contamination

### Core Tenets

1. **Pure Rust, Zero External Protocol Dependencies:** No C/C++ bindings. All protocol implementations are written from the specification in pure Rust.
2. **Real-Time Safe:** Protocol parsing and message handling are allocation-free and lock-free. Safe to call from audio callbacks.
3. **Async-First Architecture:** Network protocols (OSC, Art-Net, sACN) use async I/O for maximum throughput.
4. **Hardware Abstraction:** Unified API for different hardware control surfaces.
5. **Permissive Licensing:** No GPL/LGPL/MPL dependencies. Enforced via `cargo-deny` in CI.
6. **Composable Architecture:** Each sub-crate is independently useful. A developer can use just OSC, just MIDI, or just DMX.

---

## 2. Ecosystem Integration

`tpt-av-control` sits in the **Integration Layer** of the TPT AV Stack, providing external control and hardware integration for all processing engines.

| Crate | Role | Relationship to `tpt-av-control` |
| :--- | :--- | :--- |
| **`tpt-audio`** | Audio processing, timeline | Receives MIDI/OSC control messages to adjust parameters (volume, pan, effects). |
| **`tpt-visual`** | Video processing | Receives MIDI/OSC control messages for video effects, transitions, color grading. |
| **`tpt-av-control`** | **Studio protocols (this repo)** | Provides OSC, MIDI, DMX, and hardware control integration. |
| **External Hardware** | Physical controllers | Sends MIDI/OSC/DMX messages to control the TPT stack. |

### Data Flow

```text
External Hardware (MIDI controller, lighting console, OSC app)
    ↓
tpt-av-control (parses OSC/MIDI/DMX messages)
    ↓
Application (maps messages to parameters)
    ↓
tpt-audio / tpt-visual (applies parameter changes in real-time)
```

---

## 3. Repository Architecture (Cargo Workspace)

All sub-crates share the `tpt-av-control-` prefix for ecosystem coherence and clean namespace resolution on crates.io.

```text
tpt-av-control/                        # GitHub Repository / Workspace Root
├── Cargo.toml                         # Workspace manifest
├── deny.toml                          # cargo-deny license audit config
├── LICENSE-MIT
├── LICENSE-APACHE
├── README.md
├── DESIGN.md                          # This file
│
├── tpt-av-control-utils/              # Shared types, error handling
│   ├── src/
│   │   ├── lib.rs
│   │   ├── message.rs                 # Generic message types
│   │   ├── time.rs                    # Timecode and timing types
│   │   ├── parameter.rs               # Parameter mapping types
│   │   └── error.rs                   # `ControlError` enum
│   └── Cargo.toml
│
├── tpt-av-control-osc/                # Open Sound Control (OSC)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── message.rs                 # OSC message parsing and serialization
│   │   ├── bundle.rs                  # OSC bundle handling
│   │   ├── address.rs                 # OSC address pattern matching
│   │   ├── types.rs                   # OSC type tags (i32, f32, string, blob)
│   │   ├── server.rs                  # OSC server (UDP/TCP)
│   │   ├── client.rs                  # OSC client (UDP/TCP)
│   │   └── dispatch.rs                # Message routing and dispatch
│   ├── Cargo.toml
│   └── tests/
│
├── tpt-av-control-midi/               # MIDI 1.0 and MIDI 2.0 (UMP)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── midi1.rs                   # MIDI 1.0 message parsing
│   │   ├── ump.rs                     # Universal MIDI Packet (MIDI 2.0)
│   │   ├── messages.rs                # Note, CC, Pitch Bend, Program Change, etc.
│   │   ├── sysex.rs                   # System Exclusive messages
│   │   ├── device.rs                  # MIDI device enumeration and I/O
│   │   ├── port.rs                    # MIDI port management
│   │   ├── clock.rs                   # MIDI clock and transport
│   │   └── mapping.rs                 # MIDI to parameter mapping
│   ├── Cargo.toml
│   └── tests/
│
├── tpt-av-control-dmx/                # DMX512, Art-Net, sACN
│   ├── src/
│   │   ├── lib.rs
│   │   ├── dmx.rs                     # DMX512 universe data
│   │   ├── artnet.rs                  # Art-Net protocol (UDP)
│   │   ├── sacn.rs                    # sACN (E1.31) protocol (UDP)
│   │   ├── universe.rs                # DMX universe management
│   │   ├── server.rs                  # DMX/Art-Net/sACN server
│   │   ├── client.rs                  # DMX/Art-Net/sACN client
│   │   └── fixture.rs                 # Lighting fixture definitions
│   ├── Cargo.toml
│   └── tests/
│
├── tpt-av-control-surface/            # Hardware control surface integration
│   ├── src/
│   │   ├── lib.rs
│   │   ├── surface.rs                 # Control surface trait
│   │   ├── surfaces/                  # Specific hardware implementations
│   │   │   ├── generic_midi.rs        # Generic MIDI controller
│   │   │   ├── behringer_x32.rs       # Behringer X32/M32 console
│   │   │   ├── elgato_streamdeck.rs   # Elgato Stream Deck
│   │   │   └── custom.rs              # Custom surface definition
│   │   ├── mapping.rs                 # Surface to parameter mapping
│   │   └── feedback.rs                # LED/button feedback
│   ├── Cargo.toml
│   └── tests/
│
├── tpt-av-control-webrtc/             # WebRTC data channels (future)
│   ├── src/
│   ├── Cargo.toml
│   └── tests/
│
└── examples/                          # Demos and integration tests
    ├── osc_server.rs                  # OSC server receiving messages
    ├── midi_controller.rs             # MIDI controller integration
    ├── dmx_lighting.rs                # DMX lighting control
    └── control_surface.rs             # Hardware control surface demo
```

---

## 4. Core API Design

### 4.1. OSC Implementation (tpt-av-control-osc)

```rust
/// An OSC message.
pub struct OscMessage {
    /// OSC address pattern (e.g., "/track/1/volume").
    pub address: String,
    /// Type tags (e.g., "ff" for two floats).
    pub type_tags: String,
    /// Arguments (i32, f32, String, Vec<u8>).
    pub arguments: Vec<OscArg>,
}

/// An OSC argument value.
pub enum OscArg {
    Int(i32),
    Float(f32),
    String(String),
    Blob(Vec<u8>),
    Long(i64),
    Double(f64),
    Bool(bool),
    Nil,
    Inf,
}

/// An OSC bundle (collection of messages with a timestamp).
pub struct OscBundle {
    /// NTP timestamp (optional, None = immediate).
    pub timestamp: Option<u64>,
    /// Messages in the bundle.
    pub messages: Vec<OscMessage>,
}

/// OSC server for receiving messages.
pub struct OscServer { /* UDP socket + handler */ }

impl OscServer {
    /// Creates a new OSC server listening on the specified port.
    pub fn new(port: u16) -> Result<Self, ControlError>;
    /// Starts receiving messages (blocking).
    pub fn run(&mut self) -> Result<(), ControlError>;
    /// Starts receiving messages (async).
    pub async fn run_async(&mut self) -> Result<(), ControlError>;
}

/// OSC client for sending messages.
pub struct OscClient { /* UDP socket + target */ }

impl OscClient {
    /// Creates a new OSC client.
    pub fn new(target: SocketAddr) -> Result<Self, ControlError>;
    /// Sends an OSC message.
    pub fn send(&mut self, message: &OscMessage) -> Result<(), ControlError>;
    /// Sends an OSC bundle.
    pub fn send_bundle(&mut self, bundle: &OscBundle) -> Result<(), ControlError>;
}

/// OSC address pattern matcher.
pub struct OscAddressMatcher { /* pattern */ }

impl OscAddressMatcher {
    /// Creates a new address matcher.
    pub fn new(pattern: &str) -> Self;
    /// Checks if an address matches the pattern.
    pub fn matches(&self, address: &str) -> bool;
}
```

In addition to the owned `OscMessage`, the crate exposes a **zero-copy** `OscMessageRef<'a>` produced by the real-time-safe parser `parse_osc_message(&[u8])`, which borrows address/argument data directly from the packet buffer.

### 4.2. MIDI Implementation (tpt-av-control-midi)

```rust
/// MIDI 1.0 message.
pub enum Midi1Message {
    NoteOff { channel: u8, note: u8, velocity: u8 },
    NoteOn { channel: u8, note: u8, velocity: u8 },
    PolyphonicKeyPressure { channel: u8, note: u8, pressure: u8 },
    ControlChange { channel: u8, controller: u8, value: u8 },
    ProgramChange { channel: u8, program: u8 },
    ChannelPressure { channel: u8, pressure: u8 },
    PitchBendChange { channel: u8, value: u16 },
    SystemExclusive(Vec<u8>),
    TimeCodeQuarterFrame(u8),
    SongPositionPointer(u16),
    SongSelect(u8),
    TuneRequest,
    TimingClock,
    Start,
    Continue,
    Stop,
    ActiveSensing,
    SystemReset,
}

/// Universal MIDI Packet (MIDI 2.0).
pub struct Ump {
    /// Raw 128-bit packet.
    pub data: [u32; 4],
}

impl Ump {
    /// Parses a UMP packet.
    pub fn parse(data: [u32; 4]) -> Result<Midi2Message, ControlError>;
    /// Creates a UMP packet from a MIDI 2.0 message.
    pub fn from_message(message: &Midi2Message) -> Self;
}

/// MIDI 2.0 message.
pub enum Midi2Message {
    Utility(UtilityMessage),
    SystemCommon(SystemCommonMessage),
    SystemExclusive(SysExMessage),
    Midi1ChannelVoice(Midi1Message),   // MIDI 1.0 in UMP format
    Midi2ChannelVoice(Midi2ChannelVoice),
    DataMessage(DataMessage),
    FlexData(FlexDataMessage),
}

/// MIDI 2.0 channel voice message (high-resolution).
pub enum Midi2ChannelVoice {
    NoteOff { group: u8, channel: u8, note: u8, attribute: u16, velocity: u16 },
    NoteOn { group: u8, channel: u8, note: u8, attribute: u16, velocity: u16 },
    PolyphonicKeyPressure { group: u8, channel: u8, note: u8, pressure: u32 },
    ControlChange { group: u8, channel: u8, index: u8, value: u32 },
    ProgramChange { group: u8, channel: u8, option_flags: u8, program: u8,
                    bank_valid: bool, bank_msb: u8, bank_lsb: u8 },
    ChannelPressure { group: u8, channel: u8, pressure: u32 },
    PitchBend { group: u8, channel: u8, value: u32 },
    PerNoteRcc { group: u8, channel: u8, note: u8, index: u8, value: u32 },
    PerNoteAcc { group: u8, channel: u8, note: u8, index: u8, value: u32 },
    Rpn { group: u8, channel: u8, bank: u16, index: u16, value: u32 },
    Nrpn { group: u8, channel: u8, bank: u16, index: u16, value: u32 },
    RelativeRpn { group: u8, channel: u8, bank: u16, index: u16, value: u32 },
    RelativeNrpn { group: u8, channel: u8, bank: u16, index: u16, value: u32 },
    PerNoteManagement { group: u8, channel: u8, note: u8, option_flags: u8 },
}
```

Device I/O wraps `midir` (MIT) for transport while all parsing/encoding stays in-crate:

```rust
pub fn enumerate_devices() -> Result<Vec<MidiDevice>, ControlError>;
pub fn open_input(port: &MidiPort) -> Result<MidiInput, ControlError>;
pub fn open_output(port: &MidiPort) -> Result<MidiOutput, ControlError>;

impl MidiInput {
    /// Receives the next MIDI message (blocking).
    pub fn recv(&mut self) -> Result<MidiMessage, ControlError>;
    /// Receives the next MIDI message (non-blocking).
    pub fn try_recv(&mut self) -> Result<Option<MidiMessage>, ControlError>;
}

impl MidiOutput {
    /// Sends a MIDI 1.0 message.
    pub fn send_midi1(&mut self, message: &Midi1Message) -> Result<(), ControlError>;
    /// Sends a MIDI 2.0 UMP packet.
    pub fn send_ump(&mut self, ump: &Ump) -> Result<(), ControlError>;
}
```

### 4.3. DMX Implementation (tpt-av-control-dmx)

```rust
/// A DMX512 universe (512 channels).
pub struct DmxUniverse {
    /// Universe number.
    pub universe: u16,
    /// Channel values (0-255).
    pub channels: [u8; 512],
}

impl DmxUniverse {
    pub fn new(universe: u16) -> Self;
    pub fn set_channel(&mut self, channel: u16, value: u8);
    pub fn get_channel(&self, channel: u16) -> u8;
}

/// Art-Net protocol implementation (UDP port 6454).
pub struct ArtNet { /* UDP socket + universes */ }

impl ArtNet {
    pub fn new(port: u16) -> Result<Self, ControlError>;
    pub fn send_universe(&mut self, universe: &DmxUniverse) -> Result<(), ControlError>;
    pub fn recv_universe(&mut self) -> Result<DmxUniverse, ControlError>;
}

/// sACN (E1.31) protocol implementation (UDP port 5568).
pub struct Sacn { /* UDP socket + universes */ }

impl Sacn {
    pub fn new(port: u16) -> Result<Self, ControlError>;
    pub fn send_universe(&mut self, universe: &DmxUniverse) -> Result<(), ControlError>;
    pub fn recv_universe(&mut self) -> Result<DmxUniverse, ControlError>;
}
```

### 4.4. Control Surface Integration (tpt-av-control-surface)

```rust
/// A hardware control surface.
pub trait ControlSurface: Send {
    fn name(&self) -> &str;
    fn init(&mut self) -> Result<(), ControlError>;
    fn read_event(&mut self) -> Result<Option<ControlEvent>, ControlError>;
    fn send_feedback(&mut self, feedback: &Feedback) -> Result<(), ControlError>;
}

/// A control event from a surface.
pub enum ControlEvent {
    FaderMove { channel: u8, value: f32 },
    ButtonPress { button: u8 },
    ButtonRelease { button: u8 },
    EncoderRotate { encoder: u8, delta: i8 },
    TouchStrip { strip: u8, value: f32 },
}

/// Feedback to send to a control surface.
pub enum Feedback {
    Led { led: u8, on: bool },
    LedColor { led: u8, r: u8, g: u8, b: u8 },
    DisplayText { display: u8, text: String },
    FaderPosition { channel: u8, value: f32 },
}
```

---

## 5. Real-Time Safety Architecture

### 5.1. Protocol Processing (Real-Time Safe)

All protocol parsing and message handling is allocation-free and lock-free:

```rust
/// Parses an OSC message from raw bytes (real-time safe, zero-copy).
pub fn parse_osc_message(data: &[u8]) -> Result<OscMessageRef<'_>, ControlError>;

/// Parses a MIDI 1.0 message from raw bytes (real-time safe).
pub fn parse_midi1(data: &[u8]) -> Result<Midi1Message, ControlError>;
```

### 5.2. Network I/O (Async, Not Real-Time)

Network protocols (OSC, Art-Net, sACN) use async or blocking I/O off the real-time thread. Messages cross into the RT thread through a bounded, lock-free SPSC queue (`tpt_av_control_utils::ring::SpscRing`).

### 5.3. Thread Architecture

```text
Network Thread (Async, Can Allocate)
│  Receive OSC/Art-Net/sACN packets
│  Parse messages
│  Send to message queue
│
MIDI Thread (Blocking, Can Allocate)
│  Receive MIDI messages
│  Parse messages
│  Send to message queue
│
Audio/Video Thread (Real-Time Safe, Zero Allocation)
│  Read from message queue (lock-free)
│  Process messages
│  Apply parameter changes
```

---

## 6. Roadmap

- **Phase 1 — Foundation & OSC:** utils crate; OSC parsing/serialization; UDP server/client; address pattern matching; conformance tests.
- **Phase 2 — MIDI 1.0:** message parsing; device enumeration and I/O; clock/transport.
- **Phase 3 — MIDI 2.0 / UMP:** UMP parsing/serialization; high-resolution channel voice; MIDI-CI; MIDI 1.0 backward compatibility.
- **Phase 4 — DMX / Lighting:** DMX512 universes; Art-Net; sACN (E1.31); fixtures.
- **Phase 5 — Control Surfaces:** surface trait; generic MIDI; X32; Stream Deck; feedback.
- **Phase 6 — Advanced Features:** WebRTC data channels; MTC/MSC; automation curves.

---

## 7. Dependency & Licensing Rules

**Allowed dependencies (permissive only):**

- `tpt-av-control-*` (internal)
- `tokio` (MIT) — async runtime for network I/O
- `midir` (MIT/Apache-2.0) — MIDI I/O transport (wrapped by `tpt-av-control-midi`)
- `serde` (MIT/Apache-2.0) — serialization for configuration
- `log` (MIT/Apache-2.0) — logging facade

**Banned dependencies:**

- `rtmidi` (GPL), `portmidi` (GPL)
- Any crate with GPL, LGPL, AGPL, or MPL in its dependency tree

**Enforcement:** the [`deny.toml`](deny.toml) file enforces this automatically in CI (`cargo deny check`).

---

## 8. Contributing & License

This project is dual-licensed under the MIT License or the Apache License 2.0 (at your option). By contributing to `tpt-av-control`, you agree that:

- Your code will be dual-licensed under the same MIT OR Apache-2.0 terms.
- You will not introduce any dependency that is GPL, LGPL, AGPL, or MPL licensed.
- All protocol implementations must be validated against official protocol specifications.
- All real-time code must be allocation-free and lock-free.

This ensures `tpt-av-control` remains free, open, and unencumbered by copyleft restrictions for all future studio control software development.
