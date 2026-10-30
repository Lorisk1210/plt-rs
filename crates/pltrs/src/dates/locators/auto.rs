//! Automatic choice of date tick frequency (matplotlib's `AutoDateLocator`).

use super::{
    DateLocator, MicrosecondLocator, RRuleLocator, YearLocator, default_limits, view_dates,
};
use crate::dates::convert::MICROSECONDS_PER_DAY;
use crate::dates::rrule::{Frequency, RRuleSpec};
use crate::dates::tz::{TickZone, TimeZone, ZonedDateTime};
use crate::dates::{DateError, RelativeDelta};
use crate::ticker::{Locator, TickContext, TickerError};

/// The tick frequencies `AutoDateLocator` chooses from, coarsest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AutoFrequency {
    /// Yearly ticks.
    Yearly,
    /// Monthly ticks.
    Monthly,
    /// Daily ticks.
    Daily,
    /// Hourly ticks.
    Hourly,
    /// Ticks every minute.
    Minutely,
    /// Ticks every second.
    Secondly,
    /// Ticks every microsecond (matplotlib's `MICROSECONDLY`).
    Microsecondly,
}

impl AutoFrequency {
    /// All frequencies, coarsest first.
    pub const ALL: [Self; 7] = [
        Self::Yearly,
        Self::Monthly,
        Self::Daily,
        Self::Hourly,
        Self::Minutely,
        Self::Secondly,
        Self::Microsecondly,
    ];

    /// The constant's name, e.g. `"MONTHLY"`.
    pub const fn name(self) -> &'static str {
        match self.rrule() {
            Some(freq) => freq.name(),
            None => "MICROSECONDLY",
        }
    }

    /// The frequency with the given constant name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == name)
    }

    /// The recurrence-rule frequency, if any.
    pub const fn rrule(self) -> Option<Frequency> {
        match self {
            Self::Yearly => Some(Frequency::Yearly),
            Self::Monthly => Some(Frequency::Monthly),
            Self::Daily => Some(Frequency::Daily),
            Self::Hourly => Some(Frequency::Hourly),
            Self::Minutely => Some(Frequency::Minutely),
            Self::Secondly => Some(Frequency::Secondly),
            Self::Microsecondly => None,
        }
    }

    /// Days per tick unit (`_get_unit`).
    pub const fn unit_days(self) -> f64 {
        match self.rrule() {
            Some(freq) => freq.unit_days(),
            None => 1.0 / MICROSECONDS_PER_DAY,
        }
    }

    const fn index(self) -> usize {
        self as usize
    }

    /// matplotlib's `_byranges`: the values `interval_multiples` steps
    /// through for this frequency's `by*` rule.
    fn byrange(self) -> Option<std::ops::Range<u32>> {
        match self {
            Self::Monthly => Some(1..13),
            Self::Daily => Some(1..32),
            Self::Hourly => Some(0..24),
            Self::Minutely | Self::Secondly => Some(0..60),
            Self::Yearly | Self::Microsecondly => None,
        }
    }
}

/// The daily tick intervals: `4` instead of `3` with interval multiples, so
/// ticks avoid the 31st (1, 5, ..., 29 rather than 1, 4, ..., 31, 1).
fn daily_intervals(interval_multiples: bool) -> Vec<u32> {
    if interval_multiples {
        vec![1, 2, 4, 7, 14]
    } else {
        vec![1, 2, 3, 7, 14, 21]
    }
}

/// Picks the best of the fixed-frequency locators for the view (matplotlib's
/// `AutoDateLocator`).
///
/// The first frequency (years, months, days, ...) giving at least
/// `minticks` ticks wins; then the first allowed interval keeping the
/// count at most `maxticks` is used. With `interval_multiples` ticks land
/// on multiples of the interval (hours 0, 6, 12, 18 rather than wherever
/// the view starts).
#[derive(Clone, Debug, PartialEq)]
pub struct AutoDateLocator {
    tz: TickZone,
    minticks: u32,
    maxticks: [u32; 7],
    interval_multiples: bool,
    intervals: [Vec<u32>; 7],
}

pltrs_core::archive_struct!(AutoDateLocator {
    tz,
    minticks,
    maxticks,
    interval_multiples,
    intervals
});

impl Default for AutoDateLocator {
    fn default() -> Self {
        Self::new()
    }
}

