# tpt-av-control — Ecosystem Integration Contract

How `tpt-audio` and `tpt-visual` consume control from this suite. This
document is the integration point to review when cross-checking against
those repositories (they live outside this workspace).

---

## Threading model

```text
Network Thread (OSC / Art-Net / sACN, async or blocking — may allocate)
MIDI Thread    (device I/O — may allocate)
        │  parse protocol → tpt-av-control-utils::Message
        ▼
   MessageQueue (SpscRing<Message>)  ← the only shared object
        │  pop(): wait-free, allocation-free
        ▼
Real-Time Thread (tpt-audio / tpt-visual callback: apply changes)
```

- Consumer crates create the `MessageQueue` (recommend capacity `1024`),
  hand a clone/receiver half to this suite's I/O threads, and `pop()` from
  the RT thread. `push` returns `ControlError::QueueFull` under overload —
  **the drop policy belongs to the producer**; the RT thread is never
  blocked.
- Protocol parsing itself (`parse_osc_message`, `parse_midi1`, `Ump`)
  is allocation-free where the message shape is fixed, so an RT thread may
  also parse directly from a ring of packet buffers if an application
  prefers that topology.

## Message contract (`tpt_av_control_utils`)

| Envelope | Meaning for tpt-audio / tpt-visual |
| :--- | :--- |
| `MessageBody::Parameter { id: ParameterId, value: ParameterValue }` | Set the parameter named by the dotted path (e.g. `track.1.volume`). `ParameterValue::Float/Int/Bool/String`; numeric views via `as_f32()`. |
| `MessageBody::Transport(TransportCommand)` | `Play`, `Stop`, `Pause`, `Continue`, `ClockTick` (24 PPQ MIDI clock). |
| `MessageBody::Locate(Timecode)` | Jump to an SMPTE position. |
| `MessageBody::Raw { protocol: Protocol, data: Vec<u8> }` | Uninterpreted protocol bytes (`Protocol::Osc/Midi1/Midi2/ArtNet/Sacn/WebRtc`) for consumer-specific handling. |
| `MessageBody::Text(String)` | Free-form labels / display updates. |
| `Message.id` / `Message.timestamp` / `Message.source` | Correlation, ordering diagnostics, and origin (`MessageSource::Network(SocketAddr)` / `MidiPort(name)` / `Internal`). |

`ParameterId` strings are the shared namespace between consoles and
engines. Recommended conventions: dotted lowercase segments
(`track.1.volume`, `master.gain`, `video.layer2.opacity`); the engine owns
range/curve resolution via `utils::Mapping`/`Automation`.

## Per-protocol mapping (what arrives as what)

- **OSC** — `/track/1/volume` with float args maps naturally onto
  `ParameterId::new("track.1.volume")`; the pattern dispatcher
  (`tpt-av-control-osc`) normalizes wildcards before mapping.
- **MIDI 1.0/2.0** — `MidiParameterMapper` binds CC/note numbers to
  parameter paths + ranges + curves; MIDI 2.0 32-bit CCs downscale per the
  recommended mapping when talking to MIDI 1.0 engines.
- **Transport & sync** — MIDI clock `Start/Stop/Continue/TimingClock`,
  MTC (`tpt-av-control-midi::mtc`, yields `Timecode` → `Locate`), and MSC
  cues arrive as `TransportCommand`/`Locate`/`Text` envelopes.
- **Lighting** — Art-Net/sACN universes stay in
  `tpt-av-control-dmx::UniverseManager`; engines that render lighting pull
  universe state directly (no `Message` round-trip needed).
- **WebRTC** — `ControlEnvelope::Parameter` decodes to the same
  `ParameterId`/`ParameterValue` pair as above; transport commands likewise.
- **Surfaces** — `ControlEvent`s are UI-facing; applications map them to
  parameters via `SurfaceMapping` and then enqueue `Message`s.

## Versioning & compatibility

- The `tpt-av-control-*` crates are 0.1.0 pre-1.0: integration points are
  contractual but may evolve. Engines should pin the exact minor version.
- Breaking changes to `Message`, `MessageBody`, `ParameterId`, or
  `SpscRing` semantics will be flagged in every crate's CHANGELOG.

## Checklist for tpt-audio / tpt-visual integration

1. Size the `MessageQueue` for worst-case burst (all faders moved in one
   frame), document the drop policy.
2. Reserve parameter-path namespaces per engine (`audio.*`, `video.*`).
3. Decide MIDI 1.0 vs 2.0 ingestion: either consume translated
   `Midi1Message`s or take UMPs and translate in-engine
   (`midi2_to_midi1` exists for the interim).
4. For MTC sync, consume `Locate(Timecode)` and quarter-frame streams with
   the same `FrameRate` as the engine timeline.
5. Load-test with the loopback transports (`VirtualMidiPair`,
   `LoopbackTransport`) before hardware trials.
