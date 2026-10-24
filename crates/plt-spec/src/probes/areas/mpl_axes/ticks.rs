//! Tick parameters and what the ticks show, gridlines, tick labels on
//! fixed locations, the offset text and the axis labels.

use plt::axes::{
    Axes, Axis, AxisName, GridOptions, TickDirection, TickLevel, TickParams, TickStyle,
};
use plt::colors::Color;
use plt::figure::Figure;
use plt::fonts::FontSize;
use plt::rc;
use plt::units::AxisValues;

use super::{color, made, on_axes, strings};
use crate::json::Json;
use crate::probes::areas::canon::Canon;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The name of a tick direction.
fn direction_name(direction: TickDirection) -> &'static str {
    match direction {
        TickDirection::In => "in",
        TickDirection::Out => "out",
        TickDirection::InOut => "inout",
    }
}

/// A colour as RGBA JSON.
fn rgba(color: &Color) -> Result<Json, ProbeError> {
    Ok(encode::rgba(made(color.resolve())?))
}

/// What the ticks of one level show and how (matplotlib's state of its
/// first `Tick`).
fn tick_state(style: &TickStyle) -> Result<Json, ProbeError> {
    let family = style.label_family.clone().unwrap_or_else(|| {
        rc::params()
            .font_family
            .first()
            .cloned()
            .unwrap_or_default()
    });
    let grid = &style.grid;
    Ok(encode::object([
        ("size", Json::from(style.size)),
        ("width", Json::from(style.width)),
        ("pad", Json::from(style.pad)),
        ("direction", Json::from(direction_name(style.direction))),
        ("color", rgba(&style.color)?),
        (
            "markeredgecolor",
            rgba(style.edgecolor.as_ref().unwrap_or(&style.color))?,
        ),
        ("labelsize", Json::from(style.label_size)),
        ("labelcolor", rgba(&style.label_color)?),
        ("labelrotation", Json::from(style.label_rotation)),
        ("rotation_mode", style.label_rotation_mode.canon()),
        ("labelfamily", Json::from(family)),
        ("tick1On", Json::from(style.ticks.first)),
        ("tick2On", Json::from(style.ticks.second)),
        ("label1On", Json::from(style.labels.first)),
        ("label2On", Json::from(style.labels.second)),
        ("gridOn", Json::from(style.grid_on)),
        (
            "grid",
            Json::Array(vec![
                rgba(&grid.color)?,
                Json::from(grid.linewidth),
                Json::from(grid.alpha),
                grid.linestyle.canon(),
            ]),
        ),
    ]))
}

/// The tick states of both levels of both axes.
fn ticks_state(ax: &Axes) -> Outcome {
    let mut entries = Vec::new();
    for (name, axis) in [("x", ax.xaxis()), ("y", ax.yaxis())] {
        for (level, minor) in [("major", false), ("minor", true)] {
            entries.push((
                format!("{name}{level}"),
                tick_state(&axis.tick_style(minor))?,
            ));
        }
    }
    Ok(Json::Object(entries))
}

/// A flag of a tick parameter object.
fn flag(params: &Json, key: &str) -> Result<Option<bool>, ProbeError> {
    params
        .get(key)
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a flag")))
        })
        .transpose()
}

/// A number of a tick parameter object.
fn number(params: &Json, key: &str) -> Result<Option<f64>, ProbeError> {
    params
        .get(key)
        .map(|value| {
            value
                .as_f64()
                .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a number")))
        })
        .transpose()
}

/// A name of a tick parameter object.
fn name<'a>(params: &'a Json, key: &str) -> Option<&'a str> {
    params.get(key).and_then(Json::as_str)
}

