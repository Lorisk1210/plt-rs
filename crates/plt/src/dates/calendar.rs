//! Proleptic Gregorian calendar arithmetic.
//!
//! Every function here works on any year representable as `i64`, including
//! year 0 and negative (astronomical) years; the date types built on top
//! restrict themselves to Python's `datetime` range of years 1 to 9999.
//! Day counts are relative to the Unix epoch, 1970-01-01, using Howard
//! Hinnant's `days_from_civil` / `civil_from_days` algorithms.

use std::fmt;

/// Days in a 400-year Gregorian cycle.
const DAYS_PER_ERA: i64 = 146_097;
/// Days from 0000-03-01 to 1970-01-01.
const UNIX_EPOCH_SHIFT: i64 = 719_468;

/// Whether `year` is a Gregorian leap year.
///
/// ```
/// use plt::dates::calendar::is_leap_year;
/// assert!(is_leap_year(2000) && is_leap_year(2024) && is_leap_year(-4));
/// assert!(!is_leap_year(1900) && !is_leap_year(2023));
/// ```
pub const fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Number of days in `year` (365 or 366).
pub const fn days_in_year(year: i64) -> u32 {
    if is_leap_year(year) { 366 } else { 365 }
}

/// Number of days in `month` (1-12) of `year`; 0 for an invalid month.
pub const fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Number of days in `year` before the first day of `month` (1-12).
pub const fn days_before_month(year: i64, month: u32) -> u32 {
    const CUMULATIVE: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let leap_shift = if month > 2 && is_leap_year(year) {
        1
    } else {
        0
    };
    CUMULATIVE[(month as usize).saturating_sub(1) % 12] + leap_shift
}

/// Days from 1970-01-01 to the given civil date (negative before it).
///
/// The fields are not validated; out-of-range days simply roll over.
///
/// ```
/// use plt::dates::calendar::days_from_civil;
/// assert_eq!(days_from_civil(1970, 1, 1), 0);
/// assert_eq!(days_from_civil(2000, 3, 1), 11_017);
/// assert_eq!(days_from_civil(1, 1, 1), -719_162);
/// ```
pub const fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let m = month as i64;
    let shifted_month = if m > 2 { m - 3 } else { m + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * DAYS_PER_ERA + day_of_era - UNIX_EPOCH_SHIFT
}

/// The civil date `(year, month, day)` that is `days` after 1970-01-01.
///
/// ```
/// use plt::dates::calendar::civil_from_days;
/// assert_eq!(civil_from_days(0), (1970, 1, 1));
/// assert_eq!(civil_from_days(-1), (1969, 12, 31));
/// assert_eq!(civil_from_days(11_016), (2000, 2, 29));
/// ```
pub const fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + UNIX_EPOCH_SHIFT;
    let era = z.div_euclid(DAYS_PER_ERA);
    let day_of_era = z - era * DAYS_PER_ERA;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month as u32, day as u32)
}

/// A day of the week.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weekday {
    /// Monday (index 0, as in Python and dateutil).
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday (index 6).
    Sunday,
}

impl Weekday {
    /// All weekdays, Monday first.
    pub const ALL: [Self; 7] = [
        Self::Monday,
        Self::Tuesday,
        Self::Wednesday,
        Self::Thursday,
        Self::Friday,
        Self::Saturday,
        Self::Sunday,
    ];

    /// The weekday with Python's index (`0` = Monday ... `6` = Sunday),
    /// taken modulo 7.
    pub const fn from_index(index: i64) -> Self {
        Self::ALL[index.rem_euclid(7) as usize]
    }

    /// The weekday of the day `days` after 1970-01-01 (a Thursday).
    pub const fn from_days_since_epoch(days: i64) -> Self {
        Self::from_index(days + 3)
    }

    /// Python's index: `0` = Monday ... `6` = Sunday.
    pub const fn index(self) -> u32 {
        self as u32
    }

    /// The C-library index: `0` = Sunday ... `6` = Saturday.
    pub const fn sunday_index(self) -> u32 {
        (self as u32 + 1) % 7
    }

    /// The English name, e.g. `"Monday"`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
            Self::Sunday => "Sunday",
        }
    }

    /// The three-letter English abbreviation, e.g. `"Mon"`.
    pub fn short_name(self) -> &'static str {
        &self.name()[..3]
    }
}

impl fmt::Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// English month names, January first.
pub const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The English name of `month` (1-12).
pub(crate) fn month_name(month: u32) -> &'static str {
    MONTH_NAMES[(month as usize).saturating_sub(1) % 12]
}

/// The ISO 8601 week-numbering year and week (1-53) of a civil date.
pub fn iso_week(year: i64, month: u32, day: u32) -> (i64, u32) {
    let days = days_from_civil(year, month, day);
    // The ISO week belongs to the year containing its Thursday.
    let weekday = i64::from(Weekday::from_days_since_epoch(days).index());
    let thursday = days - weekday + 3;
    let (iso_year, _, _) = civil_from_days(thursday);
    let week = (thursday - days_from_civil(iso_year, 1, 1)) / 7 + 1;
    (iso_year, week as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leap_years_follow_gregorian_rules() {
        for year in [1600, 2000, 2004, 2400, 0, -4, -400] {
            assert!(is_leap_year(year), "{year}");
        }
        for year in [1700, 1800, 1900, 2100, 2001, -1, -100] {
            assert!(!is_leap_year(year), "{year}");
        }
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2021, 13), 0);
        assert_eq!(days_before_month(2020, 3), 60);
        assert_eq!(days_before_month(2021, 3), 59);
        assert_eq!(days_before_month(2021, 12), 334);
    }

    #[test]
    fn civil_round_trip_spans_negative_years() {
        for days in (-1_000_000..1_000_000).step_by(997) {
            let (y, m, d) = civil_from_days(days);
            assert!((1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m));
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(civil_from_days(days_from_civil(-1, 12, 31) + 1), (0, 1, 1));
        assert_eq!(days_from_civil(0, 3, 1) - days_from_civil(0, 2, 28), 2);
        assert_eq!(days_from_civil(10_000, 1, 1), 2_932_897);
    }

    #[test]
    fn weekdays_and_iso_weeks() {
        assert_eq!(Weekday::from_days_since_epoch(0), Weekday::Thursday);
        assert_eq!(
            Weekday::from_days_since_epoch(days_from_civil(1, 1, 1)),
            Weekday::Monday
        );
        assert_eq!(Weekday::Sunday.sunday_index(), 0);
        assert_eq!(Weekday::Monday.short_name(), "Mon");
        assert_eq!(iso_week(2024, 12, 30), (2025, 1));
        assert_eq!(iso_week(2021, 1, 3), (2020, 53));
        assert_eq!(iso_week(2020, 3, 5), (2020, 10));
        assert_eq!(iso_week(5, 1, 1), (4, 53));
    }
}
