//! Probe operations for the `mpl_dates` area: date conversion, locators
//! and formatters ported from matplotlib's `tests/test_dates.py` (see
//! `spec/reference/probes/test_mpl_dates.py`).
//!
//! Many operations are the `dates` area's; the tests that read the ticks
//! of a date axis are replayed by [`axes`]. The probe's `rc` is in effect
//! for a whole operation and `made` is the `timezone` rcParam while the
//! operation makes its locator or formatter ([`made_in`]).

mod axes;

use plt::dates::{
    AutoDateFormatter, AutoFrequency, ConciseDateFormatter, Date, DateFormatter, DateLocator,
    Datetime64, RRuleLocator, datestr2num, datestr2num_with_default, datetime64_to_num, get_epoch,
    num2date, reset_epoch, set_epoch,
};
use plt::rc;
use plt::ticker::{Formatter, Locator, TickContext, TickerSettings};

use super::dates::{self, date_num, ticks, zone_arg, zoned};
use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("dates.date2num", dates::date2num_op),
    Op::new("dates.date2num.datetime64", |args| {
        in_epoch(args, || date2num_datetime64(args))
    }),
    Op::new("dates.num2date", |args| {
        with_rc(args, || dates::num2date_op(args))
    }),
    Op::new("dates.num2timedelta", dates::num2timedelta_op),
    Op::new("dates.drange", dates::drange_op),
    Op::new("dates.datestr2num", datestr2num_op),
    Op::new("dates.set_epoch", set_epoch_op),
    Op::new("dates.AutoDateLocator.__call__", |args| {
        in_epoch(args, || with_rc(args, || dates::auto_call(args)))
    }),
    Op::new("dates.AutoDateLocator.maxticks", maxticks),
    Op::new("dates.AutoDateFormatter.format_ticks", |args| {
        with_rc(args, || dates::auto_format_ticks(args))
    }),
    Op::new("dates.AutoDateFormatter.__call__", |args| {
        with_rc(args, || auto_hour_label(args))
    }),
    Op::new("dates.ConciseDateFormatter.format_ticks", |args| {
        with_rc(args, || concise_ticks(args))
    }),
    Op::new("dates.ConciseDateFormatter.__call__", |args| {
        with_rc(args, || {
            concise_text(args, ConciseDateFormatter::format_data)
        })
    }),
    Op::new("dates.ConciseDateFormatter.format_data_short", |args| {
        with_rc(args, || {
            concise_text(args, ConciseDateFormatter::format_data_short)
        })
    }),
    Op::new("dates.YearLocator.__call__", dates::year_call),
    Op::new("dates.DayLocator.tick_values", day_tick_values),
    Op::new("dates.RRuleLocator.tick_values", rrule_tick_values),
    Op::new("dates.DateLocator.nonsingular", date_locator_nonsingular),
    Op::new("dates.DateLocator.datalim_to_dt", |args| {
        with_rc(args, || datalim_to_dt(args))
    }),
    Op::new("dates.DateFormatter.__call__", |args| {
        with_rc(args, || date_formatter(args))
    }),
    Op::new("axes.Axes.get_xlim", axes::xlim),
    Op::new("axis.Axis.get_ticklabels", axes::ticklabels),
    Op::new("axis.Axis.get_major_locator", axes::too_many_ticks),
    Op::new("axis.ConciseDateFormatter.get_offset", axes::offsets),
];

/// Runs `f` with the probe's `epoch` set, as the tests'
/// `_new_epoch_decorator` does, and the default epoch afterwards.
pub(super) fn in_epoch(args: Args<'_>, f: impl FnOnce() -> Outcome) -> Outcome {
    let Some(epoch) = args.opt_str("epoch")? else {
        return f();
    };
    reset_epoch();
    let outcome = set_epoch(epoch)
        .map_err(ProbeError::rejected)
        .and_then(|()| f());
    reset_epoch();
    outcome
}

