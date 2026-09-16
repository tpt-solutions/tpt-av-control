//! Conformance tests for the OSC implementation, validated against the
//! OSC 1.0 specification and known-good reference encodings.

use tpt_av_control_osc::address::OscAddressMatcher;
use tpt_av_control_osc::bundle::{OscBundle, OscPacket};
use tpt_av_control_osc::message::{parse_osc_message, OscArg, OscArgRef, OscMessage};
use tpt_av_control_osc::types::padded_len;

/// OSC 1.0 §"OSC Message": the canonical example packet for
/// "/oscillator/4/frequency" with float 440.0.
#[test]
fn spec_example_oscillator_frequency() {
    let message = OscMessage::new("/oscillator/4/frequency", &[OscArg::Float(440.0)]).unwrap();
    let bytes = message.encode();
    assert_eq!(bytes.len(), 32);
    assert_eq!(&bytes[..4], b"/osc");
    assert_eq!(&bytes[24..28], b",f\0\0");
    assert_eq!(&bytes[28..], &[0x43, 0xDC, 0x00, 0x00]); // 440.0 BE
    let parsed = parse_osc_message(&bytes).unwrap();
    assert_eq!(parsed.address(), "/oscillator/4/frequency");
    assert_eq!(parsed.arg(0), Some(OscArgRef::Float(440.0)));
}

/// Every OSC-string and OSC-blob is padded to a multiple of 4 bytes.
#[test]
fn padding_rule() {
    for len in 0..=9usize {
        assert_eq!(padded_len(len), len.div_ceil(4) * 4);
    }
    // Strings encode with their NUL inside the padded region.
    let m = OscMessage::new("/p", &[OscArg::String("ab".into())]).unwrap();
    let bytes = m.encode();
    // address "/p" + pad = 4, tags ",s" + pad = 4, "ab" + pad = 4, total 12.
    assert_eq!(bytes.len(), 12);
}

/// Type tag string always begins with `,` and matches the arguments.
#[test]
fn type_tag_string() {
    let m = OscMessage::new(
        "/tags",
        &[
            OscArg::Int(1),
            OscArg::Float(2.0),
            OscArg::String("s".into()),
            OscArg::Blob(vec![0]),
            OscArg::Long(5),
            OscArg::Time(6),
            OscArg::Double(7.0),
            OscArg::Bool(true),
            OscArg::Bool(false),
            OscArg::Nil,
            OscArg::Inf,
        ],
    )
    .unwrap();
    assert_eq!(m.type_tags(), ",ifsbhtdTFNI");
    let bytes = m.encode();
    let parsed = parse_osc_message(&bytes).unwrap();
    assert_eq!(parsed.type_tags(), "ifsbhtdTFNI");
}

/// Timetag 1 means "immediate" and is preserved as None.
#[test]
fn bundle_immediate_and_scheduled() {
    let immediate = OscBundle::new(None, vec![]);
    assert_eq!(immediate.encode()[8..16], 1u64.to_be_bytes());

    let scheduled = OscBundle::new(Some(0x83AA_7E80_0000_0000), vec![]);
    assert_eq!(
        OscBundle::decode(&scheduled.encode()).unwrap().timestamp,
        Some(0x83AA_7E80_0000_0000)
    );
}

/// Bundles may contain bundles, each element length-prefixed.
#[test]
fn bundle_nesting_conformance() {
    let inner = OscBundle::new(
        Some(10),
        vec![OscPacket::Message(
            OscMessage::new("/in", &[OscArg::Int(2)]).unwrap(),
        )],
    );
    let outer = OscBundle::new(
        Some(20),
        vec![
            OscPacket::Message(OscMessage::new("/out", &[]).unwrap()),
            OscPacket::Bundle(inner.clone()),
        ],
    );
    let decoded = OscBundle::decode(&outer.encode()).unwrap();
    assert_eq!(decoded, outer);
    match &decoded.elements[1] {
        OscPacket::Bundle(b) => {
            assert_eq!(b.timestamp, Some(10));
            assert_eq!(b.elements.len(), 1);
        }
        other => panic!("expected nested bundle, got {other:?}"),
    }
}

