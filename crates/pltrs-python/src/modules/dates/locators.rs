//! The date locators of `_native.dates`: recurrence rules (matplotlib's
//! `rrulewrapper`), the date kinds of the ticker's locator specs, and the
//! choice `AutoDateLocator` makes.
//!
//! A rule crosses as a dict of dateutil's `rrule` keywords (the `by*` ones
//! as lists, weekdays as `(index, n)` pairs, naive date-times as calendar
//! parts), a time zone as its name (with its `TZif` data where Python
//! reads it from elsewhere than the system) or as its offset from UTC in
//! seconds.

use std::collections::BTreeMap;
use std::sync::Arc;

use pltrs::dates::{
    AutoDateLocator, AutoFrequency, ChosenLocator, DateError, DateTime, Frequency, LocatorChoice,
    NthWeekday, RRule, RRuleLocator, RRuleSpec, TimeZone, UtcOffset, Weekday, YearLocator,
    ZoneInfo, ZonedDateTime, date2num,
};
use pltrs::ticker::Locator;

use crate::binding::dict_value;
use crate::binding::prelude::*;

/// A naive date-time as calendar parts `((year, month, day), (hour,
/// minute, second, microsecond))`.
pub(super) type Parts = ((i32, u32, u32), (u32, u32, u32, u32));

/// The date-time of calendar parts.
fn datetime(((year, month, day), (hour, minute, second, micro)): Parts) -> PyResult<DateTime> {
    Ok(DateTime::new(
        year, month, day, hour, minute, second, micro,
    )?)
}

/// The calendar parts of a date-time.
pub(super) fn parts(dt: DateTime) -> Parts {
    (
        (dt.year(), dt.month(), dt.day()),
        (dt.hour(), dt.minute(), dt.second(), dt.microsecond()),
    )
}

/// A time zone: its name (read as `dateutil` reads it), the key of a
/// `zoneinfo.ZoneInfo` with the `TZif` data it reads (from the system or
/// the `tzdata` package; read as `zoneinfo` reads it), or its offset from
/// UTC in seconds.
pub(crate) struct Zone(pub(crate) TimeZone);

impl FromPy<'_> for Zone {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(name) = object.extract::<String>() {
            return Ok(Self(TimeZone::named(&name)?));
        }
        if let Ok((key, Bytes(data))) = object.extract::<(String, Bytes)>() {
            return Ok(Self(TimeZone::Info(Arc::new(ZoneInfo::parse(
                &data, &key,
            )?))));
        }
        let seconds: i32 = object.extract()?;
        Ok(Self(TimeZone::Fixed(UtcOffset::from_seconds(seconds)?)))
    }
}

/// A Python `datetime` as `(parts, zone, fold)`: its calendar parts, its
/// zone as [`Zone`] takes it (`None` when it is naive) and its `fold`.
struct PyDateTime {
    local: DateTime,
    zone: Option<TimeZone>,
    fold: bool,
}

impl FromPy<'_> for PyDateTime {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (parts, zone, fold): (Parts, Option<Zone>, bool) = object.extract()?;
        Ok(Self {
            local: datetime(parts)?,
            zone: zone.map(|Zone(tz)| tz),
            fold,
        })
    }
}

impl PyDateTime {
    /// The date-time, a naive one read on the wall clock of `naive` (as a
    /// rule with a zone of its own reads naive dates, and `date2num` reads
    /// them in UTC).
    fn zoned(self, naive: &TimeZone) -> ZonedDateTime {
        ZonedDateTime::from_local(self.local, self.zone.unwrap_or_else(|| naive.clone()))
            .with_fold(self.fold)
    }
}

/// The frequency with dateutil's number (`YEARLY` = 0 ... `SECONDLY` = 6).
fn frequency(number: usize) -> PyResult<Frequency> {
    Frequency::ALL
        .get(number)
        .copied()
        .ok_or_else(|| PyErr::value_error(format!("invalid frequency: {number}")))
}

/// A recurrence rule given as dateutil's keywords.
struct Rule(RRuleSpec);

