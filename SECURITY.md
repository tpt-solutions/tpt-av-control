# Security Policy

## Supported versions

| Version | Supported |
| :--- | :--- |
| 0.1.x | security fixes (best effort, pre-1.0) |

## Reporting a vulnerability

**Do not open a public issue for security reports.**

Email **security@tptsolutions.co.nz** with:

- the affected crate(s) and version (`tpt-av-control-osc`, `-midi`, `-dmx`,
  `-surface`, `-webrtc`, `-utils`),
- a minimal reproduction (packet hex dumps are ideal — this suite is
  network-facing, so hostile-input reports are very welcome),
- your assessment of impact, if you have one.

You will receive an acknowledgment within 7 days. We aim to release a fix
within 90 days of a confirmed report, and will credit reporters in the
release notes unless you prefer otherwise.

## Hardening posture

This suite parses untrusted network input by design, so the following
guarantees are maintained in CI:

- **No panics on hostile input** — protocol parsers (`parse_osc_message`,
  `OscBundle::decode`, `parse_midi1`, `Ump::from_bytes`,
  `artnet::parse_packet`, `sacn::parse_packet`, `ControlEnvelope::decode`)
  return `Err` on malformed data. Exhaustive truncation tests verify every
  byte-prefix of valid packets fails cleanly.
- **Bounded memory** — chunk reassembly (`SysexReassembler`) and bundle
  nesting (`MAX_BUNDLE_DEPTH`) are capped.
- **Bounded CPU** — OSC address-pattern matching runs under a step budget;
  hostile pattern/address pairs degrade to "no match" rather than hanging.
- **`forbid(unsafe_code)`** everywhere except the documented SPSC ring.
- **Dependency policy** — `cargo-deny` blocks copyleft and known-vulnerable
  crates in CI.

## Scope notes

- Device/HID transports (e.g. the Stream Deck `HidDevice` impl you supply)
  and WebRTC stacks plugged into `tpt-av-control-webrtc` are out of scope
  for this policy — report issues in those dependencies upstream.
- These crates do not implement cryptographic transports; for network
  control across untrusted links, tunnel OSC/Art-Net over a VPN or use a
  DTLS-capable WebRTC data channel.
