#![no_main]
//! Invariant: `Ump::from_bytes` + `parse` either succeeds or returns `Err` — never panics.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(ump) = tpt_av_control_midi::Ump::from_bytes(data) {
        let _ = ump.parse();
    }
});
