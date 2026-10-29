//! Lines: format strings and plot arguments, infinite lines, `vlines` and
//! `hlines` colours, the colour cycle of filled areas and the fill colours
//! of stack plots.

use pltrs::axes::{Axes, FillBetweenOptions, StackplotOptions};
use pltrs::collections::{Collection, CollectionProps, EdgeColors};
use pltrs::colors::Color;
use pltrs::figure::Figure;
use pltrs::lines::{AxLineThrough, Line2D, LineProps};
use pltrs::numeric::linspace;
use pltrs::orientation::Orientation;
use pltrs::units::AxisValues;
use pltrs_core::Point;

use super::{color, colors, made, on_axes, optional, rgba_list};
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

pub(super) fn stackplot(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let x = linspace(0.0, 10.0, 10);
        let layers = [
            x.iter().map(|v| 1.0 * v).collect(),
            x.iter().map(|v| 2.0 * v + 1.0).collect(),
        ];
        // matplotlib's `facecolor` and its plural alias are one option.
        let facecolors = match optional(args, "facecolor", colors)? {
            Some(given) => Some(given),
            None => optional(args, "facecolors", colors)?,
        };
        let options = StackplotOptions {
            facecolors: facecolors.unwrap_or_default(),
            colors: optional(args, "colors", colors)?.unwrap_or_default(),
            ..StackplotOptions::default()
        };
        let drawn = made(ax.stackplot(x.clone(), &layers, options))?;
        let faces = drawn
            .iter()
            .map(|layer| Ok(rgba_list(&made(layer.colors())?.faces)))
            .collect::<Result<Vec<_>, ProbeError>>()?;
        let next = made(ax.plot(x.clone(), x))?;
        Ok(encode::object([
            ("faces", Json::Array(faces)),
            ("next", encode::rgba(made(next.rgba())?)),
        ]))
    })
}
