//! Time-zone-aware date-times (Python's aware `datetime`).

use std::cmp::Ordering;
use std::fmt;

use super::{TimeZone, UtcOffset};
use crate::dates::strftime::{ZoneFields, strftime};
use crate::dates::{DateError, DateTime, TimeDelta};

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

    /// The same wall-clock time and zone with another `fold`
    /// (`replace(fold=...)`).
    #[must_use]
    pub const fn with_fold(mut self, fold: bool) -> Self {
        self.fold = fold;
        self
    }

    /// Python's `self - earlier` of aware date-times: the difference of
    /// the wall clocks when both are in one zone, of the instants when
    /// they are in two.
    ///
    /// ```
    /// use pltrs::dates::{DateTime, TimeZone, UtcOffset, ZonedDateTime};
    /// let east = TimeZone::Fixed(UtcOffset::from_seconds(3600)?);
    /// let noon = DateTime::new(2020, 1, 1, 12, 0, 0, 0)?;
    /// let utc = ZonedDateTime::utc(noon);
    /// let there = ZonedDateTime::from_local(noon, east);
    /// assert_eq!(utc.since(&there)?.total_seconds(), 3600.0);
    /// assert_eq!(utc.since(&utc)?.total_seconds(), 0.0);
    /// # Ok::<(), pltrs::dates::DateError>(())
    /// ```
    pub fn since(&self, earlier: &Self) -> Result<TimeDelta, DateError> {
        Ok(if self.tz == earlier.tz {
            self.local - earlier.local
        } else {
            self.to_utc()? - earlier.to_utc()?
        })
    }

    /// Python's ordering of aware date-times: by wall clock within one
    /// zone, by instant across two.
    pub fn compare(&self, other: &Self) -> Result<Ordering, DateError> {
        Ok(if self.tz == other.tz {
            self.local.cmp(&other.local)
        } else {
            self.to_utc()?.cmp(&other.to_utc()?)
        })
    }

    /// The wall-clock time in `tz` of this date-time
    /// (`astimezone(tz).replace(tzinfo=None)`), which is its own when it
    /// is in `tz` already.
    pub fn local_in(&self, tz: &TimeZone) -> Result<DateTime, DateError> {
        if &self.tz == tz {
            Ok(self.local)
        } else {
            Ok(self.to_zone(tz.clone())?.local)
        }
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
        strftime(format, self.local, Some(&ZoneFields::new(self)))
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

    #[test]
    fn python_compares_one_zone_on_the_wall_clock() {
        let Ok(berlin) = TimeZone::named("Europe/Berlin") else {
            return; // no system zoneinfo database
        };
        // 02:30 twice on 2020-10-25: CEST first, CET with fold.
        let local = DateTime::new(2020, 10, 25, 2, 30, 0, 0).unwrap();
        let first = ZonedDateTime::from_local(local, berlin.clone());
        let second = first.clone().with_fold(true);
        assert_eq!(first.compare(&second), Ok(Ordering::Equal));
        assert_eq!(second.since(&first).unwrap().total_seconds(), 0.0);
        let in_utc = ZonedDateTime::utc(second.to_utc().unwrap());
        assert_eq!(first.compare(&in_utc), Ok(Ordering::Less));
        assert_eq!(in_utc.since(&first).unwrap().total_seconds(), 3600.0);
        assert_eq!(in_utc.local_in(&berlin), Ok(local));
        assert_eq!(second.local_in(&TimeZone::Utc), Ok(in_utc.local()));
    }
}
