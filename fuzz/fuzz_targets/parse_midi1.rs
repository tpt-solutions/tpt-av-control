#![no_main]
//! Invariant: `parse_midi1` either succeeds or returns `Err` — never panics.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_av_control_midi::parse_midi1(data);
});
