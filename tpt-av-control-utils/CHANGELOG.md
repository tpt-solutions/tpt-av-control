# Changelog

All notable changes to `tpt-av-control-utils` are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning:
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Doc-tested usage examples for `SpscRing`, `Curve::apply`, `Mapping::map`,
  and `Timecode::from_frames` (`cargo test --doc`).
- Crates.io metadata: `readme`, `keywords`, `categories`, `documentation`,
  `homepage`.

## [0.1.0] — initial release

### Added

- `ControlError` enum shared across the suite (I/O, invalid data, unsupported,
  device/port errors, timeout, closed, out-of-range, invalid address, buffer
  too small, queue full), with `From<io::Error>` / `From<AddrParseError>`.
- `Message` envelope (`MessageBody`: raw protocol bytes, parameter change,
  transport command, locate, text; `MessageSource`: internal, network peer,
  MIDI port) and `MessageQueue` — a bounded, lock-free, wait-free SPSC ring
  (`SpscRing`) for crossing into real-time threads. The ring contains the
  workspace's only `unsafe` code (documented SPSC algorithm with
  acquire/release ordering) and drops pending items correctly on `Drop`.
- SMPTE-style `Timecode` with 24/25/30-ndf/29.97-df `FrameRate` support:
  correct drop-frame counting (2 frames dropped each minute except every
  tenth), total-frame conversion in both directions, virtual-label
  normalization, and `Display` (`HH:MM:SS:FF` / `HH:MM:SS;FF`).
- `Timestamp` in nanoseconds with 64-bit NTP (OSC time tag) conversion and
  the `NTP_IMMEDIATE` constant.
- Parameter mapping primitives: `ParameterId` (dotted paths),
  `ParameterValue` (float/int/bool/string), `Mapping` between arbitrary
  input/output ranges, transfer `Curve` (linear, ease-in/out/in-out,
  exponential, logarithmic) with clamped monotonic application, and
  keyframe `Automation` timelines with per-segment curves and hold-at-ends
  evaluation.
- Optional `serde` feature for the mapping and value types.

[0.1.0]: https://github.com/tpt-solutions/tpt-av-control/releases/tag/v0.1.0
