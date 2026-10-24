//! Calendar-aware differences (dateutil's `relativedelta`, relative fields).

use std::ops::Neg;

use super::calendar::days_in_month;
use super::timedelta::{MICROS_PER_DAY, MICROS_PER_SECOND};
use super::{DateError, DateTime, TimeDelta};

/// A difference in years, months, days and clock units that adds to dates
/// month-aware: adding one month to January 31 gives the last day of
/// February.
///
/// Values are normalised like dateutil's: each unit is folded into the next
/// larger one when it exceeds its range (`|months| <= 11`, `|hours| <= 23`,
/// ...), keeping its sign. Units may still have mixed signs, exactly as
/// dateutil computes them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RelativeDelta {
    years: i64,
    months: i64,
    days: i64,
    hours: i64,
    minutes: i64,
    seconds: i64,
    microseconds: i64,
}

impl RelativeDelta {
    /// A normalised relative difference from its parts.
    ///
    /// ```
    /// use plt::dates::{DateTime, RelativeDelta};
    /// let one_month = RelativeDelta::new(0, 1, 0, 0, 0, 0, 0);
    /// let jan31 = DateTime::new(2020, 1, 31, 0, 0, 0, 0).unwrap();
    /// assert_eq!(one_month.add_to(jan31).unwrap().to_string(), "2020-02-29T00:00:00");
    /// ```
    pub fn new(
        years: i64,
        months: i64,
        days: i64,
        hours: i64,
        minutes: i64,
        seconds: i64,
        microseconds: i64,
    ) -> Self {
        let mut delta = Self {
            years,
            months,
            days,
            hours,
            minutes,
            seconds,
            microseconds,
        };
        delta.normalise();
        delta
    }

    /// `relativedelta(later, earlier)`: the difference such that
    /// `earlier + delta == later`, with whole months taken first.
    ///
    /// ```
    /// use plt::dates::{DateTime, RelativeDelta};
    /// let a = DateTime::new(2020, 3, 31, 12, 0, 0, 0).unwrap();
    /// let b = DateTime::new(2019, 1, 30, 0, 0, 0, 0).unwrap();
    /// let d = RelativeDelta::between(a, b).unwrap();
    /// assert_eq!((d.years(), d.months(), d.days(), d.hours()), (1, 2, 1, 12));
    /// ```
    pub fn between(later: DateTime, earlier: DateTime) -> Result<Self, DateError> {
        let mut months = i64::from(later.year() - earlier.year()) * 12 + i64::from(later.month())
            - i64::from(earlier.month());
        let step = if later < earlier { 1 } else { -1 };
        let overshoots = |shifted: DateTime| {
            if later < earlier {
                later > shifted
            } else {
                later < shifted
            }
        };
        let mut delta = Self::from_months(months);
        let mut shifted = delta.add_to(earlier)?;
        while overshoots(shifted) {
            months += step;
            delta = Self::from_months(months);
            shifted = delta.add_to(earlier)?;
        }
        let rest = later - shifted;
        delta.seconds = rest.seconds() + rest.days() * 86_400;
        delta.microseconds = rest.microseconds();
        delta.normalise();
        Ok(delta)
    }

    /// Whole years.
    pub const fn years(&self) -> i64 {
        self.years
    }

    /// Months beyond the whole years (`-11..=11`).
    pub const fn months(&self) -> i64 {
        self.months
    }

    /// Days.
    pub const fn days(&self) -> i64 {
        self.days
    }

    /// Hours (`-23..=23`).
    pub const fn hours(&self) -> i64 {
        self.hours
    }

    /// Minutes (`-59..=59`).
    pub const fn minutes(&self) -> i64 {
        self.minutes
    }

    /// Seconds (`-59..=59`).
    pub const fn seconds(&self) -> i64 {
        self.seconds
    }

    /// Microseconds (`-999_999..=999_999`).
    pub const fn microseconds(&self) -> i64 {
        self.microseconds
    }

