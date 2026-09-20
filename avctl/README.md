# avctl

**A small command-line tool for sending and monitoring OSC, MIDI, and DMX
traffic on-site.**

Part of [`tpt-av-control`](../README.md). This is a debugging aid for
integration work and on-site troubleshooting — checking that a console is
sending OSC where you expect, watching MIDI CCs arrive from a controller,
or confirming a lighting desk's Art-Net/sACN output. It is **not** a
production show-control tool: no persistence, scripting, cueing, or
scheduling.

Not published to crates.io; run it from a checkout of this workspace.

## Usage

```sh
cargo run -p avctl -- <command>
```

### OSC

```sh
# Send one message. Arguments are type:value (i = int32, f = float32, s = string).
cargo run -p avctl -- osc send 127.0.0.1:9000 /track/1/volume f:0.75

# Print every OSC message received on a bound address.
cargo run -p avctl -- osc monitor 0.0.0.0:9000
```

### MIDI

```sh
# List devices and their input/output port indices.
cargo run -p avctl -- midi list

# Print every decoded message from an input port (see `midi list` for the index).
cargo run -p avctl -- midi monitor 0
```

### DMX / Art-Net / sACN

```sh
# Bind and print a summary of changed channels per received universe.
cargo run -p avctl -- dmx sniff --protocol artnet 6454
cargo run -p avctl -- dmx sniff --protocol sacn 5568
```

## Scope

Only `tpt-av-control-osc`, `-midi`, and `-dmx` are wired up. Control-surface
(`-surface`) and WebRTC (`-webrtc`) debugging aren't covered yet — send a
raw `ControlEnvelope` or drive a `ControlSurface` directly from a short
Rust snippet in the meantime.

## Security

Like the crates it wraps, `avctl` sends/receives OSC, Art-Net, and sACN in
the clear — only use `osc monitor`/`dmx sniff` on networks you trust. See
[SECURITY.md](../SECURITY.md).

## License

Dual-licensed under MIT OR Apache-2.0 — see [LICENSE-MIT](../LICENSE-MIT) and
[LICENSE-APACHE](../LICENSE-APACHE).
