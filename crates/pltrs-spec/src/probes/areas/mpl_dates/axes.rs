//! The tests of `test_dates.py` that read the limits, the tick labels or
//! the converter of a date axis, and pltrs' own cases of how a date axis
//! takes its zone, converter and tick helpers, replayed in a fresh figure
//! in the probe's style (see
//! the module docs of `test_mpl_dates.py` for the order of the set-up).

use pltrs::axes::{Axes, Axis, AxisName, Limits};
use pltrs::dates::{
    AutoDateFormatter, DateRange, DateTime, Datetime64, RRuleLocator, TimeDelta, TimeZone,
    UtcOffset, ZonedDateTime,
};
use pltrs::figure::Figure;
use pltrs::lines::LineProps;
use pltrs::ticker::NullLocator;
use pltrs::units::{AxisValues, DateConverter, UnitConverter};

use super::in_epoch;
use crate::json::Json;
use crate::probes::areas::dates::{self, zoned};
use crate::probes::areas::mpl_ticker::{
    formatter_name, install_formatter, install_locator, locator_name, spec_parts, styled,
};
use crate::probes::areas::text_metrics::set_rc;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The strings of a JSON array, if it holds only strings.
fn strings(value: &Json) -> Option<Vec<&str>> {
    value.as_array()?.iter().map(Json::as_str).collect()
}

/// Plotted values: numbers, or ISO strings for dates, aware as the
/// reference module's `_dt` makes them (in UTC unless they give an
/// offset).
fn values(value: &Json) -> Result<AxisValues, ProbeError> {
    if let Some(numbers) = value.as_f64_vec() {
        return Ok(AxisValues::from(numbers));
    }
    let dates = strings(value)
        .ok_or_else(|| ProbeError::BadArgs(format!("not numbers or dates: {value}")))?
        .into_iter()
        .map(zoned)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AxisValues::from(dates))
}

/// The `plot` forms of evenly spaced `datetime64` values, with their
/// spacing: `datetime64[D]` and `datetime64[h]`.
const SPACINGS: [(&str, TimeDelta); 2] = [
    ("days", TimeDelta::from_hours(24)),
    ("hours", TimeDelta::from_hours(1)),
];

/// `np.arange(start, stop, dtype=...)` for values `step` apart.
fn spaced(range: &Json, step: TimeDelta) -> Result<DateRange, ProbeError> {
    let instant = |text: &str| {
        let seconds = Datetime64::parse(text)
            .map_err(ProbeError::rejected)?
            .ok_or_else(|| ProbeError::BadArgs("NaT".into()))?
            .seconds();
        let micros = seconds.checked_mul(1_000_000).unwrap_or(i64::MAX);
        DateTime::from_timestamp_micros(micros).map_err(ProbeError::rejected)
    };
    let Some(&[start, stop]) = strings(range).as_deref() else {
        return Err(ProbeError::BadArgs("a range needs two dates".into()));
    };
    DateRange::new(instant(start)?, instant(stop)?, step).map_err(ProbeError::rejected)
}

/// The zones of the probe's `tz`: one name, or a list of names taken in
/// turn.
fn zones(spec: &Json) -> Result<Vec<TimeZone>, ProbeError> {
    let names = match spec.as_str() {
        Some(name) => vec![name],
        None => strings(spec)
            .filter(|names| !names.is_empty())
            .ok_or_else(|| ProbeError::BadArgs(format!("not zones: {spec}")))?,
    };
    names
        .into_iter()
        .map(|name| TimeZone::named(name).map_err(ProbeError::rejected))
        .collect()
}

/// `ax.plot(...)` for the probe's `plot`: spaced values are aware
/// date-times in the zones of `tz`, if it is given, and naive otherwise.
fn plot(ax: &Axes, spec: &Json) -> Result<(), ProbeError> {
    let spec = Args::new(spec);
    let Some((range, step)) = SPACINGS
        .iter()
        .find_map(|&(key, step)| spec.get(key).map(|range| (range, step)))
    else {
        ax.plot(values(spec.json("x")?)?, values(spec.json("y")?)?)
            .map_err(ProbeError::rejected)?;
        return Ok(());
    };
    let times = spaced(range, step)?;
    let counts: Vec<f64> = (0..times.len()).map(|i| i as f64).collect();
    let times = match spec.get("tz") {
        None => AxisValues::from(times),
        Some(tz) => times
            .zip(zones(tz)?.into_iter().cycle())
            .map(|(instant, tz)| ZonedDateTime::from_utc(instant, tz))
            .collect::<Result<Vec<_>, _>>()
            .map_err(ProbeError::rejected)?
            .into(),
    };
    ax.plot(times, counts).map_err(ProbeError::rejected)?;
    Ok(())
}