impl FromPy<'_> for Rule {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let list = |key: &str| -> PyResult<Option<Bound<'_>>> {
            let value: Bound<'_> = dict_value(object, key)?;
            Ok((!value.is_none()).then_some(value))
        };
        let mut spec = RRuleSpec::new(frequency(dict_value(object, "freq")?)?);
        spec.interval = dict_value(object, "interval")?;
        spec.dtstart = dict_value::<Option<Parts>>(object, "dtstart")?
            .map(datetime)
            .transpose()?;
        spec.until = dict_value::<Option<Parts>>(object, "until")?
            .map(datetime)
            .transpose()?;
        spec.count = dict_value(object, "count")?;
        spec.wkst = Weekday::from_index(dict_value(object, "wkst")?);
        if let Some(weekdays) = list("byweekday")? {
            spec.byweekday = weekdays
                .extract::<Vec<(i64, Option<i32>)>>()?
                .into_iter()
                .map(|(index, nth)| NthWeekday {
                    weekday: Weekday::from_index(index),
                    nth,
                })
                .collect();
        }
        spec.bysetpos = dict_value::<Option<Vec<i32>>>(object, "bysetpos")?.unwrap_or_default();
        spec.bymonth = dict_value::<Option<Vec<u32>>>(object, "bymonth")?.unwrap_or_default();
        spec.bymonthday = dict_value::<Option<Vec<i32>>>(object, "bymonthday")?.unwrap_or_default();
        spec.byyearday = dict_value::<Option<Vec<i32>>>(object, "byyearday")?.unwrap_or_default();
        spec.byweekno = dict_value::<Option<Vec<i32>>>(object, "byweekno")?.unwrap_or_default();
        spec.byhour = dict_value::<Option<Vec<u32>>>(object, "byhour")?.unwrap_or_default();
        spec.byminute = dict_value::<Option<Vec<u32>>>(object, "byminute")?.unwrap_or_default();
        spec.bysecond = dict_value::<Option<Vec<u32>>>(object, "bysecond")?.unwrap_or_default();
        spec.byeaster = dict_value::<Option<Vec<i32>>>(object, "byeaster")?.unwrap_or_default();
        Ok(Self(spec))
    }
}

/// A list keyword of a rule, `None` when not given.
fn listed<T: Clone + IntoPy>(values: &[T]) -> PyResult<Option<Vec<T>>> {
    Ok((!values.is_empty()).then(|| values.to_vec()))
}

impl IntoPy for Rule {
    /// The keywords of the rule, as `Rule` takes them.
    fn into_py<'py>(self, py: Python<'py>) -> PyResult<Bound<'py>> {
        let spec = self.0;
        let mut keywords: BTreeMap<String, Bound<'py>> = BTreeMap::new();
        let mut put = |key: &str, value: Bound<'py>| {
            keywords.insert(key.to_owned(), value);
        };
        let freq = Frequency::ALL
            .iter()
            .position(|&f| f == spec.freq)
            .expect("every frequency is listed");
        put("freq", freq.into_py(py)?);
        put("interval", spec.interval.into_py(py)?);
        put("dtstart", spec.dtstart.map(parts).into_py(py)?);
        put("until", spec.until.map(parts).into_py(py)?);
        put("count", spec.count.into_py(py)?);
        put("wkst", spec.wkst.index().into_py(py)?);
        let weekdays: Vec<(u32, Option<i32>)> = spec
            .byweekday
            .iter()
            .map(|day| (day.weekday.index(), day.nth))
            .collect();
        put("byweekday", listed(&weekdays)?.into_py(py)?);
        put("bysetpos", listed(&spec.bysetpos)?.into_py(py)?);
        put("bymonth", listed(&spec.bymonth)?.into_py(py)?);
        put("bymonthday", listed(&spec.bymonthday)?.into_py(py)?);
        put("byyearday", listed(&spec.byyearday)?.into_py(py)?);
        put("byweekno", listed(&spec.byweekno)?.into_py(py)?);
        put("byhour", listed(&spec.byhour)?.into_py(py)?);
        put("byminute", listed(&spec.byminute)?.into_py(py)?);
        put("bysecond", listed(&spec.bysecond)?.into_py(py)?);
        put("byeaster", listed(&spec.byeaster)?.into_py(py)?);
        keywords.into_py(py)
    }
}

