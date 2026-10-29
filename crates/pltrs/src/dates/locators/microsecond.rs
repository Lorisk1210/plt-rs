//! Ticks at multiples of a number of microseconds (matplotlib's
//! `MicrosecondLocator`).

use super::{DateLocator, to_nums};
use crate::dates::DateError;
use crate::dates::convert::MICROSECONDS_PER_DAY;
use crate::dates::tz::{TickZone, TimeZone, ZonedDateTime};
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
    tz: TickZone,
}

impl MicrosecondLocator {
    /// Ticks every `interval` microseconds (at least 1), in the zone of
    /// the `timezone` rcParam.
    pub fn new(interval: u32) -> Result<Self, TickerError> {
        Self::in_zone(interval, TickZone::from_rc())
    }

    /// Ticks every `interval` microseconds in `tz`.
    pub(super) fn in_zone(interval: u32, tz: TickZone) -> Result<Self, TickerError> {
        Ok(Self {
            interval,
            multiples: MultipleLocator::new(f64::from(interval))?,
            tz,
        })
    }

    /// Places ticks in `tz` instead of the zone of the `timezone` rcParam.
    #[must_use]
    pub fn with_tz(mut self, tz: TimeZone) -> Self {
        self.tz = tz.into();
        self
    }

    /// Microseconds between ticks.
    pub const fn micros(&self) -> u32 {
        self.interval
    }
}

locate_dates_in_view!(MicrosecondLocator);

impl DateLocator for MicrosecondLocator {
    fn tz(&self) -> Result<&TimeZone, DateError> {
        self.tz.get()
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
