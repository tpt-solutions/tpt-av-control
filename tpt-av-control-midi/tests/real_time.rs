//! Real-time-safety (zero heap allocation) checks for the hot MIDI 1.0/UMP
//! parse paths, using the shared `tpt-av-test-benchmark` harness (see
//! todo.md Phase 9). Runs under plain `cargo test`.

use tpt_av_control_midi::{parse_midi1, Ump};
use tpt_av_test_benchmark::assert_real_time_safe;

#[test]
fn parse_midi1_is_allocation_free() {
    let note_on: &[u8] = &[0x90, 0x3C, 0x40];
    let parsed = assert_real_time_safe!("parse_midi1", {
        parse_midi1(note_on).expect("valid MIDI 1.0 message must parse")
    });
    let _ = parsed;
}

#[test]
fn ump_from_bytes_is_allocation_free() {
    let no_op: &[u8] = &[0x00, 0x00, 0x00, 0x00];
    let parsed = assert_real_time_safe!("Ump::from_bytes", {
        Ump::from_bytes(no_op).expect("valid UMP word must parse")
    });
    let _ = parsed;
}
