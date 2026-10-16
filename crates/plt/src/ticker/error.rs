//! The error type of the ticker module.

use std::fmt;

use super::pyfmt::FormatError;

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
    /// A value outside a function's domain, e.g. `log10(0)`.
    Domain(&'static str),
    /// A locator would generate an absurd number of ticks.
    TooManyTicks(usize),
    /// The operation is not supported by this tick helper.
    Unsupported(&'static str),
    /// A Python-style format string is invalid for the value.
    Format(FormatError),
}

impl TickerError {
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
                f.write_str("data cannot be log-scaled because all values are <= 0")
            }
            Self::NonFinite(what) => write!(f, "{what} is not finite"),
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
