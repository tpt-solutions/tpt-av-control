#![no_main]
//! Invariant: `OscBundle::decode` either succeeds or returns `Err` — never panics,
//! including under deep or lying element-size nesting.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_av_control_osc::OscBundle::decode(data);
});
