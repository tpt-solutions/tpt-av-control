//! Property-based "never panics" fuzzing for the MIDI 1.0/UMP parser entry
//! points, using the shared `tpt-av-test-fuzz` proptest harness (see
//! todo.md Phase 9).
//!
//! This complements (does not replace) the `cargo-fuzz` targets under
//! `fuzz/`: those run corpus-guided fuzzing under nightly and must be
//! invoked manually, while this runs under plain `cargo test` and therefore
//! participates in ordinary CI.

use proptest::prelude::*;
use tpt_av_control_midi::{parse_midi1, Ump};
use tpt_av_test_fuzz::fuzz_parser_never_panics;

/// Mirrors the `ump_from_bytes` cargo-fuzz target: `Ump::from_bytes` alone
/// doesn't exercise the message-body decoding in `Ump::parse`, so chain them.
fn ump_from_bytes_then_parse(data: &[u8]) {
    if let Ok(ump) = Ump::from_bytes(data) {
        let _ = ump.parse();
    }
}

proptest! {
    #[test]
    fn parse_midi1_never_panics(data in proptest::collection::vec(any::<u8>(), 0..64)) {
        fuzz_parser_never_panics!(parser: parse_midi1, input: &data);
    }

    #[test]
    fn ump_from_bytes_never_panics(data in proptest::collection::vec(any::<u8>(), 0..32)) {
        fuzz_parser_never_panics!(parser: ump_from_bytes_then_parse, input: &data);
    }
}