/// The minutes of a `["MinuteLocator", [minutes]]` spec.
fn minutes(arg: Option<&Json>) -> Result<Vec<u32>, ProbeError> {
    arg.and_then(Json::as_f64_vec)
        .map(|values| values.into_iter().map(|m| m as u32).collect())
        .ok_or_else(|| ProbeError::BadArgs("MinuteLocator needs minutes".into()))
}

/// Sets the major (or minor) locator of `axis` to the one a spec
/// describes.
fn set_locator(axis: &Axis, minor: bool, spec: &Json) -> Result<(), ProbeError> {
    match spec_parts(spec)? {
        ("AutoDateLocator", params) => install_locator(axis, minor, dates::auto_locator(params)?),
        ("MinuteLocator", arg) => {
            install_locator(axis, minor, RRuleLocator::minutes(&minutes(arg)?, 1));
        }
        ("DayLocator", None) => install_locator(
            axis,
            minor,
            RRuleLocator::days(&[], 1).map_err(ProbeError::rejected)?,
        ),
        ("NullLocator", None) => install_locator(axis, minor, NullLocator),
        _ => return Err(ProbeError::BadArgs(format!("bad locator {spec}"))),
    }
    Ok(())
}

/// Sets the major (or minor) formatter of `axis` to the one a spec
/// describes: `["ConciseDateFormatter", {formats, zero_formats, tz}]`
/// (`tz` an offset in hours) or `["AutoDateFormatter", ["MinuteLocator",
/// [minutes]]]`.
fn set_formatter(axis: &Axis, minor: bool, spec: &Json) -> Result<(), ProbeError> {
    match spec_parts(spec)? {
        ("ConciseDateFormatter", Some(params)) => {
            let params = Args::new(params);
            let mut formatter = dates::concise_formatter(params)?;
            if let Some(hours) = params.opt_i64("tz")? {
                let offset = i32::try_from(hours * 3600)
                    .map_err(ProbeError::rejected)
                    .and_then(|s| UtcOffset::from_seconds(s).map_err(ProbeError::rejected))?;
                formatter = formatter.with_tz(TimeZone::Fixed(offset));
            }
            install_formatter(axis, minor, formatter);
        }
        ("AutoDateFormatter", Some(locator)) => match spec_parts(locator)? {
            ("MinuteLocator", arg) => install_formatter(
                axis,
                minor,
                AutoDateFormatter::new(RRuleLocator::minutes(&minutes(arg)?, 1)),
            ),
            _ => return Err(ProbeError::BadArgs(format!("bad locator {locator}"))),
        },
        _ => return Err(ProbeError::BadArgs(format!("bad formatter {spec}"))),
    }
    Ok(())
}

/// `ax.set_xlim(...)` or `ax.set_ylim(...)` with two aware dates, which
/// makes axis `name` a date axis, in their zone if it had no converter.
fn set_date_limits(ax: &Axes, name: AxisName, lim: &Json) -> Result<(), ProbeError> {
    let Some(&[low, high]) = strings(lim).as_deref() else {
        return Err(ProbeError::BadArgs("lim needs two dates".into()));
    };
    let (low, high) = (zoned(low)?, zoned(high)?);
    match name {
        AxisName::X => ax.set_xlim(low, high),
        AxisName::Y => ax.set_ylim(low, high),
    }
    .map_err(ProbeError::rejected)?;
    Ok(())
}

