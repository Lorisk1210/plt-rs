//! Probe operations for the `norms` area: every norm's `__call__`,
//! `inverse` and autoscaling, plus `ScalarMappable.to_rgba`.
//!
//! `params` are the norm's constructor keywords; `values` is a number (the
//! scalar code path) or an array, optionally with a `mask`.

use plt::colors::{
    Alpha, AsinhNorm, BoundaryNorm, CenteredNorm, Extend, LogNorm, NoNorm, Norm, Normalize,
    Normalized, PowerNorm, ScalarMappable, SymLogNorm, TwoSlopeNorm, colormaps,
};

use super::colormaps::byte_row;
use super::colors::rgba_json;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// The four probed methods of each norm class.
macro_rules! norm_ops {
    ($($class:literal),* $(,)?) => {
        &[
            $(
                Op::new(concat!("colors.", $class, ".__call__"), |args| call($class, args)),
                Op::new(concat!("colors.", $class, ".inverse"), |args| inverse($class, args)),
                Op::new(concat!("colors.", $class, ".autoscale"), |args| {
                    autoscale($class, args, false)
                }),
                Op::new(concat!("colors.", $class, ".autoscale_None"), |args| {
                    autoscale($class, args, true)
                }),
            )*
            Op::new("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba),
        ]
    };
}

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = norm_ops![
    "Normalize",
    "LogNorm",
    "SymLogNorm",
    "PowerNorm",
    "BoundaryNorm",
    "TwoSlopeNorm",
    "CenteredNorm",
    "AsinhNorm",
    "NoNorm",
];

/// Constructs norm `class` from its keyword arguments.
fn make_norm(class: &str, params: &Json) -> Result<Box<dyn Norm>, ProbeError> {
    let p = Args::new(params);
    let (vmin, vmax) = (p.opt_f64("vmin")?, p.opt_f64("vmax")?);
    let clip = p.opt_bool("clip")?.unwrap_or(false);
    let mut norm: Box<dyn Norm> = match class {
        "Normalize" => Box::new(Normalize::default().with_clip(clip)),
        "LogNorm" => Box::new(LogNorm::default().with_clip(clip)),
        "SymLogNorm" => Box::new(
            SymLogNorm::new(
                p.f64("linthresh")?,
                p.opt_f64("linscale")?.unwrap_or(1.0),
                p.opt_f64("base")?.unwrap_or(10.0),
            )
            .map_err(ProbeError::rejected)?
            .with_clip(clip),
        ),
        "AsinhNorm" => Box::new(
            AsinhNorm::new(p.opt_f64("linear_width")?.unwrap_or(1.0))
                .map_err(ProbeError::rejected)?
                .with_clip(clip),
        ),
        "PowerNorm" => Box::new(PowerNorm::new(p.f64("gamma")?).with_clip(clip)),
        "NoNorm" => Box::new(NoNorm::default().with_clip(clip)),
        "TwoSlopeNorm" => {
            let norm =
                TwoSlopeNorm::new(p.f64("vcenter")?, vmin, vmax).map_err(ProbeError::rejected)?;
            return Ok(Box::new(norm));
        }
        "CenteredNorm" => {
            let vcenter = p.opt_f64("vcenter")?.unwrap_or(0.0);
            let norm = CenteredNorm::new(vcenter, p.opt_f64("halfrange")?).with_clip(clip);
            return Ok(Box::new(norm));
        }
        "BoundaryNorm" => {
            // matplotlib does not validate `extend`; unknown values extend nothing.
            let extend = p
                .opt_str("extend")?
                .map_or(Extend::Neither, |e| e.parse().unwrap_or_default());
            let ncolors = usize::try_from(p.i64("ncolors")?).map_err(ProbeError::rejected)?;
            let norm = BoundaryNorm::new(p.f64_vec("boundaries")?, ncolors, extend)
                .and_then(|norm| norm.with_clip(clip))
                .map_err(ProbeError::rejected)?;
            return Ok(Box::new(norm));
        }
        other => return Err(ProbeError::BadArgs(format!("unknown norm {other:?}"))),
    };
    norm.set_vmin(vmin);
    norm.set_vmax(vmax);
    Ok(norm)
}

/// The probe's `values` (and optional `mask`): a scalar or masked array.
fn masked_values(args: Args<'_>, key: &str) -> Result<(Vec<Option<f64>>, bool), ProbeError> {
    let value = args.json(key)?;
    if let Some(x) = value.as_f64() {
        return Ok((vec![Some(x)], true));
    }
    let values = value
        .as_f64_vec()
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} must be numbers")))?;
    let mask = match args.get("mask") {
        None => vec![false; values.len()],
        Some(mask) => mask
            .as_array()
            .and_then(|m| m.iter().map(Json::as_bool).collect::<Option<Vec<_>>>())
            .ok_or_else(|| ProbeError::BadArgs("mask must be booleans".into()))?,
    };
    let data = values
        .into_iter()
        .zip(mask)
        .map(|(v, masked)| (!masked).then_some(v))
        .collect();
    Ok((data, false))
}

