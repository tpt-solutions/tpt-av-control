# Changelog

All notable changes to `tpt-av-control-examples` are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This package is `publish = false`; it has no releases on crates.io.

## [Unreleased]

## [0.1.0] — initial set of demos

### Added

- `osc_server` — blocking UDP OSC server with dispatcher routing and raw
  printing.
- `midi_controller` — device listing, live input with CC → parameter
  mapping and message echo.
- `dmx_lighting` — patched RGBW rig with an sACN/Art-Net color chase.
- `control_surface` — virtual (loopback) custom surface plus live generic
  MIDI surface attachment.

[0.1.0]: https://github.com/tpt-solutions/tpt-av-control/releases/tag/v0.1.0
