//! Locators driven by recurrence rules: matplotlib's `RRuleLocator`, its
//! fixed-frequency subclasses and `YearLocator`.

use super::{DateLocator, to_nums};
use crate::dates::calendar::Weekday;
use crate::dates::rrule::{Frequency, NthWeekday, RRule, RRuleSpec};
use crate::dates::tz::{TickZone, TimeZone, ZonedDateTime};
use crate::dates::{DateError, DateTime, RelativeDelta};
use crate::ticker::{Locator, TickContext, TickerError};

/// Ticks at the occurrences of a recurrence rule (matplotlib's
/// `RRuleLocator`).
///
/// The rule is evaluated from one view-width before the view to one
/// view-width after it, so `interval > 1` rules keep a stable phase, and
/// the occurrences inside the view become ticks. Without any, the view
/// limits themselves are returned.
///
/// The constructors named after matplotlib's subclasses (`MonthLocator`,
/// `WeekdayLocator`, ...) build the corresponding rules.
#[derive(Clone, Debug, PartialEq)]
pub struct RRuleLocator {
    rule: RRuleSpec,
    tz: TickZone,
}

pltrs_core::archive_struct!(RRuleLocator { rule, tz });

/// The `byhour=0, byminute=0, bysecond=0` of matplotlib's `hms0d`.
fn at_midnight(mut rule: RRuleSpec) -> RRuleSpec {
    rule.byhour = vec![0];
    rule.byminute = vec![0];
    rule.bysecond = vec![0];
    rule
}

/// `values`, or `default` when empty.
fn or_all<T: Clone>(values: &[T], default: impl Iterator<Item = T>) -> Vec<T> {
    if values.is_empty() {
        default.collect()
    } else {
        values.to_vec()
    }
}

impl RRuleLocator {
    /// Ticks at the occurrences of `rule` (its `dtstart`/`until` are
    /// replaced when locating), in the zone of the `timezone` rcParam.
    pub fn new(rule: RRuleSpec) -> Self {
        Self::in_zone(rule, TickZone::from_rc())
    }

    /// Ticks at the occurrences of `rule` in `tz`.
    pub(super) const fn in_zone(rule: RRuleSpec, tz: TickZone) -> Self {
        Self { rule, tz }
    }

    /// Places ticks in `tz` instead of the zone of the `timezone` rcParam.
    #[must_use]
    pub fn with_tz(mut self, tz: TimeZone) -> Self {
        self.tz = tz.into();
        self
    }

    /// The recurrence rule.
    pub fn rule(&self) -> &RRuleSpec {
        &self.rule
    }

    /// matplotlib's `MonthLocator`: ticks on day `bymonthday` of the months
    /// in `bymonth` (every month if empty), every `interval`-th occurrence.
    pub fn months(bymonth: &[u32], bymonthday: &[i32], interval: u32) -> Self {
        Self::new(at_midnight(RRuleSpec {
            interval,
            bymonth: or_all(bymonth, 1..=12),
            bymonthday: bymonthday.to_vec(),
            ..RRuleSpec::new(Frequency::Monthly)
        }))
    }

    /// matplotlib's `WeekdayLocator`: ticks on the given weekdays.
    pub fn weekdays(byweekday: &[Weekday], interval: u32) -> Self {
        Self::new(at_midnight(RRuleSpec {
            interval,
            byweekday: byweekday.iter().copied().map(NthWeekday::from).collect(),
            ..RRuleSpec::new(Frequency::Daily)
        }))
    }

    /// matplotlib's `DayLocator`: ticks on the given days of the month
    /// (every day if empty).
    pub fn days(bymonthday: &[i32], interval: u32) -> Result<Self, DateError> {
        if interval == 0 {
            return Err(DateError::InvalidArgument(
                "interval must be an integer greater than 0".to_owned(),
            ));
        }
        Ok(Self::new(at_midnight(RRuleSpec {
            interval,
            bymonthday: or_all(bymonthday, 1..=31),
            ..RRuleSpec::new(Frequency::Daily)
        })))
    }

    /// matplotlib's `HourLocator`: ticks on the given hours (every hour if
    /// empty).
    pub fn hours(byhour: &[u32], interval: u32) -> Self {
        Self::new(RRuleSpec {
            interval,
            byhour: or_all(byhour, 0..24),
            byminute: vec![0],
            bysecond: vec![0],
            ..RRuleSpec::new(Frequency::Hourly)
        })
    }

    /// matplotlib's `MinuteLocator`: ticks on the given minutes (every
    /// minute if empty).
    pub fn minutes(byminute: &[u32], interval: u32) -> Self {
        Self::new(RRuleSpec {
            interval,
            byminute: or_all(byminute, 0..60),
            bysecond: vec![0],
            ..RRuleSpec::new(Frequency::Minutely)
        })
    }