/// The occurrences of `rule` between `after` and `before`, strictly or
/// inclusively (`rrule.between`).
fn rrule_between(rule: Rule, after: Parts, before: Parts, inc: bool) -> PyResult<Vec<Parts>> {
    let found = RRule::new(&rule.0)?.between(datetime(after)?, datetime(before)?, inc)?;
    Ok(found.into_iter().map(parts).collect())
}

/// The first occurrence of `rule` after `dt` (or at it, when `inc`), if
/// any (`rrule.after`).
fn rrule_after(rule: Rule, dt: Parts, inc: bool) -> PyResult<Option<Parts>> {
    let dt = datetime(dt)?;
    for occurrence in &RRule::new(&rule.0)? {
        let occurrence = occurrence?;
        if occurrence > dt || (inc && occurrence == dt) {
            return Ok(Some(parts(occurrence)));
        }
    }
    Ok(None)
}

/// The last occurrence of `rule` before `dt` (or at it, when `inc`), if
/// any (`rrule.before`).
fn rrule_before(rule: Rule, dt: Parts, inc: bool) -> PyResult<Option<Parts>> {
    let dt = datetime(dt)?;
    let mut last = None;
    for occurrence in &RRule::new(&rule.0)? {
        let occurrence = occurrence?;
        if occurrence > dt || (!inc && occurrence == dt) {
            break;
        }
        last = Some(parts(occurrence));
    }
    Ok(last)
}

/// The `RRuleLocator` of a `"date_rrule"` spec: its `rule`, `tz` and
/// `rule_tz` (the zone of matplotlib's `rrulewrapper`, `None` for none).
fn rrule_locator(spec: &Bound<'_>) -> PyResult<RRuleLocator> {
    let Rule(rule) = dict_value(spec, "rule")?;
    let Zone(tz) = dict_value(spec, "tz")?;
    let locator = RRuleLocator::new(rule).with_tz(tz);
    Ok(match dict_value::<Option<Zone>>(spec, "rule_tz")? {
        Some(Zone(rule_tz)) => locator.with_rule_tz(rule_tz),
        None => locator,
    })
}

/// The `YearLocator` of a `"date_year"` spec: its `base`, `month`, `day`
/// and `tz`.
fn year_locator(spec: &Bound<'_>) -> PyResult<YearLocator> {
    let Zone(tz) = dict_value(spec, "tz")?;
    Ok(YearLocator::new(
        dict_value(spec, "base")?,
        dict_value(spec, "month")?,
        dict_value(spec, "day")?,
    )?
    .with_tz(tz))
}

/// The locator of a ticker spec of a date kind: `"date_rrule"`
/// ([`rrule_locator`]) or `"date_year"` ([`year_locator`]); `None` for
/// other kinds.
pub(crate) fn date_locator(kind: &str, spec: &Bound<'_>) -> PyResult<Option<Box<dyn Locator>>> {
    Ok(Some(match kind {
        "date_rrule" => Box::new(rrule_locator(spec)?),
        "date_year" => Box::new(year_locator(spec)?),
        _ => return Ok(None),
    }))
}

/// The ticks matplotlib's `tick_values(vmin, vmax)` of the locator of a
/// `"date_rrule"` or `"date_year"` spec places, on the wall clock of the
/// dates' own zone (naive dates in the rule's zone, else UTC), as date
/// numbers; `None` when the rule has no occurrence there, for which
/// matplotlib gives `date2num([vmin, vmax])`.
fn rule_tick_values(spec: Owned, vmin: PyDateTime, vmax: PyDateTime) -> PyResult<Option<NdArray>> {
    let found = Python::with_gil(|py| {
        let spec = spec.bind(py);
        let kind: String = dict_value(&spec, "kind")?;
        PyResult::Ok(if kind == "date_rrule" {
            let locator = rrule_locator(&spec)?;
            let naive = locator.rule_tz().cloned().unwrap_or_default();
            locator.occurrences(&vmin.zoned(&naive), &vmax.zoned(&naive))?
        } else {
            let naive = TimeZone::Utc;
            year_locator(&spec)?.occurrences(&vmin.zoned(&naive), &vmax.zoned(&naive))?
        })
    })?;
    if found.is_empty() {
        return Ok(None);
    }
    let numbers = found
        .iter()
        .map(date2num)
        .collect::<Result<Vec<f64>, DateError>>()?;
    Ok(Some(NdArray::from_vec(numbers)))
}

