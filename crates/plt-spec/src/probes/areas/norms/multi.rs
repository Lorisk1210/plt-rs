//! `MultiNorm` and multivariate colouring probes of the `norms` area.
//!
//! `norms` lists one norm per variate (a scale name or a norm class and
//! its parameters); `values` holds one list per variate or one number each.

use plt::colors::multivar::{MultiColormap, MultiMappable};
use plt::colors::{MultiNorm, NormName, NormSpec};

use super::super::colors::{rgba_json, rgba_rows};
use super::{make_norm, normalized_json};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// matplotlib's `ValueError` for inputs a `MultiNorm` cannot take.
fn not_sequence(what: &str) -> ProbeError {
    ProbeError::rejected(format!("{what} must hold one entry per norm"))
}

/// One sequence of values per variate (`None` = masked).
pub(crate) type Variates = Vec<Vec<Option<f64>>>;

/// The norms of a `MultiNorm`: names or `{"class", "params"}` objects.
pub(crate) fn norm_specs(json: &Json) -> Result<Vec<NormSpec>, ProbeError> {
    let items = json.as_array().ok_or_else(|| not_sequence("norms"))?;
    items
        .iter()
        .map(|item| {
            if let Some(name) = item.as_str() {
                return name
                    .parse::<NormName>()
                    .map(NormSpec::from)
                    .map_err(ProbeError::rejected);
            }
            let spec = Args::new(item);
            make_norm(spec.str("class")?, spec.json("params")?).map(NormSpec::from)
        })
        .collect()
}

/// An optional list of limits (`null` entries clear a limit).
pub(crate) fn limits(args: Args<'_>, key: &str) -> Result<Option<Vec<Option<f64>>>, ProbeError> {
    args.get(key)
        .map(|json| {
            json.as_array()
                .ok_or_else(|| not_sequence(key))?
                .iter()
                .map(|v| match v {
                    Json::Null => Ok(None),
                    other => other.as_f64().map(Some).ok_or_else(|| not_sequence(key)),
                })
                .collect()
        })
        .transpose()
}

/// An optional list of clipping flags.
fn clips(args: Args<'_>, key: &str) -> Result<Option<Vec<bool>>, ProbeError> {
    args.get(key)
        .map(|json| {
            json.as_array()
                .and_then(|flags| flags.iter().map(Json::as_bool).collect())
                .ok_or_else(|| not_sequence(key))
        })
        .transpose()
}

/// The variates under `key`: one list per variate, masked by the lists in
/// `masks`, or one number each (the scalar code path, flagged `true`).
pub(crate) fn variates(args: Args<'_>, key: &str) -> Result<(Variates, bool), ProbeError> {
    let items = args
        .json(key)?
        .as_array()
        .ok_or_else(|| not_sequence(key))?;
    if !items.is_empty() && items.iter().all(|item| item.as_f64().is_some()) {
        let scalars = items.iter().map(|item| vec![item.as_f64()]).collect();
        return Ok((scalars, true));
    }
    let masks: Vec<Option<&Json>> = match args.get("masks").and_then(Json::as_array) {
        Some(masks) => masks.iter().map(Some).collect(),
        None => vec![None; items.len()],
    };
    let bad = || ProbeError::BadArgs(format!("{key} must be lists of numbers"));
    items
        .iter()
        .zip(masks)
        .map(|(item, mask)| {
            let values = item.as_f64_vec().ok_or_else(bad)?;
            let mask: Vec<bool> = match mask {
                Some(mask) => mask
                    .as_array()
                    .and_then(|m| m.iter().map(Json::as_bool).collect())
                    .ok_or_else(bad)?,
                None => vec![false; values.len()],
            };
            Ok(values
                .into_iter()
                .zip(mask)
                .map(|(v, masked)| (!masked).then_some(v))
                .collect())
        })
        .collect::<Result<_, _>>()
        .map(|variates| (variates, false))
}

/// The `MultiNorm` of `norms`, `vmin`, `vmax` and `clip`, set in
/// matplotlib's order.
fn multi_norm(args: Args<'_>) -> Result<MultiNorm, ProbeError> {
    // `norms: null` is Python's `None`, which is not a sequence either.
    let norms = args.get("norms").unwrap_or(&Json::Null);
    let mut norm = MultiNorm::new(norm_specs(norms)?).map_err(ProbeError::rejected)?;
    if let Some(vmin) = limits(args, "vmin")? {
        norm.set_vmin(&vmin).map_err(ProbeError::rejected)?;
    }
    if let Some(vmax) = limits(args, "vmax")? {
        norm.set_vmax(&vmax).map_err(ProbeError::rejected)?;
    }
    if let Some(clip) = clips(args, "clip")? {
        norm.set_clip(&clip).map_err(ProbeError::rejected)?;
    }
    Ok(norm)
}

/// One list per variate, or one value each on the scalar code path.
fn per_variate(items: Vec<Vec<Json>>, scalar: bool) -> Json {
    Json::Array(
        items
            .into_iter()
            .map(|mut items| match (scalar, items.len()) {
                (true, 1) => items.remove(0),
                _ => Json::Array(items),
            })
            .collect(),
    )
}

