//! Colormap calls of `test_colors.py` the `colormaps` area has no op for:
//! resampling and alpha of colormaps built from colour lists, equality
//! after a change, and the banded colormaps of `from_levels_and_colors`.

use pltrs::colors::{Color, Colormap, Extend, ScalarMappable, colormaps, from_levels_and_colors};

use super::colormaps::{apply_extremes, build_from_list, sampled_state};
use super::colors::{color_list, rgba_arg, rgba_rows};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// The `"listed"` colormap of the probe's `colors`.
fn listed(args: Args<'_>) -> Result<Colormap, ProbeError> {
    let items = args
        .json("colors")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("colors is not a list".into()))?;
    Colormap::listed("probe", color_list(items)?).map_err(ProbeError::rejected)
}

/// `{N, sample, under, over, bad}` of the `"listed"` or `"from_list"`
/// colormap of `colors` with the given extremes, resampled to `lutsize`.
pub(super) fn resampled(args: Args<'_>) -> Outcome {
    let built = match args.str("kind")? {
        "listed" => listed(args)?,
        "from_list" => build_from_list(args)?,
        other => return Err(ProbeError::BadArgs(format!("kind {other}"))),
    };
    let lutsize = usize::try_from(args.i64("lutsize")?).map_err(ProbeError::rejected)?;
    let cmap = apply_extremes(built, args)?
        .resampled(lutsize)
        .map_err(ProbeError::rejected)?;
    let mut entries = vec![("N".to_owned(), Json::Integer(cmap.n() as i64))];
    if let Json::Object(state) = sampled_state(&cmap, args.json("x")?)? {
        entries.extend(state);
    }
    Ok(Json::Object(entries))
}

/// `{sample, under, over, bad}` of `ListedColormap(colors).with_alpha(alpha)`.
pub(super) fn with_alpha(args: Args<'_>) -> Outcome {
    let cmap = listed(args)?
        .with_alpha(args.f64("alpha")?)
        .map_err(ProbeError::rejected)?;
    sampled_state(&cmap, args.json("x")?)
}

/// Whether `colormaps[name]` equals its copy after `change`: null, `"bad"`
/// (bad colour `'y'`), `"name"` (renamed) or `"colorbar_extend"` (flipped).
pub(super) fn equal_after(args: Args<'_>) -> Outcome {
    let cmap = colormaps::get(args.str("name")?).map_err(ProbeError::rejected)?;
    let other = match args.opt_str("change")? {
        None => cmap.clone(),
        Some("bad") => cmap
            .clone()
            .with_bad(Color::from("y"))
            .map_err(ProbeError::rejected)?,
        Some("name") => cmap.clone().with_name("Test"),
        Some("colorbar_extend") => {
            let flipped = match cmap.colorbar_extend() {
                None => Some(Extend::Both),
                Some(_) => None,
            };
            cmap.clone().with_colorbar_extend(flipped)
        }
        Some(other) => return Err(ProbeError::BadArgs(format!("change {other}"))),
    };
    Ok(Json::from(other == cmap))
}

/// `cmap(norm(values))` for `from_levels_and_colors(levels, colors,
/// extend)` after `cmap.with_extremes(bad=bad)`; with `masked` every value
/// is masked.
pub(super) fn levels_with_extremes(args: Args<'_>) -> Outcome {
    let colors = args
        .json("colors")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("colors is not a list".into()))?
        .iter()
        .map(|item| rgba_arg(item).map(Color::from))
        .collect::<Result<Vec<_>, _>>()?;
    let extend = match args.str("extend")? {
        "neither" => Extend::Neither,
        "min" => Extend::Min,
        "max" => Extend::Max,
        "both" => Extend::Both,
        other => return Err(ProbeError::BadArgs(format!("extend {other}"))),
    };
    let (cmap, norm) = from_levels_and_colors(&args.f64_vec("levels")?, &colors, extend)
        .map_err(ProbeError::rejected)?;
    let cmap = cmap
        .with_bad(rgba_arg(args.json("bad")?)?)
        .map_err(ProbeError::rejected)?;
    let masked = args.opt_bool("masked")?.unwrap_or(false);
    let values: Vec<Option<f64>> = args
        .f64_vec("values")?
        .into_iter()
        .map(|v| (!masked).then_some(v))
        .collect();
    let mut mappable = ScalarMappable::with_norm(cmap, Box::new(norm));
    let rgba = mappable
        .to_rgba(&values, None)
        .map_err(ProbeError::rejected)?;
    Ok(rgba_rows(&rgba))
}
