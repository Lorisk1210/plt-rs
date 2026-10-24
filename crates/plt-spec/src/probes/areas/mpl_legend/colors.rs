//! Label colours: the colours of the labels of legends of lines and
//! scatters with a `labelcolor` rule, and for `labelcolor="linecolor"` the
//! colours of the last label, its proxy and its artist.

use plt::artist::{ArtistHandle, RawHandle};
use plt::axes::{Axes, HistArtists, HistPlotOptions, MappedColors, ScatterColors, ScatterOptions};
use plt::collections::{Collection, EdgeColors};
use plt::colors::{Color, Colorize, colormaps};
use plt::figure::Figure;
use plt::legend::{Legend, LegendLabelColor, LegendOptions};
use plt::lines::{Line2D, LineProps};
use plt::patches::{Patch, PatchShape, Rectangle};
use plt::rc::{ColorOrAuto, LabelColor};
use plt::stats::HistType;
use plt_core::Rgba;

use super::super::mpl_tests::in_test_style;
use super::maybe_rgba;
use crate::json::Json;
use crate::probes::encode::numbers;
use crate::probes::{Args, Outcome, ProbeError};

/// The keyword arguments of one plotting call.
type Keywords = [(String, Json)];

/// The plotting calls of a probe: one keyword dict each.
fn calls(args: Args<'_>) -> Result<Vec<&Keywords>, ProbeError> {
    args.json("calls")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("calls is not a list".into()))?
        .iter()
        .map(|call| {
            call.as_object()
                .ok_or_else(|| ProbeError::BadArgs(format!("{call} is not a dict")))
        })
        .collect()
}

fn bad(key: &str, value: &Json) -> ProbeError {
    ProbeError::BadArgs(format!("unsupported {key}={value}"))
}

/// A colour: a name or an RGB(A) list.
fn color(value: &Json) -> Result<Color, ProbeError> {
    if let Some(name) = value.as_str() {
        return Ok(Color::from(name));
    }
    match value.as_f64_vec().as_deref() {
        Some(&[r, g, b]) => Ok(Color::from((r, g, b))),
        Some(&[r, g, b, a]) => Ok(Color::from((r, g, b, a))),
        _ => Err(bad("color", value)),
    }
}

/// A colour or a list of colours (a numpy array of them).
fn colors(value: &Json) -> Result<Vec<Color>, ProbeError> {
    match value {
        Json::Array(items) => items.iter().map(color).collect(),
        single => Ok(vec![color(single)?]),
    }
}

/// `scatter`'s `c`: a colour, values to colour-map or a colour per point.
fn scatter_colors(value: &Json) -> Result<ScatterColors, ProbeError> {
    if value.as_str().is_some() {
        return Ok(ScatterColors::Single(color(value)?));
    }
    Ok(match value.as_f64_vec() {
        Some(values) => MappedColors::new(values).into(),
        None => ScatterColors::PerPoint(colors(value)?),
    })
}

/// `plot`'s format string and keyword arguments as line properties.
fn line_props(kwargs: &Keywords) -> Result<LineProps, ProbeError> {
    let mut props = LineProps::default();
    for (key, value) in kwargs {
        props = match key.as_str() {
            "fmt" => {
                let fmt = value.as_str().ok_or_else(|| bad(key, value))?;
                LineProps::fmt(fmt)
                    .map_err(ProbeError::rejected)?
                    .overlay(props)
            }
            "label" => props.label(value.as_str().ok_or_else(|| bad(key, value))?),
            "color" | "c" => props.color(color(value)?),
            "markeredgecolor" | "mec" => props.markeredgecolor(ColorOrAuto::Color(color(value)?)),
            "markerfacecolor" | "mfc" => props.markerfacecolor(ColorOrAuto::Color(color(value)?)),
            _ => return Err(bad(key, value)),
        };
    }
    Ok(props)
}

