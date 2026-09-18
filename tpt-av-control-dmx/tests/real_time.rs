//! Real-time-safety (zero heap allocation) checks for the hot DMX channel
//! read/write paths, using the shared `tpt-av-test-benchmark` harness (see
//! todo.md Phase 9). Runs under plain `cargo test`.

use tpt_av_control_dmx::DmxUniverse;
use tpt_av_test_benchmark::assert_real_time_safe;

#[test]
fn dmx_set_and_get_channel_are_allocation_free() {
    let mut universe = DmxUniverse::new(1);
    assert_real_time_safe!("DmxUniverse::set_channel/get_channel", {
        universe.set_channel(0, 255);
        universe.set_channel(511, 128);
        assert_eq!(universe.get_channel(0), 255);
        assert_eq!(universe.get_channel(511), 128);
    });
}
