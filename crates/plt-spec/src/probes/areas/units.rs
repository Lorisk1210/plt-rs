//! Probe operations for the `units` area (see
//! `spec/reference/probes/test_units.py`): dates moved by durations,
//! exactly, error sizes that do not add to their values, and geographic
//! axes asked to share an axis.

use plt::axes::{Axes, BarOptions, ErrorbarOptions, ProjectionKind};
use plt::dates::{DateTime, TimeDelta};
use plt::figure::{Figure, Share};
use plt::mplot3d::Errorbar3DOptions;
use plt::patches::DataRectangle;

use crate::json::Json;
use crate::probes::encode::numbers;
use crate::probes::{Args, Op, Outcome, ProbeError, malformed};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Rectangle.dates", rectangle_dates),
    Op::new("Axes.bar.dates", bar_dates),
    Op::new("Axes.errorbar.dates", errorbar_dates),
    Op::new("Axes.errorbar.unaddable", errorbar_unaddable),
    Op::new("Axes3D.errorbar.timedelta", errorbar_3d_timedelta),
    Op::new("share.geographic", share_geographic),
];

/// The date `[year, month, day, hour, minute, second, microsecond]`.
fn date(fields: &[f64]) -> Result<DateTime, ProbeError> {
    let &[year, month, day, hour, minute, second, micros] = fields else {
        return Err(malformed("date"));
    };
    let field = |value: f64| value as u32;
    DateTime::new(
        year as i32,
        field(month),
        field(day),
        field(hour),
        field(minute),
        field(second),
        field(micros),
    )
    .map_err(ProbeError::rejected)
}

/// The duration `[days, seconds, microseconds]`.
fn duration(fields: &[f64]) -> Result<TimeDelta, ProbeError> {
    let &[days, seconds, micros] = fields else {
        return Err(malformed("duration"));
    };
    TimeDelta::new(days as i64, seconds as i64, micros as i64).map_err(ProbeError::rejected)
}

/// The dates of the rows of argument `key`.
fn dates(args: Args<'_>, key: &str) -> Result<Vec<DateTime>, ProbeError> {
    args.rows(key)?.iter().map(|row| date(row)).collect()
}

/// A figure and its one axes.
fn axes() -> (Figure, Axes) {
    let fig = Figure::new();
    let ax = fig.subplot();
    (fig, ax)
}

/// `[x0, x1 - x0]` of a rectangle from a date with a duration as width.
fn rectangle_dates(args: Args<'_>) -> Outcome {
    let start = date(&args.f64_vec("start")?)?;
    let width = duration(&args.f64_vec("duration")?)?;
    let (fig, ax) = axes();
    let rectangle = DataRectangle::new((start, 0.0), width, 1.0).map_err(ProbeError::rejected)?;
    let shape = ax.add_patch(rectangle).shape();
    fig.layout();
    Ok(numbers([shape.xy.x, shape.width]))
}

/// The widths of bars at dates, durations wide.
fn bar_dates(args: Args<'_>) -> Outcome {
    let starts = dates(args, "starts")?;
    let widths = args
        .rows("widths")?
        .iter()
        .map(|row| duration(row))
        .collect::<Result<Vec<_>, _>>()?;
    let (fig, ax) = axes();
    let options = BarOptions {
        thickness: widths.into(),
        ..BarOptions::default()
    };
    let bars = ax
        .bar(starts, [1.0], options)
        .map_err(ProbeError::rejected)?;
    fig.layout();
    Ok(numbers(bars.patches.iter().map(|bar| bar.shape().width)))
}

/// The `[x0, x1]` of error bars a duration wide around dates.
fn errorbar_dates(args: Args<'_>) -> Outcome {
    let x = dates(args, "x")?;
    let xerr = duration(&args.f64_vec("xerr")?)?;
    let (fig, ax) = axes();
    let zeros = vec![0.0; x.len()];
    let options = ErrorbarOptions {
        xerr: Some(xerr.into()),
        ..ErrorbarOptions::default()
    };
    let bars = ax
        .errorbar(x, zeros, options)
        .map_err(ProbeError::rejected)?;
    fig.layout();
    let segments = bars.barlinecols[0]
        .paths()
        .iter()
        .map(|path| match path.vertices() {
            [start, end, ..] => Ok(numbers([start.x, end.x])),
            _ => Err(ProbeError::rejected("an error bar without two ends")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json::Array(segments))
}

/// Error bars whose sizes do not add to their values: a duration to
/// numbers or a number to dates.
fn errorbar_unaddable(args: Args<'_>) -> Outcome {
    let (fig, ax) = axes();
    let day = TimeDelta::new(1, 0, 0).map_err(ProbeError::rejected)?;
    let made = match args.str("kind")? {
        "number+timedelta" => {
            let options = ErrorbarOptions {
                xerr: Some(day.into()),
                ..ErrorbarOptions::default()
            };
            ax.errorbar([1.0, 2.0], [1.0, 2.0], options)
        }
        "date+number" => {
            let days = [1, 2]
                .map(|day| DateTime::new(2020, 5, day, 0, 0, 0, 0))
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .map_err(ProbeError::rejected)?;
            let options = ErrorbarOptions {
                xerr: Some(0.5.into()),
                ..ErrorbarOptions::default()
            };
            ax.errorbar(days, [1.0, 2.0], options)
        }
        _ => return Err(malformed("kind")),
    };
    made.map_err(ProbeError::rejected)?;
    fig.layout();
    Ok(Json::Null)
}

/// 3-D error bars a day long on numbers.
fn errorbar_3d_timedelta(_: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot_3d();
    let day = TimeDelta::new(1, 0, 0).map_err(ProbeError::rejected)?;
    let options = Errorbar3DOptions {
        errors: [Some(day.into()), None, None],
        ..Errorbar3DOptions::default()
    };
    ax.errorbar([0.0, 1.0], [0.0, 1.0], [0.0, 1.0], options)
        .map_err(ProbeError::rejected)?;
    fig.layout();
    Ok(Json::Null)
}

/// Geographic axes made to share their x axis, or giving it.
fn share_geographic(args: Args<'_>) -> Outcome {
    let projection = ProjectionKind::from_name(args.str("projection")?)
        .ok_or_else(|| malformed("projection"))?;
    let fig = Figure::new();
    match args.str("how")? {
        "subplots" => {
            fig.subplots_with_projection(1, 2, Share::All, Share::None, projection)
                .map_err(ProbeError::rejected)?;
        }
        "geo.sharex" => {
            let rect = fig.add_subplot(1, 2, 1).map_err(ProbeError::rejected)?;
            let geo = fig
                .add_subplot_with_projection(1, 2, 2, projection)
                .map_err(ProbeError::rejected)?;
            geo.share_x(&rect).map_err(ProbeError::rejected)?;
        }
        "rect.sharex" => {
            let geo = fig
                .add_subplot_with_projection(1, 2, 2, projection)
                .map_err(ProbeError::rejected)?;
            let rect = fig.add_subplot(1, 2, 1).map_err(ProbeError::rejected)?;
            rect.share_x(&geo).map_err(ProbeError::rejected)?;
        }
        _ => return Err(malformed("how")),
    }
    fig.layout();
    Ok(Json::Null)
}
