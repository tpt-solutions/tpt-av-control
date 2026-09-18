#![no_main]
//! Invariant: `sacn::parse_packet` either succeeds or returns `Err` — never panics.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_av_control_dmx::sacn::parse_packet(data);
});
