//! Fuzz harness crate for the `tpt-av-control` protocol parsers.
//!
//! Every parser that accepts untrusted network bytes gets a target here;
//! the invariant under test is identical everywhere: **parse or return
//! `Err` — never panic, never hang**. Run with
//! `cargo +nightly fuzz run <target>` (see the `[[bin]]` targets in
//! `Cargo.toml`).
