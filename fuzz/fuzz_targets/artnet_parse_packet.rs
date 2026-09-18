#![no_main]
//! Invariant: `artnet::parse_packet` either succeeds or returns `Err` — never panics.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_av_control_dmx::artnet::parse_packet(data);
});