/// The locator `AutoDateLocator` delegates to for one view.
#[derive(Clone, Debug, PartialEq)]
pub enum ChosenLocator {
    /// Yearly ticks with interval multiples.
    Year(YearLocator),
    /// A recurrence rule from monthly to secondly (or yearly without
    /// interval multiples).
    RRule(Box<RRuleLocator>),
    /// Sub-second ticks.
    Microsecond(MicrosecondLocator),
}

pltrs_core::archive_enum!(ChosenLocator {
    Year(v0),
    RRule(v0),
    Microsecond(v0),
});

/// The outcome of [`AutoDateLocator::get_locator`].
#[derive(Clone, Debug, PartialEq)]
pub struct LocatorChoice {
    /// The delegate locator.
    pub locator: ChosenLocator,
    /// The frequency chosen, or `None` if even microseconds give fewer than
    /// `minticks` ticks (matplotlib then keeps its previous unit, yearly
    /// for a fresh locator).
    pub frequency: Option<AutoFrequency>,
    /// The interval used when none of the allowed intervals keeps the
    /// ticks within `maxticks`: the largest one, for which matplotlib warns
    /// (unless daily ticks fall on interval multiples).
    pub fallback_interval: Option<u32>,
}

impl AutoDateLocator {
    /// matplotlib's defaults: the zone of the `timezone` rcParam,
    /// `minticks = 5`, `maxticks` of 11/12/11/12/11/11/8 and interval
    /// multiples.
    pub fn new() -> Self {
        Self {
            tz: TickZone::from_rc(),
            minticks: 5,
            maxticks: [11, 12, 11, 12, 11, 11, 8],
            interval_multiples: true,
            intervals: [
                vec![
                    1, 2, 4, 5, 10, 20, 40, 50, 100, 200, 400, 500, 1000, 2000, 4000, 5000, 10_000,
                ],
                vec![1, 2, 3, 4, 6],
                daily_intervals(true),
                vec![1, 2, 3, 4, 6, 12],
                vec![1, 5, 10, 15, 30],
                vec![1, 5, 10, 15, 30],
                vec![
                    1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10_000, 20_000, 50_000,
                    100_000, 200_000, 500_000, 1_000_000,
                ],
            ],
        }
    }

    /// Places ticks in `tz` instead of the zone of the `timezone` rcParam.
    #[must_use]
    pub fn with_tz(mut self, tz: TimeZone) -> Self {
        self.tz = tz.into();
        self
    }

    /// The minimum number of ticks a frequency must give to be chosen.
    #[must_use]
    pub fn with_minticks(mut self, minticks: u32) -> Self {
        self.minticks = minticks;
        self
    }

    /// The same maximum number of ticks for every frequency.
    #[must_use]
    pub fn with_maxticks(mut self, maxticks: u32) -> Self {
        self.maxticks = [maxticks; 7];
        self
    }

    /// The maximum number of ticks for one frequency.
    #[must_use]
    pub fn with_maxticks_for(mut self, frequency: AutoFrequency, maxticks: u32) -> Self {
        self.maxticks[frequency.index()] = maxticks;
        self
    }

    /// The maximum number of ticks a frequency may give (matplotlib's
    /// `maxticks[freq]`).
    ///
    /// ```
    /// use pltrs::dates::{AutoDateLocator, AutoFrequency};
    /// let locator = AutoDateLocator::new().with_maxticks_for(AutoFrequency::Monthly, 5);
    /// assert_eq!(locator.maxticks(AutoFrequency::Monthly), 5);
    /// assert_eq!(locator.maxticks(AutoFrequency::Yearly), 11);
    /// ```
    pub fn maxticks(&self, frequency: AutoFrequency) -> u32 {
        self.maxticks[frequency.index()]
    }

    /// Whether ticks sit on multiples of the interval (default `true`);
    /// also resets the daily intervals to the matching default.
    #[must_use]
    pub fn with_interval_multiples(mut self, interval_multiples: bool) -> Self {
        self.interval_multiples = interval_multiples;
        self.intervals[AutoFrequency::Daily.index()] = daily_intervals(interval_multiples);
        self
    }

