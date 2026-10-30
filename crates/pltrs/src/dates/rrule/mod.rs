//! Recurrence rules (RFC 5545 `RRULE`s with dateutil's semantics).
//!
//! An [`RRuleSpec`] holds the parameters as a user writes them (what
//! matplotlib's `rrulewrapper` stores); [`RRule::new`] validates and
//! normalises them exactly like `dateutil.rrule.rrule.__init__`, and
//! [`RRule::iter`] yields the occurrences in order. Invalid dates such as
//! February 30 are skipped, never coerced.
//!
//! Supported: every frequency from yearly to secondly, `interval`,
//! `dtstart`, `until`, `count`, `wkst`, `bysetpos`, `bymonth`,
//! `bymonthday` (negative days count from the month's end), `byyearday`,
//! `byweekday` (with optional `n`-th occurrence), `byhour`, `byminute`,
//! `bysecond` and dateutil's `byeaster`. Not supported: `byweekno`.

mod iter;
mod yearinfo;

pub use iter::RRuleIter;

use super::calendar::Weekday;
use super::{DateError, DateTime, Time};

/// How often a rule recurs. The order matters: finer frequencies compare
/// greater, as dateutil's integer constants do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Frequency {
    /// Every year (`YEARLY`).
    Yearly,
    /// Every month (`MONTHLY`).
    Monthly,
    /// Every week (`WEEKLY`).
    Weekly,
    /// Every day (`DAILY`).
    Daily,
    /// Every hour (`HOURLY`).
    Hourly,
    /// Every minute (`MINUTELY`).
    Minutely,
    /// Every second (`SECONDLY`).
    Secondly,
}

pltrs_core::archive_enum!(Frequency {
    Yearly,
    Monthly,
    Weekly,
    Daily,
    Hourly,
    Minutely,
    Secondly,
});

impl Frequency {
    /// All frequencies, coarsest first.
    pub const ALL: [Self; 7] = [
        Self::Yearly,
        Self::Monthly,
        Self::Weekly,
        Self::Daily,
        Self::Hourly,
        Self::Minutely,
        Self::Secondly,
    ];

    /// dateutil's constant name, e.g. `"YEARLY"`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Yearly => "YEARLY",
            Self::Monthly => "MONTHLY",
            Self::Weekly => "WEEKLY",
            Self::Daily => "DAILY",
            Self::Hourly => "HOURLY",
            Self::Minutely => "MINUTELY",
            Self::Secondly => "SECONDLY",
        }
    }

    /// The frequency with dateutil's constant name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == name)
    }

    /// Approximate days per period, matplotlib's
    /// `RRuleLocator.get_unit_generic` (a month is 30 days, a year 365).
    pub const fn unit_days(self) -> f64 {
        match self {
            Self::Yearly => 365.0,
            Self::Monthly => 30.0,
            Self::Weekly => 7.0,
            Self::Daily => 1.0,
            Self::Hourly => 1.0 / 24.0,
            Self::Minutely => 1.0 / 1440.0,
            Self::Secondly => 1.0 / 86_400.0,
        }
    }
}

/// A weekday in a `byweekday` list, optionally restricted to its `n`-th
/// occurrence in the month or year (dateutil's `MO(+1)`, `FR(-1)`, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NthWeekday {
    /// The weekday.
    pub weekday: Weekday,
    /// Which occurrence (`1` = first, `-1` = last); `None` for every one.
    pub nth: Option<i32>,
}

pltrs_core::archive_struct!(NthWeekday { weekday, nth });

impl From<Weekday> for NthWeekday {
    fn from(weekday: Weekday) -> Self {
        Self { weekday, nth: None }
    }
}

/// The parameters of a recurrence rule, as given by the user.
///
/// Empty `by*` lists mean "not given". `dtstart` is required to iterate.
#[derive(Clone, Debug, PartialEq)]
pub struct RRuleSpec {
    /// The base frequency.
    pub freq: Frequency,
    /// Recur every `interval` periods (at least 1).
    pub interval: u32,
    /// The first candidate; its fields fill in unspecified `by*` values.
    /// Microseconds are dropped, as dateutil does.
    pub dtstart: Option<DateTime>,
    /// The last allowed occurrence (inclusive).
    pub until: Option<DateTime>,
    /// The maximum number of occurrences.
    pub count: Option<u32>,
    /// The first day of the week, for weekly rules.
    pub wkst: Weekday,
    /// Occurrence positions within each period (`-1` = last).
    pub bysetpos: Vec<i32>,
    /// Months, 1-12.
    pub bymonth: Vec<u32>,
    /// Days of the month; negative values count from the end.
    pub bymonthday: Vec<i32>,
    /// Days of the year; negative values count from the end.
    pub byyearday: Vec<i32>,
    /// Weekdays.
    pub byweekday: Vec<NthWeekday>,
    /// Hours, 0-23.
    pub byhour: Vec<u32>,
    /// Minutes, 0-59.
    pub byminute: Vec<u32>,
    /// Seconds, 0-59.
    pub bysecond: Vec<u32>,
    /// Days relative to Western Easter Sunday (`0` = Easter Sunday, `1`
    /// = Easter Monday; dateutil's `byeaster`).
    pub byeaster: Vec<i32>,
}

