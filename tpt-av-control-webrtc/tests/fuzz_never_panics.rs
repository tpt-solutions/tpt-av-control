//! Property-based "never panics" fuzzing for `ControlEnvelope::decode`,
//! using the shared `tpt-av-test-fuzz` proptest harness (see todo.md
//! Phase 9).
//!
//! This complements (does not replace) the `cargo-fuzz` target under
//! `fuzz/`: that runs corpus-guided fuzzing under nightly and must be
//! invoked manually, while this runs under plain `cargo test` and therefore
//! participates in ordinary CI.

use proptest::prelude::*;
use tpt_av_control_webrtc::ControlEnvelope;
use tpt_av_test_fuzz::fuzz_parser_never_panics;

proptest! {
    #[test]
    fn control_envelope_decode_never_panics(data in proptest::collection::vec(any::<u8>(), 0..256)) {
        fuzz_parser_never_panics!(parser: ControlEnvelope::decode, input: &data);
    }
}
