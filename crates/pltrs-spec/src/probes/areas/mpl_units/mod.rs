//! Probe operations for the `mpl_units` area: matplotlib's own tests of
//! unit conversion (`tests/test_units.py`; see
//! `spec/reference/probes/test_mpl_units.py`) for dates, category names
//! and the converters of an axis; [`user`] replays those of user unit
//! classes.

mod user;

use pltrs::axes::{Axes, ErrorbarOptions};
use pltrs::dates::{DateTime, Datetime64, TimeDelta, TimeZone, UtcOffset, get_epoch};
use pltrs::figure::{Figure, Share};
use pltrs::ticker::AxisName;
use pltrs::units::{DateConverter, UnitConverter};

use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::encode::{numbers, pair};
use crate::probes::{Args, Op, Outcome, ProbeError, encode, malformed};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Axes.scatter", scatter),
    Op::new("Axes.errorbar", errorbar),
    Op::new("Axis.set_units", set_units),
    Op::new("Axes.plot.shared_categories", shared_categories),
    Op::new("Axis.set_converter", set_converter),
    Op::new("Axes.set_lim.quantity", user::lim_quantity),
    Op::new("Axis.set_units.shared_quantity", user::shared_quantity),
    Op::new("Axes.default_limits.quantity", user::default_limits),
    Op::new("jpl.EpochConverter.convert", user::jpl_consistent),
    Op::new("Axes.plot.array_like", user::array_like),
];

/// Seconds in a day.
const SECONDS_PER_DAY: i64 = 86_400;

/// The spec's name of a converter: either converter of dates is
/// `"dates"`.
fn kind(converter: Option<&UnitConverter>) -> Json {
    match converter {
        Some(UnitConverter::Dates | UnitConverter::DatesBy(_)) => Json::from("dates"),
        Some(UnitConverter::Categories) => Json::from("categories"),
        Some(UnitConverter::Custom(_)) | None => Json::Null,
    }
}

/// The day number of `"YYYY-MM"` or `"YYYY-MM-DD"` (its first day).
fn day_number(text: &str) -> Result<i64, ProbeError> {
    let fields = text
        .split('-')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| malformed("date"))?;
    let (year, month, day) = match *fields.as_slice() {
        [year, month] => (year, month, 1),
        [year, month, day] => (year, month, day),
        _ => return Err(malformed("date")),
    };
    let year = i32::try_from(year).map_err(|_| malformed("date"))?;
    let date = DateTime::new(year, month, day, 0, 0, 0, 0).map_err(ProbeError::rejected)?;
    // The epoch is 1970-01-01, where day numbers count whole days.
    let days = get_epoch().date2num(date);
    Ok(days.round() as i64)
}

/// A figure and its one axes.
fn axes() -> (Figure, Axes) {
    let fig = Figure::new();
    let ax = fig.subplot();
    (fig, ax)
}

/// `scatter(np.arange(start, stop, dtype="datetime64[D]"), y)`.
fn scatter(args: Args<'_>) -> Outcome {
    styled(args, || {
        let (start, stop) = (
            day_number(args.str("start")?)?,
            day_number(args.str("stop")?)?,
        );
        let times: Vec<Datetime64> = (start..stop)
            .map(|day| Datetime64::new(day * SECONDS_PER_DAY, 0))
            .collect();
        let mut y: Vec<f64> = (0..times.len()).map(|i| i as f64).collect();
        if args.bool("nan_first")?
            && let Some(first) = y.first_mut()
        {
            *first = f64::NAN;
        }
        let (fig, ax) = axes();
        let points = ax.scatter(times, y).map_err(ProbeError::rejected)?;
        fig.layout();
        let offsets = points
            .offsets()
            .into_iter()
            .filter(|p| p.x.is_finite() && p.y.is_finite())
            .map(encode::point)
            .collect();
        Ok(encode::object([
            ("converter", kind(ax.xaxis().converter().as_ref())),
            ("xlim", pair(ax.xlim())),
            ("ylim", pair(ax.ylim())),
            ("offsets", Json::Array(offsets)),
        ]))
    })
}

