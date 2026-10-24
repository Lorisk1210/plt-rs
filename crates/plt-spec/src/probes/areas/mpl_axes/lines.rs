//! Lines: format strings and plot arguments, infinite lines, `vlines` and
//! `hlines` colours, and the colour cycle of filled areas.

use plt::axes::{Axes, FillBetweenOptions};
use plt::collections::{Collection, CollectionProps, EdgeColors};
use plt::colors::Color;
use plt::figure::Figure;
use plt::lines::{AxLineThrough, Line2D, LineProps};
use plt::orientation::Orientation;
use plt::units::AxisValues;
use plt_core::Point;

use super::{color, made, on_axes, optional, rgba_list};
use crate::json::Json;
use crate::probes::areas::canon::Canon;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// Plot data: numbers, or categories.
fn plot_values(value: &Json) -> Result<AxisValues, ProbeError> {
    if let Some(numbers) = value.as_f64_vec() {
        return Ok(AxisValues::Numbers(numbers));
    }
    super::strings(value).map(AxisValues::Categories)
}

/// `{color, marker, linestyle}` of a line.
fn line_props(line: &Line2D) -> Result<Json, ProbeError> {
    Ok(encode::object([
        ("color", encode::rgba(made(line.rgba())?)),
        ("marker", line.marker().canon()),
        ("linestyle", line.linestyle().canon()),
    ]))
}

pub(super) fn plot(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let y = plot_values(args.json("y")?)?;
        let x = match optional(args, "x", plot_values)? {
            Some(x) => x,
            None => AxisValues::Numbers((0..y.len()).map(|i| i as f64).collect()),
        };
        let mut props = match args.opt_str("fmt")? {
            Some(format) => made(LineProps::fmt(format))?,
            None => LineProps::default(),
        };
        props.color = optional(args, "color", color)?.or(props.color);
        if let Some(style) = args.opt_str("linestyle")? {
            props.linestyle = Some(made(style.parse())?);
        }
        let line = made(ax.plot_with(x, y, props))?;
        Ok(Json::Array(vec![line_props(&line)?]))
    })
}

/// A point argument `[x, y]`.
fn point(args: Args<'_>, key: &str) -> Result<Point, ProbeError> {
    match args.f64_vec(key)?[..] {
        [x, y] => Ok(Point::new(x, y)),
        _ => Err(ProbeError::BadArgs(format!("{key} is not [x, y]"))),
    }
}

pub(super) fn axline(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        made(ax.set_xscale(args.opt_str("xscale")?.unwrap_or("linear")))?;
        made(ax.set_yscale(args.opt_str("yscale")?.unwrap_or("linear")))?;
        let through = match args.opt_f64("slope")? {
            Some(slope) => AxLineThrough::Slope(slope),
            None => AxLineThrough::Point(point(args, "xy2")?),
        };
        made(ax.axline(point(args, "xy1")?, through, LineProps::default()))?;
        made(fig.render_raster())?;
        Ok(Json::Bool(true))
    })
}

/// The colours of `vlines(0.5, 0, 1)` or `hlines(0.5, 0, 1)`.
fn segment_colors(args: Args<'_>, orientation: Orientation) -> Outcome {
    on_axes(args, |ax| {
        let (at, low, high) = (vec![0.5], vec![0.0], vec![1.0]);
        let lines = match orientation {
            Orientation::Vertical => made(ax.vlines(at, low, high))?,
            Orientation::Horizontal => made(ax.hlines(at, low, high))?,
        };
        Ok(rgba_list(&made(lines.colors())?.edges))
    })
}

pub(super) fn vlines(args: Args<'_>) -> Outcome {
    segment_colors(args, Orientation::Vertical)
}

pub(super) fn hlines(args: Args<'_>) -> Outcome {
    segment_colors(args, Orientation::Horizontal)
}

/// `[faces, edges]` of a filled area.
fn fill_colors(fill: &Collection) -> Result<Json, ProbeError> {
    let colors = made(fill.colors())?;
    Ok(Json::Array(vec![
        rgba_list(&colors.faces),
        rgba_list(&colors.edges),
    ]))
}

/// `fill_between(range(3), range(3))` (or `fill_betweenx`) with `props`.
fn fill(ax: &Axes, horizontal: bool, props: CollectionProps) -> Result<Collection, ProbeError> {
    let t = vec![0.0, 1.0, 2.0];
    let options = FillBetweenOptions {
        props,
        ..FillBetweenOptions::default()
    };
    made(if horizontal {
        ax.fill_betweenx(t.clone(), t, vec![0.0; 3], options)
    } else {
        ax.fill_between(t.clone(), t, vec![0.0; 3], options)
    })
}

pub(super) fn fill_between_cycle(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let black = || Color::from("k");
        let mut fills = Vec::new();
        for horizontal in [false, false, false, true, true, true] {
            fills.push(fill(ax, horizontal, CollectionProps::default())?);
        }
        let explicit = [
            CollectionProps {
                facecolors: Some(vec![black()]),
                ..CollectionProps::default()
            },
            CollectionProps::default().color(black()),
            CollectionProps {
                edgecolors: Some(EdgeColors::Colors(vec![black()])),
                ..CollectionProps::default()
            },
            CollectionProps {
                edgecolors: Some(EdgeColors::Colors(vec![black()])),
                ..CollectionProps::default()
            },
        ];
        for props in explicit {
            fills.push(fill(ax, false, props)?);
        }
        fills
            .iter()
            .map(fill_colors)
            .collect::<Result<Vec<_>, _>>()
            .map(Json::Array)
    })
}
