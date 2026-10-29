//! Errors raised while building or using colormaps.

use std::fmt;

use super::Channel;
use crate::colors::ColorError;

/// What is wrong with a channel's segment data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentProblem {
    /// The anchor table has no rows.
    Empty,
    /// The anchors do not start at `x = 0` and end at `x = 1`.
    Endpoints,
    /// The anchor `x` values decrease somewhere.
    Decreasing,
    /// The gamma correction samples beyond `x = 1` (negative gamma).
    Gamma,
}

impl fmt::Display for SegmentProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "has no mapping points",
            Self::Endpoints => "mapping points must start with x=0 and end with x=1",
            Self::Decreasing => "mapping points must have x in increasing order",
            Self::Gamma => "gamma must not push samples beyond x=1",
        })
    }
}

/// Why a colormap could not be built or evaluated.
#[derive(Clone, Debug, PartialEq)]
pub enum ColormapError {
    /// A colour (of the map or an extreme) is invalid.
    Color(ColorError),
    /// No colours were given.
    NoColors,
    /// A lookup table needs at least one entry.
    ZeroEntries,
    /// A channel's segment data is invalid.
    Segment {
        /// The offending channel.
        channel: Channel,
        /// What is wrong with it.
        problem: SegmentProblem,
    },
    /// `(value, colour)` stops must increase monotonically from 0 to 1.
    Stops,
    /// A uniform alpha outside `[0, 1]`.
    AlphaOutOfRange(f64),
    /// Per-value alphas do not match the number of values.
    AlphaCount {
        /// Number of values.
        values: usize,
        /// Number of alphas.
        alphas: usize,
    },
    /// No colormap is registered under this name.
    Unknown(String),
    /// A built-in colormap has this name, so it cannot be registered or
    /// unregistered.
    Builtin(String),
    /// Another colormap is registered under this name.
    AlreadyRegistered(String),
}

impl fmt::Display for ColormapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Color(err) => write!(f, "invalid colormap colour: {err}"),
            Self::NoColors => f.write_str("a colormap needs at least one colour"),
            Self::ZeroEntries => f.write_str("a colormap needs at least one lookup-table entry"),
            Self::Segment { channel, problem } => {
                write!(f, "{channel} segment data {problem}")
            }
            Self::Stops => f.write_str(
                "the values of (value, colour) pairs must increase monotonically from 0 to 1",
            ),
            Self::AlphaOutOfRange(alpha) => {
                write!(f, "alpha must be between 0 and 1 inclusive, got {alpha}")
            }
            Self::AlphaCount { values, alphas } => {
                write!(f, "{alphas} alpha values do not match {values} data values")
            }
            Self::Unknown(name) => write!(f, "{name:?} is not a known colormap name"),
            Self::Builtin(name) => write!(
                f,
                "{name:?} is a builtin colormap, which cannot be registered or unregistered"
            ),
            Self::AlreadyRegistered(name) => {
                write!(f, "a colormap named {name:?} is already registered")
            }
        }
    }
}

impl std::error::Error for ColormapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Color(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ColorError> for ColormapError {
    fn from(err: ColorError) -> Self {
        Self::Color(err)
    }
}