/// `errorbar(arange(n), [datetime(2020, 5, d) ...], timedelta(days=...))`.
fn errorbar(args: Args<'_>) -> Outcome {
    styled(args, || {
        let days = args
            .f64_vec("days")?
            .into_iter()
            .map(|day| {
                let day = day as u32;
                DateTime::new(2020, 5, day, 0, 0, 0, 0).map_err(ProbeError::rejected)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let seconds = (args.f64("yerr_days")? * SECONDS_PER_DAY as f64).round() as i64;
        let yerr = TimeDelta::new(0, seconds, 0).map_err(ProbeError::rejected)?;
        let (fig, ax) = axes();
        let x: Vec<f64> = (0..days.len()).map(|i| i as f64).collect();
        let options = ErrorbarOptions {
            yerr: Some(yerr.into()),
            ..ErrorbarOptions::default()
        };
        let bars = ax
            .errorbar(x, days, options)
            .map_err(ProbeError::rejected)?;
        fig.layout();
        let segments = bars
            .barlinecols
            .first()
            .ok_or_else(|| ProbeError::rejected("no error bars"))?
            .paths()
            .iter()
            .map(|path| match path.vertices() {
                [start, end, ..] => Ok(numbers([start.y, end.y])),
                _ => Err(ProbeError::rejected("an error bar without two ends")),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(encode::object([
            ("converter", kind(ax.yaxis().converter().as_ref())),
            ("xlim", pair(ax.xlim())),
            ("ylim", pair(ax.ylim())),
            ("segments", Json::Array(segments)),
        ]))
    })
}

/// The first days of the months of `year`.
fn months(year: i32) -> Result<Vec<DateTime>, ProbeError> {
    (1..=12)
        .map(|month| DateTime::new(year, month, 1, 0, 0, 0, 0).map_err(ProbeError::rejected))
        .collect()
}

/// Dates on two axes sharing y, then the first axis' unit set to a fixed
/// time zone: the second axis' ticks.
fn set_units(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let axs = fig
            .subplots_shared(1, 2, Share::None, Share::All)
            .map_err(ProbeError::rejected)?;
        let (ax1, ax2) = (axs[(0, 0)].clone(), axs[(0, 1)].clone());
        let index: Vec<f64> = (0..12).map(f64::from).collect();
        ax1.plot(index.clone(), months(2020)?)
            .map_err(ProbeError::rejected)?;
        ax2.plot(index, months(2021)?)
            .map_err(ProbeError::rejected)?;
        let seconds =
            i32::try_from(args.i64("tz_hours")? * 3600).map_err(|_| malformed("tz_hours"))?;
        let offset = UtcOffset::from_seconds(seconds).map_err(ProbeError::rejected)?;
        ax1.yaxis().set_units(TimeZone::Fixed(offset));
        fig.layout();
        let ticks = ax2.ticks(AxisName::Y).map_err(ProbeError::rejected)?;
        let locations: Vec<f64> = ticks.major.iter().map(|tick| tick.loc).collect();
        let labels = ticks
            .major
            .iter()
            .map(|tick| Json::from(tick.label.as_str()))
            .collect();
        Ok(encode::object([
            ("ticks", numbers(&locations)),
            ("labels", Json::Array(labels)),
        ]))
    })
}

/// The probe's list of category names `name`.
fn names(args: Args<'_>, name: &str) -> Result<Vec<String>, ProbeError> {
    args.json(name)?
        .as_array()
        .ok_or_else(|| malformed(name))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| malformed(name))
        })
        .collect()
}

/// Strings as JSON.
fn texts<'a>(values: impl IntoIterator<Item = &'a str>) -> Json {
    Json::Array(values.into_iter().map(Json::from).collect())
}

/// Category names plotted on two axes sharing x and y (nothing on the
/// second for an empty list).
fn shared_categories(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let axs = fig
            .subplots_shared(1, 2, Share::All, Share::All)
            .map_err(ProbeError::rejected)?;
        let axs = [axs[(0, 0)].clone(), axs[(0, 1)].clone()];
        let mut lines = Vec::new();
        for (name, ax) in ["first", "second"].into_iter().zip(&axs) {
            let categories = names(args, name)?;
            if categories.is_empty() {
                continue;
            }
            let y: Vec<f64> = (0..categories.len()).map(|i| i as f64).collect();
            lines.push(ax.plot(categories, y).map_err(ProbeError::rejected)?);
        }
        fig.layout();
        let mapping = |ax: &Axes| {
            ax.xaxis()
                .categories()
                .map_or(Json::Array(Vec::new()), |categories| {
                    texts(categories.names().iter().map(String::as_str))
                })
        };
        let labels = axs
            .iter()
            .map(|ax| {
                let ticks = ax.ticks(AxisName::X).map_err(ProbeError::rejected)?;
                Ok(texts(ticks.major.iter().map(|tick| tick.label.as_str())))
            })
            .collect::<Result<Vec<_>, ProbeError>>()?;
        Ok(encode::object([
            ("names", Json::Array(axs.iter().map(mapping).collect())),
            (
                "xdata",
                Json::Array(lines.iter().map(|line| numbers(line.xdata())).collect()),
            ),
            ("labels", Json::Array(labels)),
        ]))
    })
}

/// The converter of an axis through `set_converter`, plotting and reading
/// it.
fn set_converter(args: Args<'_>) -> Outcome {
    styled(args, || {
        let (_fig, ax) = axes();
        let axis = ax.xaxis();
        let mut recorded = Vec::new();
        for step in args
            .json("steps")?
            .as_array()
            .ok_or_else(|| malformed("steps"))?
        {
            let parts = step.as_array().ok_or_else(|| malformed("step"))?;
            match (
                parts.first().and_then(Json::as_str),
                parts.get(1).and_then(Json::as_str),
            ) {
                (Some("set"), Some(name)) => {
                    let converter = match name {
                        "categories" => UnitConverter::Categories,
                        "dates" => DateConverter::Auto.into(),
                        _ => return Err(malformed("converter")),
                    };
                    recorded.push(match axis.set_converter(converter) {
                        Ok(_) => Json::Null,
                        Err(_) => Json::from("error"),
                    });
                }
                (Some("plot"), None) => {
                    ax.plot(vec!["a", "b"], [1.0, 2.0])
                        .map_err(ProbeError::rejected)?;
                    recorded.push(Json::Null);
                }
                (Some("get"), None) => recorded.push(kind(axis.converter().as_ref())),
                _ => return Err(malformed("step")),
            }
        }
        Ok(Json::Array(recorded))
    })
}
