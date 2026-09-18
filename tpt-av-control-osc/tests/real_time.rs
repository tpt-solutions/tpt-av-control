//! Real-time-safety (zero heap allocation) checks for the hot OSC parse
//! path, using the shared `tpt-av-test-benchmark` harness (see todo.md
//! Phase 9). Runs under plain `cargo test`.

use tpt_av_control_osc::{parse_osc_message, OscArg, OscMessage};
use tpt_av_test_benchmark::assert_real_time_safe;

#[test]
fn parse_osc_message_is_allocation_free() {
    let message = OscMessage::new("/track/1/volume", &[OscArg::Float(0.75), OscArg::Int(3)])
        .expect("valid OSC message");
    let encoded = message.encode();

    let parsed = assert_real_time_safe!("parse_osc_message", {
        parse_osc_message(&encoded).expect("valid buffer must parse")
    });
    assert_eq!(parsed.address(), "/track/1/volume");
}