fn limits_json(norm: &MultiNorm) -> Vec<(String, Json)> {
    let list = |limits: Vec<Option<f64>>| Json::Array(limits.into_iter().map(Json::from).collect());
    vec![
        ("vmin".into(), list(norm.vmin())),
        ("vmax".into(), list(norm.vmax())),
    ]
}

/// `colors.MultiNorm`: the attributes of a new `MultiNorm`.
pub(crate) fn construct(args: Args<'_>) -> Outcome {
    let norm = multi_norm(args)?;
    let mut entries = vec![(
        "n_components".to_owned(),
        Json::Integer(i64::try_from(norm.n_components()).map_err(ProbeError::rejected)?),
    )];
    entries.extend(limits_json(&norm));
    entries.push((
        "clip".to_owned(),
        Json::Array(norm.clip().into_iter().map(Json::Bool).collect()),
    ));
    entries.push(("scaled".to_owned(), Json::Bool(norm.scaled())));
    Ok(Json::Object(entries))
}

/// `colors.MultiNorm.__call__`.
pub(crate) fn call(args: Args<'_>) -> Outcome {
    let mut norm = multi_norm(args)?;
    let (data, scalar) = variates(args, "values")?;
    let clip = clips(args, "call_clip")?;
    let result = norm
        .normalize(&data, clip.as_deref())
        .map_err(ProbeError::rejected)?;
    let result = result
        .into_iter()
        .map(|variate| variate.into_iter().map(normalized_json).collect())
        .collect();
    let mut entries = vec![("result".to_owned(), per_variate(result, scalar))];
    entries.extend(limits_json(&norm));
    Ok(Json::Object(entries))
}

/// `colors.MultiNorm.inverse`.
pub(crate) fn inverse(args: Args<'_>) -> Outcome {
    let norm = multi_norm(args)?;
    let (data, scalar) = variates(args, "values")?;
    let values: Vec<Vec<f64>> = data
        .into_iter()
        .map(|variate| variate.into_iter().flatten().collect())
        .collect();
    let result = norm.inverse(&values).map_err(ProbeError::rejected)?;
    let result = result
        .into_iter()
        .map(|variate| variate.into_iter().map(Json::from).collect())
        .collect();
    Ok(per_variate(result, scalar))
}

/// `colors.MultiNorm.autoscale` and `autoscale_None`.
pub(crate) fn autoscale(args: Args<'_>, only_unset: bool) -> Outcome {
    let mut norm = multi_norm(args)?;
    let (data, _) = variates(args, "data")?;
    let scaled = if only_unset {
        norm.autoscale_none(&data)
    } else {
        norm.autoscale(&data)
    };
    scaled.map_err(ProbeError::rejected)?;
    Ok(Json::Object(limits_json(&norm)))
}

/// The multivariate mappable of a probe's `cmap`, `norm`, `vmin` and
/// `vmax`, with its norms autoscaled to `data` (matplotlib's `Colorizer`
/// and `_scale_norm`).
pub(crate) fn mappable(
    args: Args<'_>,
    data: &[Vec<Option<f64>>],
) -> Result<MultiMappable, ProbeError> {
    let cmap = MultiColormap::get(args.str("cmap")?).map_err(ProbeError::rejected)?;
    let norm = match args.get("norm") {
        Some(norms) => MultiNorm::new(norm_specs(norms)?).map_err(ProbeError::rejected)?,
        None => MultiNorm::linear(cmap.n_variates()).map_err(ProbeError::rejected)?,
    };
    let mut mappable = MultiMappable::with_norm(cmap, norm).map_err(ProbeError::rejected)?;
    let (vmin, vmax) = (limits(args, "vmin")?, limits(args, "vmax")?);
    mappable
        .set_clim(vmin.as_deref(), vmax.as_deref())
        .map_err(ProbeError::rejected)?;
    mappable
        .autoscale_none(data)
        .map_err(ProbeError::rejected)?;
    Ok(mappable)
}

/// `colorizer.Colorizer.to_rgba` with a bivariate or multivariate
/// colormap; non-finite values are masked, as `safe_masked_invalid` does.
pub(crate) fn colorizer_to_rgba(args: Args<'_>) -> Outcome {
    let (data, scalar) = variates(args, "values")?;
    let data: Vec<Vec<Option<f64>>> = data
        .into_iter()
        .map(|variate| {
            variate
                .into_iter()
                .map(|v| v.filter(|v| v.is_finite()))
                .collect()
        })
        .collect();
    let mut mappable = mappable(args, &data)?;
    let colors = mappable
        .to_rgba(&data, args.opt_f64("alpha")?)
        .map_err(ProbeError::rejected)?;
    // One number per variate is one colour, as in matplotlib.
    Ok(match colors.as_slice() {
        [color] if scalar => rgba_json(*color),
        _ => rgba_rows(&colors),
    })
}