/// A string or a list of strings mapped by `map`.
fn one_or_many(value: &Json, map: impl Fn(&str) -> Result<Json, ProbeError>) -> Outcome {
    match (value.as_str(), value.as_array()) {
        (Some(text), _) => map(text),
        (_, Some(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .ok_or_else(|| ProbeError::BadArgs(format!("not a string: {item}")))
                    .and_then(&map)
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Json::Array),
        _ => Err(ProbeError::BadArgs(format!("not strings: {value}"))),
    }
}

/// `date2num(np.array(d, dtype=f"datetime64[{unit}]"))`: the unit only
/// sets the precision numpy keeps, which the probes' whole-second values
/// do not exceed.
fn date2num_datetime64(args: Args<'_>) -> Outcome {
    one_or_many(args.json("d")?, |text| {
        Datetime64::parse(text)
            .map(|value| Json::from(datetime64_to_num(value)))
            .map_err(ProbeError::rejected)
    })
}

/// Makes a locator or formatter with `make` while the `timezone` rcParam
/// is the probe's `made`, or as it is without one (the reference module's
/// `_made_in`).
fn made_in<T>(
    args: Args<'_>,
    make: impl FnOnce() -> Result<T, ProbeError>,
) -> Result<T, ProbeError> {
    let Some(zone) = args.opt_str("made")? else {
        return make();
    };
    rc::context(|| {
        rc::set("timezone", zone).map_err(ProbeError::rejected)?;
        make()
    })
}

/// `datestr2num(s, default)`; `default` is a `YYYY-MM-DD` date.
fn datestr2num_op(args: Args<'_>) -> Outcome {
    let default = args
        .opt_str("default")?
        .map(|text| {
            let parts: Option<Vec<u32>> = text.split('-').map(|p| p.parse().ok()).collect();
            match parts.as_deref() {
                Some(&[year, month, day]) => {
                    let year = i32::try_from(year).map_err(ProbeError::rejected)?;
                    Date::new(year, month, day).map_err(ProbeError::rejected)
                }
                _ => Err(ProbeError::BadArgs(format!("bad default {text}"))),
            }
        })
        .transpose()?;
    one_or_many(args.json("s")?, |text| {
        match default {
            Some(default) => datestr2num_with_default(text, default),
            None => datestr2num(text),
        }
        .map(Json::from)
        .map_err(ProbeError::rejected)
    })
}

/// `set_epoch(epoch)` after resetting it, and reading it first when the
/// probe says the epoch was `used`; the default epoch is restored.
fn set_epoch_op(args: Args<'_>) -> Outcome {
    reset_epoch();
    if args.bool("used")? {
        get_epoch();
    }
    let outcome = set_epoch(args.str("epoch")?);
    reset_epoch();
    outcome.map(|()| Json::Null).map_err(ProbeError::rejected)
}

/// `AutoDateLocator(**params).maxticks` keyed by frequency name.
fn maxticks(args: Args<'_>) -> Outcome {
    let locator = dates::auto_locator(args.get("params"))?;
    Ok(Json::Object(
        AutoFrequency::ALL
            .into_iter()
            .map(|freq| {
                (
                    freq.name().to_owned(),
                    Json::from(f64::from(locator.maxticks(freq))),
                )
            })
            .collect(),
    ))
}

/// `ConciseDateFormatter.format_ticks`: the `dates` area's op, or with
/// `locs` the labels of those locations for a formatter without an axis.
fn concise_ticks(args: Args<'_>) -> Outcome {
    if args.get("locs").is_none() {
        return dates::concise_format_ticks(args);
    }
    let settings = TickerSettings::default();
    ConciseDateFormatter::new()
        .format_ticks(&args.f64_vec("locs")?, &TickContext::detached(&settings))
        .map(|labels| Json::from(labels.labels))
        .map_err(ProbeError::rejected)
}

/// What `ConciseDateFormatter(AutoDateLocator())` makes of `x` without an
/// axis.
fn concise_text(
    args: Args<'_>,
    method: fn(
        &ConciseDateFormatter,
        f64,
        &TickContext<'_>,
    ) -> Result<String, plt::ticker::TickerError>,
) -> Outcome {
    let formatter = made_in(args, || Ok(ConciseDateFormatter::new()))?;
    detached_label(&formatter, method, args.f64("x")?)
}

/// What `AutoDateFormatter(HourLocator())` makes of the probe's `x`
/// without an axis.
fn auto_hour_label(args: Args<'_>) -> Outcome {
    let formatter = made_in(args, || {
        Ok(AutoDateFormatter::new(RRuleLocator::hours(&[], 1)))
    })?;
    detached_label(&formatter, AutoDateFormatter::format_data, args.f64("x")?)
}

/// The label `method` gives `x` without an axis.
fn detached_label<F>(
    formatter: &F,
    method: fn(&F, f64, &TickContext<'_>) -> Result<String, plt::ticker::TickerError>,
    x: f64,
) -> Outcome {
    let settings = TickerSettings::default();
    method(formatter, x, &TickContext::detached(&settings))
        .map(Json::from)
        .map_err(ProbeError::rejected)
}

/// `DayLocator().tick_values(vmin, vmax)` for date-times.
fn day_tick_values(args: Args<'_>) -> Outcome {
    let locator = RRuleLocator::days(&[], 1).map_err(ProbeError::rejected)?;
    tick_dates(&locator, args)
}

/// `RRuleLocator(rrulewrapper(freq, **rule)).tick_values(vmin, vmax)`.
fn rrule_tick_values(args: Args<'_>) -> Outcome {
    let locator = RRuleLocator::new(dates::rrule_spec(Args::new(args.json("rule")?))?);
    tick_dates(&locator, args)
}

/// `{ticks, iso}` of `locator.tick_values(vmin, vmax)` with date-time
/// limits.
fn tick_dates(locator: &impl DateLocator, args: Args<'_>) -> Outcome {
    let found = locator
        .tick_dates(&zoned(args.str("vmin")?)?, &zoned(args.str("vmax")?)?)
        .map_err(ProbeError::rejected)?;
    ticks(found)
}

/// `DateLocator().nonsingular(vmin, vmax)`. plt-rs's `DateLocator` is a
/// trait, so a `DayLocator` answers: in matplotlib it inherits
/// `DateLocator.nonsingular` unchanged.
fn date_locator_nonsingular(args: Args<'_>) -> Outcome {
    let locator = RRuleLocator::days(&[], 1).map_err(ProbeError::rejected)?;
    let (lo, hi) = locator.nonsingular(
        args.f64("vmin")?,
        args.f64("vmax")?,
        &TickContext::default(),
    );
    Ok(Json::from(vec![lo, hi]))
}

/// `DateLocator(tz=tz).datalim_to_dt()`: the ordered data interval as
/// date-times in the locator's zone; the dummy axis' data interval is
/// `(0, 1)` unless the probe gives `data`.
fn datalim_to_dt(args: Args<'_>) -> Outcome {
    let mut locator = made_in(args, || {
        RRuleLocator::days(&[], 1).map_err(ProbeError::rejected)
    })?;
    if args.get("tz").is_some() {
        locator = locator.with_tz(zone_arg(args)?);
    }
    let tz = locator.tz().map_err(ProbeError::rejected)?;
    let (a, b) = match args.json("data").ok().and_then(Json::as_array) {
        Some([a, b]) => {
            let num = |value: &Json| {
                value
                    .as_str()
                    .ok_or_else(|| ProbeError::BadArgs("data are dates".into()))
                    .and_then(date_num)
            };
            (num(a)?, num(b)?)
        }
        Some(_) => return Err(ProbeError::BadArgs("data needs two dates".into())),
        None => (0.0, 1.0),
    };
    [a.min(b), a.max(b)]
        .into_iter()
        .map(|x| {
            num2date(x, tz)
                .map(|d| Json::from(d.isoformat()))
                .map_err(ProbeError::rejected)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Json::Array)
}

/// `DateFormatter(fmt)`, given `set_tzinfo(tz)` when the probe names a
/// zone, applied to `d`.
fn date_formatter(args: Args<'_>) -> Outcome {
    let mut formatter = made_in(args, || Ok(DateFormatter::new(args.str("fmt")?)))?;
    if args.get("tz").is_some() {
        formatter = formatter.with_tz(zone_arg(args)?);
    }
    formatter
        .label(date_num(args.str("d")?)?, &TickContext::default())
        .map(Json::from)
        .map_err(ProbeError::rejected)
}
