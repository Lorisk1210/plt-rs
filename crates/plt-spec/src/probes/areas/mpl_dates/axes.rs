//! The tests of `test_dates.py` that read the limits or the tick labels of
//! a date axis, and plt-rs' own cases of how a date axis takes its zone
//! and tick helpers, replayed in a fresh figure in the probe's style (see
//! the module docs of `test_mpl_dates.py` for the order of the set-up).

use plt::axes::{Axes, Axis, AxisName, Limits};
use plt::dates::{AutoDateFormatter, DateTime, Datetime64, RRuleLocator, TimeZone, UtcOffset};
use plt::figure::Figure;
use plt::lines::LineProps;
use plt::ticker::NullLocator;
use plt::units::{AxisValues, UnitConverter};

use super::in_epoch;
use crate::json::Json;
use crate::probes::areas::dates::{self, date_num, zoned};
use crate::probes::areas::mpl_ticker::{
    formatter_name, install_formatter, install_locator, locator_name, spec_parts, styled,
};
use crate::probes::areas::text_metrics::set_rc;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// Plotted values: numbers, or ISO strings for dates.
fn values(value: &Json) -> Result<AxisValues, ProbeError> {
    if let Some(numbers) = value.as_f64_vec() {
        return Ok(AxisValues::from(numbers));
    }
    let dates = value
        .as_array()
        .and_then(|items| items.iter().map(Json::as_str).collect::<Option<Vec<_>>>())
        .ok_or_else(|| ProbeError::BadArgs(format!("not numbers or dates: {value}")))?
        .into_iter()
        .map(|text| zoned(text)?.to_utc().map_err(ProbeError::rejected))
        .collect::<Result<Vec<DateTime>, _>>()?;
    Ok(AxisValues::from(dates))
}

/// The `plot` forms of evenly spaced `datetime64` values, with their
/// spacing in seconds: `datetime64[D]` and `datetime64[h]`.
const SPACINGS: [(&str, usize); 2] = [("days", 86_400), ("hours", 3_600)];

/// `np.arange(start, stop, dtype=...)` for values `step` seconds apart.
fn spaced(range: &Json, step: usize) -> Result<Vec<Datetime64>, ProbeError> {
    let bounds = range
        .as_array()
        .and_then(|items| items.iter().map(Json::as_str).collect::<Option<Vec<_>>>())
        .ok_or_else(|| ProbeError::BadArgs("a range needs two dates".into()))?;
    let instant = |text: &str| {
        Datetime64::parse(text)
            .map_err(ProbeError::rejected)?
            .ok_or_else(|| ProbeError::BadArgs("NaT".into()))
    };
    let [start, stop] = bounds[..] else {
        return Err(ProbeError::BadArgs("a range needs two dates".into()));
    };
    let (first, last) = (instant(start)?.seconds(), instant(stop)?.seconds());
    Ok((first..last)
        .step_by(step)
        .map(|seconds| Datetime64::new(seconds, 0))
        .collect())
}

/// `ax.plot(...)` for the probe's `plot`.
fn plot(ax: &Axes, spec: &Json) -> Result<(), ProbeError> {
    let spec = Args::new(spec);
    if let Some((range, step)) = SPACINGS
        .iter()
        .find_map(|&(key, step)| spec.get(key).map(|range| (range, step)))
    {
        let times = spaced(range, step)?;
        let counts: Vec<f64> = (0..times.len()).map(|i| i as f64).collect();
        ax.plot(times, counts).map_err(ProbeError::rejected)?;
        return Ok(());
    }
    ax.plot(values(spec.json("x")?)?, values(spec.json("y")?)?)
        .map_err(ProbeError::rejected)?;
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

/// Sets the view limits of axis `name` to two dates, which makes it a
/// date axis as matplotlib's unit conversion of `set_xlim` does.
fn set_date_limits(ax: &Axes, name: AxisName, lim: &Json) -> Result<(), ProbeError> {
    let Some([low, high]) = lim.as_array() else {
        return Err(ProbeError::BadArgs("lim needs two dates".into()));
    };
    let num = |value: &Json| {
        value
            .as_str()
            .ok_or_else(|| ProbeError::BadArgs("lim holds dates".into()))
            .and_then(date_num)
    };
    let (low, high) = (num(low)?, num(high)?);
    ax.axis_of(name).axis_date(None);
    ax.set_limits(name, low, high)
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

/// The converter a probe's `converter` names (only `"dates"`, matplotlib's
/// `DateConverter`).
fn converter(name: &str) -> Result<UnitConverter, ProbeError> {
    match name {
        "dates" => Ok(UnitConverter::Dates),
        _ => Err(ProbeError::BadArgs(format!("bad converter {name:?}"))),
    }
}

/// `ax.xaxis_date(tz)` for the probe's `date_first`, `{tz}` with `tz` a
/// zone name or `null`.
fn date_first(ax: &Axes, spec: &Json) -> Result<(), ProbeError> {
    let tz = Args::new(spec)
        .opt_str("tz")?
        .map(TimeZone::named)
        .transpose()
        .map_err(ProbeError::rejected)?;
    ax.xaxis_date(tz.as_ref());
    Ok(())
}

/// Sets the probe's axes up, in the order of the reference module's
/// `_on_axes`: the converter, `date_first`, the line, the plots, the date
/// axis calls and the config of each axis.
fn set_up(ax: &Axes, args: Args<'_>) -> Result<(), ProbeError> {
    if let Some(name) = args.opt_str("converter")? {
        ax.xaxis()
            .set_converter(converter(name)?)
            .map_err(ProbeError::rejected)?;
    }
    if let Some(spec) = args.get("date_first") {
        date_first(ax, spec)?;
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
    if args.opt_bool("xaxis_date")? == Some(true) {
        ax.xaxis_date(None);
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

/// `test_too_many_date_ticks`: identical date limits widened by the date
/// locator, then a `DayLocator`: `{count, xlim}`.
pub(super) fn too_many_ticks(args: Args<'_>) -> Outcome {
    on_axes(args, |_, ax| {
        let t0 = date_num("2000-01-20T00:00:00")?;
        ax.xaxis().axis_date(None);
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
