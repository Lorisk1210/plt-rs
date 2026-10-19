//! Axes steps about axis labels and the sides ticks and tick labels are
//! drawn on: `label_outer`, `tick_left` and friends, `get_tick_params`.

use plt::axes::{Axes, AxisName, LabelLoc};
use plt::spines::SpineSide;

use super::call::{Call, KEYWORD_ONLY};
use super::scene::{Handled, Produced};
use crate::json::Json;
use crate::probes::ProbeError;

/// Runs a label method of `ax`.
pub(super) fn call(ax: &Axes, call: &Call<'_>) -> Handled {
    use AxisName::{X, Y};
    let method = call.method;
    let axis_name = if method.starts_with('x') || method.contains("_x") {
        X
    } else {
        Y
    };
    Some(match method {
        "set_xlabel" | "set_ylabel" => call.str(0, "label").and_then(|text| {
            if axis_name == X {
                ax.set_xlabel(text);
            } else {
                ax.set_ylabel(text);
            }
            if let Some(loc) = call.opt_str(KEYWORD_ONLY, "loc")? {
                ax.axis_of(axis_name)
                    .set_label_loc(label_loc(axis_name, loc)?);
            }
            Ok(Produced::Nothing)
        }),
        "get_xlabel" | "get_ylabel" => Ok(Produced::Value(ax.label(axis_name).into())),
        "xaxis.label.get_horizontalalignment" | "yaxis.label.get_horizontalalignment" => {
            let align = match ax.axis_of(axis_name).label_loc() {
                LabelLoc::Start => "left",
                LabelLoc::Center => "center",
                LabelLoc::End => "right",
            };
            Ok(Produced::Value(align.into()))
        }
        "label_outer" => call
            .opt_bool(KEYWORD_ONLY, "remove_inner_ticks")
            .map(|remove| {
                ax.label_outer_with(remove.unwrap_or(false));
                Produced::Nothing
            }),
        "xaxis.set_label_position" | "yaxis.set_label_position" => {
            call.str(0, "position").and_then(|name| {
                ax.axis_of(axis_name)
                    .set_label_position(side(name)?)
                    .map_err(ProbeError::rejected)?;
                Ok(Produced::Nothing)
            })
        }
        "xaxis.tick_bottom" | "xaxis.tick_top" | "yaxis.tick_left" | "yaxis.tick_right" => {
            let (_, name) = method.split_once(".tick_").unwrap_or_default();
            side(name).and_then(|side| {
                ax.axis_of(axis_name)
                    .tick_to(side)
                    .map_err(ProbeError::rejected)?;
                Ok(Produced::Nothing)
            })
        }
        "xaxis.get_tick_params" | "yaxis.get_tick_params" => call
            .opt_str(KEYWORD_ONLY, "which")
            .and_then(|which| match which {
                None | Some("major") => Ok(tick_params(ax, axis_name, false)),
                Some("minor") => Ok(tick_params(ax, axis_name, true)),
                Some(other) => Err(ProbeError::rejected(format!("bad which {other:?}"))),
            }),
        "xaxis.offsetText.get_visible" | "yaxis.offsetText.get_visible" => Ok(Produced::Value(
            ax.axis_of(axis_name).offset_text_visible().into(),
        )),
        _ => return None,
    })
}

/// matplotlib's `loc` of `set_xlabel` (`left`, `center`, `right`) and
/// `set_ylabel` (`bottom`, `center`, `top`).
fn label_loc(axis: AxisName, loc: &str) -> Result<LabelLoc, ProbeError> {
    match (axis, loc) {
        (_, "center") => Ok(LabelLoc::Center),
        (AxisName::X, "left") | (AxisName::Y, "bottom") => Ok(LabelLoc::Start),
        (AxisName::X, "right") | (AxisName::Y, "top") => Ok(LabelLoc::End),
        _ => Err(ProbeError::rejected(format!("bad label loc {loc:?}"))),
    }
}

fn side(name: &str) -> Result<SpineSide, ProbeError> {
    Ok(match name {
        "left" => SpineSide::Left,
        "right" => SpineSide::Right,
        "bottom" => SpineSide::Bottom,
        "top" => SpineSide::Top,
        other => return Err(ProbeError::rejected(format!("unknown side {other:?}"))),
    })
}

/// matplotlib's `get_tick_params()` of an axis whose tick parameters
/// were only ever set through the sides: which sides have tick marks and
/// labels, and whether gridlines are on.
fn tick_params(ax: &Axes, name: AxisName, minor: bool) -> Produced {
    let style = ax.axis_of(name).tick_style(minor);
    let (first, second) = match name {
        AxisName::X => ("bottom", "top"),
        AxisName::Y => ("left", "right"),
    };
    Produced::Value(Json::Object(vec![
        (first.to_owned(), style.ticks.first.into()),
        (second.to_owned(), style.ticks.second.into()),
        (format!("label{first}"), style.labels.first.into()),
        (format!("label{second}"), style.labels.second.into()),
        ("gridOn".to_owned(), style.grid_on.into()),
    ]))
}
