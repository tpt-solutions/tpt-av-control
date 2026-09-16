//! Conformance tests for UMP/MIDI 2.0 against M2-101-U reference
//! encodings, plus MIDI 1.0/2.0 translation checks.

use tpt_av_control_midi::messages::*;
use tpt_av_control_midi::midi1::{parse_midi1, Midi1Message};
use tpt_av_control_midi::ump::{midi1_to_midi2, midi2_to_midi1, SysexReassembler, Ump};
use tpt_av_control_midi::Midi2Message;

/// Note On reference vector (MIDI 2.0 CV, group 0, channel 3):
/// `4093 3C00 8000 0000` — note 60, velocity 0x8000 (half scale).
#[test]
fn ump_note_on_reference_vector() {
    let ump = Ump::from_message(&Midi2Message::Midi2ChannelVoice(
        Midi2ChannelVoice::NoteOn {
            group: 0,
            channel: 3,
            note: 60,
            attribute_type: 0,
            attribute: 0,
            velocity: 0x8000,
        },
    ));
    assert_eq!(ump.words[0], 0x4093_3C00);
    assert_eq!(ump.words[1], 0x8000_0000);
    let bytes = ump.to_bytes();
    assert_eq!(bytes, vec![0x40, 0x93, 0x3C, 0x00, 0x80, 0x00, 0x00, 0x00]);
    assert_eq!(Ump::from_bytes(&bytes).unwrap(), ump);
}

/// Control Change reference vector: group 2, channel 7, index 74,
/// full-scale value.
#[test]
fn ump_control_change_reference_vector() {
    let ump = Ump::from_message(&Midi2Message::Midi2ChannelVoice(
        Midi2ChannelVoice::ControlChange {
            group: 2,
            channel: 7,
            index: 74,
            value: 0xFFFF_FFFF,
        },
    ));
    assert_eq!(ump.words[0], 0x42B7_4A00);
    assert_eq!(ump.words[1], 0xFFFF_FFFF);
}

/// MIDI 1.0 CV wrapped in UMP stays one 32-bit word.
#[test]
fn ump_midi1_cv_word() {
    let ump = Ump::from_message(&Midi2Message::Midi1ChannelVoice(Midi1ChannelVoice {
        group: 0,
        message: Midi1Message::ControlChange {
            channel: 4,
            controller: 7,
            value: 100,
        },
    }));
    assert_eq!(ump.words[0], 0x20B4_0764);
    assert_eq!(ump.num_words(), 1);
}

/// All sixteen MIDI 2.0 CV opcodes round-trip through parse.
#[test]
fn all_midi2_opcodes_roundtrip() {
    let messages = vec![
        Midi2ChannelVoice::PerNoteRcc {
            group: 1,
            channel: 2,
            note: 60,
            index: 5,
            value: 0x1234_5678,
        },
        Midi2ChannelVoice::PerNoteAcc {
            group: 1,
            channel: 2,
            note: 60,
            index: 5,
            value: 0x8765_4321,
        },
        Midi2ChannelVoice::Rpn {
            group: 0,
            channel: 0,
            bank: 0,
            index: 0,
            value: 0x2000_0000,
        },
        Midi2ChannelVoice::Nrpn {
            group: 0,
            channel: 0,
            bank: 3,
            index: 100,
            value: 1,
        },
        Midi2ChannelVoice::RelativeRpn {
            group: 0,
            channel: 1,
            bank: 0,
            index: 0,
            value: 0xFFFF_0000,
        },
        Midi2ChannelVoice::RelativeNrpn {
            group: 0,
            channel: 1,
            bank: 0,
            index: 1,
            value: 0x0000_FFFF,
        },
        Midi2ChannelVoice::PerNotePitchBend {
            group: 0,
            channel: 2,
            note: 64,
            value: 0x8000_0000,
        },
        Midi2ChannelVoice::NoteOff {
            group: 0,
            channel: 2,
            note: 64,
            attribute_type: 0,
            attribute: 0,
            velocity: 0x4000,
        },
        Midi2ChannelVoice::NoteOn {
            group: 0,
            channel: 2,
            note: 64,
            attribute_type: 1,
            attribute: 3,
            velocity: 0xFFFF,
        },
        Midi2ChannelVoice::PolyphonicKeyPressure {
            group: 0,
            channel: 2,
            note: 64,
            pressure: 0x7FFF_FFFF,
        },
        Midi2ChannelVoice::ControlChange {
            group: 0,
            channel: 2,
            index: 1,
            value: 2,
        },
        Midi2ChannelVoice::ProgramChange {
            group: 0,
            channel: 2,
            option_flags: 0,
            program: 9,
            bank_valid: true,
            bank_msb: 1,
            bank_lsb: 2,
        },
        Midi2ChannelVoice::ChannelPressure {
            group: 0,
            channel: 2,
            pressure: 42,
        },
        Midi2ChannelVoice::PitchBend {
            group: 0,
            channel: 2,
            value: 0x8000_0000,
        },
        Midi2ChannelVoice::PerNoteManagement {
            group: 0,
            channel: 2,
            note: 64,
            option_flags: 0x03,
        },
    ];
    for cv in messages {
        let ump = Ump::from_message(&Midi2Message::Midi2ChannelVoice(cv));
        assert_eq!(ump.num_words(), 2, "opcode {:#x}", cv.opcode());
        let parsed = ump.parse().unwrap();
        match parsed {
            Midi2Message::Midi2ChannelVoice(back) => assert_eq!(back, cv),
            other => panic!("expected CV, got {other:?}"),
        }
    }
}