    /// The allowed tick intervals for one frequency (matplotlib's
    /// `intervald[freq]`), in increasing order.
    pub fn with_intervals(
        mut self,
        frequency: AutoFrequency,
        intervals: Vec<u32>,
    ) -> Result<Self, DateError> {
        if intervals.is_empty() || intervals.contains(&0) {
            return Err(DateError::InvalidArgument(
                "intervals must be a non-empty list of positive integers".to_owned(),
            ));
        }
        self.intervals[frequency.index()] = intervals;
        Ok(self)
    }

    /// Picks the delegate locator for the range `dmin..dmax` (either order;
    /// both in this locator's zone), matplotlib's `get_locator`.
    pub fn get_locator(
        &self,
        dmin: &ZonedDateTime,
        dmax: &ZonedDateTime,
    ) -> Result<LocatorChoice, TickerError> {
        let counts = tick_counts(dmin, dmax)?;
        let mut byranges: [Option<Vec<u32>>; 7] = [
            None,
            Some(vec![1]),
            Some(vec![1]),
            Some(vec![0]),
            Some(vec![0]),
            Some(vec![0]),
            None,
        ];
        let mut chosen = None;
        let mut fallback_interval = None;
        for frequency in AutoFrequency::ALL {
            let i = frequency.index();
            if counts[i] < f64::from(self.minticks) {
                byranges[i] = None;
                continue;
            }
            let limit = f64::from(self.maxticks[i]) - 1.0;
            let candidates = &self.intervals[i];
            let fitting = candidates
                .iter()
                .copied()
                .find(|&interval| counts[i] <= f64::from(interval) * limit);
            let mut interval = fitting.or_else(|| candidates.last().copied()).unwrap_or(1);
            let daily_multiples = self.interval_multiples && frequency == AutoFrequency::Daily;
            if fitting.is_none() && !daily_multiples {
                fallback_interval = Some(interval);
            }
            match frequency.byrange() {
                Some(range) if self.interval_multiples => {
                    // Avoid ticks on the 29th-31st. matplotlib applies this to
                    // the hourly level too (it tests `i in (DAILY, WEEKLY)`);
                    // that only matters for custom hourly intervals.
                    let day_like =
                        matches!(frequency, AutoFrequency::Daily | AutoFrequency::Hourly);
                    byranges[i] = Some(match (day_like, interval) {
                        (true, 14) => vec![1, 15],
                        (true, 7) => vec![1, 8, 15, 22],
                        _ => range.step_by(interval as usize).collect(),
                    });
                    interval = 1;
                }
                range => byranges[i] = range.map(Iterator::collect),
            }
            chosen = Some((frequency, interval));
            break;
        }
        let (frequency, interval) = chosen.unzip();
        let interval = interval.unwrap_or(1);
        let tz = self.tz.clone();
        let locator = match frequency.and_then(AutoFrequency::rrule) {
            Some(Frequency::Yearly) if self.interval_multiples => {
                ChosenLocator::Year(YearLocator::in_zone(interval, 1, 1, tz)?)
            }
            Some(freq) => {
                let [_, bymonth, bymonthday, byhour, byminute, bysecond, _] = byranges;
                let rule = RRuleSpec {
                    interval,
                    bymonth: bymonth.unwrap_or_default(),
                    bymonthday: bymonthday
                        .unwrap_or_default()
                        .into_iter()
                        .map(|d| d as i32)
                        .collect(),
                    byhour: byhour.unwrap_or_default(),
                    byminute: byminute.unwrap_or_default(),
                    bysecond: bysecond.unwrap_or_default(),
                    ..RRuleSpec::new(freq)
                };
                ChosenLocator::RRule(Box::new(RRuleLocator::in_zone(rule, tz)))
            }
            None => ChosenLocator::Microsecond(MicrosecondLocator::in_zone(interval, tz)?),
        };
        Ok(LocatorChoice {
            locator,
            frequency,
            fallback_interval,
        })
    }

    /// The choice for the view interval `(vmin, vmax)` in date numbers.
    pub fn choose(&self, vmin: f64, vmax: f64) -> Result<LocatorChoice, TickerError> {
        let (dmin, dmax) = view_dates(vmin, vmax, self.tz.get()?)?;
        self.get_locator(&dmin, &dmax)
    }
}

