//! Ticks at multiples of a number of microseconds (matplotlib's
//! `MicrosecondLocator`).

use super::{DateLocator, date_nonsingular, ticks_in_view, to_nums};
use crate::dates::convert::MICROSECONDS_PER_DAY;
use crate::dates::tz::{TimeZone, ZonedDateTime};
use crate::ticker::{Locator, MultipleLocator, TickContext, TickerError};

/// Ticks every `interval` microseconds, counted from the start of the day
/// containing the view's lower limit.
///
/// Date numbers only resolve microseconds within about 70 years of the
/// epoch; further away, ticks are spaced correctly but their labels jitter.
#[derive(Clone, Debug, PartialEq)]
pub struct MicrosecondLocator {
    interval: u32,
    multiples: MultipleLocator,
    tz: TimeZone,
}

impl MicrosecondLocator {
    /// Ticks every `interval` microseconds (at least 1), in UTC.
    pub fn new(interval: u32) -> Result<Self, TickerError> {
        Ok(Self {
            interval,
            multiples: MultipleLocator::new(f64::from(interval))?,
            tz: TimeZone::Utc,
        })
    }

    /// Places ticks in `tz` instead of UTC.
    #[must_use]
    pub fn with_tz(mut self, tz: TimeZone) -> Self {
        self.tz = tz;
        self
    }

    /// Microseconds between ticks.
    pub const fn micros(&self) -> u32 {
        self.interval
    }
}

impl Locator for MicrosecondLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        _: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        ticks_in_view(self, vmin, vmax)
    }

    fn nonsingular(&self, v0: f64, v1: f64, ctx: &TickContext<'_>) -> (f64, f64) {
        date_nonsingular(v0, v1, self.unit(ctx), self.interval())
    }

    fn date_unit(&self, ctx: &TickContext<'_>) -> Option<f64> {
        Some(self.unit(ctx))
    }
}

impl DateLocator for MicrosecondLocator {
    fn tz(&self) -> &TimeZone {
        &self.tz
    }

    fn unit(&self, _: &TickContext<'_>) -> f64 {
        1.0 / MICROSECONDS_PER_DAY
    }

    fn interval(&self) -> f64 {
        f64::from(self.interval)
    }

    fn tick_dates(
        &self,
        dmin: &ZonedDateTime,
        dmax: &ZonedDateTime,
    ) -> Result<Vec<f64>, TickerError> {
        let limits = to_nums([dmin, dmax])?;
        let day = limits[0].floor();
        let lo = (limits[0] - day) * MICROSECONDS_PER_DAY;
        let hi = (limits[1] - day) * MICROSECONDS_PER_DAY;
        let ticks = self
            .multiples
            .tick_values(lo, hi, &TickContext::default())?;
        Ok(ticks
            .into_iter()
            .map(|t| t / MICROSECONDS_PER_DAY + day)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_bracket_the_view() {
        let locator = MicrosecondLocator::new(10).unwrap();
        let ticks = locator
            .tick_values(0.0, 100.0 / MICROSECONDS_PER_DAY, &TickContext::default())
            .unwrap();
        assert_eq!(ticks.len(), 13);
        assert_eq!(ticks[0], -1.157_407_407_407_407_4e-10);
        assert_eq!(ticks[1], 0.0);
        assert!(MicrosecondLocator::new(0).is_err());
        assert_eq!(
            locator.nonsingular(f64::INFINITY, 0.0, &TickContext::default()),
            (0.0, 1.0)
        );
    }
}
