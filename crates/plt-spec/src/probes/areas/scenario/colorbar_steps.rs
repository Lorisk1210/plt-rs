//! Steps making and querying colorbars: `Figure.colorbar` and the tick
//! methods of a colorbar (the methods of its axes, `cbar.ax.<method>`, are
//! replayed as axes steps).

use plt::colorbar::{Colorbar, ColorbarAxes, ColorbarFormat, ColorbarOptions, ColorbarTicks};
use plt::colors::Extend;
use plt::orientation::Orientation;

use super::call::{Call, KEYWORD_ONLY};
use super::scene::{Handled, Object, Produced, Scene};
use crate::probes::ProbeError;

/// Runs a colorbar method of the figure.
pub(super) fn figure_call(scene: &Scene, call: &Call<'_>) -> Handled {
    if call.method != "colorbar" {
        return None;
    }
    Some(colorbar(scene, call))
}

/// Runs a method of a colorbar.
pub(super) fn colorbar_call(colorbar: &Colorbar, call: &Call<'_>) -> Handled {
    Some(match call.method {
        "get_ticks" => call.opt_bool(KEYWORD_ONLY, "minor").and_then(|minor| {
            colorbar
                .long_axis()
                .ticklocs(minor.unwrap_or(false))
                .map(Produced::numbers)
                .map_err(ProbeError::rejected)
        }),
        "set_ticks" => call.f64_vec(0, "ticks").map(|ticks| {
            colorbar.set_ticks(ticks);
            Produced::Nothing
        }),
        "minorticks_on" => {
            colorbar.minorticks_on();
            Ok(Produced::Nothing)
        }
        "minorticks_off" => {
            colorbar.minorticks_off();
            Ok(Produced::Nothing)
        }
        _ => return None,
    })
}

/// matplotlib's `fig.colorbar(mappable, ax=..., orientation=...,
/// extend=..., ticks=..., shrink=..., format=...)`.
fn colorbar(scene: &Scene, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let mut options = ColorbarOptions::default();
    if let Some(ax) = call.keyword("ax") {
        options.axes = ColorbarAxes::Parent(scene.axes_argument(ax)?);
    }
    options.orientation = match call.opt_str(KEYWORD_ONLY, "orientation")? {
        None => None,
        Some("vertical") => Some(Orientation::Vertical),
        Some("horizontal") => Some(Orientation::Horizontal),
        Some(other) => return Err(ProbeError::rejected(format!("bad orientation {other:?}"))),
    };
    options.extend = match call.opt_str(KEYWORD_ONLY, "extend")? {
        None => None,
        Some("neither") => Some(Extend::Neither),
        Some("both") => Some(Extend::Both),
        Some("min") => Some(Extend::Min),
        Some("max") => Some(Extend::Max),
        Some(other) => return Err(ProbeError::rejected(format!("bad extend {other:?}"))),
    };
    if call.keyword("ticks").is_some() {
        options.ticks = Some(ColorbarTicks::Values(call.f64_vec(KEYWORD_ONLY, "ticks")?));
    }
    if let Some(shrink) = call.opt_f64(KEYWORD_ONLY, "shrink")? {
        options.shrink = shrink;
    }
    options.format = call
        .opt_str(KEYWORD_ONLY, "format")?
        .map(|template| ColorbarFormat::Template(template.to_owned()));
    let made = match scene.reference(call.str(0, "mappable")?)? {
        Object::Collection(collection) => scene.fig.colorbar(&collection, options),
        Object::Image(image) => scene.fig.colorbar(&image, options),
        Object::Contours(contours) => scene.fig.colorbar(&contours, options),
        _ => return Err(call.bad("not a mappable")),
    };
    made.map(|colorbar| Produced::Object(Object::Colorbar(colorbar)))
        .map_err(ProbeError::rejected)
}
