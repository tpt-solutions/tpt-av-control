default:
    @just --list

# Format all code
fmt:
    cargo fmt --all

# Check formatting without writing
fmt-check:
    cargo fmt --all -- --check

# Lint with clippy (CI settings)
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Run the full test suite
test:
    cargo test --workspace

# Run doc tests only
doctest:
    cargo test --doc --workspace

# Build docs (warnings are errors, like CI)
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# cargo-deny: licenses + bans + advisories + sources
deny:
    cargo deny check

# Everything CI runs
ci: fmt-check clippy test doctest doc deny

# Package a crate as crates.io would (dry run; e.g. `just package tpt-av-control-osc`)
package crate:
    cargo package -p {{crate}} --allow-dirty

# Run an example binary (e.g. `just run osc_server`)
run binary *args:
    cargo run -p tpt-av-control-examples --bin {{binary}} {{args}}

# Run the avctl debugging CLI (e.g. `just avctl osc monitor 0.0.0.0:9000`)
avctl *args:
    cargo run -p avctl -- {{args}}

# Run one cargo-fuzz target for a bounded time (e.g. `just fuzz parse_osc_message 30`)
fuzz target seconds="60":
    cargo +nightly fuzz run {{target}} -- -max_total_time={{seconds}}
