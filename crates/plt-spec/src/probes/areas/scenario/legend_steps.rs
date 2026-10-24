//! Steps making and querying legends: `Axes.legend`, `Figure.legend`,
//! `get_legend_handles_labels` and the methods of a legend.

use plt::artist::ArtistHandle;
use plt::axes::Axes;
use plt::colors::Color;
use plt::legend::{Legend, LegendHandle, LegendLabelColor, LegendLoc, LegendOptions};

use super::axes_limits::bbox;
use super::call::{Call, KEYWORD_ONLY};
use super::scene::{Handled, Object, Produced, Scene};
use crate::json::Json;
use crate::probes::ProbeError;
use crate::probes::encode::resolved_rgba;

/// Runs a legend method of `ax`.
pub(super) fn axes_call(scene: &Scene, ax: &Axes, call: &Call<'_>) -> Handled {
    Some(match call.method {
        "legend" => options(scene, call).and_then(|options| {
            ax.legend(options)
                .map(|legend| Produced::Object(Object::Legend(legend)))
                .map_err(ProbeError::rejected)
        }),
        "get_legend_handles_labels" => {
            let (handles, labels) = ax.legend_handles_labels();
            references(scene, &handles)
                .map(|handles| Produced::Value(Json::Array(vec![handles, Json::from(labels)])))
        }
        "get_legend" => Ok(match ax.current_legend() {
            Some(legend) => Produced::Object(Object::Legend(legend)),
            None => Produced::Value(Json::Null),
        }),
        "get_legend.get_texts" => ax
            .current_legend()
            .ok_or_else(|| ProbeError::rejected("the axes has no legend"))
            .map(|legend| Produced::Value(Json::from(legend.labels()))),
        _ => return None,
    })
}

/// Runs a legend method of the figure.
pub(super) fn figure_call(scene: &Scene, call: &Call<'_>) -> Handled {
    Some(match call.method {
        "legend" => options(scene, call).and_then(|options| {
            scene
                .fig
                .legend(options)
                .map(|legend| Produced::Object(Object::Legend(legend)))
                .map_err(ProbeError::rejected)
        }),
        "legends" => scene
            .fig
            .legends()
            .into_iter()
            .map(|legend| scene.reference_of(&Object::Legend(legend)))
            .collect::<Result<_, _>>()
            .map(|legends| Produced::Value(Json::Array(legends))),
        _ => return None,
    })
}

/// Runs a method of a legend.
pub(super) fn legend_call(legend: &Legend, call: &Call<'_>) -> Handled {
    Some(match call.method {
        "get_texts" => Ok(Produced::Value(Json::from(legend.labels()))),
        "get_texts.get_color" => legend
            .texts()
            .iter()
            .map(|text| resolved_rgba(&text.color()))
            .collect::<Result<_, _>>()
            .map(|colors| Produced::Value(Json::Array(colors))),
        "get_title" => Ok(Produced::Value(legend.title().text().into())),
        "get_title.get_visible" => Ok(Produced::Value(legend.title().visible().into())),
        "set_loc" => call.required(0, "loc").and_then(location).and_then(|loc| {
            legend.set_loc(loc).map_err(ProbeError::rejected)?;
            Ok(Produced::Nothing)
        }),
        "get_window_extent" => legend
            .window_extent()
            .map(|extent| Produced::Value(bbox(extent)))
            .ok_or_else(|| ProbeError::rejected("the legend was not laid out")),
        "remove" => {
            legend.remove();
            Ok(Produced::Nothing)
        }
        _ => return None,
    })
}

/// The handles as the references of the steps that made them.
fn references(scene: &Scene, handles: &[LegendHandle]) -> Result<Json, ProbeError> {
    handles
        .iter()
        .map(|handle| scene.reference_of_handle(handle))
        .collect::<Result<_, _>>()
        .map(Json::Array)
}

/// matplotlib's `legend(handles, labels)`, `legend(labels)` and
/// `legend(handles=..., labels=..., loc=..., reverse=..., labelcolor=...)`.
fn options(scene: &Scene, call: &Call<'_>) -> Result<LegendOptions, ProbeError> {
    let (handles, labels) = match call.arity() {
        0 => (call.keyword("handles"), call.keyword("labels")),
        1 => (None, call.arg(0, "labels")),
        2 => (call.arg(0, "handles"), call.arg(1, "labels")),
        _ => return Err(call.bad("unsupported legend arguments")),
    };
    let mut options = LegendOptions::default();
    if let Some(handles) = handles {
        let handles = handles
            .as_array()
            .ok_or_else(|| call.bad("handles are a list"))?
            .iter()
            .map(|handle| scene.legend_handle(handle))
            .collect::<Result<Vec<_>, _>>()?;
        options = options.handles(handles);
    }
    if let Some(labels) = labels {
        options = options.labels(strings(labels).ok_or_else(|| call.bad("labels are strings"))?);
    }
    if let Some(loc) = call.keyword("loc") {
        options.loc = Some(location(loc)?);
    }
    options.reverse = call.opt_bool(KEYWORD_ONLY, "reverse")?.unwrap_or(false);
    options.labelcolor = call.keyword("labelcolor").map(label_color).transpose()?;
    Ok(options)
}

fn strings(value: &Json) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}

/// A `loc`: a name, a code or a point; anything else is rejected like
/// matplotlib rejects it.
fn location(value: &Json) -> Result<LegendLoc, ProbeError> {
    match value {
        Json::String(name) => Ok(LegendLoc::Name(name.clone())),
        Json::Integer(code) => Ok(LegendLoc::Code(*code)),
        Json::Array(point) => match point.as_slice() {
            [x, y] => match (x.as_f64(), y.as_f64()) {
                (Some(x), Some(y)) => Ok(LegendLoc::Point(x, y)),
                _ => Err(ProbeError::rejected("a legend point has two numbers")),
            },
            _ => Err(ProbeError::rejected("a legend point has two numbers")),
        },
        other => Err(ProbeError::rejected(format!("{other} is not a location"))),
    }
}

/// A `labelcolor`: a rule or colour name, an RGB list, or a list of
/// colour names.
fn label_color(value: &Json) -> Result<LegendLabelColor, ProbeError> {
    if let Some(spec) = value.as_str() {
        return Ok(LegendLabelColor::from(spec));
    }
    match value.as_f64_vec().as_deref() {
        Some(&[r, g, b]) => return Ok(LegendLabelColor::from(Color::from((r, g, b)))),
        Some(&[r, g, b, a]) => return Ok(LegendLabelColor::from(Color::from((r, g, b, a)))),
        _ => {}
    }
    strings(value)
        .map(|names| LegendLabelColor::Cycle(names.into_iter().map(Color::from).collect()))
        .ok_or_else(|| ProbeError::BadArgs(format!("bad labelcolor {value}")))
}
