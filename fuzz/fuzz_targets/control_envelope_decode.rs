#![no_main]
//! Invariant: `ControlEnvelope::decode` either succeeds or returns `Err` — never panics.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_av_control_webrtc::ControlEnvelope::decode(data);
});
