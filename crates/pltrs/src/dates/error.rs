//! The error type shared by every date operation.

use std::fmt;

/// Why a date operation failed.
///
/// The variants mirror the exceptions matplotlib (via `datetime`, numpy and
/// dateutil) raises for the same inputs, so callers can tell a bad calendar
/// field from an out-of-range date number or an unparsable string.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DateError {
    /// A calendar or clock field is outside its valid range, e.g. month 13,
    /// February 30 or hour 24.
    FieldOutOfRange {
        /// The offending field (`"month"`, `"day"`, `"hour"`, ...).
        field: &'static str,
        /// The rejected value.
        value: i64,
    },
    /// A computation produced a date outside the representable years
    /// `0001..=9999` (Python's `OverflowError` for datetime arithmetic).
    Overflow,
    /// A Matplotlib date number converts to a date outside `0001..=9999`.
    DateNumberOutOfRange(f64),
    /// A date number is NaN or infinite where a finite value is required.
    NonFinite(f64),
    /// A string could not be parsed as a date.
    Parse {
        /// The rejected input.
        input: String,
        /// What was wrong with it.
        reason: &'static str,
    },
    /// The epoch was already used (or set) and can no longer change.
    EpochAlreadySet,
    /// A time zone name could not be resolved.
    UnknownTimeZone(String),
    /// A time zone file is malformed.
    InvalidTimeZoneData(&'static str),
    /// A recurrence rule can never produce a date.
    EmptyRule(&'static str),
    /// An argument has an invalid value (matplotlib's `ValueError` for
    /// parameters such as a zero interval or a wrong number of formats).
    InvalidArgument(String),
    /// A [`DateRange`](super::DateRange) step that is zero or negative,
    /// which never reaches the end.
    NonPositiveStep(super::TimeDelta),
    /// A range of this many dates, more than
    /// [`MAX_RANGE_DATES`](super::MAX_RANGE_DATES).
    TooManyRangeDates(u64),
}

pltrs_core::archive_enum!(DateError {
    FieldOutOfRange { field, value },
    Overflow,
    DateNumberOutOfRange(v0),
    NonFinite(v0),
    Parse { input, reason },
    EpochAlreadySet,
    UnknownTimeZone(v0),
    InvalidTimeZoneData(v0),
    EmptyRule(v0),
    InvalidArgument(v0),
    NonPositiveStep(v0),
    TooManyRangeDates(v0),
});

impl fmt::Display for DateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldOutOfRange { field, value } => {
                write!(f, "{field} {value} is out of range")
            }
            Self::Overflow => f.write_str("date value out of range (years 1..=9999)"),
            Self::DateNumberOutOfRange(x) => write!(
                f,
                "date number {x} converts to a date outside years 0001..=9999"
            ),
            Self::NonFinite(x) => write!(f, "cannot convert non-finite date number {x}"),
            Self::Parse { input, reason } => write!(f, "cannot parse date {input:?}: {reason}"),
            Self::EpochAlreadySet => f.write_str("set_epoch must be called before dates plotted."),
            Self::UnknownTimeZone(name) => write!(f, "{name} is not a valid time zone"),
            Self::InvalidTimeZoneData(why) => write!(f, "invalid time zone data: {why}"),
            Self::EmptyRule(why) => write!(f, "recurrence rule is empty: {why}"),
            Self::InvalidArgument(why) => f.write_str(why),
            Self::NonPositiveStep(step) => {
                write!(f, "the step of a date range must be positive, not {step}")
            }
            Self::TooManyRangeDates(count) => write!(
                f,
                "a date range of {count} dates is more than the {} allowed",
                super::MAX_RANGE_DATES
            ),
        }
    }
}

impl std::error::Error for DateError {}

impl DateError {
    /// Shorthand for [`DateError::FieldOutOfRange`].
    pub(crate) fn field(field: &'static str, value: impl Into<i64>) -> Self {
        Self::FieldOutOfRange {
            field,
            value: value.into(),
        }
    }

    /// Shorthand for [`DateError::Parse`].
    pub(crate) fn parse(input: &str, reason: &'static str) -> Self {
        Self::Parse {
            input: input.to_owned(),
            reason,
        }
    }
}
