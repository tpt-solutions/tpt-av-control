//! MIDI Show Control (MSC): cue firing over SysEx.
//!
//! Wire format: `F0 7F <device> 02 <command_fmt> <command> [data] F7`.

use crate::midi1::Midi1Message;
use tpt_av_control_utils::ControlError;

/// MSC command format: General.
pub const FORMAT_GENERAL: u8 = 0x00;
/// MSC command format: Lighting.
pub const FORMAT_LIGHTING: u8 = 0x01;
/// MSC command format: Sound.
pub const FORMAT_SOUND: u8 = 0x02;
/// MSC command format: Machinery.
pub const FORMAT_MACHINERY: u8 = 0x03;
/// MSC command format: Video.
pub const FORMAT_VIDEO: u8 = 0x04;
/// MSC command format: Projection.
pub const FORMAT_PROJECTION: u8 = 0x05;
/// MSC command format: Pyro.
pub const FORMAT_PYRO: u8 = 0x06;

#[derive(Debug, Clone, PartialEq, Eq)]
/// An MSC command.
pub enum MscCommand {
    /// GO — run the (optional) cue number, optionally in a list.
    Go {
        /// Cue number text (optional).
        cue: Option<String>,
        /// Cue list number text (optional).
        cue_list: Option<String>,
    },
    /// STOP — stop the (optional) cue.
    /// Cue number text (optional).
    /// Cue list number text (optional).
    /// STOP — stop the (optional) cue.
    Stop {
        /// Cue number text (optional).
        cue: Option<String>,
    },
    /// FIRE a macro.
    /// Cue number text (optional).
    /// Cue list number text (optional).
    /// FIRE a macro.
    Fire {
        /// Macro number (0-127).
        macro_number: u8,
    },
    /// GO OFF (reverse of GO for "blackout" style cues).
    GoOff {
        /// Cue number text (optional).
        cue: Option<String>,
        /// Cue list number text (optional).
        cue_list: Option<String>,
    },
    /// RESET.
    Reset,
    /// All lights/general off.
    /// Cue number text (optional).
    /// GO JAM — jump to a cue with all channels cancelled.
    GoJam {
        /// Cue number text (optional).
        cue: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A complete MSC message.
pub struct MscMessage {
    /// Target device ID (0x00-0x7E; 0x7F = all devices).
    pub device_id: u8,
    /// Command format (see the `FORMAT_*` constants).
    pub command_format: u8,
    /// The command itself.
    pub command: MscCommand,
}

fn valid_cue_char(b: u8) -> bool {
    (0x20..=0x7E).contains(&b)
}

impl MscMessage {
    /// Encodes to a MIDI 1.0 SysEx message.
    pub fn to_midi1(&self) -> Result<Midi1Message, ControlError> {
        let mut payload = vec![self.command_format & 0x7F];
        let encode_cue = |out: &mut Vec<u8>, cue: &Option<String>| -> Result<(), ControlError> {
            if let Some(cue) = cue {
                for &b in cue.as_bytes() {
                    if !valid_cue_char(b) || b >= 0x7F {
                        return Err(ControlError::InvalidData(format!(
                            "illegal cue character {b:#04x}"
                        )));
                    }
                }
                out.extend_from_slice(cue.as_bytes());
            }
            Ok(())
        };
        match &self.command {
            MscCommand::Go { cue, cue_list } => {
                payload.push(b'G');
                encode_cue(&mut payload, cue)?;
                if cue_list.is_some() {
                    payload.push(0x00); // control separator
                    encode_cue(&mut payload, cue_list)?;
                }
            }
            MscCommand::GoOff { cue, cue_list } => {
                payload.extend_from_slice(b"G+");
                encode_cue(&mut payload, cue)?;
                if cue_list.is_some() {
                    payload.push(0x00);
                    encode_cue(&mut payload, cue_list)?;
                }
            }
            MscCommand::Stop { cue } => {
                payload.push(b'S');
                encode_cue(&mut payload, cue)?;
            }
            MscCommand::Fire { macro_number } => {
                payload.push(b'F');
                payload.push(macro_number & 0x7F);
            }
            MscCommand::Reset => payload.push(b'R'),
            MscCommand::GoJam { cue } => {
                payload.push(b'J');
                encode_cue(&mut payload, cue)?;
            }
        }
        Ok(crate::sysex::sysex_universal(
            crate::sysex::UNIVERSAL_REALTIME,
            self.device_id & 0x7F,
            crate::sysex::SHOW_CONTROL,
            0x01,
            &payload,
        ))
    }

    /// Parses an MSC message from a MIDI 1.0 SysEx.
    pub fn from_midi1(message: &Midi1Message) -> Result<Self, ControlError> {
        let view = crate::sysex::parse_sysex(message)?;
        if view.manufacturer != crate::sysex::UNIVERSAL_REALTIME
            || view.sub_id1 != Some(crate::sysex::SHOW_CONTROL)
        {
            return Err(ControlError::InvalidData(
                "not a MIDI Show Control message".into(),
            ));
        }
        let device_id = view.device_id.unwrap_or(0x7F);
        let payload = view.payload;
        if payload.len() < 2 {
            return Err(ControlError::InvalidData("MSC payload too short".into()));
        }
        let command_format = payload[0];
        let rest = &payload[1..];

        // First byte (or two for GO OFF) selects the command.
        let command = match rest {
            [b'G', b'+', tail @ ..] => {
                let (cue, cue_list) = split_cue_pair(tail)?;
                MscCommand::GoOff { cue, cue_list }
            }
            [b'G', tail @ ..] => {
                let (cue, cue_list) = split_cue_pair(tail)?;
                MscCommand::Go { cue, cue_list }
            }
            [b'S', tail @ ..] => MscCommand::Stop {
                cue: parse_cue(tail),
            },
            [b'F', macro_number, ..] => MscCommand::Fire {
                macro_number: *macro_number & 0x7F,
            },
            [b'R', ..] => MscCommand::Reset,
            [b'J', tail @ ..] => MscCommand::GoJam {
                cue: parse_cue(tail),
            },
            other => {
                return Err(ControlError::InvalidData(format!(
                    "unknown MSC command byte {other:?}"
                )))
            }
        };
        Ok(Self {
            device_id,
            command_format,
            command,
        })
    }
}

fn parse_cue(bytes: &[u8]) -> Option<String> {
    let end = bytes.iter().position(|&b| b == 0x00).unwrap_or(bytes.len());
    if end == 0 {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes[..end]).into_owned())
}

fn split_cue_pair(bytes: &[u8]) -> Result<(Option<String>, Option<String>), ControlError> {
    let mut parts = bytes.split(|&b| b == 0x00);
    let cue = parts
        .next()
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned());
    let list = parts
        .next()
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned());
    if parts.next().is_some() {
        return Err(ControlError::InvalidData("too many MSC cue fields".into()));
    }
    Ok((cue, list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi1::parse_midi1;

    fn roundtrip(message: MscMessage) {
        let encoded = message.to_midi1().unwrap();
        let parsed = parse_midi1(&encoded.to_bytes()).unwrap();
        let back = MscMessage::from_midi1(&parsed).unwrap();
        assert_eq!(back, message);
    }

    #[test]
    fn go_without_cue() {
        roundtrip(MscMessage {
            device_id: 0x7F,
            command_format: FORMAT_LIGHTING,
            command: MscCommand::Go {
                cue: None,
                cue_list: None,
            },
        });
    }

    #[test]
    fn go_with_cue_and_list() {
        roundtrip(MscMessage {
            device_id: 0x01,
            command_format: FORMAT_SOUND,
            command: MscCommand::Go {
                cue: Some("12.5".into()),
                cue_list: Some("3".into()),
            },
        });
    }

    #[test]
    fn stop_fire_reset_jam() {
        roundtrip(MscMessage {
            device_id: 0x00,
            command_format: FORMAT_GENERAL,
            command: MscCommand::Stop {
                cue: Some("42".into()),
            },
        });
        roundtrip(MscMessage {
            device_id: 0x00,
            command_format: FORMAT_GENERAL,
            command: MscCommand::Fire { macro_number: 7 },
        });
        roundtrip(MscMessage {
            device_id: 0x00,
            command_format: FORMAT_GENERAL,
            command: MscCommand::Reset,
        });
        roundtrip(MscMessage {
            device_id: 0x00,
            command_format: FORMAT_VIDEO,
            command: MscCommand::GoJam {
                cue: Some("99".into()),
            },
        });
        roundtrip(MscMessage {
            device_id: 0x00,
            command_format: FORMAT_VIDEO,
            command: MscCommand::GoOff {
                cue: Some("5".into()),
                cue_list: Some("1".into()),
            },
        });
    }

    #[test]
    fn rejects_illegal_cue_characters() {
        let message = MscMessage {
            device_id: 0,
            command_format: 0,
            command: MscCommand::Stop {
                cue: Some("bad\u{7F}".into()),
            },
        };
        assert!(message.to_midi1().is_err());
    }

    #[test]
    fn rejects_non_msc() {
        let other = crate::sysex::identity_request(0x7F);
        assert!(MscMessage::from_midi1(&other).is_err());
    }
}
