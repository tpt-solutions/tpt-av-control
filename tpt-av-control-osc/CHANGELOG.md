# Changelog

All notable changes to `tpt-av-control-osc` are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning:
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Doc-tested usage examples for `OscMessage::new` and
  `OscAddressMatcher::matches`.
- Crates.io metadata: `readme`, `keywords`, `categories`, `documentation`,
  `homepage`.

### Changed

- **Hardening:** `OscAddressMatcher::matches` now runs under a fixed
  match-step budget (10,000 steps), so pathological pattern/address pairs
  degrade to "no match" instead of unbounded backtracking.

## [0.1.0] — initial release

### Added

- OSC 1.0/1.1 message model: `OscMessage` (owned), `OscArg` (all standard
  tags: `i`, `f`, `s`, `b`, `h`, `t`, `d`, `S`, `c`, `r`, `m`, `T`, `F`,
  `N`, `I`), type-tag string derivation (`,`-prefixed), and address
  validation (leading `/`, printable ASCII, no spaces).
- Byte-level primitives (`types`): big-endian scalar readers, NUL-terminated
  string reads with 4-byte-aligned padding, `padded_len`/padding writers.
- Spec-exact encoding (`encode`, `encode_into`) matching the canonical
  OSC 1.0 example packet byte-for-byte.
- Real-time-safe zero-copy parser `parse_osc_message` returning
  `OscMessageRef` (borrowed address/tags/strings/blobs, on-demand argument
  access, owned conversion); rejects every truncated prefix of a valid
  packet with `InvalidData` instead of garbage.
- `OscBundle` / `OscPacket` with nested bundles, per-element length
  prefixes, NTP time tags, and the "immediate" time tag (wire `1`)
  round-tripping as `None`; `MAX_BUNDLE_DEPTH` guards hostile nesting.
- `OscServer` (UDP): blocking `run`/`recv_packet` and tokio `run_async`,
  recursive bundle expansion to the handler, non-blocking mode toggle,
  `parse_bytes` helper; `OscClient` (UDP): `send`, `send_bundle`,
  `send_raw` with byte counts.
- `OscAddressMatcher`: OSC pattern language — `?`, `*` (single part),
  `[...]` classes with `!` negation and ranges, `{a,b}` alternation, and the
  OSC 1.1 `//` recursive-parts wildcard; `OscDispatcher` with
  replace-on-add routes and multiple-match delivery, bundle expansion.
- Conformance suite (`tests/conformance.rs`): spec example packets, padding
  rule, tag-string rule, immediate/scheduled time tags, bundle nesting,
  full-argument round-trip, address validation, exhaustive truncation
  rejection, and pattern-matching vectors.
- UDP loopback tests covering the blocking and async server paths and
  bundle delivery (async test uses `spawn_blocking` so a
  current-thread runtime is not starved).

[0.1.0]: https://github.com/tpt-solutions/tpt-av-control/releases/tag/v0.1.0
