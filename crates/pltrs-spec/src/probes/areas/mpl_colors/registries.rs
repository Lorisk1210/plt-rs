//! The colormap and colour sequence registries: what `test_colors.py`
//! registers, reads back and removes. Every op undoes its registrations.

use pltrs::colors::{
    Color, color_sequence, colormaps, register_color_sequence, unregister_color_sequence,
};

use super::colors::{rgba_arg, rgba_json, rgba_rows};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// Registering `colormaps[name]` under its own name: an error for a
/// built-in colormap.
pub(super) fn register_cmap(args: Args<'_>) -> Outcome {
    let name = args.str("name")?;
    let cmap = colormaps::get(name).map_err(ProbeError::rejected)?;
    colormaps::register(cmap).map_err(ProbeError::rejected)?;
    colormaps::unregister(name).map_err(ProbeError::rejected)?;
    Ok(Json::Null)
}

/// `{name, N, over}` of the colormap read back after registering
/// `colormaps[name].with_extremes(over=over)` under `as_name`.
pub(super) fn register_and_get_cmap(args: Args<'_>) -> Outcome {
    let as_name = args.str("as_name")?;
    let over = rgba_arg(args.json("over")?)?;
    let cmap = colormaps::get(args.str("name")?)
        .and_then(|cmap| cmap.with_over(over))
        .map_err(ProbeError::rejected)?
        .with_name(as_name);
    colormaps::register(cmap).map_err(ProbeError::rejected)?;
    let returned = colormaps::get(as_name);
    colormaps::unregister(as_name).map_err(ProbeError::rejected)?;
    let returned = returned.map_err(ProbeError::rejected)?;
    Ok(Json::Object(vec![
        ("name".into(), Json::from(returned.name())),
        ("N".into(), Json::Integer(returned.n() as i64)),
        ("over".into(), rgba_json(returned.over())),
    ]))
}

/// The probe's colour names under `key`.
fn colors(args: Args<'_>, key: &str) -> Result<Vec<Color>, ProbeError> {
    args.json(key)?
        .as_array()
        .and_then(|items| items.iter().map(|c| c.as_str().map(Color::from)).collect())
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a list of colour names")))
}

/// Registering `colors` under `name` (then removing them again).
pub(super) fn register_sequence(args: Args<'_>) -> Outcome {
    let name = args.str("name")?;
    register_color_sequence(name, &colors(args, "colors")?).map_err(ProbeError::rejected)?;
    unregister_color_sequence(name).map_err(ProbeError::rejected)?;
    Ok(Json::Null)
}

/// The colours of sequence `name`, registered from `register` first when
/// given; an unknown name is an error.
pub(super) fn get_sequence(args: Args<'_>) -> Outcome {
    let name = args.str("name")?;
    let registered = args.get("register").is_some();
    if registered {
        register_color_sequence(name, &colors(args, "register")?).map_err(ProbeError::rejected)?;
    }
    let found = color_sequence(name);
    if registered {
        unregister_color_sequence(name).map_err(ProbeError::rejected)?;
    }
    found
        .map(|rgba| rgba_rows(&rgba))
        .ok_or_else(|| ProbeError::Rejected(format!("unknown color sequence {name:?}")))
}

/// Unregistering `name` `times` times.
pub(super) fn unregister_sequence(args: Args<'_>) -> Outcome {
    let name = args.str("name")?;
    for _ in 0..args.i64("times")? {
        unregister_color_sequence(name).map_err(ProbeError::rejected)?;
    }
    Ok(Json::Null)
}
