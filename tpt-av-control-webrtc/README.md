# tpt-av-control-webrtc

**WebRTC data channels for networked AV control — envelope codec plus a pluggable transport.**

Part of [`tpt-av-control`](../README.md) — the pure-Rust, real-time safe studio
protocol suite (OSC, MIDI 2.0, DMX, hardware control).

**License:** MIT OR Apache-2.0 · **MSRV:** 1.75 · **Status:** pre-1.0

---

## Overview

This crate defines the *control plane* for remote control over WebRTC:

- **[`ControlEnvelope`]** — the message multiplexed over a data channel:
  a parameter change, a transport command (`Play`/`Stop`/`Pause`/
  `Continue`/`ClockTick`), or free-form text.
- **Binary codec** — a compact, dependency-free encoding (1 type byte +
  payload) that round-trips exactly and rejects every truncated or
  unknown-typed input. Optional `json` feature renders envelopes as JSON
  for browser peers.
- **[`DataChannelTransport`] trait** — the seam where any WebRTC stack
  (ICE/DTLS/SCTP) plugs in. The suite ships no SCTP implementation by
  design; it stays dependency-light and lets you pick a WebRTC crate (or
  bridge to a browser via your own signaling) without pulling it into
  every consumer.
- **[`LoopbackTransport`]** — a reliable, ordered in-process pair for
  tests, demos, and local IPC, plus a `try_drain` helper.

[`ControlEnvelope`]: https://docs.rs/tpt-av-control-webrtc/latest/tpt_av_control_webrtc/envelope/enum.ControlEnvelope.html
[`DataChannelTransport`]: https://docs.rs/tpt-av-control-webrtc/latest/tpt_av_control_webrtc/transport/trait.DataChannelTransport.html
[`LoopbackTransport`]: https://docs.rs/tpt-av-control-webrtc/latest/tpt_av_control_webrtc/transport/struct.LoopbackTransport.html

## Installation

```toml
[dependencies]
tpt-av-control-webrtc = "0.1"
# JSON rendering for browser peers:
tpt-av-control-webrtc = { version = "0.1", features = ["json"] }
```

## Usage

### Encode and decode envelopes

```rust
use tpt_av_control_utils::parameter::{ParameterId, ParameterValue};
use tpt_av_control_utils::TransportCommand;
use tpt_av_control_webrtc::{ControlEnvelope, ParameterChangeRequest};

let envelope = ControlEnvelope::Parameter(ParameterChangeRequest {
    parameter: ParameterId::new("track.1.volume"),
    value: ParameterValue::Float(0.75),
});

let bytes = envelope.encode();        // compact binary, ready for SCTP
let decoded = ControlEnvelope::decode(&bytes)?;
assert_eq!(decoded, envelope);

let cmd = ControlEnvelope::Command(TransportCommand::Play);
let _ = cmd.encode();
```

### Send and receive over a data channel

```rust
use std::time::Duration;
use tpt_av_control_webrtc::{
    ControlEnvelope, DataChannelTransport, LoopbackTransport, TransportCommand,
};

// Swap `LoopbackTransport::pair()` for your WebRTC data channel
// implementation of DataChannelTransport.
let (mut remote_a, mut remote_b) = LoopbackTransport::pair();

remote_a.send(&ControlEnvelope::Command(TransportCommand::Play))?;
let received = remote_b.recv()?;          // blocking
assert_eq!(received, ControlEnvelope::Command(TransportCommand::Play));

// Non-blocking drain on the receiving side.
remote_a.send(&ControlEnvelope::Text("cue 42 GO".into()))?;
// tpt_av_control_webrtc::try_drain(&mut remote_b)? collects pending envelopes.
```

### Wiring a real WebRTC stack

Implement the trait over your chosen stack's reliable/ordered channel and
the rest of this crate is unchanged:

```rust,ignore
impl DataChannelTransport for MySctpChannel {
    fn send(&mut self, envelope: &ControlEnvelope) -> Result<(), ControlError> {
        self.send_raw(&envelope.encode())
    }
    fn recv(&mut self) -> Result<ControlEnvelope, ControlError> { /* ... */ }
    fn recv_timeout(&mut self, t: Duration) -> Result<Option<ControlEnvelope>, ControlError> { /* ... */ }
}
```

Envelopes decode into `tpt_av_control_utils` types
(`ParameterId`/`ParameterValue`/`TransportCommand`), so a remote change
lands in the same `MessageBody::Parameter` flow as OSC/MIDI control.

## Testing

```sh
cargo test -p tpt-av-control-webrtc
```

Covers binary round-trips for every envelope variant (all four parameter
value types, all five transport commands, text), truncation and unknown-type
rejection, loopback reliability/ordering, and drain semantics including
channel-close behavior.

## Design notes

- The binary format is deliberately trivial: one type byte, then a
  NUL-terminated parameter name and a tagged value, a command byte, or a
  length-prefixed UTF-8 string. Every decoder error is
  `ControlError::InvalidData` — no panics on hostile input.
- Parameter semantics (ranges, curves) live in `tpt-av-control-utils`; this
  crate is only the wire shape.

## License

Dual-licensed under MIT OR Apache-2.0 — see [LICENSE-MIT](../LICENSE-MIT) and
[LICENSE-APACHE](../LICENSE-APACHE). See also the per-crate
[CHANGELOG](CHANGELOG.md).