/// A message with every argument type round-trips bit-exactly.
#[test]
fn full_argument_roundtrip() {
    let args = vec![
        OscArg::Int(-123456),
        OscArg::Float(-0.125),
        OscArg::String("string arg".into()),
        OscArg::Blob((0..=255u8).cycle().take(1000).collect()),
        OscArg::Long(-9_876_543_210),
        OscArg::Time(0x1234_5678_9ABC_DEF0),
        OscArg::Double(std::f64::consts::E),
        OscArg::Symbol("a-symbol".into()),
        OscArg::Char('~'),
        OscArg::Color(0xDEAD_BEEF),
        OscArg::Midi(0x80_24_00_7F),
        OscArg::Bool(true),
        OscArg::Bool(false),
        OscArg::Nil,
        OscArg::Inf,
    ];
    let m = OscMessage::new("/everything", &args).unwrap();
    let decoded = OscMessage::decode(&m.encode()).unwrap();
    assert_eq!(decoded, m);
}

/// Addresses must start with '/' and use printable ASCII.
#[test]
fn address_validation_rules() {
    for good in ["/", "/a", "/track/1/volume", "/x-tab_y"] {
        assert!(OscMessage::new(good, &[]).is_ok(), "{good}");
    }
    for bad in ["", "no-slash", "/spa ce", "/tab\tx"] {
        assert!(OscMessage::new(bad, &[]).is_err(), "{bad}");
    }
}

/// Truncating a valid packet at any byte must fail parsing, never panic or
/// produce garbage.
#[test]
fn truncation_is_always_rejected() {
    let m = OscMessage::new(
        "/trunc",
        &[
            OscArg::String("abcd".into()),
            OscArg::Float(1.0),
            OscArg::Blob(vec![9; 7]),
        ],
    )
    .unwrap();
    let bytes = m.encode();
    for len in 0..bytes.len() {
        assert!(
            OscMessage::decode(&bytes[..len]).is_err(),
            "truncation to {len} bytes should fail"
        );
    }
    let b = OscBundle::new(None, vec![OscPacket::Message(m)]);
    let bb = b.encode();
    for len in 0..16usize {
        assert!(
            OscBundle::decode(&bb[..len]).is_err(),
            "bundle truncation to {len} bytes should fail"
        );
    }
    // 16 bytes is the valid empty-bundle prefix (marker + timetag); skip it.
    for len in 17..bb.len() {
        assert!(
            OscBundle::decode(&bb[..len]).is_err(),
            "bundle truncation to {len} bytes should fail"
        );
    }
}

/// Pattern matching vectors from the OSC 1.0 spec plus common practice.
#[test]
fn address_pattern_conformance() {
    let cases: &[(&str, &str, bool)] = &[
        // (pattern, address, expected)
        ("/f?o/bar", "/foo/bar", true),
        ("/f?o/bar", "/fo/bar", false),
        ("/f?o/bar", "/fwoo/bar", false),
        ("/f*o/bar", "/foo/bar", true),
        ("/f*o/bar", "/foooooo/bar", true),
        ("/f*o/bar", "/f/bar", false),
        ("/*", "/anything", true),
        ("/*", "/any/thing", false),
        ("/foo/[0-9]bar", "/foo/1bar", true),
        ("/foo/[0-9]bar", "/foo/xbar", false),
        ("/foo/[!0-9]bar", "/foo/xbar", true),
        ("/foo/[!0-9]bar", "/foo/5bar", false),
        ("/foo/{bar,baz}", "/foo/baz", true),
        ("/foo/{bar,baz}", "/foo/quux", false),
        ("/f[a-cx]o", "/fxo", true),
        ("/f[a-cx]o", "/fdo", false),
        ("/", "/", true),
    ];
    for (pattern, address, expected) in cases {
        assert_eq!(
            OscAddressMatcher::new(pattern).matches(address),
            *expected,
            "pattern {pattern} vs address {address}"
        );
    }
}

/// The zero-copy parser must produce the same values as the owned decoder.
#[test]
fn zero_copy_matches_owned() {
    let m = OscMessage::new(
        "/compare",
        &[
            OscArg::String("text".into()),
            OscArg::Blob(vec![1, 2]),
            OscArg::Double(9.5),
        ],
    )
    .unwrap();
    let bytes = m.encode();
    let borrowed = parse_osc_message(&bytes).unwrap();
    assert_eq!(borrowed.to_owned(), m);
    // Borrowed accessors don't allocate: addresses/tags are &str views.
    assert_eq!(borrowed.address(), "/compare");
    assert_eq!(borrowed.type_tags(), "sbd");
}