/// Applies the config of one axis: limits, locators, formatters, the
/// second minor locator and the orientation, in that order.
fn configure(ax: &Axes, name: AxisName, config: Args<'_>) -> Result<(), ProbeError> {
    let axis = ax.axis_of(name);
    if let Some(lim) = config.get("lim") {
        set_date_limits(ax, name, lim)?;
    }
    for (key, minor) in [("major", false), ("minor", true)] {
        if let Some(spec) = config.get(key) {
            set_locator(&axis, minor, spec)?;
        }
    }
    for (key, minor) in [("major_formatter", false), ("minor_formatter", true)] {
        if let Some(spec) = config.get(key) {
            set_formatter(&axis, minor, spec)?;
        }
    }
    if let Some(spec) = config.get("minor2") {
        set_locator(&axis, true, spec)?;
    }
    if config.opt_bool("invert")? == Some(true) {
        axis.set_inverted(true);
    }
    Ok(())
}

/// The converter a probe's converter name gives: `"dates"` and `"auto"`
/// matplotlib's `DateConverter()`, `"concise"` its
/// `ConciseDateConverter()`, and `"switchable"` the
/// `_SwitchableDateConverter` plotting dates gives.
fn converter(name: &str) -> Result<UnitConverter, ProbeError> {
    match name {
        "dates" | "auto" => Ok(DateConverter::Auto.into()),
        "concise" => Ok(DateConverter::Concise.into()),
        "switchable" => Ok(UnitConverter::Dates),
        _ => Err(ProbeError::BadArgs(format!("bad converter {name:?}"))),
    }
}

/// `ax.xaxis.set_converter(...)` with the converter the probe's `key`
/// names, if it names one.
fn set_converter(ax: &Axes, args: Args<'_>, key: &str) -> Result<(), ProbeError> {
    if let Some(name) = args.opt_str(key)? {
        ax.xaxis()
            .set_converter(converter(name)?)
            .map_err(ProbeError::rejected)?;
    }
    Ok(())
}

/// `ax.xaxis_date(tz)` for the probe's `date_first` or `xaxis_date`:
/// `{tz}`, `tz` a zone name or `null`; `xaxis_date` may also be `true`,
/// `ax.xaxis_date()`, or `false`, no call.
fn xaxis_date(ax: &Axes, spec: &Json) -> Result<(), ProbeError> {
    let tz = match spec.as_bool() {
        Some(false) => return Ok(()),
        Some(true) => None,
        None => Args::new(spec)
            .opt_str("tz")?
            .map(TimeZone::named)
            .transpose()
            .map_err(ProbeError::rejected)?,
    };
    ax.xaxis_date(tz.as_ref());
    Ok(())
}

/// Sets the probe's axes up, in the order of the reference module's
/// `_on_axes`: the converter, `date_first`, the line, the plots, the late
/// converter, the date axis calls and the config of each axis.
fn set_up(ax: &Axes, args: Args<'_>) -> Result<(), ProbeError> {
    set_converter(ax, args, "converter")?;
    if let Some(spec) = args.get("date_first") {
        xaxis_date(ax, spec)?;
    }
    if let Some(y) = args.opt_f64("axhline")? {
        ax.axhline(y, LineProps::default());
    }
    if let Some(spec) = args.get("plot") {
        plot(ax, spec)?;
    }
    if let Some(replot) = args.get("replot").map(Args::new) {
        set_rc(replot.get("rc"))?;
        plot(ax, replot.json("plot")?)?;
    }
    set_converter(ax, args, "late_converter")?;
    if let Some(spec) = args.get("xaxis_date") {
        xaxis_date(ax, spec)?;
    }
    if args.opt_bool("axis_date")? == Some(true) {
        ax.xaxis().axis_date(None);
    }
    for (key, name) in [("x", AxisName::X), ("y", AxisName::Y)] {
        if let Some(config) = args.get(key) {
            configure(ax, name, Args::new(config))?;
        }
    }
    Ok(())
}

/// Builds the probe's axes, in its epoch and style, and answers `query`.
fn on_axes(args: Args<'_>, query: impl FnOnce(&Figure, &Axes) -> Outcome) -> Outcome {
    in_epoch(args, || {
        styled(args, || {
            let figure = Figure::new();
            let ax = figure.subplot();
            set_up(&ax, args)?;
            query(&figure, &ax)
        })
    })
}

/// The x limits after drawing.
pub(super) fn xlim(args: Args<'_>) -> Outcome {
    on_axes(args, |figure, ax| {
        figure.layout();
        let (low, high) = ax.xlim();
        Ok(Json::from(vec![low, high]))
    })
}