pltrs_core::archive_struct!(RRuleSpec {
    freq,
    interval,
    dtstart,
    until,
    count,
    wkst,
    bysetpos,
    bymonth,
    bymonthday,
    byyearday,
    byweekday,
    byhour,
    byminute,
    bysecond,
    byeaster
});

impl RRuleSpec {
    /// A rule recurring every period of `freq`, with nothing else set.
    pub const fn new(freq: Frequency) -> Self {
        Self {
            freq,
            interval: 1,
            dtstart: None,
            until: None,
            count: None,
            wkst: Weekday::Monday,
            bysetpos: Vec::new(),
            bymonth: Vec::new(),
            bymonthday: Vec::new(),
            byyearday: Vec::new(),
            byweekday: Vec::new(),
            byhour: Vec::new(),
            byminute: Vec::new(),
            bysecond: Vec::new(),
            byeaster: Vec::new(),
        }
    }
}

/// A validated, normalised recurrence rule (dateutil's `rrule`).
#[derive(Clone, Debug, PartialEq)]
pub struct RRule {
    pub(crate) freq: Frequency,
    pub(crate) interval: u32,
    pub(crate) dtstart: DateTime,
    pub(crate) until: Option<DateTime>,
    pub(crate) count: Option<u32>,
    pub(crate) wkst: Weekday,
    pub(crate) bysetpos: Vec<i32>,
    pub(crate) bymonth: Vec<u32>,
    pub(crate) byyearday: Vec<i32>,
    pub(crate) bymonthday: Vec<i32>,
    pub(crate) bynmonthday: Vec<i32>,
    pub(crate) byweekday: Vec<Weekday>,
    pub(crate) bynweekday: Vec<(Weekday, i32)>,
    pub(crate) byhour: Vec<u32>,
    pub(crate) byminute: Vec<u32>,
    pub(crate) bysecond: Vec<u32>,
    pub(crate) byeaster: Vec<i32>,
    /// The times of day for frequencies coarser than hourly.
    pub(crate) timeset: Vec<Time>,
}

impl RRule {
    /// Validates `spec` and applies dateutil's defaults: without any
    /// day-level rule, yearly rules recur on `dtstart`'s month and day,
    /// monthly ones on its day and weekly ones on its weekday; the clock
    /// fields default to `dtstart`'s for frequencies coarser than them.
    pub fn new(spec: &RRuleSpec) -> Result<Self, DateError> {
        let dtstart = spec
            .dtstart
            .ok_or_else(|| DateError::InvalidArgument("rrule needs a dtstart".to_owned()))?
            .with_microsecond(0)?;
        validate(spec)?;
        let freq = spec.freq;
        let mut bymonth = spec.bymonth.clone();
        let mut bymonthday = spec.bymonthday.clone();
        let mut byweekday_spec = spec.byweekday.clone();
        if spec.byyearday.is_empty()
            && bymonthday.is_empty()
            && byweekday_spec.is_empty()
            && spec.byeaster.is_empty()
        {
            match freq {
                Frequency::Yearly => {
                    if bymonth.is_empty() {
                        bymonth.push(dtstart.month());
                    }
                    bymonthday.push(dtstart.day() as i32);
                }
                Frequency::Monthly => bymonthday.push(dtstart.day() as i32),
                Frequency::Weekly => byweekday_spec.push(dtstart.weekday().into()),
                _ => {}
            }
        }
        let (positive, negative): (Vec<i32>, Vec<i32>) =
            sorted_unique(bymonthday).into_iter().partition(|&d| d > 0);
        let mut byweekday = Vec::new();
        let mut bynweekday = Vec::new();
        for entry in byweekday_spec {
            match entry.nth {
                Some(n) if freq <= Frequency::Monthly => bynweekday.push((entry.weekday, n)),
                _ => byweekday.push(entry.weekday),
            }
        }

        let byhour = clock_set(&spec.byhour, spec, Frequency::Hourly, dtstart.hour(), 24)?;
        let byminute = clock_set(
            &spec.byminute,
            spec,
            Frequency::Minutely,
            dtstart.minute(),
            60,
        )?;
        let bysecond = clock_set(
            &spec.bysecond,
            spec,
            Frequency::Secondly,
            dtstart.second(),
            60,
        )?;
        let timeset = if freq < Frequency::Hourly {
            times_of_day(&byhour, &byminute, &bysecond)?
        } else {
            Vec::new()
        };

        Ok(Self {
            freq,
            interval: spec.interval,
            dtstart,
            until: spec.until,
            count: spec.count,
            wkst: spec.wkst,
            bysetpos: spec.bysetpos.clone(),
            bymonth: sorted_unique(bymonth),
            byyearday: sorted_unique(spec.byyearday.clone()),
            bymonthday: positive,
            bynmonthday: negative,
            byweekday: sorted_unique(byweekday),
            bynweekday: sorted_unique(bynweekday),
            byhour,
            byminute,
            bysecond,
            byeaster: sorted_unique(spec.byeaster.clone()),
            timeset,
        })
    }