    /// matplotlib's `SecondLocator`: ticks on the given seconds (every
    /// second if empty).
    pub fn seconds(bysecond: &[u32], interval: u32) -> Self {
        Self::new(RRuleSpec {
            interval,
            bysecond: or_all(bysecond, 0..60),
            ..RRuleSpec::new(Frequency::Secondly)
        })
    }
}

/// The occurrences of `rule` between `after` and `before` (inclusive) when
/// it is evaluated from `start` to `stop`, as date numbers; the view
/// limits when there are none.
fn rule_ticks(
    rule: &RRuleSpec,
    tz: &TimeZone,
    (start, stop): (DateTime, DateTime),
    (after, before): (DateTime, DateTime),
    view: (&ZonedDateTime, &ZonedDateTime),
) -> Result<Vec<f64>, TickerError> {
    let rule = RRule::new(&RRuleSpec {
        dtstart: Some(start),
        until: Some(stop),
        ..rule.clone()
    })?;
    let dates = rule.between(after, before, true)?;
    if dates.is_empty() {
        return to_nums([view.0, view.1]);
    }
    let zoned: Vec<ZonedDateTime> = dates
        .into_iter()
        .map(|d| ZonedDateTime::from_local(d, tz.clone()))
        .collect();
    to_nums(&zoned)
}

locate_dates_in_view!(RRuleLocator);

impl DateLocator for RRuleLocator {
    fn tz(&self) -> Result<&TimeZone, DateError> {
        self.tz.get()
    }

    fn unit(&self, _: &TickContext<'_>) -> f64 {
        self.rule.freq.unit_days()
    }

    fn interval(&self) -> f64 {
        f64::from(self.rule.interval)
    }

    fn tick_dates(
        &self,
        dmin: &ZonedDateTime,
        dmax: &ZonedDateTime,
    ) -> Result<Vec<f64>, TickerError> {
        let (lo, hi) = (dmin.local(), dmax.local());
        let delta = RelativeDelta::between(hi, lo)?;
        // matplotlib caps the expanded range at the representable dates.
        let start = delta
            .subtract_from(lo)
            .unwrap_or(DateTime::midnight(crate::dates::Date::MIN));
        let stop = delta.add_to(hi).unwrap_or_else(|_| last_second());
        rule_ticks(
            &self.rule,
            self.tz.get()?,
            (start, stop),
            (lo, hi),
            (dmin, dmax),
        )
    }
}

/// 9999-12-31T23:59:59.
fn last_second() -> DateTime {
    DateTime::new(9999, 12, 31, 23, 59, 59, 0).expect("9999-12-31T23:59:59 is valid")
}

/// Ticks on a given day of every `base`-th year (matplotlib's
/// `YearLocator`), on years that are multiples of `base`.
#[derive(Clone, Debug, PartialEq)]
pub struct YearLocator {
    base: u32,
    month: u32,
    day: u32,
    tz: TickZone,
}

pltrs_core::archive_struct!(YearLocator {
    base,
    month,
    day,
    tz
});

impl YearLocator {
    /// Ticks on `month`/`day` of every `base`-th year, in the zone of the
    /// `timezone` rcParam.
    pub fn new(base: u32, month: u32, day: u32) -> Result<Self, DateError> {
        Self::in_zone(base, month, day, TickZone::from_rc())
    }

    /// Ticks on `month`/`day` of every `base`-th year in `tz`.
    pub(super) fn in_zone(
        base: u32,
        month: u32,
        day: u32,
        tz: TickZone,
    ) -> Result<Self, DateError> {
        if base == 0 {
            return Err(DateError::InvalidArgument(
                "'step' must be positive".to_owned(),
            ));
        }
        Ok(Self {
            base,
            month,
            day,
            tz,
        })
    }

    /// Places ticks in `tz` instead of the zone of the `timezone` rcParam.
    #[must_use]
    pub fn with_tz(mut self, tz: TimeZone) -> Self {
        self.tz = tz.into();
        self
    }

    /// Years between ticks.
    pub const fn base(&self) -> u32 {
        self.base
    }

    /// The month of each tick.
    pub const fn month(&self) -> u32 {
        self.month
    }

    /// The day of the month of each tick.
    pub const fn day(&self) -> u32 {
        self.day
    }

    fn rule(&self) -> RRuleSpec {
        at_midnight(RRuleSpec {
            interval: self.base,
            bymonth: vec![self.month],
            bymonthday: vec![self.day as i32],
            ..RRuleSpec::new(Frequency::Yearly)
        })
    }
}

locate_dates_in_view!(YearLocator);

impl DateLocator for YearLocator {
    fn tz(&self) -> Result<&TimeZone, DateError> {
        self.tz.get()
    }