/// The `AutoDateLocator` of a spec: its `tz`, `minticks`, `maxticks` and
/// `intervald` (by matplotlib's frequency numbers) and
/// `interval_multiples`.
fn auto_locator(spec: &Bound<'_>) -> PyResult<AutoDateLocator> {
    let Zone(tz) = dict_value(spec, "tz")?;
    let mut locator = AutoDateLocator::new()
        .with_tz(tz)
        .with_minticks(dict_value(spec, "minticks")?)
        .with_interval_multiples(dict_value(spec, "interval_multiples")?);
    let maxticks: Vec<(usize, u32)> = dict_value(spec, "maxticks")?;
    let intervald: Vec<(usize, Vec<u32>)> = dict_value(spec, "intervald")?;
    for (number, count) in maxticks {
        if let Some(frequency) = auto_frequency(number) {
            locator = locator.with_maxticks_for(frequency, count);
        }
    }
    for (number, intervals) in intervald {
        if let Some(frequency) = auto_frequency(number) {
            locator = locator.with_intervals(frequency, intervals)?;
        }
    }
    Ok(locator)
}

/// The `AutoDateLocator` frequency of matplotlib's number: dateutil's
/// constants, with weekly unused, and 7 for microseconds.
fn auto_frequency(number: usize) -> Option<AutoFrequency> {
    match number {
        7 => Some(AutoFrequency::Microsecondly),
        _ => frequency(number).ok().and_then(|freq| {
            AutoFrequency::ALL
                .into_iter()
                .find(|auto| auto.rrule() == Some(freq))
        }),
    }
}

/// The locator `AutoDateLocator` chose, as Python rebuilds it: a dict with
/// the `kind` of locator (`"rrule"`, `"year"` or `"microsecond"`), its
/// parameters (`rule`; `base`, `month`, `day`; `interval`), the
/// `frequency` chosen (as matplotlib numbers it, `None` when none gives
/// enough ticks) and the `fallback_interval` matplotlib warns about.
struct Choice(LocatorChoice);

impl IntoPy for Choice {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let Choice(choice) = self;
        let mut description: BTreeMap<String, Bound<'_>> = BTreeMap::new();
        let frequency = choice.frequency.map(|frequency| match frequency.rrule() {
            Some(freq) => Frequency::ALL
                .iter()
                .position(|&f| f == freq)
                .expect("every frequency is listed"),
            None => 7,
        });
        description.insert("frequency".to_owned(), frequency.into_py(py)?);
        description.insert(
            "fallback_interval".to_owned(),
            choice.fallback_interval.into_py(py)?,
        );
        let kind = match choice.locator {
            ChosenLocator::RRule(locator) => {
                description.insert("rule".to_owned(), Rule(locator.rule().clone()).into_py(py)?);
                "rrule"
            }
            ChosenLocator::Year(locator) => {
                description.insert("base".to_owned(), locator.base().into_py(py)?);
                description.insert("month".to_owned(), locator.month().into_py(py)?);
                description.insert("day".to_owned(), locator.day().into_py(py)?);
                "year"
            }
            ChosenLocator::Microsecond(locator) => {
                description.insert("interval".to_owned(), locator.micros().into_py(py)?);
                "microsecond"
            }
        };
        description.insert("kind".to_owned(), kind.into_py(py)?);
        description.into_py(py)
    }
}

/// The locator `AutoDateLocator` (the spec of `auto_locator`) delegates to
/// from `dmin` to `dmax` (matplotlib's `get_locator`), counted on the wall
/// clock of their zone (naive dates in UTC).
fn auto_date_choice(spec: Owned, dmin: PyDateTime, dmax: PyDateTime) -> PyResult<Choice> {
    let locator = Python::with_gil(|py| auto_locator(&spec.bind(py)))?;
    let (dmin, dmax) = (dmin.zoned(&TimeZone::Utc), dmax.zoned(&TimeZone::Utc));
    Ok(Choice(locator.get_locator(&dmin, &dmax)?))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("rrule_between", rrule_between)?;
    module.add("rrule_after", rrule_after)?;
    module.add("rrule_before", rrule_before)?;
    module.add("rule_tick_values", rule_tick_values)?;
    module.add("auto_date_choice", auto_date_choice)
}
