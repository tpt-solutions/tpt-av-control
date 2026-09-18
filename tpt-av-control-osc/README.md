# tpt-av-control-osc

**Open Sound Control 1.0/1.1 — pure Rust, real-time safe parse/encode, UDP server & client.**

Part of [`tpt-av-control`](../README.md) — the pure-Rust, real-time safe studio
protocol suite (OSC, MIDI 2.0, DMX, hardware control).

**License:** MIT OR Apache-2.0 · **MSRV:** 1.75 · **Status:** pre-1.0

---

## Overview

- **Complete OSC 1.0/1.1 wire format.** All standard type tags —
  `i32`, `f32`, OSC-string, OSC-blob, `i64`, OSC-timetag, `f64`, symbol,
  char, RGBA color, 4-byte MIDI, `T`/`F` booleans, `N`il, `I`nfinitum —
  with spec-correct 4-byte padding for strings and blobs, and bundles with
  nested bundles and the "immediate" time tag (wire value `1`).
- **Real-time-safe zero-copy parsing.** [`parse_osc_message`] validates and
  interprets a packet **without allocating**, returning an `OscMessageRef`
  that borrows the address, type tags, strings, and blobs directly from the
  packet buffer. Convert to an owned `OscMessage` once, off the RT thread.
- **UDP server & client.** `OscServer` runs a blocking receive loop
  (`run`) or a tokio-based one (`run_async`), expanding bundles recursively;
  `OscClient` sends messages, bundles, or raw packets.
- **Address pattern matching.** `OscAddressMatcher` implements the OSC
  pattern language — `?`, `*` (never crossing `/`), `[...]` classes with `!`
  negation and `a-z` ranges, `{a,b,c}` alternation, plus the OSC 1.1 `//`
  any-parts wildcard.
- **Message routing.** `OscDispatcher` binds patterns to handlers with
  OSC multiple-match semantics.

[`parse_osc_message`]: https://docs.rs/tpt-av-control-osc/latest/tpt_av_control_osc/message/fn.parse_osc_message.html

## Installation

```toml
[dependencies]
tpt-av-control-osc = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] } # for run_async
```

## Usage

### Sending

```rust
use tpt_av_control_osc::{OscArg, OscBundle, OscClient, OscMessage, OscPacket};
use std::net::SocketAddr;

let target: SocketAddr = "127.0.0.1:8000".parse()?;
let mut client = OscClient::new(target)?;

let msg = OscMessage::new("/track/1/volume", &[OscArg::Float(0.75)])?;
client.send(&msg)?;

let bundle = OscBundle::new(None, vec![OscPacket::Message(msg)]);
client.send_bundle(&bundle)?;
```

### Receiving (blocking)

```rust
use tpt_av_control_osc::{OscArg, OscDispatcher, OscMessage, OscServer};

let mut server = OscServer::new(8000)?;
println!("listening on {}", server.local_addr()?);

let mut dispatcher = OscDispatcher::new();
dispatcher.add_route("/track/*/volume", |msg: OscMessage| {
    let value = msg.arguments.first().and_then(OscArg::as_f32);
    println!("{} → {value:?}", msg.address);
});

server.set_handler(move |msg: OscMessage, src| {
    if dispatcher.dispatch(msg.clone()) == 0 {
        println!("{src} sent unhandled {}", msg.address);
    }
});
server.run()?; // runs forever
```

### Receiving (async)

```rust
use tpt_av_control_osc::OscServer;

let mut server = OscServer::new(8000)?;
server.set_handler(|msg, _src| println!("{}", msg.address));
server.run_async().await?; // tokio
```

### Real-time-safe parsing

```rust
use tpt_av_control_osc::parse_osc_message;

fn on_packet(bytes: &[u8]) {
    // Borrows from `bytes`; no allocation — safe on the audio thread.
    if let Ok(parsed) = parse_osc_message(bytes) {
        let _addr: &str = parsed.address();
        let _tags: &str = parsed.type_tags();
        let _first = parsed.arg(0); // parsed on demand from the buffer
    }
}
```

### Pattern matching

```rust
use tpt_av_control_osc::OscAddressMatcher;

let m = OscAddressMatcher::new("/track/{1,2}/[fv]*");
assert!(m.matches("/track/1/volume"));
assert!(m.matches("/track/2/fader"));
assert!(!m.matches("/track/3/volume"));
```

## Conformance

`tests/conformance.rs` validates the implementation against the OSC 1.0
specification: the canonical `/oscillator/4/frequency` example packet, the
padding rule, the `,`-prefixed type tag string, bundle time tags and nesting,
full argument round-trips, address validation rules, exhaustive truncation
rejection (every prefix of a valid packet fails to parse — never garbage),
and a pattern-matching vector table.

## Real-time safety

`parse_osc_message`, `OscMessageRef`, and `OscMessage::write_bytes`-style
encoding into caller buffers perform no allocation. Network I/O
(`OscServer`/`OscClient`) may allocate and belongs off the RT thread; hand
results across with `tpt_av_control_utils::MessageQueue`.

## Testing

```sh
cargo test -p tpt-av-control-osc
```

Includes UDP loopback tests for both the blocking and async servers
(skipped-safe on CI without sockets where applicable), bundle expansion,
dispatcher routing, and fuzz-ish truncation tables.

## License

Dual-licensed under MIT OR Apache-2.0 — see [LICENSE-MIT](../LICENSE-MIT) and
[LICENSE-APACHE](../LICENSE-APACHE). See also the per-crate
[CHANGELOG](CHANGELOG.md).
