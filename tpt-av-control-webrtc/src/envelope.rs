//! Control message envelopes carried over WebRTC data channels.

use tpt_av_control_utils::parameter::{ParameterId, ParameterValue};
use tpt_av_control_utils::TransportCommand;

/// A request to change one parameter on the receiving end.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterChangeRequest {
    /// The parameter to change.
    pub parameter: ParameterId,
    /// Its new value.
    pub value: ParameterValue,
}

/// The envelope multiplexed over a data channel.
#[derive(Debug, Clone, PartialEq)]
pub enum ControlEnvelope {
    /// Change a parameter.
    Parameter(ParameterChangeRequest),
    /// Issue a transport command.
    Command(TransportCommand),
    /// Free-form text (chat, comments).
    Text(String),
}

impl ControlEnvelope {
    /// Binary encoding: 1 type byte + payload. Compact and dependency-free.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            ControlEnvelope::Parameter(p) => {
                out.push(0x01);
                out.extend_from_slice(p.parameter.as_str().as_bytes());
                out.push(0x00);
                match &p.value {
                    ParameterValue::Float(v) => {
                        out.push(0x01);
                        out.extend_from_slice(&v.to_bits().to_be_bytes());
                    }
                    ParameterValue::Int(v) => {
                        out.push(0x02);
                        out.extend_from_slice(&v.to_be_bytes());
                    }
                    ParameterValue::Bool(v) => {
                        out.push(0x03);
                        out.push(u8::from(*v));
                    }
                    ParameterValue::String(v) => {
                        out.push(0x04);
                        out.extend_from_slice((v.len() as u32).to_be_bytes().as_slice());
                        out.extend_from_slice(v.as_bytes());
                    }
                }
            }
            ControlEnvelope::Command(c) => {
                out.push(0x02);
                out.push(match c {
                    TransportCommand::Play => 1,
                    TransportCommand::Stop => 2,
                    TransportCommand::Pause => 3,
                    TransportCommand::Continue => 4,
                    TransportCommand::ClockTick => 5,
                });
            }
            ControlEnvelope::Text(t) => {
                out.push(0x03);
                out.extend_from_slice((t.len() as u32).to_be_bytes().as_slice());
                out.extend_from_slice(t.as_bytes());
            }
        }
        out
    }

    /// Decodes a binary envelope.
    pub fn decode(data: &[u8]) -> Result<Self, tpt_av_control_utils::ControlError> {
        use tpt_av_control_utils::ControlError;
        let read_u32 = |off: usize| -> Result<u32, ControlError> {
            data.get(off..off + 4)
                .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
                .ok_or_else(|| ControlError::InvalidData("envelope truncated".into()))
        };
        match data.first() {
            Some(&0x01) => {
                let nul = data[1..]
                    .iter()
                    .position(|&b| b == 0)
                    .map(|p| p + 1)
                    .ok_or_else(|| {
                        ControlError::InvalidData("parameter name not terminated".into())
                    })?;
                let parameter =
                    ParameterId::new(String::from_utf8_lossy(&data[1..nul]).into_owned());
                // Skip past the NUL terminator before the value tag.
                let mut cursor = nul + 1;
                let tag = *data.get(cursor).ok_or_else(|| {
                    ControlError::InvalidData("envelope truncated at value tag".into())
                })?;
                cursor += 1;
                let value = match tag {
                    0x01 => {
                        let b = data.get(cursor..cursor + 4).ok_or_else(|| {
                            ControlError::InvalidData("float value truncated".into())
                        })?;
                        ParameterValue::Float(f32::from_bits(u32::from_be_bytes([
                            b[0], b[1], b[2], b[3],
                        ])))
                    }
                    0x02 => {
                        let b = data.get(cursor..cursor + 8).ok_or_else(|| {
                            ControlError::InvalidData("int value truncated".into())
                        })?;
                        ParameterValue::Int(i64::from_be_bytes([
                            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
                        ]))
                    }
                    0x03 => ParameterValue::Bool(
                        data.get(cursor).copied().ok_or_else(|| {
                            ControlError::InvalidData("bool value truncated".into())
                        })? != 0,
                    ),
                    0x04 => {
                        let len = read_u32(cursor)? as usize;
                        let text = data.get(cursor + 4..cursor + 4 + len).ok_or_else(|| {
                            ControlError::InvalidData("string value truncated".into())
                        })?;
                        ParameterValue::String(String::from_utf8_lossy(text).into_owned())
                    }
                    other => {
                        return Err(ControlError::InvalidData(format!(
                            "unknown value tag {other:#04x}"
                        )))
                    }
                };
                Ok(ControlEnvelope::Parameter(ParameterChangeRequest {
                    parameter,
                    value,
                }))
            }
            Some(&0x02) => {
                let command = match data.get(1) {
                    Some(1) => TransportCommand::Play,
                    Some(2) => TransportCommand::Stop,
                    Some(3) => TransportCommand::Pause,
                    Some(4) => TransportCommand::Continue,
                    Some(5) => TransportCommand::ClockTick,
                    other => {
                        return Err(ControlError::InvalidData(format!(
                            "unknown command {other:?}"
                        )))
                    }
                };
                Ok(ControlEnvelope::Command(command))
            }
            Some(&0x03) => {
                let len = read_u32(1)? as usize;
                let text = data
                    .get(5..5 + len)
                    .ok_or_else(|| ControlError::InvalidData("text truncated".into()))?;
                Ok(ControlEnvelope::Text(
                    String::from_utf8_lossy(text).into_owned(),
                ))
            }
            other => Err(ControlError::InvalidData(format!(
                "unknown envelope type {other:?}"
            ))),
        }
    }

    /// JSON encoding (requires the `json` feature).
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> String {
        match self {
            ControlEnvelope::Parameter(p) => format!(
                r#"{{"type":"parameter","name":{:?},"value":{}}}"#,
                p.parameter.as_str(),
                match &p.value {
                    ParameterValue::Float(v) => format!("{v}"),
                    ParameterValue::Int(v) => format!("{v}"),
                    ParameterValue::Bool(v) => format!("{v}"),
                    ParameterValue::String(v) => format!("{v:?}"),
                }
            ),
            ControlEnvelope::Command(c) => format!(
                r#"{{"type":"command","command":{:?}}}"#,
                match c {
                    TransportCommand::Play => "play",
                    TransportCommand::Stop => "stop",
                    TransportCommand::Pause => "pause",
                    TransportCommand::Continue => "continue",
                    TransportCommand::ClockTick => "clock",
                }
            ),
            ControlEnvelope::Text(t) => format!(r#"{{"type":"text","text":{t:?}}}"#),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_roundtrip_all_variants() {
        let envelopes = vec![
            ControlEnvelope::Parameter(ParameterChangeRequest {
                parameter: ParameterId::new("track.1.volume"),
                value: ParameterValue::Float(0.75),
            }),
            ControlEnvelope::Parameter(ParameterChangeRequest {
                parameter: ParameterId::new("preset"),
                value: ParameterValue::Int(42),
            }),
            ControlEnvelope::Parameter(ParameterChangeRequest {
                parameter: ParameterId::new("mute"),
                value: ParameterValue::Bool(true),
            }),
            ControlEnvelope::Parameter(ParameterChangeRequest {
                parameter: ParameterId::new("scene"),
                value: ParameterValue::String("verse".into()),
            }),
            ControlEnvelope::Command(TransportCommand::Play),
            ControlEnvelope::Command(TransportCommand::ClockTick),
            ControlEnvelope::Text("hello from the booth".into()),
        ];
        for envelope in envelopes {
            let bytes = envelope.encode();
            let back = ControlEnvelope::decode(&bytes).unwrap();
            assert_eq!(back, envelope, "roundtrip {envelope:?}");
        }
    }

    #[test]
    fn truncated_envelopes_rejected() {
        let full = ControlEnvelope::Text("0123456789".into()).encode();
        for len in [0usize, 1, 3, 5, 9] {
            assert!(ControlEnvelope::decode(&full[..len]).is_err(), "len {len}");
        }
        assert!(ControlEnvelope::decode(&full).is_ok());
        assert!(
            ControlEnvelope::decode(&[0x7F, 0x00]).is_err(),
            "unknown type"
        );
    }
}
