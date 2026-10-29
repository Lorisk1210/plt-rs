//! Time-zone-aware date-times (Python's aware `datetime`).

use std::fmt;

use super::{TimeZone, UtcOffset};
use crate::dates::strftime::strftime;
use crate::dates::{DateError, DateTime};

/// A wall-clock date-time in a [`TimeZone`].
///
/// As in Python, arithmetic and comparisons within one zone act on the wall
/// clock; [`to_utc`](Self::to_utc) resolves the offset. `fold` picks the
/// second occurrence of a wall time repeated when clocks go back. Equality
/// is structural (same wall time, zone and fold), not instant equality.
///
/// Zoned date-times plot as the instants they stand for, and an axis
/// without a unit takes the zone of the first one as its unit, as
/// matplotlib does with aware `datetime`s (see [`crate::units`]).
#[derive(Clone, Debug, PartialEq)]
pub struct ZonedDateTime {
    local: DateTime,
    tz: TimeZone,
    fold: bool,
}

impl ZonedDateTime {
    /// Attaches `tz` to a wall-clock time (`dt.replace(tzinfo=tz)`).
    pub const fn from_local(local: DateTime, tz: TimeZone) -> Self {
        Self {
            local,
            tz,
            fold: false,
        }
    }

    /// The wall-clock time in `tz` of the UTC instant `utc`
    /// (`utc.astimezone(tz)`).
    pub fn from_utc(utc: DateTime, tz: TimeZone) -> Result<Self, DateError> {
        let (local, fold) = tz.local_from_utc(utc)?;
        Ok(Self { local, tz, fold })
    }

    /// A UTC date-time.
    pub const fn utc(utc: DateTime) -> Self {
        Self::from_local(utc, TimeZone::Utc)
    }

    /// The wall-clock time.
    pub const fn local(&self) -> DateTime {
        self.local
    }

    /// The time zone.
    pub const fn tz(&self) -> &TimeZone {
        &self.tz
    }

    /// Whether this is the second occurrence of a repeated wall time.
    pub const fn fold(&self) -> bool {
        self.fold
    }

    /// The offset from UTC in effect (`utcoffset()`).
    pub fn offset(&self) -> UtcOffset {
        self.tz.offset_at_local(self.local, self.fold)
    }

    /// The zone abbreviation in effect (`tzname()`).
    pub fn tz_name(&self) -> String {
        self.tz.name_at_local(self.local, self.fold)
    }

    /// The same instant as a naive UTC date-time.
    pub fn to_utc(&self) -> Result<DateTime, DateError> {
        self.local.checked_sub(self.offset().as_timedelta())
    }

    /// The same instant in another zone (`astimezone(tz)`).
    pub fn to_zone(&self, tz: TimeZone) -> Result<Self, DateError> {
        Self::from_utc(self.to_utc()?, tz)
    }

    /// Another wall-clock time in the same zone, with the fold reset as
    /// after Python datetime arithmetic.
    #[must_use]
    pub fn with_local(&self, local: DateTime) -> Self {
        Self::from_local(local, self.tz.clone())
    }

    /// The same zone and fold at another wall-clock time (Python's
    /// `replace()` of time fields).
    pub(crate) fn replace_local(&self, local: DateTime) -> Self {
        Self {
            local,
            tz: self.tz.clone(),
            fold: self.fold,
        }
    }

    /// Formats with a C-library/Python `strftime` format string.
    pub fn strftime(&self, format: &str) -> String {
        strftime(format, self.local, Some((self.offset(), &self.tz_name())))
    }

    /// Python's `isoformat()`, e.g. `2020-01-01T00:00:00+00:00`.
    pub fn isoformat(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for ZonedDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.local, self.offset())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_offsets_convert_both_ways() {
        let tz = TimeZone::Fixed(UtcOffset::from_seconds(-(5 * 3600 + 1800)).unwrap());
        let utc = DateTime::new(2020, 6, 15, 17, 30, 0, 0).unwrap();
        let zoned = ZonedDateTime::from_utc(utc, tz).unwrap();
        assert_eq!(zoned.isoformat(), "2020-06-15T12:00:00-05:30");
        assert_eq!(zoned.to_utc(), Ok(utc));
        assert_eq!(zoned.strftime("%z %Z"), "-0530 UTC-05:30");
        assert_eq!(
            ZonedDateTime::utc(utc).to_string(),
            "2020-06-15T17:30:00+00:00"
        );
        let early = ZonedDateTime::from_local(
            DateTime::new(1, 1, 1, 0, 0, 0, 0).unwrap(),
            TimeZone::Fixed(UtcOffset::from_seconds(3600).unwrap()),
        );
        assert_eq!(early.to_utc(), Err(DateError::Overflow));
    }
}
