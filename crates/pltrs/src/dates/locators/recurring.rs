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
/// The rule runs on the wall clock of its own zone when it has one
/// ([`with_rule_tz`](Self::with_rule_tz), matplotlib's `rrulewrapper(...,
/// tzinfo=tz)`), else on that of the dates located: the locator's zone
/// for a view, the dates' own for [`DateLocator::tick_dates`] (as
/// matplotlib's `tick_values` takes them).
///
/// The constructors named after matplotlib's subclasses (`MonthLocator`,
/// `WeekdayLocator`, ...) build the corresponding rules.
#[derive(Clone, Debug, PartialEq)]
pub struct RRuleLocator {
    rule: RRuleSpec,
    tz: TickZone,
    rule_tz: Option<TimeZone>,
}

pltrs_core::archive_struct!(RRuleLocator { rule, tz, rule_tz });

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
        Self {
            rule,
            tz,
            rule_tz: None,
        }
    }

    /// Places ticks in `tz` instead of the zone of the `timezone` rcParam.
    #[must_use]
    pub fn with_tz(mut self, tz: TimeZone) -> Self {
        self.tz = tz.into();
        self
    }

    /// Runs the rule on the wall clock of `tz` (matplotlib's
    /// `rrulewrapper(..., tzinfo=tz)`), whatever zone the dates located
    /// are in.
    #[must_use]
    pub fn with_rule_tz(mut self, tz: TimeZone) -> Self {
        self.rule_tz = Some(tz);
        self
    }

    /// The recurrence rule.
    pub fn rule(&self) -> &RRuleSpec {
        &self.rule
    }

    /// The zone of the rule, if it has one of its own.
    pub const fn rule_tz(&self) -> Option<&TimeZone> {
        self.rule_tz.as_ref()
    }

    /// The occurrences of the rule between `dmin` and `dmax`, inclusive,
    /// which become the ticks (matplotlib's `_create_rrule` and
    /// `rule.between(dmin, dmax, True)`): the rule is started one span
    /// (`relativedelta(dmax, dmin)`) before `dmin` and stopped one after
    /// `dmax`, at the first and last representable seconds in UTC when the
    /// span reaches beyond them, and runs on the wall clock of its own
    /// zone, else of `dmin`'s.
    pub fn occurrences(
        &self,
        dmin: &ZonedDateTime,
        dmax: &ZonedDateTime,
    ) -> Result<Vec<ZonedDateTime>, TickerError> {
        let delta = RelativeDelta::between_zoned(dmax, dmin)?;
        let start = delta.subtract_from(dmin.local()).map_or_else(
            |_| ZonedDateTime::utc(DateTime::midnight(crate::dates::Date::MIN)),
            |local| dmin.with_local(local),
        );
        let stop = delta.add_to(dmax.local()).map_or_else(
            |_| ZonedDateTime::utc(last_second()),
            |local| dmax.with_local(local),
        );
        occurrences(
            &self.rule,
            self.rule_tz.as_ref(),
            (&start, &stop),
            (dmin, dmax),
        )
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

/// The occurrences of `rule`, started at `start` and stopped at `stop`,
/// from `after` to `before` (inclusive), as matplotlib's `rrulewrapper`
/// finds them: on the wall clock of `rule_tz`, else of `start`'s zone, in
/// which the other date-times are read.
fn occurrences(
    rule: &RRuleSpec,
    rule_tz: Option<&TimeZone>,
    (start, stop): (&ZonedDateTime, &ZonedDateTime),
    (after, before): (&ZonedDateTime, &ZonedDateTime),
) -> Result<Vec<ZonedDateTime>, TickerError> {
    let zone = rule_tz.unwrap_or_else(|| start.tz());
    let rule = RRule::new(&RRuleSpec {
        dtstart: Some(start.local_in(zone)?),
        until: Some(stop.local_in(zone)?),
        ..rule.clone()
    })?;
    let found = rule.between(after.local_in(zone)?, before.local_in(zone)?, true)?;
    Ok(found
        .into_iter()
        .map(|d| ZonedDateTime::from_local(d, zone.clone()))
        .collect())
}

/// The ticks at `occurrences`, or the view limits when there are none.
fn ticks_or_view(
    occurrences: &[ZonedDateTime],
    view: (&ZonedDateTime, &ZonedDateTime),
) -> Result<Vec<f64>, TickerError> {
    if occurrences.is_empty() {
        to_nums([view.0, view.1])
    } else {
        to_nums(occurrences)
    }
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
        ticks_or_view(&self.occurrences(dmin, dmax)?, (dmin, dmax))
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

    /// The days of the years between `dmin` and `dmax` which become the
    /// ticks: the rule runs from the multiple of `base` at or below
    /// `dmin`'s year to the one at or above `dmax`'s (clamped to 1..=9999),
    /// on the wall clock of `dmin`'s zone (matplotlib's `_create_rrule`,
    /// which replaces the date and time of day of `dmin`, keeping its
    /// microseconds).
    pub fn occurrences(
        &self,
        dmin: &ZonedDateTime,
        dmax: &ZonedDateTime,
    ) -> Result<Vec<ZonedDateTime>, TickerError> {
        let base = i64::from(self.base);
        let lo = dmin.local();
        let first_year = (i64::from(lo.year()).div_euclid(base) * base).max(1);
        let last_year = (-(-i64::from(dmax.local().year())).div_euclid(base) * base).min(9999);
        let anchored = |year: i64| {
            DateTime::new(year as i32, self.month, self.day, 0, 0, 0, lo.microsecond())
                .map(|local| dmin.replace_local(local))
        };
        let (start, stop) = (anchored(first_year)?, anchored(last_year)?);
        occurrences(&self.rule(), None, (&start, &stop), (&start, &stop))
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

    fn tick_dates(
        &self,
        dmin: &ZonedDateTime,
        dmax: &ZonedDateTime,
    ) -> Result<Vec<f64>, TickerError> {
        ticks_or_view(&self.occurrences(dmin, dmax)?, (dmin, dmax))
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

    fn utc(text: &str) -> ZonedDateTime {
        ZonedDateTime::parse_iso(text).unwrap()
    }

    #[test]
    fn dates_located_bring_their_zone() {
        let Ok(berlin) = TimeZone::named("Europe/Berlin") else {
            return; // no system zoneinfo database
        };
        // Recorded from matplotlib: `HourLocator(tz="Europe/Berlin")
        // .tick_values(a, b)` runs the rule on the wall clock of `a`, so
        // dates in UTC tick on UTC hours, and those in Berlin across its
        // skipped hour twice at 01:00 CET (where dateutil puts 02:00).
        let hours = RRuleLocator::hours(&[], 1).with_tz(berlin.clone());
        let (a, b) = (utc("2020-03-29T00:00"), utc("2020-03-29T03:00"));
        let in_utc = [
            18_350.0,
            18_350.041_666_666_668,
            18_350.083_333_333_332,
            18_350.125,
        ];
        assert_eq!(hours.tick_dates(&a, &b).unwrap(), in_utc);
        let (a, b) = (
            a.to_zone(berlin.clone()).unwrap(),
            b.to_zone(berlin).unwrap(),
        );
        let ticks = hours.tick_dates(&a, &b).unwrap();
        assert_eq!(ticks[..2], [18_350.0, 18_350.0]);
        assert_eq!(ticks[2..], in_utc[1..]);
        let years = YearLocator::new(1, 1, 1).unwrap();
        let (a, b) = (utc("2019-06-01"), utc("2022-06-01"));
        let ticks = years.tick_dates(&a, &b).unwrap();
        assert_eq!(ticks, [17_897.0, 18_262.0, 18_628.0, 18_993.0]);
    }

    #[test]
    fn a_rule_with_a_zone_of_its_own_runs_in_it() {
        let Ok(berlin) = TimeZone::named("Europe/Berlin") else {
            return; // no system zoneinfo database
        };
        // `RRuleLocator(rrulewrapper(DAILY, tzinfo=UTC, byhour=[0, 12],
        // ...), tz="Europe/Berlin")` on an axis, from matplotlib.
        let rule = RRuleSpec {
            byhour: vec![0, 12],
            byminute: vec![0],
            bysecond: vec![0],
            ..RRuleSpec::new(Frequency::Daily)
        };
        let locator = RRuleLocator::new(rule)
            .with_tz(berlin)
            .with_rule_tz(TimeZone::Utc);
        assert_eq!(locator.rule_tz(), Some(&TimeZone::Utc));
        let (a, b) = view("2020-10-23T01:00", "2020-10-25T01:00");
        let ticks = locator.tick_values(a, b, &TickContext::default()).unwrap();
        assert_eq!(ticks, [18_558.5, 18_559.0, 18_559.5, 18_560.0]);
    }

    #[test]
    fn rules_stopped_past_9999_in_utc_overflow_in_other_zones() {
        let east = TimeZone::Fixed(crate::dates::UtcOffset::from_seconds(3600).unwrap());
        let days = RRuleLocator::days(&[], 1).unwrap().with_tz(east);
        let (a, b) = view("9999-12-20", "9999-12-31");
        assert_eq!(
            days.tick_values(a, b, &TickContext::default()),
            Err(TickerError::DateOverflow)
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
