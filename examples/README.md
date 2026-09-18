# tpt-av-control-examples

**Runnable demos for the TPT AV control suite.**

This package is `publish = false` — it exists so every crate ships with a
demonstration you can run locally, and so CI compiles the examples as part
of `cargo build --workspace --all-targets`.

**License:** MIT OR Apache-2.0 · **MSRV:** 1.75

```sh
cargo run -p tpt-av-control-examples --example <name> [args]
```

## Examples

### `osc_server`

Receives OSC over UDP (default port 8000), routes `/track/*/volume`
messages through a pattern dispatcher, and prints everything else raw.

```sh
cargo run -p tpt-av-control-examples --bin osc_server -- [port]
# then send OSC to udp://127.0.0.1:8000 from any OSC app
```

Demonstrates: `OscServer`, `OscDispatcher`, `OscAddressMatcher` patterns,
argument handling.

### `midi_controller`

Lists MIDI devices, opens the chosen input (index from `argv[1]`, default
0), maps CC 7 → `master.volume` and CC 1 → `synth.vibrato` through
`MidiParameterMapper`, and echoes messages back for LED feedback.

```sh
cargo run -p tpt-av-control-examples --bin midi_controller -- [in-port-index]
```

Demonstrates: `enumerate_devices`, `open_input`/`open_output`,
`MidiSource`/`MidiSink`, parameter mapping.

### `dmx_lighting`

Patches four RGBW PARs and streams a color chase as sACN (default) or
Art-Net to a console/interface.

```sh
TPT_PROTOCOL=artnet cargo run -p tpt-av-control-examples --bin dmx_lighting -- [target-ip]
```

Demonstrates: `Fixture::patch`, `FixtureDefinition::rgbw`, `DmxUniverse`,
`DmxClient` with protocol multiplexing, standard ports.

### `control_surface`

Drives a closure-built `CustomSurface` over the `VirtualMidiPair`
in-memory loopback (no hardware needed), then attaches a live
`GenericMidiSurface` if a bidirectional MIDI device is present.

```sh
cargo run -p tpt-av-control-examples --bin control_surface
```

Demonstrates: `CustomSurface::builder`, `ControlEvent`/`Feedback`,
`GenericMidiSurface` with `SurfaceMapping`.

### `spsc_pipeline`

The real-time architecture from `DESIGN.md` §5.3 end to end: an OSC
server thread feeds a bounded lock-free `MessageQueue`, and a stand-in
audio thread pops messages without allocating or blocking. Runs for a
fixed duration, then exits.

```sh
cargo run -p tpt-av-control-examples --bin spsc_pipeline [seconds]
```

Demonstrates: `MessageQueue`/`SpscRing`, `Message`/`MessageBody`,
drop policy on overflow, the thread split from
[INTEGRATION.md](../INTEGRATION.md).

## Environment variables

| Variable | Used by | Meaning |
| :--- | :--- | :--- |
| `TPT_PROTOCOL` | `dmx_lighting` | `sacn` (default) or `artnet` |
| `RUST_LOG` | all (via `env_logger`) | e.g. `RUST_LOG=debug` |

The crates' own hardware integration tests use `TPT_MIDI_IN_PORT`,
`TPT_MIDI_OUT_PORT`, `TPT_DMX_TARGET`, and `TPT_DMX_PROTOCOL` — see the
per-crate READMEs under [`../tpt-av-control-*`](..).

## License

Dual-licensed under MIT OR Apache-2.0 — see [LICENSE-MIT](../LICENSE-MIT)
and [LICENSE-APACHE](../LICENSE-APACHE).
