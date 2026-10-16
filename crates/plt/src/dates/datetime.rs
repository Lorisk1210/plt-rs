//! Naive civil dates, times of day and date-times (Python's `date`, `time`
//! and naive `datetime`), restricted to years 1 through 9999.

use std::fmt;
use std::ops::Sub;

use super::DateError;
use super::calendar::{self, Weekday};
use super::timedelta::{MICROS_PER_DAY, MICROS_PER_SECOND, TimeDelta};

/// The smallest representable year.
pub const MIN_YEAR: i32 = 1;
/// The largest representable year.
pub const MAX_YEAR: i32 = 9999;

/// A proleptic Gregorian calendar date between 0001-01-01 and 9999-12-31.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

impl Date {
    /// The earliest date, 0001-01-01.
    pub const MIN: Self = Self {
        year: MIN_YEAR,
        month: 1,
        day: 1,
    };
    /// The latest date, 9999-12-31.
    pub const MAX: Self = Self {
        year: MAX_YEAR,
        month: 12,
        day: 31,
    };

    /// A validated date.
    ///
    /// ```
    /// use plt::dates::Date;
    /// assert!(Date::new(2020, 2, 29).is_ok());
    /// assert!(Date::new(2021, 2, 29).is_err());
    /// ```
    pub fn new(year: i32, month: u32, day: u32) -> Result<Self, DateError> {
        if !(MIN_YEAR..=MAX_YEAR).contains(&year) {
            return Err(DateError::field("year", year));
        }
        if !(1..=12).contains(&month) {
            return Err(DateError::field("month", month));
        }
        if day < 1 || day > calendar::days_in_month(i64::from(year), month) {
            return Err(DateError::field("day", day));
        }
        Ok(Self {
            year,
            month: month as u8,
            day: day as u8,
        })
    }

    /// The date `days` after 1970-01-01.
    pub fn from_days_since_epoch(days: i64) -> Result<Self, DateError> {
        let (year, month, day) = calendar::civil_from_days(days);
        if !(i64::from(MIN_YEAR)..=i64::from(MAX_YEAR)).contains(&year) {
            return Err(DateError::Overflow);
        }
        Ok(Self {
            year: year as i32,
            month: month as u8,
            day: day as u8,
        })
    }

    /// Days from 1970-01-01 to this date.
    pub fn days_since_epoch(self) -> i64 {
        calendar::days_from_civil(i64::from(self.year), self.month(), self.day())
    }

    /// The year (1-9999).
    pub const fn year(self) -> i32 {
        self.year
    }

    /// The month (1-12).
    pub const fn month(self) -> u32 {
        self.month as u32
    }

    /// The day of the month (1-31).
    pub const fn day(self) -> u32 {
        self.day as u32
    }

    /// The day of the week.
    pub fn weekday(self) -> Weekday {
        Weekday::from_days_since_epoch(self.days_since_epoch())
    }

    /// The day of the year, 1-366.
    pub fn ordinal(self) -> u32 {
        calendar::days_before_month(i64::from(self.year), self.month()) + self.day()
    }

    /// The ISO 8601 week-numbering year and week number.
    pub fn iso_week(self) -> (i64, u32) {
        calendar::iso_week(i64::from(self.year), self.month(), self.day())
    }

    /// The date `days` days later (earlier if negative).
    pub fn checked_add_days(self, days: i64) -> Result<Self, DateError> {
        let target = self
            .days_since_epoch()
            .checked_add(days)
            .ok_or(DateError::Overflow)?;
        Self::from_days_since_epoch(target)
    }
}

impl fmt::Display for Date {
    /// ISO 8601: `YYYY-MM-DD`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// A time of day with microsecond resolution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    hour: u8,
    minute: u8,
    second: u8,
    microsecond: u32,
}

impl Time {
    /// Midnight, 00:00:00.
    pub const MIDNIGHT: Self = Self {
        hour: 0,
        minute: 0,
        second: 0,
        microsecond: 0,
    };

