//! Axes steps making labelled collections and texts: `scatter`, `hist`,
//! `fill_between` and `text`, with the keyword arguments legends read.

use plt::artist::ArtistHandle;
use plt::axes::{
    Axes, FillBetweenOptions, HistPlotOptions, MappedColors, ScatterColors, ScatterOptions,
};
use plt::collections::EdgeColors;
use plt::colors::Color;
use plt::stats::{Bins, HistType};
use plt::text::VAlign;

use super::axes_plot::{artist, xy};
use super::call::{Call, KEYWORD_ONLY};
use super::scene::{Handled, Object, Produced};
use crate::json::Json;
use crate::probes::ProbeError;

/// Runs a collection or text method of `ax`.
pub(super) fn call(ax: &Axes, call: &Call<'_>) -> Handled {
    Some(match call.method {
        "scatter" => scatter(ax, call),
        "hist" => hist(ax, call),
        "fill_between" => fill_between(ax, call),
        "text" => text(ax, call),
        _ => return None,
    })
}

/// The `label` keyword.
fn label<'a>(call: &Call<'a>) -> Result<Option<&'a str>, ProbeError> {
    call.opt_str(KEYWORD_ONLY, "label")
}

/// matplotlib's `scatter(x, y, c=..., edgecolor=..., label=...)`: `c` is
/// a colour, a list of colours or a list of values to colour-map.
fn scatter(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let (x, y) = xy(call)?;
    let colors = match call.keyword("c") {
        None => ScatterColors::Cycle,
        Some(Json::String(color)) => ScatterColors::Single(Color::from(color.as_str())),
        Some(values) => match values.as_f64_vec() {
            Some(values) => MappedColors::new(values).into(),
            None => ScatterColors::PerPoint(colors(values).ok_or_else(|| call.bad("bad c"))?),
        },
    };
    let edgecolors = call
        .opt_str(KEYWORD_ONLY, "edgecolor")?
        .map(|color| EdgeColors::Colors(vec![Color::from(color)]));
    let options = ScatterOptions {
        colors,
        edgecolors,
        ..ScatterOptions::default()
    };
    let collection = ax
        .scatter_with(x, y, options)
        .map_err(ProbeError::rejected)?;
    if let Some(label) = label(call)? {
        collection.set_label(label);
    }
    Ok(Produced::Object(Object::Collection(collection)))
}

fn colors(value: &Json) -> Option<Vec<Color>> {
    value
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(Color::from))
        .collect()
}

/// matplotlib's `hist(x, bins=..., histtype=..., label=...)` of one
/// dataset.
fn hist(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let x = call.f64_vec(0, "x")?;
    let mut options = HistPlotOptions::default();
    options.layout.bins = match call.keyword("bins") {
        None => None,
        Some(Json::Array(_)) => Some(Bins::Edges(call.f64_vec(KEYWORD_ONLY, "bins")?)),
        Some(count) => Some(Bins::Count(
            count
                .as_i64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| call.bad("bad bins"))?,
        )),
    };
    options.layout.histtype = match call.opt_str(KEYWORD_ONLY, "histtype")? {
        None | Some("bar") => HistType::Bar,
        Some("barstacked") => HistType::BarStacked,
        Some("step") => HistType::Step,
        Some("stepfilled") => HistType::StepFilled,
        Some(other) => return Err(ProbeError::rejected(format!("bad histtype {other:?}"))),
    };
    options.label = label(call)?.map(str::to_owned).into_iter().collect();
    ax.hist(&[x], options)
        .map(|_| Produced::Nothing)
        .map_err(ProbeError::rejected)
}

/// matplotlib's `fill_between(x, y1, y2, color=..., alpha=..., label=...)`.
fn fill_between(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let mut options = FillBetweenOptions::default();
    if let Some(color) = call.opt_str(KEYWORD_ONLY, "color")? {
        options.props = options.props.color(color);
    }
    options.props.base.alpha = call.opt_f64(KEYWORD_ONLY, "alpha")?;
    options.props.base.label = label(call)?.map(str::to_owned);
    let (x, y1, y2) = (
        call.f64_vec(0, "x")?,
        call.f64_vec(1, "y1")?,
        call.f64_vec(2, "y2")?,
    );
    ax.fill_between(x, y1, y2, options)
        .map(|collection| Produced::Object(Object::Collection(collection)))
        .map_err(ProbeError::rejected)
}

/// matplotlib's `text(x, y, s, verticalalignment=...)`.
fn text(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let text = ax.text(call.f64(0, "x")?, call.f64(1, "y")?, call.str(2, "s")?);
    let valign = match call.opt_str(KEYWORD_ONLY, "verticalalignment")? {
        None | Some("baseline") => VAlign::Baseline,
        Some("top") => VAlign::Top,
        Some("bottom") => VAlign::Bottom,
        Some("center") => VAlign::Center,
        Some("center_baseline") => VAlign::CenterBaseline,
        Some(other) => return Err(ProbeError::rejected(format!("bad alignment {other:?}"))),
    };
    text.set_valign(valign);
    Ok(artist(&text))
}