/// The keywords of one `tick_params(...)` call.
fn tick_params(params: &Json) -> Result<TickParams, ProbeError> {
    let parsed = |key: &str| name(params, key).map(str::to_owned);
    Ok(TickParams {
        axis: match name(params, "axis") {
            None | Some("both") => None,
            Some("x") => Some(AxisName::X),
            Some("y") => Some(AxisName::Y),
            Some(other) => return Err(ProbeError::BadArgs(format!("bad axis {other}"))),
        },
        which: match name(params, "which").unwrap_or("major") {
            "major" => TickLevel::Major,
            "minor" => TickLevel::Minor,
            "both" => TickLevel::Both,
            other => return Err(ProbeError::BadArgs(format!("bad which {other}"))),
        },
        reset: flag(params, "reset")?.unwrap_or(false),
        direction: parsed("direction")
            .map(|d| made(d.parse::<TickDirection>()))
            .transpose()?,
        color: params.get("color").map(color).transpose()?,
        labelcolor: params.get("labelcolor").map(color).transpose()?,
        labelsize: match params.get("labelsize") {
            None => None,
            Some(Json::String(size)) => Some(made(size.parse::<FontSize>())?),
            Some(size) => {
                Some(FontSize::Points(size.as_f64().ok_or_else(|| {
                    ProbeError::BadArgs("bad labelsize".into())
                })?))
            }
        },
        labelrotation: number(params, "labelrotation")?,
        labelrotation_mode: parsed("labelrotation_mode")
            .map(|mode| made(mode.parse()))
            .transpose()?,
        labelfontfamily: parsed("labelfontfamily"),
        bottom: flag(params, "bottom")?,
        top: flag(params, "top")?,
        left: flag(params, "left")?,
        right: flag(params, "right")?,
        labelbottom: flag(params, "labelbottom")?,
        labeltop: flag(params, "labeltop")?,
        labelleft: flag(params, "labelleft")?,
        labelright: flag(params, "labelright")?,
        grid_color: params.get("grid_color").map(color).transpose()?,
        grid_alpha: number(params, "grid_alpha")?,
        grid_linewidth: number(params, "grid_linewidth")?,
        grid_linestyle: parsed("grid_linestyle")
            .map(|style| made(style.parse()))
            .transpose()?,
        ..TickParams::default()
    })
}

pub(super) fn tick_params_state(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        for call in args.get("calls").and_then(Json::as_array).unwrap_or(&[]) {
            ax.tick_params(&tick_params(call)?);
        }
        if args.opt_bool("draw")? == Some(true) {
            fig.layout();
        }
        ticks_state(&ax)
    })
}

pub(super) fn rc_context_ticks(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = rc::context(|| {
            for (key, value) in [
                ("xtick.direction", "in"),
                ("xtick.major.size", "30"),
                ("xtick.major.width", "5"),
                ("xtick.color", "C0"),
                ("xtick.major.pad", "12"),
                ("xtick.bottom", "True"),
                ("xtick.top", "True"),
                ("xtick.labelsize", "14"),
                ("xtick.labelcolor", "C1"),
            ] {
                made(rc::set(key, value))?;
            }
            let fig = Figure::new();
            let ax = fig.subplot();
            Ok::<_, ProbeError>((fig, ax))
        })?;
        let (fig, ax) = fig;
        fig.layout();
        // Every major tick shares the level's style.
        Ok(encode::object([
            ("tick", tick_state(&ax.xaxis().tick_style(false))?),
            ("same", Json::Bool(true)),
        ]))
    })
}

pub(super) fn grid(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let [r, g, b, a] = args.f64_vec("color")?[..] else {
            return Err(ProbeError::BadArgs("color is not RGBA".into()));
        };
        let options = GridOptions {
            color: Some(Color::from([r, g, b, a])),
            ..GridOptions::default()
        };
        ax.grid_with(true, options);
        [ax.xaxis(), ax.yaxis()]
            .iter()
            .map(|axis| {
                let grid = axis.tick_style(false).grid;
                Ok(Json::Array(vec![
                    Json::from(grid.alpha),
                    encode::rgba(made(grid.color.resolve())?),
                ]))
            })
            .collect::<Result<Vec<_>, ProbeError>>()
            .map(Json::Array)
    })
}

/// `tick_params(reset=True, which="major", labelsize=10)`.
fn reset_ticks(ax: &Axes) {
    ax.tick_params(&TickParams {
        reset: true,
        labelsize: Some(FontSize::Points(10.0)),
        ..TickParams::default()
    });
}