    /// A validated time of day.
    pub fn new(hour: u32, minute: u32, second: u32, microsecond: u32) -> Result<Self, DateError> {
        if hour > 23 {
            return Err(DateError::field("hour", hour));
        }
        if minute > 59 {
            return Err(DateError::field("minute", minute));
        }
        if second > 59 {
            return Err(DateError::field("second", second));
        }
        if microsecond > 999_999 {
            return Err(DateError::field("microsecond", microsecond));
        }
        Ok(Self {
            hour: hour as u8,
            minute: minute as u8,
            second: second as u8,
            microsecond,
        })
    }

    /// The time `micros` microseconds after midnight (`0..86_400_000_000`).
    const fn from_micros_of_day(micros: i64) -> Self {
        let seconds = micros / MICROS_PER_SECOND;
        Self {
            hour: (seconds / 3600) as u8,
            minute: (seconds / 60 % 60) as u8,
            second: (seconds % 60) as u8,
            microsecond: (micros % MICROS_PER_SECOND) as u32,
        }
    }

    /// Microseconds since midnight.
    pub const fn micros_of_day(self) -> i64 {
        ((self.hour as i64 * 60 + self.minute as i64) * 60 + self.second as i64) * MICROS_PER_SECOND
            + self.microsecond as i64
    }

    /// The hour (0-23).
    pub const fn hour(self) -> u32 {
        self.hour as u32
    }

    /// The minute (0-59).
    pub const fn minute(self) -> u32 {
        self.minute as u32
    }

    /// The second (0-59).
    pub const fn second(self) -> u32 {
        self.second as u32
    }

    /// The microsecond (0-999999).
    pub const fn microsecond(self) -> u32 {
        self.microsecond
    }
}

impl fmt::Display for Time {
    /// ISO 8601: `HH:MM:SS` plus `.ffffff` when the microsecond is non-zero.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)?;
        match self.microsecond {
            0 => Ok(()),
            us => write!(f, ".{us:06}"),
        }
    }
}

/// A naive (time-zone-less) date and time with microsecond resolution.
///
/// matplotlib treats naive date-times as UTC; [`ZonedDateTime`] attaches a
/// time zone.
///
/// [`ZonedDateTime`]: super::ZonedDateTime
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime {
    date: Date,
    time: Time,
}

impl DateTime {
    /// Combines a date and a time of day.
    pub const fn from_parts(date: Date, time: Time) -> Self {
        Self { date, time }
    }

    /// A validated date-time from its fields.
    ///
    /// ```
    /// use plt::dates::DateTime;
    /// let dt = DateTime::new(2020, 3, 5, 14, 7, 9, 123_456).unwrap();
    /// assert_eq!(dt.to_string(), "2020-03-05T14:07:09.123456");
    /// ```
    pub fn new(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        microsecond: u32,
    ) -> Result<Self, DateError> {
        Ok(Self {
            date: Date::new(year, month, day)?,
            time: Time::new(hour, minute, second, microsecond)?,
        })
    }

    /// Midnight at the start of `date`.
    pub const fn midnight(date: Date) -> Self {
        Self {
            date,
            time: Time::MIDNIGHT,
        }
    }

    /// The date-time `micros` microseconds after 1970-01-01T00:00:00.
    pub fn from_timestamp_micros(micros: i64) -> Result<Self, DateError> {
        let date = Date::from_days_since_epoch(micros.div_euclid(MICROS_PER_DAY))?;
        let time = Time::from_micros_of_day(micros.rem_euclid(MICROS_PER_DAY));
        Ok(Self { date, time })
    }

    /// Microseconds since 1970-01-01T00:00:00 (negative before it).
    pub fn timestamp_micros(self) -> i64 {
        self.date.days_since_epoch() * MICROS_PER_DAY + self.time.micros_of_day()
    }

    /// The calendar date.
    pub const fn date(self) -> Date {
        self.date
    }

    /// The time of day.
    pub const fn time(self) -> Time {
        self.time
    }

    /// The year (1-9999).
    pub const fn year(self) -> i32 {
        self.date.year()
    }

    /// The month (1-12).
    pub const fn month(self) -> u32 {
        self.date.month()
    }

    /// The day of the month (1-31).
    pub const fn day(self) -> u32 {
        self.date.day()
    }

    /// The hour (0-23).
    pub const fn hour(self) -> u32 {
        self.time.hour()
    }