/// `{labels, offset}` of an axis after drawing.
pub(super) fn ticklabels(args: Args<'_>) -> Outcome {
    on_axes(args, |_, ax| {
        let name = match args.str("axis")? {
            "x" => AxisName::X,
            "y" => AxisName::Y,
            other => return Err(ProbeError::BadArgs(format!("axis {other}"))),
        };
        let labels = ax
            .axis_of(name)
            .ticklabels(args.bool("minor")?)
            .map_err(ProbeError::rejected)?;
        let offset = ax.ticks(name).map_err(ProbeError::rejected)?.offset;
        Ok(encode::object([
            ("labels", Json::from(labels)),
            ("offset", Json::from(offset)),
        ]))
    })
}

/// `{locator, formatter}`: the class names of the x axis' major tick
/// helpers.
pub(super) fn major(args: Args<'_>) -> Outcome {
    on_axes(args, |_, ax| {
        let axis = ax.xaxis();
        Ok(encode::object([
            ("locator", Json::from(locator_name(&axis.major_locator())?)),
            (
                "formatter",
                Json::from(formatter_name(&axis.major_formatter())?),
            ),
        ]))
    })
}

/// The x axis' unit: `null`, or the abbreviation of its zone on
/// 2020-01-01.
pub(super) fn units(args: Args<'_>) -> Outcome {
    on_axes(args, |_, ax| {
        let Some(unit) = ax.xaxis().units() else {
            return Ok(Json::Null);
        };
        let zone = unit
            .downcast_ref::<TimeZone>()
            .ok_or_else(|| ProbeError::rejected(format!("the unit {unit:?} is no zone")))?;
        let new_year = DateTime::new(2020, 1, 1, 0, 0, 0, 0).map_err(ProbeError::rejected)?;
        Ok(Json::from(zone.name_at_local(new_year, false)))
    })
}

/// The x axis' converter of dates, named as the probe's `converter` names
/// it (`"auto"` for `DateConverter()`), or `null` without a converter.
pub(super) fn converter_name(args: Args<'_>) -> Outcome {
    on_axes(args, |_, ax| match ax.xaxis().converter() {
        None => Ok(Json::Null),
        Some(UnitConverter::Dates) => Ok(Json::from("switchable")),
        Some(UnitConverter::DatesBy(kind)) => Ok(Json::from(kind.name())),
        Some(other) => Err(ProbeError::rejected(format!("{other:?} converts no dates"))),
    })
}

/// `test_too_many_date_ticks`: identical date limits widened by the date
/// locator, then a `DayLocator`: `{count, xlim}`.
pub(super) fn too_many_ticks(args: Args<'_>) -> Outcome {
    on_axes(args, |_, ax| {
        let t0 = ax.xaxis().convert_value(zoned("2000-01-20T00:00:00")?);
        let limits = Limits {
            auto: Some(true),
            ..Limits::new(t0, t0)
        };
        ax.set_limits_with(AxisName::X, limits)
            .map_err(ProbeError::rejected)?;
        ax.plot(Vec::<f64>::new(), Vec::<f64>::new())
            .map_err(ProbeError::rejected)?;
        ax.xaxis()
            .set_major_locator(RRuleLocator::days(&[], 1).map_err(ProbeError::rejected)?);
        let count = ax
            .xaxis()
            .ticklocs(false)
            .map_err(ProbeError::rejected)?
            .len();
        let (low, high) = ax.xlim();
        Ok(encode::object([
            ("count", Json::from(count as f64)),
            ("xlim", Json::from(vec![low, high])),
        ]))
    })
}

/// The offset text after drawing at each of the probe's successive x
/// limits (`null` keeps the current ones).
pub(super) fn offsets(args: Args<'_>) -> Outcome {
    on_axes(args, |_, ax| {
        let xlims = args
            .json("xlims")?
            .as_array()
            .ok_or_else(|| ProbeError::BadArgs("xlims is not a list".into()))?;
        let mut texts = Vec::new();
        for lim in xlims {
            if !lim.is_null() {
                set_date_limits(ax, AxisName::X, lim)?;
            }
            texts.push(Json::from(
                ax.ticks(AxisName::X).map_err(ProbeError::rejected)?.offset,
            ));
        }
        Ok(Json::Array(texts))
    })
}
