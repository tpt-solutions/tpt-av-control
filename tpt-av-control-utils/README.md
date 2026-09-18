# tpt-av-control-utils

**Shared types, errors, timing, and parameter mapping for the TPT AV control suite.**

Part of [`tpt-av-control`](../README.md) — the pure-Rust, real-time safe studio
protocol suite (OSC, MIDI 2.0, DMX, hardware control).

**License:** MIT OR Apache-2.0 · **MSRV:** 1.75 · **Status:** pre-1.0

---

## Overview

This crate is the dependency-free foundation every other `tpt-av-control-*`
crate builds on. It contains the types that cross protocol and thread
boundaries:

- **`ControlError`** — one error enum shared by the whole suite (I/O, invalid
  protocol data, device/port errors, range checks, bounded-queue overflow).
- **`Message` + `MessageQueue`** — a protocol-neutral control-message envelope
  (`MessageBody`, `MessageSource`, `Protocol`) and a bounded, lock-free
  **`SpscRing`** for handing messages from network/MIDI threads to the
  real-time audio/video thread.
- **Timecode & timing** — SMPTE-style `Timecode` (24/25/30 ndf/29.97 df) with
  correct drop-frame counting and inverse, `FrameRate`, and `Timestamp` with
  64-bit NTP conversion (the OSC time-tag format).
- **Parameter mapping** — `ParameterId` (dotted paths), `ParameterValue`,
  transfer `Curve`s (linear, ease, exponential, logarithmic), `Mapping` between
  input/output ranges, and keyframe `Automation` timelines with per-segment
  curves.

## Installation

```toml
[dependencies]
tpt-av-control-utils = "0.1"
```

Optional feature: `serde` — derives `Serialize`/`Deserialize` on the mapping
and value types.

## Usage

### Bridging a network thread to a real-time thread

```rust
use tpt_av_control_utils::{Message, MessageBody, MessageQueue, ParameterId, ParameterValue, MessageSource};

let queue: MessageQueue = MessageQueue::new(1024); // rounded to a power of two

// Producer (network thread; may allocate):
let msg = Message::now(
    1,
    MessageSource::Internal,
    MessageBody::Parameter {
        id: ParameterId::new("track.1.volume"),
        value: ParameterValue::Float(0.75),
    },
);
queue.push(msg).unwrap();

// Consumer (audio thread): pop() never allocates, locks, or blocks.
if let Some(msg) = queue.pop() {
    // apply the parameter change — allocation-free
}
```

When the queue is full, `push` returns `ControlError::QueueFull` and the
*caller* decides the drop policy — the ring never blocks either side.

### Timecode (including 29.97 drop-frame)

```rust
use tpt_av_control_utils::{FrameRate, Timecode};

let tc = Timecode::new(1, 2, 3, 4, FrameRate::Fps24).unwrap();
assert_eq!(tc.to_string(), "01:02:03:04");

// Drop-frame counting matches SMPTE: the first minute holds 1798 frames.
let df = Timecode::new(0, 1, 0, 2, FrameRate::Fps2997Df).unwrap();
assert_eq!(df.to_frames(), 1800);

// Inverse is total: every frame count maps back onto a real label.
assert_eq!(Timecode::from_frames(17_982, FrameRate::Fps2997Df).to_string(), "00:10:00;00");
```

`Timestamp` converts to/from 64-bit NTP timestamps (OSC time tags) and
provides `Timestamp::now()`.

### Parameter mapping and automation

```rust
use tpt_av_control_utils::parameter::{Automation, AutomationPoint, Curve, Mapping};

// A MIDI CC (0..127) onto a filter cutoff (20 Hz .. 20 kHz), exponential.
let m = Mapping {
    input_range: (0.0, 127.0),
    output_range: (20.0, 20_000.0),
    curve: Curve::Exponential { rate: 3.0 },
};
assert!((m.map(127.0) - 20_000.0).abs() < 0.1);

// Keyframe automation with per-segment curves.
let mut auto = Automation::new();
auto.add_point(AutomationPoint { time: 0.0, value: 0.0, curve: Curve::EaseIn });
auto.add_point(AutomationPoint { time: 2.0, value: 1.0, curve: Curve::EaseIn });
assert!((auto.value_at(1.0).unwrap() - 0.25).abs() < 1e-6); // t² at midpoint
```

## Real-time safety

`SpscRing::push`/`pop` are wait-free and allocation-free; the ring contains
the **only `unsafe` code in the workspace** (a bounded SPSC algorithm with
acquire/release ordering), clearly marked in `ring.rs`. Everything else in
this crate is safe Rust.

## Testing

```sh
cargo test -p tpt-av-control-utils
```

Covers error rendering, ring ordering/overflow/wraparound/threaded transfer/
drop accounting, timecode round-trips and SMPTE drop-frame checkpoints, NTP
round-trips, curve monotonicity, and automation interpolation.

## License

Dual-licensed under MIT OR Apache-2.0 — see [LICENSE-MIT](../LICENSE-MIT) and
[LICENSE-APACHE](../LICENSE-APACHE). See also the root
[CHANGELOG](../CHANGELOG.md) and per-crate [CHANGELOG](CHANGELOG.md).