/// `scatter`'s keyword arguments, the label last.
fn scatter_options(kwargs: &Keywords) -> Result<(ScatterOptions, Option<&str>), ProbeError> {
    let mut options = ScatterOptions::default();
    let (mut label, mut cmap) = (None, None);
    for (key, value) in kwargs {
        match key.as_str() {
            "label" => label = Some(value.as_str().ok_or_else(|| bad(key, value))?),
            "c" | "facecolor" | "fc" => options.colors = scatter_colors(value)?,
            "edgecolor" | "ec" => options.edgecolors = Some(EdgeColors::Colors(colors(value)?)),
            "cmap" => {
                let name = value.as_str().ok_or_else(|| bad(key, value))?;
                cmap = Some(colormaps::get(name).map_err(ProbeError::rejected)?);
            }
            _ => return Err(bad(key, value)),
        }
    }
    if let Some(cmap) = cmap {
        options.colors = match std::mem::take(&mut options.colors) {
            ScatterColors::Mapped(mapped) => mapped.cmap(cmap).into(),
            _ => return Err(ProbeError::BadArgs("cmap without values to map".into())),
        };
    }
    Ok((options, label))
}

/// A scatter of `x` against `y` with `kwargs`.
fn scatter(
    ax: &Axes,
    x: Vec<f64>,
    y: Vec<f64>,
    kwargs: &Keywords,
) -> Result<Collection, ProbeError> {
    let (options, label) = scatter_options(kwargs)?;
    let collection = ax
        .scatter_with(x, y, options)
        .map_err(ProbeError::rejected)?;
    if let Some(label) = label {
        collection.set_label(label);
    }
    Ok(collection)
}

/// `[r, g, b, a]` of a colour.
fn resolved(color: &Color) -> Result<Rgba, ProbeError> {
    color.resolve().map_err(ProbeError::rejected)
}

/// The legends's `labelcolor` argument.
fn label_color(value: &Json) -> Result<LegendLabelColor, ProbeError> {
    value
        .as_str()
        .map(LegendLabelColor::from)
        .ok_or_else(|| bad("labelcolor", value))
}

/// The legend of `ax`, with `labelcolor` if given.
fn legend(ax: &Axes, labelcolor: Option<LegendLabelColor>) -> Result<Legend, ProbeError> {
    let options = LegendOptions {
        labelcolor,
        ..LegendOptions::default()
    };
    ax.legend(options).map_err(ProbeError::rejected)
}

/// The label colours of a legend of one `plot` or `scatter` of
/// `arange(10)` against `k * arange(10)` per keyword dict.
pub(super) fn label_colors(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        if let Some(value) = args.get("rc_labelcolor") {
            let labelcolor = LabelColor::Color(color(value)?);
            plt::rc::update(|params| params.legend_labelcolor = labelcolor);
        }
        let ax = Figure::new().subplot();
        let x: Vec<f64> = (0..10).map(f64::from).collect();
        for (k, kwargs) in (1..).zip(calls(args)?) {
            let y: Vec<f64> = x.iter().map(|v| v * f64::from(k)).collect();
            match args.str("kind")? {
                "plot" => {
                    ax.plot_with(x.clone(), y, line_props(kwargs)?)
                        .map_err(ProbeError::rejected)?;
                }
                "scatter" => {
                    scatter(&ax, x.clone(), y, kwargs)?;
                }
                other => return Err(ProbeError::BadArgs(format!("unknown kind {other}"))),
            }
        }
        let labelcolor = args.get("labelcolor").map(label_color).transpose()?;
        let legend = legend(&ax, labelcolor)?;
        legend
            .texts()
            .iter()
            .map(|text| resolved(&text.color()).map(|c| numbers(c.to_array())))
            .collect::<Result<_, _>>()
            .map(Json::Array)
    })
}

/// The colour, marker face colour and marker edge colour of a line.
fn line_colors(line: &Line2D) -> Result<Json, ProbeError> {
    [
        line.color(),
        line.effective_markerfacecolor(),
        line.effective_markeredgecolor(),
    ]
    .iter()
    .map(|c| resolved(c).map(|rgba| maybe_rgba(Some(rgba))))
    .collect::<Result<_, _>>()
    .map(Json::Array)
}

