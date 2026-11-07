//! The zones the `dates_tz` probes write as `tzinfo` classes
//! (`spec/reference/probes/_dates_tz.py`), as a pltrs (Rust) program
//! writes them: [`ZoneRules`] answering as the Python classes do.

use pltrs::dates::{DateError, DateTime, TimeDelta, UtcOffset, ZoneRules};

/// An hour.
const HOUR: TimeDelta = TimeDelta::from_micros(3_600_000_000);

/// `FixedClass`: a fixed offset from UTC with a name of its own.
#[derive(Debug)]
pub(super) struct FixedClass {
    /// The offset.
    pub(super) offset: UtcOffset,
    /// The name.
    pub(super) name: String,
}

impl ZoneRules for FixedClass {
    fn offset_at_local(&self, _: DateTime, _: bool) -> UtcOffset {
        self.offset
    }

    fn name_at_local(&self, _: DateTime, _: bool) -> String {
        self.name.clone()
    }

    /// Python's own `fromutc`, which adds the offset.
    fn local_from_utc(&self, utc: DateTime) -> Result<(DateTime, bool), DateError> {
        Ok((utc.checked_add(self.offset.as_timedelta())?, false))
    }
}

/// `USClass`: the United States' rules for daylight saving time.
#[derive(Debug)]
pub(super) struct UsClass {
    /// The offset of standard time.
    pub(super) standard: UtcOffset,
    /// The offset of daylight time, an hour more.
    pub(super) daylight: UtcOffset,
    /// The name of standard time.
    pub(super) std: String,
    /// The name of daylight time.
    pub(super) dst: String,
}

/// The first Sunday at or after `dt` (`weekday()` counts from Monday).
fn first_sunday_on_or_after(dt: DateTime) -> Result<DateTime, DateError> {
    let days = 6 - i64::from(dt.weekday().index());
    dt.checked_add(TimeDelta::from_micros(days * 24 * HOUR.total_micros()))
}

/// When daylight time starts and ends in `year`, on the wall clock (the
/// second Sunday of March, the first of November, at 2:00).
fn dst_range(year: i32) -> Result<(DateTime, DateTime), DateError> {
    Ok((
        first_sunday_on_or_after(DateTime::new(year, 3, 8, 2, 0, 0, 0)?)?,
        first_sunday_on_or_after(DateTime::new(year, 11, 1, 2, 0, 0, 0)?)?,
    ))
}

impl UsClass {
    /// Whether daylight time is in effect at `local` (`dst()`), the
    /// repeated and the skipped hours told apart by `fold`.
    fn in_daylight(local: DateTime, fold: bool) -> bool {
        let Ok((start, end)) = dst_range(local.year()) else {
            return false;
        };
        let (Ok(end_less), Ok(start_more)) = (end.checked_sub(HOUR), start.checked_add(HOUR))
        else {
            return false;
        };
        if start_more <= local && local < end_less {
            true
        } else if end_less <= local && local < end {
            !fold
        } else if start <= local && local < start_more {
            fold
        } else {
            false
        }
    }
}

impl ZoneRules for UsClass {
    fn offset_at_local(&self, local: DateTime, fold: bool) -> UtcOffset {
        if Self::in_daylight(local, fold) {
            self.daylight
        } else {
            self.standard
        }
    }

    fn name_at_local(&self, local: DateTime, fold: bool) -> String {
        if Self::in_daylight(local, fold) {
            self.dst.clone()
        } else {
            self.std.clone()
        }
    }

    /// The class' `fromutc`.
    fn local_from_utc(&self, utc: DateTime) -> Result<(DateTime, bool), DateError> {
        let (start, end) = dst_range(utc.year())?;
        let std_time = utc.checked_add(self.standard.as_timedelta())?;
        let dst_time = std_time.checked_add(HOUR)?;
        Ok(if end <= dst_time && dst_time < end.checked_add(HOUR)? {
            (std_time, true)
        } else if std_time < start || dst_time >= end {
            (std_time, false)
        } else {
            (dst_time, false)
        })
    }
}
