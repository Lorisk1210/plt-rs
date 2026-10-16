//! The crate-wide error type.

use std::fmt;
use std::io;

use crate::window::WindowId;

/// Everything that can go wrong while talking to the windowing system.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// No usable backend: e.g. `$DISPLAY` is unset, or the requested
    /// backend is not supported on this operating system.
    BackendUnavailable(String),
    /// The display name (`$DISPLAY`) could not be parsed.
    InvalidDisplay(String),
    /// An I/O operation on the display connection failed.
    Io {
        /// What was being done.
        operation: &'static str,
        /// The I/O error category.
        kind: io::ErrorKind,
    },
    /// The display server refused the connection (e.g. bad credentials).
    ConnectionRefused(String),
    /// The display server closed the connection.
    ConnectionClosed,
    /// The display server sent data this crate cannot interpret.
    Malformed(&'static str),
    /// The display server lacks a feature this crate needs (e.g. a
    /// `TrueColor` visual).
    Unsupported(&'static str),
    /// The X server reported an error for a request.
    X11(X11Error),
    /// The window id does not belong to a live window of this loop.
    UnknownWindow(WindowId),
    /// A pixel buffer has the wrong length for its dimensions.
    BufferSize {
        /// Bytes required by the dimensions.
        expected: usize,
        /// Bytes provided.
        actual: usize,
    },
    /// An argument is out of range.
    InvalidArgument(&'static str),
    /// The event loop this proxy belongs to no longer exists.
    EventLoopClosed,
    /// The server's resource-id space is exhausted.
    ResourceIdsExhausted,
}

impl Error {
    /// Builds an [`Error::Io`] from an I/O error and a description of the
    /// failed operation.
    pub(crate) fn io(operation: &'static str, error: &io::Error) -> Self {
        Self::Io {
            operation,
            kind: error.kind(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendUnavailable(why) => write!(f, "no windowing backend available: {why}"),
            Self::InvalidDisplay(name) => write!(f, "invalid display name {name:?}"),
            Self::Io { operation, kind } => write!(f, "{operation} failed: {kind}"),
            Self::ConnectionRefused(reason) => {
                write!(f, "display server refused the connection: {reason}")
            }
            Self::ConnectionClosed => f.write_str("display server closed the connection"),
            Self::Malformed(what) => write!(f, "malformed message from display server: {what}"),
            Self::Unsupported(what) => write!(f, "display server unsupported: {what}"),
            Self::X11(error) => error.fmt(f),
            Self::UnknownWindow(id) => write!(f, "unknown window {id:?}"),
            Self::BufferSize { expected, actual } => {
                write!(f, "pixel buffer has {actual} bytes, expected {expected}")
            }
            Self::InvalidArgument(what) => write!(f, "invalid argument: {what}"),
            Self::EventLoopClosed => f.write_str("the event loop no longer exists"),
            Self::ResourceIdsExhausted => f.write_str("X resource ids exhausted"),
        }
    }
}

impl std::error::Error for Error {}

/// An error reply from the X server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X11Error {
    /// The X error code (e.g. 3 = `BadWindow`).
    pub code: u8,
    /// Full sequence number of the failed request.
    pub sequence: u64,
    /// The offending resource id or value, if meaningful.
    pub bad_value: u32,
    /// Major opcode of the failed request.
    pub major_opcode: u8,
    /// Minor opcode of the failed request (extensions only).
    pub minor_opcode: u16,
}

impl X11Error {
    /// The symbolic name of the error code (`"BadWindow"`, ...).
    pub const fn name(&self) -> &'static str {
        match self.code {
            1 => "BadRequest",
            2 => "BadValue",
            3 => "BadWindow",
            4 => "BadPixmap",
            5 => "BadAtom",
            6 => "BadCursor",
            7 => "BadFont",
            8 => "BadMatch",
            9 => "BadDrawable",
            10 => "BadAccess",
            11 => "BadAlloc",
            12 => "BadColormap",
            13 => "BadGContext",
            14 => "BadIDChoice",
            15 => "BadName",
            16 => "BadLength",
            17 => "BadImplementation",
            _ => "extension error",
        }
    }
}

impl fmt::Display for X11Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "X11 {} (code {}) for request {}.{} (sequence {}, value {:#x})",
            self.name(),
            self.code,
            self.major_opcode,
            self.minor_opcode,
            self.sequence,
            self.bad_value
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_informative() {
        let err = Error::X11(X11Error {
            code: 3,
            sequence: 42,
            bad_value: 0x1234,
            major_opcode: 4,
            minor_opcode: 0,
        });
        let text = err.to_string();
        assert!(text.contains("BadWindow") && text.contains("42") && text.contains("0x1234"));
        let err = Error::io("connect", &io::Error::from(io::ErrorKind::NotFound));
        assert!(err.to_string().starts_with("connect failed"));
        assert!(
            Error::BufferSize {
                expected: 4,
                actual: 3
            }
            .to_string()
            .contains("expected 4")
        );
    }

    #[test]
    fn unknown_error_codes_are_named_generically() {
        let err = X11Error {
            code: 200,
            sequence: 1,
            bad_value: 0,
            major_opcode: 130,
            minor_opcode: 2,
        };
        assert_eq!(err.name(), "extension error");
    }
}