/// The number of years, months, days, hours, minutes, seconds and
/// microseconds spanned (matplotlib mixes `relativedelta` and `timedelta`
/// fields here, and so do we).
fn tick_counts(dmin: &ZonedDateTime, dmax: &ZonedDateTime) -> Result<[f64; 7], DateError> {
    let (lo, hi) = (dmin.local(), dmax.local());
    let mut delta = RelativeDelta::between(hi, lo)?;
    let mut span = hi - lo;
    if lo > hi {
        delta = -delta;
        span = span.checked_neg().ok_or(DateError::Overflow)?;
    }
    let years = delta.years() as f64;
    let months = years * 12.0 + delta.months() as f64;
    let days = span.days() as f64;
    let hours = days * 24.0 + delta.hours() as f64;
    let minutes = hours * 60.0 + delta.minutes() as f64;
    let seconds = span.total_seconds().floor();
    let micros = (span.total_seconds() * 1e6).floor();
    Ok([years, months, days, hours, minutes, seconds, micros])
}

impl Locator for AutoDateLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        self.choose(vmin, vmax)?
            .locator
            .tick_values(vmin, vmax, ctx)
    }

    /// Without finite data, 1970-01-01 to 1970-01-02; a single date is
    /// widened by two years on each side.
    #[expect(
        clippy::float_cmp,
        reason = "matplotlib widens only exactly equal limits"
    )]
    fn nonsingular(&self, v0: f64, v1: f64, _: &TickContext<'_>) -> (f64, f64) {
        if !v0.is_finite() || !v1.is_finite() {
            return default_limits();
        }
        let (v0, v1) = if v1 < v0 { (v1, v0) } else { (v0, v1) };
        if v0 == v1 {
            (v0 - 2.0 * 365.0, v1 + 2.0 * 365.0)
        } else {
            (v0, v1)
        }
    }

    fn date_unit(&self, ctx: &TickContext<'_>) -> Option<f64> {
        Some(self.unit(ctx))
    }
}

impl DateLocator for AutoDateLocator {
    fn tz(&self) -> Result<&TimeZone, DateError> {
        self.tz.get()
    }

    /// The unit of the frequency chosen for the context's view.
    fn unit(&self, ctx: &TickContext<'_>) -> f64 {
        let (vmin, vmax) = ctx.view_interval();
        self.choose(vmin, vmax)
            .ok()
            .and_then(|choice| choice.frequency)
            .unwrap_or(AutoFrequency::Yearly)
            .unit_days()
    }

    fn interval(&self) -> f64 {
        1.0
    }

    fn tick_dates(
        &self,
        dmin: &ZonedDateTime,
        dmax: &ZonedDateTime,
    ) -> Result<Vec<f64>, TickerError> {
        self.get_locator(dmin, dmax)?.locator.tick_dates(dmin, dmax)
    }
}

impl ChosenLocator {
    fn inner(&self) -> &dyn DateLocator {
        match self {
            Self::Year(locator) => locator,
            Self::RRule(locator) => locator.as_ref(),
            Self::Microsecond(locator) => locator,
        }
    }
}

impl Locator for ChosenLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        self.inner().tick_values(vmin, vmax, ctx)
    }

    fn nonsingular(&self, v0: f64, v1: f64, ctx: &TickContext<'_>) -> (f64, f64) {
        self.inner().nonsingular(v0, v1, ctx)
    }

    fn date_unit(&self, ctx: &TickContext<'_>) -> Option<f64> {
        Some(self.unit(ctx))
    }
}

impl DateLocator for ChosenLocator {
    fn tz(&self) -> Result<&TimeZone, DateError> {
        self.inner().tz()
    }

    fn unit(&self, ctx: &TickContext<'_>) -> f64 {
        self.inner().unit(ctx)
    }

    fn interval(&self) -> f64 {
        self.inner().interval()
    }