    /// The rule's frequency.
    pub const fn freq(&self) -> Frequency {
        self.freq
    }

    /// The rule's interval.
    pub const fn interval(&self) -> u32 {
        self.interval
    }

    /// The occurrences, in increasing order.
    pub fn iter(&self) -> RRuleIter<'_> {
        RRuleIter::new(self)
    }

    /// dateutil's `between(after, before, inc)`: the occurrences strictly
    /// between the bounds, or including them when `inclusive`.
    pub fn between(
        &self,
        after: DateTime,
        before: DateTime,
        inclusive: bool,
    ) -> Result<Vec<DateTime>, DateError> {
        let mut found = Vec::new();
        for occurrence in self {
            let occurrence = occurrence?;
            let past_end = if inclusive {
                occurrence > before
            } else {
                occurrence >= before
            };
            if past_end {
                break;
            }
            let started = !found.is_empty()
                || if inclusive {
                    occurrence >= after
                } else {
                    occurrence > after
                };
            if started {
                found.push(occurrence);
            }
        }
        Ok(found)
    }
}

impl<'a> IntoIterator for &'a RRule {
    type Item = Result<DateTime, DateError>;
    type IntoIter = RRuleIter<'a>;

    fn into_iter(self) -> RRuleIter<'a> {
        self.iter()
    }
}

/// Rejects parameters dateutil would choke on. Clock values out of range
/// are let through, as dateutil does: they never match a clock position,
/// and only fail as times of day (for frequencies coarser than hourly).
fn validate(spec: &RRuleSpec) -> Result<(), DateError> {
    let invalid = |why: &str| Err(DateError::InvalidArgument(why.to_owned()));
    if spec.interval == 0 {
        return invalid("rrule interval must be at least 1");
    }
    validate_range("bymonth", &spec.bymonth, 1, 12)?;
    if spec
        .bysetpos
        .iter()
        .any(|&pos| pos == 0 || !(-366..=366).contains(&pos))
    {
        return invalid("bysetpos must be between 1 and 366, or between -366 and -1");
    }
    if spec.byweekday.iter().any(|w| w.nth == Some(0)) {
        return invalid("can't create weekday with n == 0");
    }
    Ok(())
}

/// Every combination of the given clock fields, sorted.
fn times_of_day(hours: &[u32], minutes: &[u32], seconds: &[u32]) -> Result<Vec<Time>, DateError> {
    let mut times = Vec::with_capacity(hours.len() * minutes.len() * seconds.len());
    for &hour in hours {
        for &minute in minutes {
            for &second in seconds {
                times.push(Time::new(hour, minute, second, 0)?);
            }
        }
    }
    times.sort_unstable();
    Ok(times)
}

fn validate_range(name: &str, values: &[u32], min: u32, max: u32) -> Result<(), DateError> {
    match values.iter().find(|v| !(min..=max).contains(v)) {
        Some(v) => Err(DateError::InvalidArgument(format!(
            "{name} value {v} is outside {min}..={max}"
        ))),
        None => Ok(()),
    }
}

fn sorted_unique<T: Ord>(mut values: Vec<T>) -> Vec<T> {
    values.sort_unstable();
    values.dedup();
    values
}

