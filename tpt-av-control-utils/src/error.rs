//! Error type shared by every `tpt-av-control` crate.

use std::fmt;

/// Errors produced across the TPT AV control suite.
#[derive(Debug)]
pub enum ControlError {
    /// Underlying I/O failure (socket, device, etc.).
    Io(std::io::Error),
    /// Malformed protocol data.
    InvalidData(String),
    /// The operation or message is not supported by this implementation.
    Unsupported(String),
    /// The requested device does not exist.
    DeviceNotFound(String),
    /// The requested device or port exists but is already in use.
    PortBusy(String),
    /// A timed operation expired.
    Timeout,
    /// A channel, stream, or connection was closed.
    Closed,
    /// A value is out of the range the protocol allows.
    OutOfRange {
        /// The offending value.
        value: i64,
        /// Inclusive minimum.
        min: i64,
        /// Inclusive maximum.
        max: i64,
    },
    /// An OSC/MIDI/DMX address string is malformed.
    InvalidAddress(String),
    /// A caller-provided buffer is too small for the encoded output.
    BufferTooSmall {
        /// Number of bytes required.
        needed: usize,
        /// Number of bytes provided.
        available: usize,
    },
    /// A bounded queue has no room for the item.
    QueueFull,
}

impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlError::Io(e) => write!(f, "I/O error: {e}"),
            ControlError::InvalidData(detail) => write!(f, "invalid data: {detail}"),
            ControlError::Unsupported(what) => write!(f, "unsupported: {what}"),
            ControlError::DeviceNotFound(name) => write!(f, "device not found: {name}"),
            ControlError::PortBusy(name) => write!(f, "port busy: {name}"),
            ControlError::Timeout => write!(f, "operation timed out"),
            ControlError::Closed => write!(f, "channel closed"),
            ControlError::OutOfRange { value, min, max } => {
                write!(f, "value {value} out of range [{min}, {max}]")
            }
            ControlError::InvalidAddress(addr) => write!(f, "invalid address: {addr:?}"),
            ControlError::BufferTooSmall { needed, available } => {
                write!(f, "buffer too small: need {needed} bytes, have {available}")
            }
            ControlError::QueueFull => write!(f, "queue is full"),
        }
    }
}

impl std::error::Error for ControlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ControlError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ControlError {
    fn from(e: std::io::Error) -> Self {
        ControlError::Io(e)
    }
}

impl From<std::net::AddrParseError> for ControlError {
    fn from(e: std::net::AddrParseError) -> Self {
        ControlError::InvalidAddress(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_informative() {
        let e = ControlError::OutOfRange { value: 600, min: 0, max: 511 };
        assert!(e.to_string().contains("600"));
        assert!(e.to_string().contains("511"));
    }

    #[test]
    fn io_error_converts() {
        let e: ControlError = std::io::Error::new(std::io::ErrorKind::Other, "boom").into();
        assert!(matches!(e, ControlError::Io(_)));
        assert!(std::error::Error::source(&e).is_some());
    }
}
