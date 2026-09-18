# Changelog

All notable changes to `tpt-av-control-dmx` are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning:
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Doc-tested usage examples for `DmxUniverse::set_channel` and
  `Fixture::patch`.
- Crates.io metadata: `readme`, `keywords`, `categories`, `documentation`,
  `homepage`.
- Fuzz targets for `artnet::parse_packet` and `sacn::parse_packet`
  (`../fuzz`).

### Fixed

- **Hardening:** `Sacn::recv_universe` now parses only the received
  datagram (`&buf[..len]`) instead of the whole buffer, which could leak
  stale bytes from a previous, longer packet into the parse.
- **Hardening:** `Fixture` writes re-check universe bounds defensively, so
  mutating the public `start_address` after patching can no longer panic
  or write out of range.

## [0.1.0] — initial release

### Added

- `DmxUniverse` (`dmx`): 512-channel universe with zero-based
  `set_channel`/`get_channel` (panicking and `try_` variants), out-of-range
  reads returning 0, bulk `set_channels` with truncation at the universe
  end, `is_blackout`/`clear` helpers.
- **Art-Net 4** (`artnet`): ArtDmx build/parse with the exact wire layout
  (`Art-Net\0` header, little-endian opcode `0x5000`, protocol version 14,
  sequence, physical, SubUni/Net universe, big-endian length, 512 slots);
  minimal ArtPoll parse and ArtPollReply build (node announcement with
  port types); `ArtNet` UDP server with `send_universe_to`,
  `recv_universe` (answers polls), and the `ARTNET_PORT` constant.
- **sACN / ANSI E1.31** (`sacn`): 638-byte data packets — root layer
  (`ASC-E1.17` identifier, `0x7` flag-length encoding, root vector, 16-byte
  `Cid`), framing layer (64-byte source name, priority defaulting to 100,
  sync address, per-packet sequence, options, universe), DMP layer
  (`SET_PROPERTY`, address type `0xA1`, start code 0x00, 512 slots);
  packet parsing with framing offsets and truncation rejection; `Sacn` UDP
  server with source identity override; `SACN_PORT` constant.
- **Server** (`server`): `DmxServer` auto-detecting Art-Net vs sACN on one
  socket, `DmxProtocol` tagging, merged `UniverseManager` state,
  `recv_universe`/`run` loops.
- **Client** (`client`): `DmxClient` with protocol multiplexing,
  per-protocol sequence counters, sACN source identity override.
- **Fixtures** (`fixture`): `FixtureChannel` roles (dimmer, RGB, white,
  pan/tilt, generic), `FixtureDefinition` profiles (`dimmer`, `rgb`,
  `rgbw`, or arbitrary channel lists with count validation), `Fixture`
  patching with universe-bounds validation, `set_intensity`/`set_color`/
  `set_position` writers that skip absent channels, and per-channel
  read-back.
- **Universe management** (`universe`): `UniverseManager` with on-demand
  creation, merge, replacement, ordered iteration.
- Tests: unit conformance vectors for both protocols and fixtures,
  `tests/loopback.rs` end-to-end show tests over real UDP for both
  protocols, and `tests/hardware.rs` env-gated visual chase.

[0.1.0]: https://github.com/tpt-solutions/tpt-av-control/releases/tag/v0.1.0