/// Normalises `byhour`/`byminute`/`bysecond`: defaults to `dtstart`'s
/// value for coarser frequencies; at the rule's own frequency keeps only
/// values reachable from `start` in steps of `interval` (dateutil's
/// `__construct_byset`).
fn clock_set(
    values: &[u32],
    spec: &RRuleSpec,
    own: Frequency,
    start: u32,
    base: u32,
) -> Result<Vec<u32>, DateError> {
    let (freq, interval) = (spec.freq, spec.interval);
    if values.is_empty() {
        return Ok(if freq < own { vec![start] } else { Vec::new() });
    }
    if freq != own {
        return Ok(sorted_unique(values.to_vec()));
    }
    let step = gcd(interval, base);
    let reachable: Vec<u32> = values
        .iter()
        .copied()
        .filter(|&v| {
            step == 1 || (i64::from(v) - i64::from(start)).rem_euclid(i64::from(step)) == 0
        })
        .collect();
    if reachable.is_empty() {
        return Err(DateError::EmptyRule(
            "byxxx values unreachable with this interval",
        ));
    }
    Ok(sorted_unique(reachable))
}

/// Greatest common divisor.
pub(crate) const fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime {
        DateTime::new(y, mo, d, h, mi, s, 0).unwrap()
    }

    fn spec(freq: Frequency, start: DateTime) -> RRuleSpec {
        RRuleSpec {
            dtstart: Some(start),
            ..RRuleSpec::new(freq)
        }
    }

    #[test]
    fn defaults_come_from_dtstart() {
        let rule = RRule::new(&spec(Frequency::Yearly, dt(2020, 2, 29, 9, 30, 15))).unwrap();
        assert_eq!(rule.bymonth, vec![2]);
        assert_eq!(rule.bymonthday, vec![29]);
        assert_eq!(rule.timeset, vec![Time::new(9, 30, 15, 0).unwrap()]);
        let rule = RRule::new(&spec(Frequency::Weekly, dt(2020, 1, 1, 0, 0, 0))).unwrap();
        assert_eq!(rule.byweekday, vec![Weekday::Wednesday]);
        let rule = RRule::new(&spec(Frequency::Minutely, dt(2020, 1, 1, 5, 6, 7))).unwrap();
        assert_eq!(
            (
                rule.byhour.clone(),
                rule.byminute.clone(),
                rule.bysecond.clone()
            ),
            (vec![], vec![], vec![7])
        );
    }

    #[test]
    fn invalid_specs_are_rejected() {
        let start = dt(2020, 1, 1, 0, 0, 0);
        let mut bad = spec(Frequency::Daily, start);
        bad.interval = 0;
        assert!(RRule::new(&bad).is_err());
        assert!(RRule::new(&RRuleSpec::new(Frequency::Daily)).is_err());
        let mut bad = spec(Frequency::Hourly, start);
        bad.byhour = vec![3];
        bad.interval = 2;
        assert_eq!(
            RRule::new(&bad),
            Err(DateError::EmptyRule(
                "byxxx values unreachable with this interval"
            ))
        );
        let mut bad = spec(Frequency::Daily, start);
        bad.bysetpos = vec![0];
        assert!(RRule::new(&bad).is_err());
        let mut bad = spec(Frequency::Daily, start);
        bad.bymonth = vec![13];
        assert!(RRule::new(&bad).is_err());
        let mut bad = spec(Frequency::Daily, start);
        bad.byhour = vec![24];
        assert!(RRule::new(&bad).is_err(), "no such time of day");
    }

    #[test]
    fn clock_values_out_of_range_never_match() {
        // matplotlib's `HourLocator(range(0, 25, 6))`.
        let mut rule = spec(Frequency::Hourly, dt(2000, 3, 2, 0, 0, 0));
        rule.byhour = vec![0, 6, 12, 18, 24];
        let rule = RRule::new(&rule).unwrap();
        let hours: Vec<u32> = rule.iter().take(5).map(|t| t.unwrap().hour()).collect();
        assert_eq!(hours, [0, 6, 12, 18, 0]);
    }

    #[test]
    fn frequency_names_and_units() {
        assert_eq!(Frequency::from_name("WEEKLY"), Some(Frequency::Weekly));
        assert_eq!(Frequency::from_name("weekly"), None);
        assert_eq!(Frequency::Monthly.unit_days(), 30.0);
        assert!(Frequency::Yearly < Frequency::Secondly);
        assert_eq!(gcd(90, 1440), 90);
    }
}