    /// `dt + self`: shifts the year and month (clamping the day to the
    /// month's length), then adds the remaining units exactly.
    pub fn add_to(&self, dt: DateTime) -> Result<DateTime, DateError> {
        let mut year = i64::from(dt.year()) + self.years;
        let mut month = i64::from(dt.month()) + self.months;
        if month > 12 {
            year += 1;
            month -= 12;
        } else if month < 1 {
            year -= 1;
            month += 12;
        }
        let year = i32::try_from(year).map_err(|_| DateError::Overflow)?;
        let month = month as u32;
        let day = dt.day().min(days_in_month(i64::from(year), month));
        let time = dt.time();
        let anchored = DateTime::new(
            year,
            month,
            day,
            time.hour(),
            time.minute(),
            time.second(),
            time.microsecond(),
        )?;
        let micros = i128::from(self.days) * i128::from(MICROS_PER_DAY)
            + (i128::from(self.hours) * 3600
                + i128::from(self.minutes) * 60
                + i128::from(self.seconds))
                * i128::from(MICROS_PER_SECOND)
            + i128::from(self.microseconds);
        let shift = i64::try_from(micros).map_err(|_| DateError::Overflow)?;
        anchored.checked_add(TimeDelta::from_micros(shift))
    }

    /// `dt - self`.
    pub fn subtract_from(&self, dt: DateTime) -> Result<DateTime, DateError> {
        (-*self).add_to(dt)
    }

    /// dateutil's `_set_months`: a delta of `months` months only.
    fn from_months(months: i64) -> Self {
        let mut delta = Self {
            months,
            ..Self::default()
        };
        delta.normalise();
        delta
    }

    /// dateutil's `_fix`: carries each unit into the next, keeping signs.
    fn normalise(&mut self) {
        fn carry(value: &mut i64, next: &mut i64, limit: i64) {
            if value.abs() >= limit {
                let sign = value.signum();
                let magnitude = value.abs();
                *next += magnitude / limit * sign;
                *value = magnitude % limit * sign;
            }
        }
        carry(&mut self.microseconds, &mut self.seconds, 1_000_000);
        carry(&mut self.seconds, &mut self.minutes, 60);
        carry(&mut self.minutes, &mut self.hours, 60);
        carry(&mut self.hours, &mut self.days, 24);
        carry(&mut self.months, &mut self.years, 12);
    }
}

impl Neg for RelativeDelta {
    type Output = Self;

    /// Negates every unit (saturating at the `i64` limits).
    fn neg(self) -> Self {
        Self::new(
            self.years.saturating_neg(),
            self.months.saturating_neg(),
            self.days.saturating_neg(),
            self.hours.saturating_neg(),
            self.minutes.saturating_neg(),
            self.seconds.saturating_neg(),
            self.microseconds.saturating_neg(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::datetime;

    fn parts(d: RelativeDelta) -> [i64; 7] {
        [
            d.years,
            d.months,
            d.days,
            d.hours,
            d.minutes,
            d.seconds,
            d.microseconds,
        ]
    }

    #[test]
    fn between_matches_dateutil() {
        let d = RelativeDelta::between(
            datetime(2020, 3, 1, 0, 0, 0, 0),
            datetime(2020, 1, 31, 0, 0, 0, 0),
        )
        .unwrap();
        assert_eq!(parts(d), [0, 1, 1, 0, 0, 0, 0]);
        // dateutil keeps a positive microsecond part on a negative delta.
        let d = RelativeDelta::between(
            datetime(2020, 1, 1, 0, 0, 0, 0),
            datetime(2020, 1, 1, 0, 0, 1, 500_000),
        )
        .unwrap();
        assert_eq!(parts(d), [0, 0, 0, 0, 0, -2, 500_000]);
        assert_eq!(parts(-d), [0, 0, 0, 0, 0, 2, -500_000]);
        let d = RelativeDelta::between(
            datetime(2019, 1, 1, 0, 0, 0, 0),
            datetime(2020, 1, 15, 6, 0, 0, 0),
        )
        .unwrap();
        assert_eq!(parts(d), [-1, 0, -14, -6, 0, 0, 0]);
    }

    #[test]
    fn adding_clamps_days_and_reports_overflow() {
        let year = RelativeDelta::new(1, 0, 0, 0, 0, 0, 0);
        assert_eq!(
            year.add_to(datetime(2020, 2, 29, 0, 0, 0, 0)).unwrap(),
            datetime(2021, 2, 28, 0, 0, 0, 0)
        );
        assert_eq!(
            year.subtract_from(datetime(1, 6, 1, 0, 0, 0, 0)),
            Err(DateError::field("year", 0))
        );
        let carry = RelativeDelta::new(0, 14, 0, 25, 0, 0, 1_500_000);
        assert_eq!(parts(carry), [1, 2, 1, 1, 0, 1, 500_000]);
        let far = RelativeDelta::new(0, 0, 0, 0, 0, 1, 0);
        assert_eq!(
            far.add_to(datetime(9999, 12, 31, 23, 59, 59, 0)),
            Err(DateError::Overflow)
        );
    }
}
