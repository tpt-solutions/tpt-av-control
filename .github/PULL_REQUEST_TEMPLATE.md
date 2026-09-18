## Summary

<!-- What does this PR change, and why? -->

## Changes

-

## Checklist

- [ ] `just ci` (or `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo test --doc --workspace && cargo deny check`) passes
- [ ] Protocol behavior is validated against the official specification (OSC 1.0/1.1, M2-101-U, M2-115, ANSI E1.31, Art-Net 4), with tests added
- [ ] Real-time paths stay allocation-free and lock-free (no new `unsafe` outside `tpt-av-control-utils::ring`)
- [ ] New dependencies are permissively licensed (MIT/Apache-2.0/BSD/ISC/Zlib) — `deny.toml` enforces this
- [ ] Public API items are documented; per-crate `CHANGELOG.md` updated under *Unreleased*
