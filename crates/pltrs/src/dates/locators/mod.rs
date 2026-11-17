//! Date tick locators (matplotlib's `DateLocator` family).
//!
//! Every locator implements the ticker [`Locator`] trait on Matplotlib date
//! numbers: [`Locator::tick_values`] takes a view interval in date numbers,
//! converts it to date-times in the locator's time zone (`viewlim_to_dt`)
//! and places ticks there, exactly like matplotlib's `locator()` on an
//! axis. [`DateLocator::tick_dates`] is the date-time level entry point
//! (matplotlib's `DateLocator.tick_values`).
//!
//! Every constructor settles the zone the `timezone` rcParam names, as
//! matplotlib's `DateLocator.__init__` does with `tz=None`; `with_tz`
//! replaces it. A name the system does not know makes locating fail with
//! [`DateError::UnknownTimeZone`], where matplotlib's constructors raise,
//! so that no constructor fails for it and a zone given with `with_tz`
//! never depends on the rcParam.

/// Implements [`Locator`] for a date locator in a child module whose
/// ticks are its [`DateLocator::tick_dates`] across the view (matplotlib's
/// `DateLocator.__call__` and `DateLocator.nonsingular`).
macro_rules! locate_dates_in_view {
    ($locator:ty) => {
        impl Locator for $locator {
            fn tick_values(
                &self,
                vmin: f64,
                vmax: f64,
                _: &TickContext<'_>,
            ) -> Result<Vec<f64>, TickerError> {
                super::ticks_in_view(self, vmin, vmax)
            }

            fn nonsingular(&self, v0: f64, v1: f64, ctx: &TickContext<'_>) -> (f64, f64) {
                super::date_nonsingular(v0, v1, self.unit(ctx), self.interval())
            }

            fn date_unit(&self, ctx: &TickContext<'_>) -> Option<f64> {
                Some(self.unit(ctx))
            }
        }
    };
}

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
    /// The time zone ticks are placed in, or why the `timezone` rcParam
    /// named none when the locator was made.
    fn tz(&self) -> Result<&TimeZone, DateError>;

    /// Days per tick unit (matplotlib's `_get_unit`), which drives
    /// [`AutoDateFormatter`](super::AutoDateFormatter). It may depend on the
    /// view interval in `ctx`.
    fn unit(&self, ctx: &TickContext<'_>) -> f64;

    /// Units between ticks (matplotlib's `_get_interval`).
    fn interval(&self) -> f64;

    /// Tick locations (date numbers) between two date-times, `dmin <=
    /// dmax` (matplotlib's `tick_values`). As matplotlib's, the ticks are
    /// placed on the wall clock of the dates' zone (that of `dmin`), not
    /// necessarily the locator's: a rule with a zone of its own runs in
    /// that one ([`RRuleLocator::with_rule_tz`]).
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
            DateError::Overflow => Self::DateOverflow,
            DateError::DateNumberOutOfRange(x) => Self::DateNumberOutOfRange(x),
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
    match view_dates(vmin, vmax, locator.tz()?) {
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
    Ok(super::convert::date2num_all(dates)?)
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

    #[test]
    fn an_unknown_rc_zone_fails_when_locating_unless_replaced() {
        let (hours, years, micros) = crate::rc::context(|| {
            crate::rc::set("timezone", "Nowhere/Atlantis").unwrap();
            (
                RRuleLocator::hours(&[], 1),
                YearLocator::new(1, 1, 1).unwrap(),
                MicrosecondLocator::new(10).unwrap(),
            )
        });
        let ctx = TickContext::default();
        let unknown = DateError::UnknownTimeZone("Nowhere/Atlantis".to_owned());
        let locators: [&dyn DateLocator; 3] = [&hours, &years, &micros];
        for locator in locators {
            assert_eq!(locator.tz(), Err(unknown.clone()));
            assert!(locator.tick_values(0.0, 1.0, &ctx).is_err());
        }
        let utc = hours.with_tz(TimeZone::Utc);
        assert_eq!(utc.tick_values(0.0, 0.25, &ctx).unwrap().len(), 7);
        assert!(
            years
                .with_tz(TimeZone::Utc)
                .tick_values(0.0, 1.0, &ctx)
                .is_ok()
        );
        assert!(
            micros
                .with_tz(TimeZone::Utc)
                .tick_values(0.0, 1e-9, &ctx)
                .is_ok()
        );
    }
}
