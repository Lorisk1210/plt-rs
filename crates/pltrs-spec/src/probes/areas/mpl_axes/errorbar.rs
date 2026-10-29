//! Error bars: the data line, the bars' colours and styles, and argument
//! checks.

use pltrs::axes::{Axes, ErrorSize, ErrorbarOptions};
use pltrs::collections::Collection;
use pltrs::lines::{Line2D, LineProps, LineStyle, MarkEvery};
use pltrs::markers::Marker;
use pltrs::rc;
use pltrs_render::{CapStyle, JoinStyle};

use super::{count, made, numbers, on_axes};
use crate::json::Json;
use crate::probes::areas::canon::Canon;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// An error size: a number, one per point, or `[lower, upper]` lists.
fn error_size(value: &Json) -> Result<ErrorSize, ProbeError> {
    if let Some(error) = value.as_f64() {
        return Ok(error.into());
    }
    if let Some(errors) = value.as_f64_vec() {
        return Ok(errors.into());
    }
    match super::datasets(value)?.as_slice() {
        [lower, upper] => Ok(ErrorSize::Asymmetric {
            lower: lower.clone().into(),
            upper: upper.clone().into(),
        }),
        _ => Err(ProbeError::BadArgs(format!("bad error sizes {value}"))),
    }
}

/// `errorevery`: `[start, step]` integers or one flag per point.
fn error_every(value: &Json) -> Result<MarkEvery, ProbeError> {
    let items = value
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("bad errorevery {value}")))?;
    if let Some(flags) = items.iter().map(Json::as_bool).collect::<Option<Vec<_>>>() {
        return Ok(MarkEvery::Mask(flags));
    }
    match items {
        [Json::Integer(start), Json::Integer(step)] => Ok(MarkEvery::Every {
            start: *start,
            step: *step,
        }),
        _ => Err(ProbeError::BadArgs(format!("bad errorevery {value}"))),
    }
}

/// A cap style argument.
fn cap_style(args: Args<'_>, key: &str) -> Result<Option<CapStyle>, ProbeError> {
    args.opt_str(key)?
        .map(|name| CapStyle::from_name(name).ok_or_else(|| ProbeError::rejected(name)))
        .transpose()
}

/// A join style argument.
fn join_style(args: Args<'_>, key: &str) -> Result<Option<JoinStyle>, ProbeError> {
    args.opt_str(key)?
        .map(|name| JoinStyle::from_name(name).ok_or_else(|| ProbeError::rejected(name)))
        .transpose()
}

/// The data line's keywords the probes pass (`ls`, `marker`,
/// `fillstyle`, `drawstyle` and the cap and join styles).
fn line_props(args: Args<'_>) -> Result<LineProps, ProbeError> {
    let mut props = LineProps::default();
    if let Some(style) = args.opt_str("ls")? {
        props.linestyle = Some(made(style.parse::<LineStyle>())?);
    }
    if let Some(marker) = args.opt_str("marker")? {
        props.marker = Some(made(marker.parse::<Marker>())?);
    }
    if let Some(fill) = args.opt_str("fillstyle")? {
        props.fillstyle = Some(made(fill.parse())?);
    }
    if let Some(draw) = args.opt_str("drawstyle")? {
        props.drawstyle = Some(made(draw.parse())?);
    }
    props.dash_capstyle = cap_style(args, "dash_capstyle")?;
    props.dash_joinstyle = join_style(args, "dash_joinstyle")?;
    props.solid_capstyle = cap_style(args, "solid_capstyle")?;
    props.solid_joinstyle = join_style(args, "solid_joinstyle")?;
    Ok(props)
}

/// `{color, marker, linestyle, fillstyle, drawstyle}` of a line.
fn line_style(line: &Line2D) -> Result<Json, ProbeError> {
    Ok(encode::object([
        ("color", encode::rgba(made(line.rgba())?)),
        ("marker", line.marker().canon()),
        ("linestyle", line.linestyle().canon()),
        ("fillstyle", Json::from(line.fillstyle().name())),
        ("drawstyle", Json::from(line.drawstyle().to_string())),
    ]))
}

/// `{colors, linestyles}` of a bar collection; a line style is its
/// `[offset, dashes]` scaled by the line width, `dashes` `null` when
/// solid.
fn bar_style(bars: &Collection) -> Result<Json, ProbeError> {
    let rc = rc::params();
    let style = bars.style();
    let width = style
        .linewidths
        .as_ref()
        .and_then(|widths| widths.first().copied())
        .unwrap_or(rc.lines_linewidth);
    let linestyles = style.linestyles.iter().map(|linestyle| {
        let dashes = linestyle.scaled_dashes(width, &rc);
        let pattern = dashes.pattern.as_deref().map_or(Json::Null, numbers);
        Json::Array(vec![Json::from(dashes.offset), pattern])
    });
    Ok(encode::object([
        ("colors", super::rgba_list(&made(bars.colors())?.edges)),
        ("linestyles", Json::Array(linestyles.collect())),
    ]))
}