    fn tick_dates(
        &self,
        dmin: &ZonedDateTime,
        dmax: &ZonedDateTime,
    ) -> Result<Vec<f64>, TickerError> {
        self.inner().tick_dates(dmin, dmax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::datestr2num;

    fn choice(a: &str, b: &str, locator: &AutoDateLocator) -> LocatorChoice {
        locator
            .choose(datestr2num(a).unwrap(), datestr2num(b).unwrap())
            .unwrap()
    }

    #[test]
    fn one_day_ticks_every_three_hours() {
        let auto = AutoDateLocator::new();
        let chosen = choice("2020-01-01", "2020-01-02", &auto);
        assert_eq!(chosen.frequency, Some(AutoFrequency::Hourly));
        let ChosenLocator::RRule(rule) = chosen.locator else {
            panic!("expected an rrule locator");
        };
        assert_eq!(rule.rule().byhour, vec![0, 3, 6, 9, 12, 15, 18, 21]);
        assert_eq!(rule.rule().interval, 1);
        assert_eq!(rule.rule().bymonth, Vec::<u32>::new());
    }

    #[test]
    fn too_few_intervals_fall_back_to_the_largest() {
        let auto = AutoDateLocator::new()
            .with_interval_multiples(false)
            .with_intervals(AutoFrequency::Daily, vec![2])
            .unwrap();
        let chosen = choice("2001-01-10", "2001-03-03", &auto);
        assert_eq!(chosen.frequency, Some(AutoFrequency::Daily));
        assert_eq!(chosen.fallback_interval, Some(2));
        let fitting = choice("2001-01-10", "2001-01-20", &AutoDateLocator::new());
        assert_eq!(fitting.fallback_interval, None);
    }

    #[test]
    fn long_and_tiny_spans() {
        let auto = AutoDateLocator::new();
        let chosen = choice("1800-01-01", "2200-01-01", &auto);
        assert_eq!(
            chosen.locator,
            ChosenLocator::Year(YearLocator::new(40, 1, 1).unwrap())
        );
        let chosen = choice("2020-01-01", "2020-01-01", &auto);
        assert_eq!(chosen.frequency, None);
        let chosen = choice("2020-01-01T00:00:00", "2020-01-01T00:00:00.000050", &auto);
        assert_eq!(chosen.frequency, Some(AutoFrequency::Microsecondly));
        let ticks = auto.tick_values(0.0, 1.0, &TickContext::default()).unwrap();
        assert_eq!(ticks.len(), 9);
    }

    #[test]
    fn the_zone_is_the_rc_param_when_made() {
        let Ok(berlin) = TimeZone::named("Europe/Berlin") else {
            return; // no system zoneinfo database
        };
        let (made_in_berlin, unknown) = crate::rc::context(|| {
            crate::rc::set("timezone", "Europe/Berlin").unwrap();
            let made = AutoDateLocator::new();
            crate::rc::set("timezone", "Nowhere/Atlantis").unwrap();
            (made, AutoDateLocator::new())
        });
        assert_eq!(made_in_berlin.tz(), Ok(&berlin));
        // Around 2020-03-29T01:00Z, when Berlin's clocks skip from 02:00 to
        // 03:00: the rule's 02:00 lands on 01:00, as dateutil puts it.
        let (a, b) = (
            datestr2num("2020-03-28T22:00").unwrap(),
            datestr2num("2020-03-29T04:00").unwrap(),
        );
        let ctx = TickContext::default();
        let ticks = made_in_berlin.tick_values(a, b, &ctx).unwrap();
        let hours: Vec<String> = ticks
            .iter()
            .map(|&t| {
                crate::dates::num2date(t, &berlin)
                    .unwrap()
                    .strftime("%H:%M %Z")
            })
            .collect();
        assert_eq!(
            hours,
            [
                "23:00 CET",
                "00:00 CET",
                "01:00 CET",
                "01:00 CET",
                "03:00 CEST",
                "04:00 CEST",
                "05:00 CEST",
                "06:00 CEST"
            ]
        );
        assert!(unknown.tick_values(a, b, &ctx).is_err(), "an unknown zone");
        assert!(
            unknown
                .with_tz(TimeZone::Utc)
                .tick_values(a, b, &ctx)
                .is_ok()
        );
    }

    #[test]
    fn nonsingular_defaults() {
        let auto = AutoDateLocator::new();
        let ctx = TickContext::default();
        assert_eq!(auto.nonsingular(5.0, 5.0, &ctx), (-725.0, 735.0));
        assert_eq!(
            auto.nonsingular(f64::NEG_INFINITY, f64::INFINITY, &ctx),
            (0.0, 1.0)
        );
        assert_eq!(
            AutoFrequency::from_name("MICROSECONDLY"),
            Some(AutoFrequency::Microsecondly)
        );
    }
}
