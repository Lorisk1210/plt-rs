//! The `date.*` cases: the automatic formats of every tick scale, the
//! concise converter and interval multiples.

use pltrs::dates::DateTime;
use pltrs::figure::Figure;
use pltrs::numeric::linspace;

use super::{DATED, rc_cases};
use crate::figures::{CaseResult, FigureCase};

const MILLISECOND: i64 = 1000;
const SECOND: i64 = 1000 * MILLISECOND;
const MINUTE: i64 = 60 * SECOND;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;

/// `_dates(start, step, n)`: a curve over `n` dates `step` microseconds
/// apart from `start` (year, month, day, hour, minute).
fn dates(fig: &Figure, start: (i32, u32, u32, u32, u32), step: i64, n: usize) -> CaseResult {
    let (year, month, day, hour, minute) = start;
    let origin = DateTime::new(year, month, day, hour, minute, 0, 0)?.timestamp_micros();
    let x = (0..n as i64)
        .map(|i| DateTime::from_timestamp_micros(origin + i * step))
        .collect::<Result<Vec<_>, _>>()?;
    let y: Vec<f64> = linspace(0.0, 6.0, n).into_iter().map(f64::sin).collect();
    fig.subplot().plot(x, y)?;
    Ok(())
}

/// `_YEARS`: ten years.
fn years(fig: &Figure) -> CaseResult {
    dates(fig, (2012, 1, 1, 0, 0), 60 * DAY, 61)
}

/// `_MONTHS`: seven months.
fn months(fig: &Figure) -> CaseResult {
    dates(fig, (2023, 11, 15, 0, 0), 7 * DAY, 30)
}

/// `_DAYS`: eight days.
fn days(fig: &Figure) -> CaseResult {
    dates(fig, (2024, 2, 20, 7, 0), 6 * HOUR, 33)
}

/// `_HOURS`: twelve hours from 07:20.
fn hours(fig: &Figure) -> CaseResult {
    dates(fig, (2024, 2, 20, 7, 20), 30 * MINUTE, 25)
}

/// Forty minutes.
fn minutes(fig: &Figure) -> CaseResult {
    dates(fig, (2024, 3, 1, 12, 0), 2 * MINUTE, 21)
}

/// A minute.
fn seconds(fig: &Figure) -> CaseResult {
    dates(fig, (2024, 3, 1, 12, 0), 3 * SECOND, 21)
}

/// Half a second.
fn microseconds(fig: &Figure) -> CaseResult {
    dates(fig, (2024, 3, 1, 12, 0), 25 * MILLISECOND, 21)
}

pub(crate) const CASES: &[FigureCase] = rc_cases! {
    years, DATED => {
        "date_autoformatter_year": [("date.autoformatter.year", "Y%y")],
    }
    months, DATED => {
        "date_autoformatter_month": [("date.autoformatter.month", "%b %y")],
    }
    days, DATED => {
        "date_autoformatter_day": [("date.autoformatter.day", "%d.%m.")],
        "date_converter": [("date.converter", "concise")],
    }
    hours, DATED => {
        "date_autoformatter_hour": [("date.autoformatter.hour", "%Hh")],
        "date_interval_multiples": [("date.interval_multiples", "False")],
    }
    minutes, DATED => {
        "date_autoformatter_minute": [("date.autoformatter.minute", "%H.%M")],
    }
    seconds, DATED => {
        "date_autoformatter_second": [("date.autoformatter.second", "%Mm%Ss")],
    }
    microseconds, DATED => {
        "date_autoformatter_microsecond": [("date.autoformatter.microsecond", "%S.%f")],
    }
};