/// MIDI 1.0 → 2.0 → 1.0 translation is stable for every channel message.
#[test]
fn translation_stability() {
    let m1_messages = vec![
        Midi1Message::NoteOn {
            channel: 0,
            note: 60,
            velocity: 64,
        },
        Midi1Message::NoteOff {
            channel: 0,
            note: 60,
            velocity: 64,
        },
        Midi1Message::ControlChange {
            channel: 1,
            controller: 74,
            value: 100,
        },
        Midi1Message::ProgramChange {
            channel: 2,
            program: 42,
        },
        Midi1Message::ChannelPressure {
            channel: 3,
            pressure: 7,
        },
        Midi1Message::PolyphonicKeyPressure {
            channel: 4,
            note: 60,
            pressure: 90,
        },
        Midi1Message::PitchBendChange {
            channel: 5,
            value: 8192,
        },
        Midi1Message::PitchBendChange {
            channel: 5,
            value: 0,
        },
        Midi1Message::PitchBendChange {
            channel: 5,
            value: 16383,
        },
    ];
    for m1 in m1_messages {
        let up = midi1_to_midi2(&m1, 0);
        assert_eq!(up.len(), 1, "{m1:?}");
        let down = midi2_to_midi1(&up[0]);
        assert_eq!(down.len(), 1, "{m1:?}");
        assert_eq!(down[0], m1, "translation drift for {m1:?}");
    }
}

/// SysEx longer than six bytes chunks and reassembles exactly.
#[test]
fn sysex_chunking_reassembly() {
    let payload: Vec<u8> = (0..=127u8).cycle().take(200).collect();
    let m1 = Midi1Message::SystemExclusive(
        std::iter::once(0xF0)
            .chain(payload.iter().copied())
            .chain(std::iter::once(0xF7))
            .collect(),
    );
    let umps = midi1_to_midi2(&m1, 1);
    assert_eq!(umps.len(), 200usize.div_ceil(6));

    let mut reassembler = SysexReassembler::default();
    let mut reassembled = None;
    for message in &umps {
        match Ump::from_message(message).parse().unwrap() {
            Midi2Message::SystemExclusive(chunk) => {
                if let Some(all) = reassembler.feed(&chunk).unwrap() {
                    reassembled = Some(all);
                }
            }
            _ => panic!("expected sysex chunk"),
        }
    }
    assert_eq!(reassembled.unwrap(), payload);
}

/// Truncated UMPs never parse successfully.
#[test]
fn truncated_umps_rejected() {
    let ump = Ump::from_message(&Midi2Message::Midi2ChannelVoice(
        Midi2ChannelVoice::NoteOn {
            group: 0,
            channel: 0,
            note: 60,
            attribute_type: 0,
            attribute: 0,
            velocity: 1,
        },
    ));
    let bytes = ump.to_bytes();
    for len in [0usize, 2, 4, 6, 7] {
        assert!(Ump::from_bytes(&bytes[..len]).is_err(), "len {len}");
    }
    assert!(Ump::from_bytes(&bytes).is_ok());
}

/// Quarter-frame MTC messages survive the MIDI 1.0 parser.
#[test]
fn mtc_quarter_frames_parse() {
    for piece in 0u8..0x7F {
        let bytes = [0xF1, piece];
        let parsed = parse_midi1(&bytes).unwrap();
        assert_eq!(parsed, Midi1Message::TimeCodeQuarterFrame(piece));
    }
}
