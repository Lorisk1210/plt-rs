//! Date tick locators (matplotlib's `DateLocator` family).
//!
//! Every locator implements the ticker [`Locator`] trait on Matplotlib date
//! numbers: [`Locator::tick_values`] takes a view interval in date numbers,
//! converts it to date-times in the locator's time zone (`viewlim_to_dt`)
//! and places ticks there, exactly like matplotlib's `locator()` on an
//! axis. [`DateLocator::tick_dates`] is the date-time level entry point
//! (matplotlib's `DateLocator.tick_values`).

mod auto;
mod microsecond;
mod recurring;

pub use auto::{AutoDateLocator, AutoFrequency, ChosenLocator, LocatorChoice};
pub use microsecond::MicrosecondLocator;
pub use recurring::{RRuleLocator, YearLocator};

use super::convert::{date_to_num, num2date};
use super::tz::{TimeZone, ZonedDateTime};
use super::{Date, DateError};
use crate::ticker::{Locator, TickContext, TickerError};

/// What all date locators provide beyond [`Locator`].
pub trait DateLocator: Locator {
    /// The time zone ticks are placed in.
    fn tz(&self) -> &TimeZone;

    /// Days per tick unit (matplotlib's `_get_unit`), which drives
    /// [`AutoDateFormatter`](super::AutoDateFormatter). It may depend on the
    /// view interval in `ctx`.
    fn unit(&self, ctx: &TickContext<'_>) -> f64;

    /// Units between ticks (matplotlib's `_get_interval`).
    fn interval(&self) -> f64;

    /// Tick locations (date numbers) between two date-times in the
    /// locator's zone, `dmin <= dmax` (matplotlib's `tick_values`).
    fn tick_dates(
        &self,
        dmin: &ZonedDateTime,
        dmax: &ZonedDateTime,
    ) -> Result<Vec<f64>, TickerError>;
}

impl From<DateError> for TickerError {
    fn from(err: DateError) -> Self {
        match err {
            DateError::NonFinite(_) => Self::NonFinite("date number"),
            other => Self::InvalidParameter {
                name: "date",
                reason: other.to_string(),
            },
        }
    }
}

/// matplotlib's `viewlim_to_dt`: the ordered view interval as date-times
/// in `tz`.
fn view_dates(
    vmin: f64,
    vmax: f64,
    tz: &TimeZone,
) -> Result<(ZonedDateTime, ZonedDateTime), DateError> {
    let (lo, hi) = if vmin > vmax {
        (vmax, vmin)
    } else {
        (vmin, vmax)
    };
    Ok((num2date(lo, tz)?, num2date(hi, tz)?))
}

/// The ticks of a locator whose `__call__` returns no ticks when the view
/// is not a valid date range (matplotlib catches the `ValueError`).
fn ticks_in_view<L: DateLocator + ?Sized>(
    locator: &L,
    vmin: f64,
    vmax: f64,
) -> Result<Vec<f64>, TickerError> {
    match view_dates(vmin, vmax, locator.tz()) {
        Ok((dmin, dmax)) => locator.tick_dates(&dmin, &dmax),
        Err(DateError::DateNumberOutOfRange(_) | DateError::NonFinite(_)) => Ok(Vec::new()),
        Err(err) => Err(err.into()),
    }
}

/// `date2num` of 1970-01-01 and 1970-01-02: the limits used when there
/// are no finite data.
fn default_limits() -> (f64, f64) {
    let first = Date::new(1970, 1, 1).expect("1970-01-01 is a valid date");
    let second = Date::new(1970, 1, 2).expect("1970-01-02 is a valid date");
    (date_to_num(first), date_to_num(second))
}

/// `DateLocator.nonsingular`: default limits without finite data, and a
/// range widened by two tick spacings (`unit * interval` days) when empty.
fn date_nonsingular(vmin: f64, vmax: f64, unit: f64, interval: f64) -> (f64, f64) {
    if !vmin.is_finite() || !vmax.is_finite() {
        return default_limits();
    }
    let (vmin, vmax) = if vmax < vmin {
        (vmax, vmin)
    } else {
        (vmin, vmax)
    };
    if (vmax - vmin).abs() < 1e-6 {
        let pad = 2.0 * unit * interval;
        (vmin - pad, vmax + pad)
    } else {
        (vmin, vmax)
    }
}

/// `date2num` for tick results.
fn to_nums<'a>(
    dates: impl IntoIterator<Item = &'a ZonedDateTime>,
) -> Result<Vec<f64>, TickerError> {
    dates
        .into_iter()
        .map(|d| super::convert::date2num(d).map_err(TickerError::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonsingular_widens_empty_ranges() {
        assert_eq!(date_nonsingular(f64::NAN, 1.0, 1.0, 1.0), (0.0, 1.0));
        assert_eq!(date_nonsingular(3.0, 1.0, 1.0, 1.0), (1.0, 3.0));
        assert_eq!(date_nonsingular(5.0, 5.0, 365.0, 5.0), (-3645.0, 3655.0));
        let err: TickerError = DateError::NonFinite(f64::NAN).into();
        assert_eq!(err, TickerError::NonFinite("date number"));
    }
}
