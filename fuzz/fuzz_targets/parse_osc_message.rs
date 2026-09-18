#![no_main]
//! Invariant: `parse_osc_message` either succeeds or returns `Err` — never panics.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_av_control_osc::parse_osc_message(data);
});