pub(super) fn reset_grid(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        reset_ticks(ax);
        let before = ax.has_grid(AxisName::X, true);
        let after = rc::context(|| {
            made(rc::set("axes.grid", "True"))?;
            ax.clear();
            reset_ticks(ax);
            Ok::<_, ProbeError>(ax.has_grid(AxisName::X, true))
        })?;
        Ok(Json::Array(vec![Json::from(before), Json::from(after)]))
    })
}

/// The axis named by argument `axis`.
fn axis_arg(ax: &Axes, args: Args<'_>) -> Result<Axis, ProbeError> {
    match args.str("axis")? {
        "x" => Ok(ax.xaxis()),
        "y" => Ok(ax.yaxis()),
        other => Err(ProbeError::BadArgs(format!("unknown axis {other}"))),
    }
}

pub(super) fn offset_text(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        made(ax.plot_with(
            AxisValues::Numbers(vec![0.0, 1.0, 2.0]),
            args.f64_vec("data")?,
            plt::lines::LineProps::default(),
        ))?;
        let axis = axis_arg(ax, args)?;
        if let Some(params) = args.get("params") {
            axis.set_tick_params(&tick_params(params)?);
        }
        // The offset text takes the colour of the major tick labels.
        Ok(encode::object([
            ("color", rgba(&axis.tick_style(false).label_color)?),
            ("visible", Json::from(axis.offset_text_visible())),
        ]))
    })
}

pub(super) fn set_ticklabels(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        if args.opt_bool("bar")? == Some(true) {
            made(ax.bar(
                vec![1.0, 2.0],
                vec![1.0, 2.0],
                plt::axes::BarOptions::default(),
            ))?;
        }
        ax.set_xticks(args.f64_vec("ticks")?);
        made(ax.set_xticklabels(strings(args.json("labels")?)?))?;
        fig.layout();
        Ok(Json::from(made(ax.xaxis().ticklabels(false))?))
    })
}

/// The tick parameters matplotlib's `get_tick_params(which=...)` lists
/// for the y axis: the sides, the gridlines and, once set, the direction
/// and the label size and colour.
fn y_tick_params(style: &TickStyle, set: bool) -> Result<Json, ProbeError> {
    let mut entries = Vec::new();
    if set {
        entries.push((
            "direction".to_owned(),
            Json::from(direction_name(style.direction)),
        ));
    }
    entries.extend([
        ("left".to_owned(), Json::from(style.ticks.first)),
        ("right".to_owned(), Json::from(style.ticks.second)),
        ("labelleft".to_owned(), Json::from(style.labels.first)),
        ("labelright".to_owned(), Json::from(style.labels.second)),
        ("gridOn".to_owned(), Json::from(style.grid_on)),
    ]);
    if set {
        entries.push(("labelsize".to_owned(), Json::from(style.label_size)));
        entries.push(("labelcolor".to_owned(), rgba(&style.label_color)?));
    }
    Ok(Json::Object(entries))
}

pub(super) fn get_tick_params(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let axis = ax.yaxis();
        let levels = |set: bool| -> Result<Json, ProbeError> {
            Ok(encode::object([
                ("major", y_tick_params(&axis.tick_style(false), set)?),
                ("minor", y_tick_params(&axis.tick_style(true), set)?),
            ]))
        };
        let before = levels(false)?;
        axis.set_tick_params(&TickParams {
            which: TickLevel::Both,
            labelsize: Some(FontSize::Points(30.0)),
            labelcolor: Some(Color::from("red")),
            direction: Some(TickDirection::Out),
            ..TickParams::default()
        });
        Ok(encode::object([
            ("before", before),
            ("after", levels(true)?),
        ]))
    })
}

pub(super) fn label_formatting(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let axis = ax.xaxis();
        let font = axis.label_font();
        Ok(Json::Array(vec![
            rgba(&axis.label_color())?,
            Json::from(font.size),
            font.weight.canon(),
        ]))
    })
}
