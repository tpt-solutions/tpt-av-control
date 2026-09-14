# Contributing to tpt-av-control

Thank you for contributing to the TPT AV stack's studio protocol suite.

## Licensing

This project is dual-licensed under the **MIT License** or the **Apache License 2.0**, at your option ([LICENSE-MIT](LICENSE-MIT) / [LICENSE-APACHE](LICENSE-APACHE)).

By contributing code, you agree that your contributions will be dual-licensed under the same terms (**MIT OR Apache-2.0**). You retain copyright; no CLA is required.

## Dependency rules

This project must remain free of copyleft contamination:

- **Allowed licenses:** MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib.
- **Banned licenses:** GPL, LGPL, AGPL, MPL — in *any* dependency, transitively included.
- `rtmidi` and `portmidi` (GPL) are banned; MIDI I/O uses `midir` and all protocol logic is written in-crate.

This is enforced by [`deny.toml`](deny.toml) in CI (`cargo deny check`). If your change needs a new dependency, prefer no dependency; otherwise justify it in your PR.

## Protocol correctness

- Implement protocols from their official specifications (OSC 1.0/1.1, MIDI 1.0, M2-101-U UMP, MIDI-CI, Art-Net 4, ANSI E1.31).
- Add conformance tests against the spec vectors when you touch a protocol module.

## Real-time safety

Code that runs on, or feeds, an audio/video callback must be:

- **Allocation-free** — no `Vec`/`String` growth, no boxing on the hot path.
- **Lock-free** — use atomics or the provided `SpscRing`; never take a mutex on the RT thread.
- **Non-blocking** — no syscalls, no I/O on the RT thread.

Network and device I/O belong on dedicated threads or async tasks.

## Development workflow

```sh
cargo fmt --all            # format before committing
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check licenses bans
```

CI runs all of the above on Linux, macOS, and Windows. Please make sure it passes locally first.

## Pull requests

1. Keep PRs focused; one protocol or feature per PR where practical.
2. Add tests for behavior changes and new protocol paths.
3. Update `CHANGELOG.md` under the *Unreleased* section.
4. Document public API items; crates are `#![deny(missing_docs)]`-style documented.