fn normalized_json(value: Normalized) -> Json {
    match value {
        Normalized::Fraction(x) => Json::from(x),
        Normalized::Index(i) => Json::Integer(i),
        Normalized::Masked => Json::Null,
    }
}

/// A list result, or its only element for the scalar code path.
fn shaped(mut items: Vec<Json>, scalar: bool) -> Json {
    match (scalar, items.len()) {
        (true, 1) => items.remove(0),
        _ => Json::Array(items),
    }
}

fn limits_json(norm: &dyn Norm) -> Vec<(String, Json)> {
    vec![
        ("vmin".into(), Json::from(norm.vmin())),
        ("vmax".into(), Json::from(norm.vmax())),
    ]
}

fn call(class: &str, args: Args<'_>) -> Outcome {
    let mut norm = make_norm(class, args.json("params")?)?;
    let (data, scalar) = masked_values(args, "values")?;
    let result = norm
        .normalize_masked(&data, args.opt_bool("clip")?)
        .map_err(ProbeError::rejected)?;
    let result = shaped(result.into_iter().map(normalized_json).collect(), scalar);
    let mut entries = vec![("result".to_owned(), result)];
    entries.extend(limits_json(norm.as_ref()));
    Ok(Json::Object(entries))
}

fn inverse(class: &str, args: Args<'_>) -> Outcome {
    let norm = make_norm(class, args.json("params")?)?;
    let (data, scalar) = masked_values(args, "values")?;
    let values: Vec<f64> = data.into_iter().flatten().collect();
    let result = norm.inverse(&values).map_err(ProbeError::rejected)?;
    Ok(shaped(result.into_iter().map(Json::from).collect(), scalar))
}

fn autoscale(class: &str, args: Args<'_>, only_unset: bool) -> Outcome {
    let mut norm = make_norm(class, args.json("params")?)?;
    let (data, _) = masked_values(args, "data")?;
    if only_unset {
        norm.autoscale_none(&data);
    } else {
        norm.autoscale(&data);
    }
    Ok(Json::Object(limits_json(norm.as_ref())))
}

fn scalar_mappable_to_rgba(args: Args<'_>) -> Outcome {
    let cmap = colormaps::get_cmap(args.opt_str("cmap")?).map_err(ProbeError::rejected)?;
    let norm = match args.get("norm") {
        None => Box::new(Normalize::default()),
        Some(spec) => make_norm(
            Args::new(spec).str("class")?,
            Args::new(spec).json("params")?,
        )?,
    };
    let mut mappable = ScalarMappable::with_norm(cmap, norm);
    let (data, scalar) = masked_values(args, "values")?;
    let alpha = args.opt_f64("alpha")?;
    let bytes = args.opt_bool("bytes")?.unwrap_or(false);
    let values = if args.opt_bool("apply_norm")?.unwrap_or(true) {
        mappable.normalize(&data).map_err(ProbeError::rejected)?
    } else {
        data.iter().copied().map(Normalized::from).collect()
    };
    let cmap = mappable.cmap();
    let alpha = Alpha::from(alpha);
    let rows: Vec<Json> = if bytes {
        let rows = cmap
            .map_bytes(&values, &alpha)
            .map_err(ProbeError::rejected)?;
        rows.into_iter().map(byte_row).collect()
    } else {
        let rows = cmap.map(&values, &alpha).map_err(ProbeError::rejected)?;
        rows.into_iter().map(rgba_json).collect()
    };
    Ok(match (scalar, rows.as_slice()) {
        (true, [row]) => row.clone(),
        _ => Json::Array(rows),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_results_unwrap() {
        assert_eq!(shaped(vec![Json::from(1.0)], true), Json::from(1.0));
        assert_eq!(
            shaped(vec![Json::from(1.0)], false),
            Json::Array(vec![Json::from(1.0)])
        );
    }
}
