# Changelog

All notable changes to `tpt-av-control-webrtc` are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning:
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Doc-tested usage example for `ControlEnvelope::encode`/`decode`.
- Crates.io metadata: `readme`, `keywords`, `categories`, `documentation`,
  `homepage`.
- Fuzz target for `ControlEnvelope::decode` (`../fuzz`).

### Changed

- **Hardening:** `ControlEnvelope::decode` uses checked arithmetic for all
  length+offset slicing, so hostile length prefixes return
  `ControlError::InvalidData` instead of overflowing on 32-bit targets.

## [0.1.0] — initial release

### Added

- `ControlEnvelope` (parameter change / transport command / text) with a
  compact binary codec: 1 type byte + NUL-terminated parameter name +
  tagged value (float/int/bool/length-prefixed string), command byte
  (`Play`/`Stop`/`Pause`/`Continue`/`ClockTick`), or length-prefixed
  UTF-8 text; exact round-trips, truncation and unknown-type rejection
  via `ControlError::InvalidData` (no panics on hostile input).
- `ParameterChangeRequest` carrying `tpt-av-control-utils`
  `ParameterId`/`ParameterValue`.
- `json` feature rendering envelopes as JSON strings for browser peers.
- `DataChannelTransport` trait (`send`/`recv`/`recv_timeout`) as the
  integration seam for any ICE/DTLS/SCTP WebRTC stack.
- `LoopbackTransport::pair()` — reliable, ordered in-process transport
  pair for tests and local IPC — and `try_drain` for non-blocking
  receive-side draining.
- Tests: binary round-trips for every variant, truncation tables,
  loopback ordering/reliability, drain and channel-close semantics.

[0.1.0]: https://github.com/tpt-solutions/tpt-av-control/releases/tag/v0.1.0
