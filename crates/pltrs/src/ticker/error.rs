//! The error type of the ticker module.

use std::fmt;

use super::pyfmt::FormatError;
use crate::artist::DrawErrorKind;

/// Why a locator or formatter could not produce a result.
#[derive(Clone, Debug, PartialEq)]
pub enum TickerError {
    /// A parameter is outside its valid domain.
    InvalidParameter {
        /// The parameter's name.
        name: &'static str,
        /// What is wrong with it.
        reason: String,
    },
    /// Log locators cannot place ticks when all data are `<= 0`.
    NonPositiveLogData,
    /// A computation needed a finite value but got NaN or infinity
    /// (matplotlib raises `ValueError`/`OverflowError` here).
    NonFinite(&'static str),
    /// A value to round to an integer (`math.floor`, `round`) was NaN or,
    /// when `infinite`, infinite: Python's `ValueError` and
    /// `OverflowError`, with their messages.
    NotAnInteger {
        /// Whether the value was infinite rather than NaN.
        infinite: bool,
    },
    /// A logit locator of the radial axis of polar axes (matplotlib's
    /// `NotImplementedError`).
    PolarLogit,
    /// The ticks of a level cannot be made: matplotlib's ticks give their
    /// gridline this `Line2D` keyword (`marker`, `visible`) themselves,
    /// and the tick parameters gave it too (matplotlib's `TypeError`).
    TickKeywordTwice(&'static str),
    /// A value outside a function's domain, e.g. `log10(0)`.
    Domain(&'static str),
    /// A locator would generate an absurd number of ticks (this many), or
    /// more than an axis draws ([`MAX_DRAWN_TICKS`](super::MAX_DRAWN_TICKS)).
    TooManyTicks(usize),
    /// The operation is not supported by this tick helper.
    Unsupported(&'static str),
    /// A Python-style format string is invalid for the value.
    Format(FormatError),
}

pltrs_core::archive_enum!(TickerError {
    InvalidParameter { name, reason },
    NonPositiveLogData,
    NonFinite(v0),
    NotAnInteger { infinite },
    PolarLogit,
    TickKeywordTwice(v0),
    Domain(v0),
    TooManyTicks(v0),
    Unsupported(v0),
    Format(v0),
});

impl TickerError {
    /// The exception matplotlib's locators and formatters raise for the
    /// error when a figure is drawn.
    pub fn draw_kind(&self) -> DrawErrorKind {
        match self {
            Self::NonPositiveLogData | Self::NotAnInteger { infinite: false } => {
                DrawErrorKind::InvalidValue
            }
            Self::NotAnInteger { infinite: true } => DrawErrorKind::Overflow,
            Self::PolarLogit => DrawErrorKind::Unsupported,
            Self::TickKeywordTwice(_) => DrawErrorKind::InvalidType,
            _ => DrawErrorKind::Other,
        }
    }

    /// Shorthand for [`TickerError::InvalidParameter`].
    pub(crate) fn invalid(name: &'static str, reason: impl Into<String>) -> Self {
        Self::InvalidParameter {
            name,
            reason: reason.into(),
        }
    }
}

impl fmt::Display for TickerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter { name, reason } => write!(f, "invalid {name}: {reason}"),
            Self::NonPositiveLogData => {
                f.write_str("Data cannot be log-scaled because all values are <= 0.")
            }
            Self::NonFinite(what) => write!(f, "{what} is not finite"),
            Self::NotAnInteger { infinite: true } => {
                f.write_str("cannot convert float infinity to integer")
            }
            Self::NotAnInteger { infinite: false } => {
                f.write_str("cannot convert float NaN to integer")
            }
            Self::PolarLogit => f.write_str("Polar axis cannot be logit scaled yet"),
            Self::TickKeywordTwice(keyword) => {
                write!(
                    f,
                    "Line2D() got multiple values for keyword argument '{keyword}'"
                )
            }
            Self::Domain(what) => write!(f, "math domain error: {what}"),
            Self::TooManyTicks(n) => write!(f, "refusing to generate {n} ticks"),
            Self::Unsupported(what) => write!(f, "unsupported: {what}"),
            Self::Format(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for TickerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Format(err) => Some(err),
            _ => None,
        }
    }
}

impl From<FormatError> for TickerError {
    fn from(err: FormatError) -> Self {
        Self::Format(err)
    }
}