    /// The minute (0-59).
    pub const fn minute(self) -> u32 {
        self.time.minute()
    }

    /// The second (0-59).
    pub const fn second(self) -> u32 {
        self.time.second()
    }

    /// The microsecond (0-999999).
    pub const fn microsecond(self) -> u32 {
        self.time.microsecond()
    }

    /// The day of the week.
    pub fn weekday(self) -> Weekday {
        self.date.weekday()
    }

    /// The same instant with the microsecond replaced.
    pub fn with_microsecond(self, microsecond: u32) -> Result<Self, DateError> {
        let t = self.time;
        Ok(Self {
            date: self.date,
            time: Time::new(t.hour(), t.minute(), t.second(), microsecond)?,
        })
    }

    /// `self + delta`, failing outside years 1-9999.
    pub fn checked_add(self, delta: TimeDelta) -> Result<Self, DateError> {
        let micros = self
            .timestamp_micros()
            .checked_add(delta.total_micros())
            .ok_or(DateError::Overflow)?;
        Self::from_timestamp_micros(micros)
    }

    /// `self - delta`, failing outside years 1-9999.
    pub fn checked_sub(self, delta: TimeDelta) -> Result<Self, DateError> {
        self.checked_add(delta.checked_neg().ok_or(DateError::Overflow)?)
    }

    /// Python's `isoformat(sep)`.
    pub fn isoformat_with(self, separator: char) -> String {
        format!("{}{separator}{}", self.date, self.time)
    }
}

impl Sub for DateTime {
    type Output = TimeDelta;

    /// The exact duration between two date-times.
    fn sub(self, rhs: Self) -> TimeDelta {
        TimeDelta::from_micros(self.timestamp_micros() - rhs.timestamp_micros())
    }
}

impl fmt::Display for DateTime {
    /// ISO 8601 with a `T` separator (Python's `isoformat()`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}T{}", self.date, self.time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_rejects_impossible_fields() {
        assert!(Date::new(0, 1, 1).is_err());
        assert!(Date::new(10_000, 1, 1).is_err());
        assert!(Date::new(2020, 13, 1).is_err());
        assert!(Date::new(1900, 2, 29).is_err());
        assert!(Date::new(2000, 2, 29).is_ok());
        assert!(Time::new(24, 0, 0, 0).is_err());
        assert!(Time::new(0, 0, 0, 1_000_000).is_err());
    }

    #[test]
    fn timestamps_round_trip_across_boundaries() {
        let cases = [
            DateTime::new(1, 1, 1, 0, 0, 0, 0).unwrap(),
            DateTime::new(1969, 12, 31, 23, 59, 59, 999_999).unwrap(),
            DateTime::new(1970, 1, 1, 0, 0, 0, 0).unwrap(),
            DateTime::new(2000, 2, 29, 12, 0, 0, 0).unwrap(),
            DateTime::new(9999, 12, 31, 23, 59, 59, 999_999).unwrap(),
        ];
        for dt in cases {
            assert_eq!(
                DateTime::from_timestamp_micros(dt.timestamp_micros()),
                Ok(dt)
            );
        }
        assert_eq!(cases[1].timestamp_micros(), -1);
        assert!(DateTime::from_timestamp_micros(cases[4].timestamp_micros() + 1).is_err());
    }

    #[test]
    fn arithmetic_and_formatting() {
        let dt = DateTime::new(2020, 12, 31, 23, 0, 0, 0).unwrap();
        let later = dt.checked_add(TimeDelta::new(0, 7200, 0).unwrap()).unwrap();
        assert_eq!(later.to_string(), "2021-01-01T01:00:00");
        assert_eq!(later - dt, TimeDelta::new(0, 7200, 0).unwrap());
        assert_eq!(dt.weekday(), Weekday::Thursday);
        assert_eq!(dt.date().ordinal(), 366);
        assert_eq!(Date::new(5, 3, 4).unwrap().to_string(), "0005-03-04");
        assert!(Date::MIN.checked_add_days(-1).is_err());
        assert_eq!(dt.isoformat_with(' '), "2020-12-31 23:00:00");
    }
}
