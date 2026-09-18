//! Property-based "never panics" fuzzing for the OSC parser entry point,
//! using the shared `tpt-av-test-fuzz` proptest harness (see todo.md Phase 9).
//!
//! This complements (does not replace) the `cargo-fuzz` targets under
//! `fuzz/`: those run corpus-guided fuzzing under nightly and must be
//! invoked manually, while this runs under plain `cargo test` and therefore
//! participates in ordinary CI.

use proptest::prelude::*;
use tpt_av_control_osc::parse_osc_message;
use tpt_av_test_fuzz::fuzz_parser_never_panics;

proptest! {
    #[test]
    fn parse_osc_message_never_panics(data in proptest::collection::vec(any::<u8>(), 0..256)) {
        fuzz_parser_never_panics!(parser: parse_osc_message, input: &data);
    }
}