/// The face and edge colours of a patch.
fn patch_colors<S: PatchShape>(patch: &Patch<S>) -> Result<Json, ProbeError> {
    let colors = patch.colors().map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![
        maybe_rgba(Some(colors.face)),
        maybe_rgba(Some(colors.edge)),
    ]))
}

/// The first face and edge colours of a collection.
fn collection_colors(collection: &Collection) -> Result<Json, ProbeError> {
    let colors = collection.colors().map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![
        maybe_rgba(colors.faces.first().copied()),
        maybe_rgba(colors.edges.first().copied()),
    ]))
}

/// `hist(arange(10), **kwargs)`: the colours of its first patch.
fn hist(ax: &Axes, kwargs: &Keywords) -> Result<Json, ProbeError> {
    let mut options = HistPlotOptions::default();
    for (key, value) in kwargs {
        match key.as_str() {
            "label" => {
                options.label = vec![value.as_str().ok_or_else(|| bad(key, value))?.to_owned()];
            }
            "color" => options.color = vec![color(value)?],
            "fc" => options.props.facecolor = Some(color(value)?),
            "ec" => options.props.edgecolor = Some(color(value)?),
            "histtype" => {
                options.layout.histtype = match value.as_str() {
                    Some("bar") => HistType::Bar,
                    Some("step") => HistType::Step,
                    Some("stepfilled") => HistType::StepFilled,
                    _ => return Err(bad(key, value)),
                };
            }
            _ => return Err(bad(key, value)),
        }
    }
    let data: Vec<f64> = (0..10).map(f64::from).collect();
    let plot = ax.hist(&[data], options).map_err(ProbeError::rejected)?;
    match &plot.artists {
        HistArtists::Bars(containers) => containers
            .last()
            .and_then(|bars| bars.patches.first())
            .map(patch_colors),
        HistArtists::Steps(outlines) => outlines.last().map(patch_colors),
    }
    .ok_or_else(|| ProbeError::rejected("the histogram drew no patch"))?
}

/// For every plotting call added to one axes, after
/// `legend(labelcolor="linecolor")`: the colour of the last label and the
/// colours of its proxy and of the artist.
pub(super) fn last_entry_colors(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let ax = Figure::new().subplot();
        let x: Vec<f64> = (0..5).map(f64::from).collect();
        let kind = args.str("kind")?;
        let mut out = Vec::new();
        for kwargs in calls(args)? {
            let artist = match kind {
                "hist" => hist(&ax, kwargs)?,
                "plot" => {
                    let line = ax
                        .plot_with(x.clone(), x.clone(), line_props(kwargs)?)
                        .map_err(ProbeError::rejected)?;
                    line_colors(&line)?
                }
                "scatter" => collection_colors(&scatter(&ax, x.clone(), x.clone(), kwargs)?)?,
                other => return Err(ProbeError::BadArgs(format!("unknown kind {other}"))),
            };
            let legend = legend(&ax, Some(LegendLabelColor::from("linecolor")))?;
            let label = legend
                .texts()
                .last()
                .map(|text| resolved(&text.color()))
                .transpose()?;
            let proxy = match kind {
                "hist" => legend
                    .patches()
                    .last()
                    .and_then(RawHandle::downcast::<Patch<Rectangle>>)
                    .map(|patch| patch_colors(&patch)),
                "plot" => legend.lines().last().map(line_colors),
                _ => legend
                    .handles()
                    .last()
                    .and_then(Option::as_ref)
                    .and_then(RawHandle::downcast::<Collection>)
                    .map(|collection| collection_colors(&collection)),
            }
            .ok_or_else(|| ProbeError::rejected("the legend has no proxy of that kind"))??;
            out.push(Json::Array(vec![maybe_rgba(label), proxy, artist]));
        }
        Ok(Json::Array(out))
    })
}