pub(super) fn errorbar(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = ErrorbarOptions {
            xerr: super::optional(args, "xerr", error_size)?,
            yerr: super::optional(args, "yerr", error_size)?,
            fmt: made(args.opt_str("fmt")?.unwrap_or_default().parse())?,
            line: line_props(args)?,
            elinewidth: args.opt_f64("elinewidth")?,
            elinestyle: args
                .opt_str("elinestyle")?
                .map(|style| made(style.parse()))
                .transpose()?,
            errorevery: super::optional(args, "errorevery", error_every)?,
            ..ErrorbarOptions::default()
        };
        let bars = made(ax.errorbar(args.f64_vec("x")?, args.f64_vec("y")?, options))?;
        let line = bars.data_line.as_ref().map(line_style).transpose()?;
        let barlines = bars
            .barlinecols
            .iter()
            .map(bar_style)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(encode::object([
            ("line", line.unwrap_or(Json::Null)),
            ("bars", Json::Array(barlines)),
            ("caps", count(bars.caplines.len())),
        ]))
    })
}

/// `[containers, lines, collections]`: the lengths of the axes' lists.
fn counts(ax: &Axes) -> Json {
    Json::Array(
        [
            ax.containers().len(),
            ax.lines().len(),
            ax.collections().len(),
        ]
        .map(count)
        .to_vec(),
    )
}

pub(super) fn remove(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = ErrorbarOptions {
            xerr: super::optional(args, "xerr", error_size)?,
            yerr: super::optional(args, "yerr", error_size)?,
            ..ErrorbarOptions::default()
        };
        let bars = made(ax.errorbar(args.f64_vec("x")?, args.f64_vec("y")?, options))?;
        let before = counts(ax);
        bars.remove();
        Ok(Json::Array(vec![before, counts(ax)]))
    })
}

pub(super) fn caps(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = ErrorbarOptions {
            yerr: Some(error_size(args.json("yerr")?)?),
            fmt: made(args.opt_str("fmt")?.unwrap_or_default().parse())?,
            ecolor: super::optional(args, "ecolor", super::color)?,
            ..ErrorbarOptions::default()
        };
        let bars = made(ax.errorbar(args.f64_vec("x")?, args.f64_vec("y")?, options))?;
        let caps = bars
            .caplines
            .iter()
            .map(|cap| {
                Ok(Json::Array(vec![
                    encode::rgba(made(cap.effective_markeredgecolor().resolve())?),
                    Json::from(cap.markersize()),
                    Json::from(cap.markeredgewidth()),
                ]))
            })
            .collect::<Result<Vec<_>, ProbeError>>()?;
        let barlines = bars.barlinecols.iter().map(|bar| numbers(bar.linewidths()));
        Ok(encode::object([
            ("line", Json::from(bars.data_line.is_some())),
            ("caps", Json::Array(caps)),
            ("barlines", Json::Array(barlines.collect())),
        ]))
    })
}

pub(super) fn empty_legend(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        for (label, horizontal) in [("empty y", true), ("empty x", false)] {
            let empty = Some(ErrorSize::from(Vec::<f64>::new()));
            let (xerr, yerr) = if horizontal {
                (empty, None)
            } else {
                (None, empty)
            };
            let options = ErrorbarOptions {
                xerr,
                yerr,
                line: LineProps::default().label(label),
                ..ErrorbarOptions::default()
            };
            made(ax.errorbar(Vec::<f64>::new(), Vec::<f64>::new(), options))?;
        }
        let legend = made(ax.legend(pltrs::legend::LegendOptions::default()))?;
        Ok(Json::Array(
            legend.labels().into_iter().map(Json::from).collect(),
        ))
    })
}

pub(super) fn color_cycle(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let x = args.f64_vec("x")?;
        let y: Vec<f64> = x.iter().map(|v| 2.0 * v).collect();
        let scaled = |factor: f64| y.iter().map(|v| factor * v).collect::<Vec<f64>>();
        let mut colors = Vec::new();
        for factor in [1.0, 2.0] {
            let bars = made(ax.errorbar(x.clone(), scaled(factor), ErrorbarOptions::default()))?;
            let line = bars
                .data_line
                .ok_or_else(|| ProbeError::BadArgs("no data line".into()))?;
            colors.push(encode::rgba(made(line.rgba())?));
        }
        let line = made(ax.plot(x.clone(), scaled(4.0)))?;
        colors.push(encode::rgba(made(line.rgba())?));
        Ok(Json::Array(colors))
    })
}
