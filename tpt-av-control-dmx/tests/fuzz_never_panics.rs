//! Property-based "never panics" fuzzing for the DMX/Art-Net/sACN parser
//! entry points, using the shared `tpt-av-test-fuzz` proptest harness (see
//! todo.md Phase 9).
//!
//! This complements (does not replace) the `cargo-fuzz` targets under
//! `fuzz/`: those run corpus-guided fuzzing under nightly and must be
//! invoked manually, while this runs under plain `cargo test` and therefore
//! participates in ordinary CI.

use proptest::prelude::*;
use tpt_av_control_dmx::{artnet, sacn, DmxUniverse};
use tpt_av_test_fuzz::fuzz_parser_never_panics;

/// `DmxUniverse::set_channel` panics by design on an out-of-range channel
/// (see its doc comment); `try_set_channel` is the fallible, never-panics
/// equivalent, so that's what gets the never-panics fuzz treatment.
fn dmx_try_set_channel_never_panics((channel, value): (u16, u8)) {
    let mut universe = DmxUniverse::new(1);
    let _ = universe.try_set_channel(channel, value);
}

fn dmx_get_channel_never_panics(channel: u16) {
    let universe = DmxUniverse::new(1);
    let _ = universe.get_channel(channel);
}

proptest! {
    #[test]
    fn artnet_parse_packet_never_panics(data in proptest::collection::vec(any::<u8>(), 0..600)) {
        fuzz_parser_never_panics!(parser: artnet::parse_packet, input: &data);
    }

    #[test]
    fn sacn_parse_packet_never_panics(data in proptest::collection::vec(any::<u8>(), 0..700)) {
        fuzz_parser_never_panics!(parser: sacn::parse_packet, input: &data);
    }

    #[test]
    fn dmx_try_set_channel_never_panics_prop(channel in any::<u16>(), value in any::<u8>()) {
        fuzz_parser_never_panics!(parser: dmx_try_set_channel_never_panics, input: (channel, value));
    }

    #[test]
    fn dmx_get_channel_never_panics_prop(channel in any::<u16>()) {
        fuzz_parser_never_panics!(parser: dmx_get_channel_never_panics, input: channel);
    }
}
