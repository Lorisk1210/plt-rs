//! Axes steps about placement and structure: aspect, positions, twins,
//! clearing and removing, visibility and z-order.

use plt::artist::ArtistHandle;
use plt::axes::{Adjustable, Axes, AxisName, TitleLocation, WhichPosition};
use plt::rc::Aspect;

use super::axes_limits::bbox;
use super::call::{Call, KEYWORD_ONLY};
use super::figure_steps::rect;
use super::scene::{Handled, Object, Produced, Scene};
use crate::json::Json;
use crate::probes::ProbeError;

/// Runs a layout method of `ax`.
pub(super) fn call(scene: &Scene, ax: &Axes, call: &Call<'_>) -> Handled {
    Some(match call.method {
        "set_aspect" => set_aspect(ax, call),
        "get_aspect" => Ok(Produced::Value(match ax.aspect() {
            Aspect::Auto => Json::from("auto"),
            Aspect::Equal => Json::from(1.0),
            Aspect::Ratio(ratio) => Json::from(ratio),
        })),
        "set_adjustable" => adjustable(call.str(0, "adjustable")).map(|adjustable| {
            ax.set_adjustable(adjustable);
            Produced::Nothing
        }),
        "get_adjustable" => Ok(Produced::Value(Json::from(match ax.adjustable() {
            Adjustable::Box => "box",
            Adjustable::Datalim => "datalim",
        }))),
        "apply_aspect" => {
            ax.apply_aspect();
            Ok(Produced::Nothing)
        }
        "set_box_aspect" => call.opt_f64(0, "aspect").and_then(|aspect| {
            ax.set_box_aspect(aspect).map_err(ProbeError::rejected)?;
            Ok(Produced::Nothing)
        }),
        "get_box_aspect" => Ok(Produced::Value(ax.box_aspect().into())),
        "set_position" => set_position(ax, call),
        "get_position" => call.opt_bool(0, "original").map(|original| {
            Produced::Value(bbox(if original == Some(true) {
                ax.original_position()
            } else {
                ax.position()
            }))
        }),
        "twinx" | "twiny" => call.opt_f64(KEYWORD_ONLY, "delta_zorder").map(|delta| {
            let shared = if call.method == "twinx" {
                AxisName::X
            } else {
                AxisName::Y
            };
            Produced::Object(Object::Axes(ax.twin_with(shared, delta.unwrap_or(0.0))))
        }),
        "inset_axes" => {
            rect(call).map(|bounds| Produced::Object(Object::Axes(ax.inset_axes(bounds))))
        }
        "set_title" => call.str(0, "label").and_then(|text| {
            if call.has_keyword("loc") {
                ax.set_title_at(title_location(call)?, text);
            } else {
                ax.set_title(text);
            }
            Ok(Produced::Nothing)
        }),
        "get_title" => title_location(call).map(|loc| Produced::Value(ax.title(loc).into())),
        "cla" | "clear" => {
            ax.clear();
            Ok(Produced::Nothing)
        }
        "remove" => {
            ax.remove();
            Ok(Produced::Nothing)
        }
        "get_zorder" => Ok(Produced::Value(ax.zorder().into())),
        "set_zorder" => call.f64(0, "level").map(|zorder| {
            ax.set_zorder(zorder);
            Produced::Nothing
        }),
        "get_shared_x_axes.joined" | "get_shared_y_axes.joined" => {
            let name = if call.method.contains("_x_") {
                AxisName::X
            } else {
                AxisName::Y
            };
            let pair = (call.required(0, "a"), call.required(1, "b"));
            pair.0.and_then(|a| {
                let (a, b) = (scene.axes_argument(a)?, scene.axes_argument(pair.1?)?);
                Ok(Produced::Value(a.shared_axes(name).contains(&b).into()))
            })
        }
        "patch.get_visible" => Ok(Produced::Value(ax.patch_visible().into())),
        "xaxis.get_visible" => Ok(Produced::Value(ax.xaxis().visible().into())),
        "yaxis.get_visible" => Ok(Produced::Value(ax.yaxis().visible().into())),
        "xaxis.get_inverted" => Ok(Produced::Value(ax.xaxis().inverted().into())),
        "yaxis.get_inverted" => Ok(Produced::Value(ax.yaxis().inverted().into())),
        "xaxis.set_inverted" | "yaxis.set_inverted" => call.opt_bool(0, "inverted").map(|on| {
            let axis = if call.method.starts_with('x') {
                ax.xaxis()
            } else {
                ax.yaxis()
            };
            axis.set_inverted(on.unwrap_or(false));
            Produced::Nothing
        }),
        _ => return None,
    })
}

/// The `loc` of `set_title`/`get_title` (`"center"` by default).
fn title_location(call: &Call<'_>) -> Result<TitleLocation, ProbeError> {
    match call.opt_str(KEYWORD_ONLY, "loc")? {
        None | Some("center") => Ok(TitleLocation::Center),
        Some("left") => Ok(TitleLocation::Left),
        Some("right") => Ok(TitleLocation::Right),
        Some(other) => Err(ProbeError::rejected(format!(
            "bad title location {other:?}"
        ))),
    }
}

fn adjustable(name: Result<&str, ProbeError>) -> Result<Adjustable, ProbeError> {
    match name? {
        "box" => Ok(Adjustable::Box),
        "datalim" => Ok(Adjustable::Datalim),
        other => Err(ProbeError::rejected(format!("bad adjustable {other:?}"))),
    }
}

/// matplotlib's `set_aspect(aspect, adjustable=None, share=False)`.
fn set_aspect(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let aspect = match call.required(0, "aspect")? {
        Json::String(name) if name == "auto" => Aspect::Auto,
        Json::String(name) if name == "equal" => Aspect::Equal,
        value => Aspect::Ratio(
            value
                .as_f64()
                .ok_or_else(|| ProbeError::rejected(format!("bad aspect {value}")))?,
        ),
    };
    let adjustable = call
        .opt_str(1, "adjustable")?
        .map(|name| adjustable(Ok(name)))
        .transpose()?
        .unwrap_or_else(|| ax.adjustable());
    if call.opt_bool(KEYWORD_ONLY, "share")? == Some(true) {
        ax.set_shared_aspect(aspect, adjustable)
            .map_err(ProbeError::rejected)?;
    } else {
        ax.set_aspect(aspect).map_err(ProbeError::rejected)?;
        ax.set_adjustable(adjustable);
    }
    Ok(Produced::Nothing)
}

/// matplotlib's `set_position(pos, which='both')`.
fn set_position(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let which = match call.opt_str(1, "which")? {
        None | Some("both") => WhichPosition::Both,
        Some("original") => WhichPosition::Original,
        Some("active") => WhichPosition::Active,
        Some(other) => return Err(ProbeError::rejected(format!("bad which {other:?}"))),
    };
    ax.set_position_which(rect(call)?, which);
    Ok(Produced::Nothing)
}