    fn unit(&self, _: &TickContext<'_>) -> f64 {
        Frequency::Yearly.unit_days()
    }

    fn interval(&self) -> f64 {
        f64::from(self.base)
    }

    /// Evaluates the rule from the multiple of `base` at or below `dmin`'s
    /// year to the one at or above `dmax`'s (clamped to 1..=9999).
    fn tick_dates(
        &self,
        dmin: &ZonedDateTime,
        dmax: &ZonedDateTime,
    ) -> Result<Vec<f64>, TickerError> {
        let base = i64::from(self.base);
        let (lo, hi) = (dmin.local(), dmax.local());
        let first_year = (i64::from(lo.year()).div_euclid(base) * base).max(1);
        let last_year = (-(-i64::from(hi.year())).div_euclid(base) * base).min(9999);
        let anchored =
            |year: i64| DateTime::new(year as i32, self.month, self.day, 0, 0, 0, lo.microsecond());
        let start = anchored(first_year)?;
        let stop = anchored(last_year)?;
        rule_ticks(
            &self.rule(),
            self.tz.get()?,
            (start, stop),
            (start, stop),
            (dmin, dmax),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::convert::num2date;

    fn view(a: &str, b: &str) -> (f64, f64) {
        let num = |s: &str| crate::dates::datestr2num(s).unwrap();
        (num(a), num(b))
    }

    fn iso_ticks(locator: &dyn Locator, (a, b): (f64, f64)) -> Vec<String> {
        locator
            .tick_values(a, b, &TickContext::default())
            .unwrap()
            .into_iter()
            .map(|t| num2date(t, &TimeZone::Utc).unwrap().local().to_string())
            .collect()
    }

    #[test]
    fn month_and_weekday_rules() {
        let months = RRuleLocator::months(&[1, 4, 7, 10], &[1], 1);
        assert_eq!(
            iso_ticks(&months, view("2020-01-01", "2021-01-01")),
            [
                "2020-01-01T00:00:00",
                "2020-04-01T00:00:00",
                "2020-07-01T00:00:00",
                "2020-10-01T00:00:00",
                "2021-01-01T00:00:00"
            ]
        );
        let mondays = RRuleLocator::weekdays(&[Weekday::Monday], 1);
        assert_eq!(
            iso_ticks(&mondays, view("2020-02-25", "2020-03-06")),
            ["2020-03-02T00:00:00"]
        );
        // No occurrence in view: the view limits come back.
        let june = RRuleLocator::months(&[6], &[1], 1);
        assert_eq!(
            iso_ticks(&june, view("2020-01-05", "2020-01-25")),
            ["2020-01-05T00:00:00", "2020-01-25T00:00:00"]
        );
        assert!(RRuleLocator::days(&[], 0).is_err());
    }

    #[test]
    fn year_locator_aligns_to_multiples() {
        let years = YearLocator::new(5, 1, 1).unwrap();
        let ticks = iso_ticks(&years, view("2000-03-15", "2031-01-01"));
        assert_eq!(
            ticks.first().map(String::as_str),
            Some("2000-01-01T00:00:00")
        );
        assert_eq!(
            ticks.last().map(String::as_str),
            Some("2035-01-01T00:00:00")
        );
        let leap = YearLocator::new(1, 2, 29).unwrap();
        let (a, b) = view("1999-12-31", "2001-01-02");
        assert!(leap.tick_values(a, b, &TickContext::default()).is_err());
        assert!(YearLocator::new(0, 1, 1).is_err());
        assert_eq!(
            years.nonsingular(18_262.0, 18_262.0, &TickContext::default()),
            (14_612.0, 21_912.0)
        );
    }

    #[test]
    fn zoned_ticks_follow_the_wall_clock() {
        let Ok(tz) = TimeZone::named("America/New_York") else {
            return; // no system zoneinfo database
        };
        // Clocks go back at 06:00 UTC: local 01:00-02:00 happens twice and
        // the rule's naive wall-clock hours resolve to the first pass.
        let (a, b) = view("2020-11-01T04:00", "2020-11-01T08:00");
        let hours = RRuleLocator::hours(&[], 1).with_tz(tz);
        let ticks = hours.tick_values(a, b, &TickContext::default()).unwrap();
        assert_eq!(
            ticks,
            [
                18_567.166_666_666_668,
                18_567.208_333_333_332,
                18_567.291_666_666_668,
                18_567.333_333_333_332
            ]
        );
    }

    #[test]
    fn out_of_range_views_give_no_ticks() {
        let hours = RRuleLocator::hours(&[], 1);
        assert_eq!(
            hours.tick_values(-1e7, 0.0, &TickContext::default()),
            Ok(vec![])
        );
    }
}
