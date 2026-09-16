//! Behringer X32/M32 console surface over OSC.
//!
//! The X32/M32 family exposes OSC on UDP port 10023. Fader paths are
//! `/ch/NN/mix/fader` (1-based, zero-padded), `/main/mix/fader`,
//! `/auxin/NN/mix/fader`, `/fxrtn/NN/mix/fader`, and
//! `/bus/NN/mix/fader`, with values normalized 0.0-1.0.

use crate::feedback::Feedback;
use crate::surface::{ControlEvent, ControlSurface};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tpt_av_control_osc::{OscArg, OscClient, OscMessage, OscServer};
use tpt_av_control_utils::ControlError;

/// The standard X32/M32 OSC port.
pub const X32_PORT: u16 = 10023;

/// Total channel indices supported: 32 channels plus main (index 32).
pub const X32_CHANNEL_COUNT: u8 = 33;

/// A Behringer X32/M32 mixing console used as a control surface.
pub struct BehringerX32Surface {
    console: SocketAddr,
    client: OscClient,
    server: OscServer,
    events: Arc<Mutex<VecDeque<ControlEvent>>>,
}

impl BehringerX32Surface {
    /// Creates a surface pointed at a console at `console` (usually
    /// `console-ip:10023`). The local OSC listener binds an ephemeral
    /// port; use [`BehringerX32Surface::local_addr`] when configuring
    /// static OSC routing on the console.
    pub fn new(console: SocketAddr) -> Result<Self, ControlError> {
        let client = OscClient::new(console)?;
        let mut server = OscServer::bind("0.0.0.0:0".parse()?)?;
        let events: Arc<Mutex<VecDeque<ControlEvent>>> = Arc::default();
        let sink = Arc::clone(&events);
        server.set_handler(move |message, _src| {
            if let Some(event) = osc_to_event(&message) {
                sink.lock().expect("event queue poisoned").push_back(event);
            }
        });
        Ok(Self {
            console,
            client,
            server,
            events,
        })
    }

    /// The local OSC listener address.
    pub fn local_addr(&self) -> Result<SocketAddr, ControlError> {
        self.server.local_addr()
    }

    /// The console address.
    pub fn console(&self) -> SocketAddr {
        self.console
    }

    fn send(&mut self, address: &str, args: &[OscArg]) -> Result<(), ControlError> {
        let message = OscMessage::new(address, args)?;
        self.client.send(&message)?;
        Ok(())
    }
}

/// Maps an inbound X32 OSC message onto a [`ControlEvent`].
fn osc_to_event(message: &OscMessage) -> Option<ControlEvent> {
    let value = message.arguments.first().and_then(OscArg::as_f32)?;
    for prefix in ["/ch/", "/auxin/", "/fxrtn/", "/bus/"] {
        if let Some(rest) = message.address.strip_prefix(prefix) {
            let mut parts = rest.split('/');
            let channel: u8 = parts.next()?.parse().ok()?;
            if parts.next() == Some("mix") && parts.next() == Some("fader") {
                return Some(ControlEvent::FaderMove {
                    channel: channel.checked_sub(1)?,
                    value: value.clamp(0.0, 1.0),
                });
            }
            return None;
        }
    }
    if message.address == "/main/mix/fader" {
        return Some(ControlEvent::FaderMove {
            channel: X32_CHANNEL_COUNT - 1, // main = index 32
            value: value.clamp(0.0, 1.0),
        });
    }
    None
}

fn fader_address(channel: u8) -> Result<String, ControlError> {
    match channel.checked_add(1) {
        Some(n) if n < X32_CHANNEL_COUNT => Ok(format!("/ch/{:02}/mix/fader", n)),
        Some(X32_CHANNEL_COUNT) => Ok("/main/mix/fader".to_string()),
        _ => Err(ControlError::OutOfRange {
            value: i64::from(channel),
            min: 0,
            max: i64::from(X32_CHANNEL_COUNT) - 1,
        }),
    }
}

impl ControlSurface for BehringerX32Surface {
    fn name(&self) -> &str {
        "Behringer X32/M32"
    }

    fn init(&mut self) -> Result<(), ControlError> {
        // Subscribe to remote-control updates from the console.
        self.send("/xremote", &[])?;
        self.send("/info", &[])
    }

    fn read_event(&mut self) -> Result<Option<ControlEvent>, ControlError> {
        // Pump the OSC socket (non-blocking) before draining the queue.
        self.server.set_nonblocking(true)?;
        match self.server.recv_packet() {
            Ok(_) => {}
            Err(ControlError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e),
        }
        self.server.set_nonblocking(false)?;
        Ok(self
            .events
            .lock()
            .expect("event queue poisoned")
            .pop_front())
    }

    fn send_feedback(&mut self, feedback: &Feedback) -> Result<(), ControlError> {
        match feedback {
            Feedback::FaderPosition { channel, value } => {
                let address = fader_address(*channel)?;
                self.send(&address, &[OscArg::Float(value.clamp(0.0, 1.0))])
            }
            Feedback::Led { led, on } => {
                // Map LED feedback onto the channel mute lamp.
                let address = fader_address(*led)?;
                let mute_address = address.replace("fader", "on");
                self.send(&mute_address, &[OscArg::Int(i32::from(!*on))])
            }
            Feedback::DisplayText { .. } => Err(ControlError::Unsupported(
                "X32 consoles do not accept arbitrary display text over OSC".into(),
            )),
            Feedback::LedColor { .. } => Err(ControlError::Unsupported(
                "X32 lamps are single-color; use Led".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_av_control_osc::OscMessage;

    #[test]
    fn osc_fader_messages_map_to_events() {
        let message = OscMessage::new("/ch/07/mix/fader", &[OscArg::Float(0.5)]).unwrap();
        match osc_to_event(&message) {
            Some(ControlEvent::FaderMove { channel, value }) => {
                assert_eq!(channel, 6); // /ch/07 → index 6
                assert!((value - 0.5).abs() < 1e-6);
            }
            other => panic!("expected fader move, got {other:?}"),
        }
        let main = OscMessage::new("/main/mix/fader", &[OscArg::Float(1.0)]).unwrap();
        match osc_to_event(&main) {
            Some(ControlEvent::FaderMove { channel, .. }) => assert_eq!(channel, 32),
            other => panic!("expected main fader, got {other:?}"),
        }
        // Unrelated messages are ignored.
        assert!(osc_to_event(&OscMessage::new("/info", &[]).unwrap()).is_none());
    }

    #[test]
    fn fader_addresses_round_trip() {
        assert_eq!(fader_address(0).unwrap(), "/ch/01/mix/fader");
        assert_eq!(fader_address(31).unwrap(), "/ch/32/mix/fader");
        assert_eq!(fader_address(32).unwrap(), "/main/mix/fader");
        assert!(fader_address(33).is_err());
    }
}
