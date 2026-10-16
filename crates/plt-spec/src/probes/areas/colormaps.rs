//! Probe operations for the `colormaps` area: the registry, sampling and
//! colormap construction.

use plt::colors::{
    Alpha, Channel, Colormap, ColormapKind, Normalized, Segment, SegmentData, colormaps,
    to_rgba_array,
};
use plt::numeric::linspace;

use super::colors::{alpha_arg, color_list, rgba_arg, rgba_json, rgba_rows};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("colormaps.names", names),
    Op::new("colormaps.info", info),
    Op::new("colormaps.segmentdata", segmentdata),
    Op::new("colormaps.sample", sample),
    Op::new("colormaps.sample_index", sample_index),
    Op::new("colormaps.sample_bytes", sample_bytes),
    Op::new("colormaps.sample_alpha", sample_alpha),
    Op::new("colormaps.extremes", extremes),
    Op::new("colormaps.resampled", resampled),
    Op::new("colormaps.reversed", reversed),
    Op::new("colormaps.with_extremes", with_extremes),
    Op::new("colormaps.with_alpha", with_alpha),
    Op::new("colormaps.equal", equal),
    Op::new("colormaps.get_cmap", get_cmap),
    Op::new("pyplot.get_cmap", pyplot_get_cmap),
    Op::new("colors.ListedColormap", listed),
    Op::new("colors.LinearSegmentedColormap", segmented),
    Op::new("colors.LinearSegmentedColormap.from_list", from_list),
    Op::new(
        "colors.LinearSegmentedColormap.from_list.resampled",
        from_list_resampled,
    ),
];

/// The registered colormap `args[key]`.
pub(crate) fn registered(args: Args<'_>, key: &str) -> Result<Colormap, ProbeError> {
    colormaps::get(args.str(key)?).map_err(ProbeError::rejected)
}

/// `cmap(np.arange(cmap.N))`: the full table of colours.
pub(crate) fn lut_rows(cmap: &Colormap) -> Json {
    rgba_rows(cmap.lut())
}

/// `cmap(x)` over a (nested) array of floats.
fn sample_floats(cmap: &Colormap, x: &Json) -> Outcome {
    match x {
        Json::Array(items) => items
            .iter()
            .map(|item| sample_floats(cmap, item))
            .collect::<Result<Vec<_>, _>>()
            .map(Json::Array),
        value => value
            .as_f64()
            .map(|x| rgba_json(cmap.sample(x)))
            .ok_or_else(|| ProbeError::BadArgs(format!("{value} is not a number"))),
    }
}

/// `[r, g, b, a]` as integers.
pub(crate) fn byte_row(row: [u8; 4]) -> Json {
    Json::Array(row.iter().map(|&b| Json::Integer(i64::from(b))).collect())
}

fn byte_rows(rows: &[[u8; 4]]) -> Json {
    Json::Array(rows.iter().copied().map(byte_row).collect())
}

fn fractions(args: Args<'_>) -> Result<Vec<Normalized>, ProbeError> {
    Ok(args
        .f64_vec("x")?
        .into_iter()
        .map(Normalized::Fraction)
        .collect())
}

/// `{under, over, bad}` entries.
fn extreme_entries(cmap: &Colormap) -> Vec<(String, Json)> {
    vec![
        ("under".into(), rgba_json(cmap.under())),
        ("over".into(), rgba_json(cmap.over())),
        ("bad".into(), rgba_json(cmap.bad())),
    ]
}

/// `{"sample": cmap(x), under, over, bad}`.
fn sampled_state(cmap: &Colormap, x: &Json) -> Outcome {
    let mut entries = vec![("sample".to_owned(), sample_floats(cmap, x)?)];
    entries.extend(extreme_entries(cmap));
    Ok(Json::Object(entries))
}

/// `{N, lut, under, over, bad}`.
fn table(cmap: &Colormap) -> Vec<(String, Json)> {
    let mut entries = vec![
        ("N".to_owned(), Json::Integer(cmap.n() as i64)),
        ("lut".to_owned(), lut_rows(cmap)),
    ];
    entries.extend(extreme_entries(cmap));
    entries
}

/// Applies the optional `bad`/`under`/`over` colour arguments.
fn apply_extremes(mut cmap: Colormap, args: Args<'_>) -> Result<Colormap, ProbeError> {
    if let Some(bad) = args.get("bad") {
        cmap = cmap
            .with_bad(rgba_arg(bad)?)
            .map_err(ProbeError::rejected)?;
    }
    if let Some(under) = args.get("under") {
        cmap = cmap
            .with_under(rgba_arg(under)?)
            .map_err(ProbeError::rejected)?;
    }
    if let Some(over) = args.get("over") {
        cmap = cmap
            .with_over(rgba_arg(over)?)
            .map_err(ProbeError::rejected)?;
    }
    Ok(cmap)
}

fn names(_: Args<'_>) -> Outcome {
    Ok(Json::from(
        colormaps::names()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    ))
}

fn info(args: Args<'_>) -> Outcome {
    let cmap = registered(args, "name")?;
    let class = match cmap.kind() {
        ColormapKind::Listed => "ListedColormap",
        ColormapKind::LinearSegmented => "LinearSegmentedColormap",
    };
    Ok(Json::Object(vec![
        ("class".into(), Json::from(class)),
        ("name".into(), Json::from(cmap.name())),
        ("N".into(), Json::Integer(cmap.n() as i64)),
        ("is_gray".into(), Json::from(cmap.is_gray())),
        (
            "colorbar_extend".into(),
            Json::from(cmap.colorbar_extend().is_some()),
        ),
    ]))
}

fn segmentdata(args: Args<'_>) -> Outcome {
    let cmap = registered(args, "name")?;
    let Some(data) = cmap.segment_data() else {
        return Ok(Json::Object(vec![(
            "listed".into(),
            Json::Integer(cmap.n() as i64),
        )]));
    };
    let channels = [Channel::Red, Channel::Green, Channel::Blue, Channel::Alpha];
    Ok(Json::Object(
        channels
            .into_iter()
            .filter_map(|channel| {
                let value = match data.channel(channel)? {
                    Segment::Anchors(rows) => {
                        Json::Array(rows.iter().map(|row| Json::from(row.to_vec())).collect())
                    }
                    Segment::Func(func) => Json::from(format!("callable:{}", func.name())),
                };
                Some((channel.name().to_owned(), value))
            })
            .collect(),
    ))
}

fn sample(args: Args<'_>) -> Outcome {
    sample_floats(&registered(args, "name")?, args.json("x")?)
}

fn sample_index(args: Args<'_>) -> Outcome {
    let cmap = registered(args, "name")?;
    let indices = args.json("i")?.as_array().unwrap_or_default();
    indices
        .iter()
        .map(|i| {
            i.as_i64()
                .map(|i| rgba_json(cmap.sample_index(i)))
                .ok_or_else(|| ProbeError::BadArgs(format!("{i} is not an index")))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Json::Array)
}

fn sample_bytes(args: Args<'_>) -> Outcome {
    let cmap = registered(args, "name")?;
    cmap.map_bytes(&fractions(args)?, &Alpha::Keep)
        .map(|rows| byte_rows(&rows))
        .map_err(ProbeError::rejected)
}

fn sample_alpha(args: Args<'_>) -> Outcome {
    let cmap = registered(args, "name")?;
    let values = fractions(args)?;
    let alpha = alpha_arg(args.get("alpha"))?;
    if args.opt_bool("bytes")?.unwrap_or(false) {
        cmap.map_bytes(&values, &alpha).map(|rows| byte_rows(&rows))
    } else {
        cmap.map(&values, &alpha).map(|rows| rgba_rows(&rows))
    }
    .map_err(ProbeError::rejected)
}

fn extremes(args: Args<'_>) -> Outcome {
    Ok(Json::Object(extreme_entries(&registered(args, "name")?)))
}

fn resampled(args: Args<'_>) -> Outcome {
    let lutsize = usize::try_from(args.i64("lutsize")?).map_err(ProbeError::rejected)?;
    let cmap = registered(args, "name")?
        .resampled(lutsize)
        .map_err(ProbeError::rejected)?;
    let mut entries = vec![("N".to_owned(), Json::Integer(cmap.n() as i64))];
    if let Json::Object(state) = sampled_state(&cmap, args.json("x")?)? {
        entries.extend(state);
    }
    Ok(Json::Object(entries))
}

fn reversed(args: Args<'_>) -> Outcome {
    let cmap = registered(args, "name")?.reversed();
    let mut entries = vec![
        ("name".to_owned(), Json::from(cmap.name())),
        ("N".to_owned(), Json::Integer(cmap.n() as i64)),
    ];
    if let Json::Object(state) = sampled_state(&cmap, args.json("x")?)? {
        entries.extend(state);
    }
    Ok(Json::Object(entries))
}

fn with_extremes(args: Args<'_>) -> Outcome {
    let mut cmap = apply_extremes(registered(args, "name")?, args)?;
    if args.opt_bool("reverse")?.unwrap_or(false) {
        cmap = cmap.reversed();
    }
    sampled_state(&cmap, args.json("x")?)
}

fn with_alpha(args: Args<'_>) -> Outcome {
    let cmap = registered(args, "name")?
        .with_alpha(args.f64("alpha")?)
        .map_err(ProbeError::rejected)?;
    sampled_state(&cmap, args.json("x")?)
}

fn equal(args: Args<'_>) -> Outcome {
    Ok(Json::from(registered(args, "a")? == registered(args, "b")?))
}

fn get_cmap(args: Args<'_>) -> Outcome {
    let cmap = colormaps::get_cmap(args.opt_str("cmap")?).map_err(ProbeError::rejected)?;
    Ok(Json::Object(vec![
        ("name".into(), Json::from(cmap.name())),
        ("N".into(), Json::Integer(cmap.n() as i64)),
    ]))
}

fn pyplot_get_cmap(args: Args<'_>) -> Outcome {
    let mut cmap = colormaps::get_cmap(args.opt_str("name")?).map_err(ProbeError::rejected)?;
    if let Some(lut) = args.opt_i64("lut")? {
        let lut = usize::try_from(lut).map_err(ProbeError::rejected)?;
        cmap = cmap.resampled(lut).map_err(ProbeError::rejected)?;
    }
    let x = Json::from(linspace(0.0, 1.0, 17));
    Ok(Json::Object(vec![
        ("name".into(), Json::from(cmap.name())),
        ("N".into(), Json::Integer(cmap.n() as i64)),
        ("sample".into(), sample_floats(&cmap, &x)?),
    ]))
}

/// The `N` argument as a LUT size.
fn lut_size(args: Args<'_>, key: &str) -> Result<Option<usize>, ProbeError> {
    args.opt_i64(key)?
        .map(|n| usize::try_from(n).map_err(ProbeError::rejected))
        .transpose()
}

fn listed(args: Args<'_>) -> Outcome {
    let value = args.json("colors")?;
    let items: Vec<Json> = match (value, lut_size(args, "N")?) {
        (Json::Array(items), None) => items.clone(),
        // matplotlib's deprecated N repeats or truncates the colours.
        (Json::Array(items), Some(n)) => items.iter().cycle().take(n).cloned().collect(),
        (single, Some(n)) => vec![single.clone(); n],
        (single, None) => vec![single.clone()],
    };
    let name = args.opt_str("name")?.unwrap_or("unnamed");
    let cmap = Colormap::listed(name, color_list(&items)?).map_err(ProbeError::rejected)?;
    let cmap = apply_extremes(cmap, args)?;
    let mut entries = vec![("name".to_owned(), Json::from(cmap.name()))];
    entries.extend(table(&cmap));
    entries.push(("monochrome".into(), Json::from(cmap.is_monochrome())));
    entries.push(("is_gray".into(), Json::from(cmap.is_gray())));
    Ok(Json::Object(entries))
}

/// Decodes `{"red": [[x, y0, y1], ...], ...}`.
fn segment_data(value: &Json) -> Result<SegmentData, ProbeError> {
    let channel = |key: &str| -> Result<Option<Segment>, ProbeError> {
        value
            .get(key)
            .map(|rows| {
                rows.as_array()
                    .and_then(|rows| {
                        rows.iter()
                            .map(|row| match row.as_f64_vec().as_deref() {
                                Some(&[x, y0, y1]) => Some([x, y0, y1]),
                                _ => None,
                            })
                            .collect::<Option<Vec<_>>>()
                    })
                    .map(Segment::Anchors)
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad {key} segment data")))
            })
            .transpose()
    };
    let required = |key: &str| {
        channel(key)?.ok_or_else(|| ProbeError::BadArgs(format!("missing {key} segment data")))
    };
    let data = SegmentData::new(required("red")?, required("green")?, required("blue")?);
    Ok(match channel("alpha")? {
        Some(alpha) => data.with_alpha(alpha),
        None => data,
    })
}

fn segmented(args: Args<'_>) -> Outcome {
    let data = segment_data(args.json("segmentdata")?)?;
    let n = lut_size(args, "N")?.unwrap_or(256);
    let gamma = args.opt_f64("gamma")?.unwrap_or(1.0);
    let cmap = Colormap::segmented("probe", data, n, gamma).map_err(ProbeError::rejected)?;
    Ok(Json::Object(table(&cmap)))
}

/// A `(value, colour)` pair of `from_list`.
fn stop(item: &Json) -> Result<(f64, plt_core::Rgba), ProbeError> {
    let not_a_pair = || ProbeError::rejected(format!("{item} is not a (value, colour) pair"));
    let [value, color] = item.as_array().ok_or_else(not_a_pair)? else {
        return Err(not_a_pair());
    };
    let value = value.as_f64().ok_or_else(not_a_pair)?;
    Ok((value, rgba_arg(color)?))
}

/// `LinearSegmentedColormap.from_list("probe", colors, N, gamma)`: the
/// entries are colours if they all resolve, else `(value, colour)` pairs.
fn build_from_list(args: Args<'_>) -> Result<Colormap, ProbeError> {
    let items = args
        .json("colors")?
        .as_array()
        .ok_or_else(|| ProbeError::rejected("colors must be iterable"))?;
    let n = lut_size(args, "N")?.unwrap_or(256);
    let gamma = args.opt_f64("gamma")?.unwrap_or(1.0);
    let as_colors = color_list(items)
        .and_then(|colors| to_rgba_array(colors, Alpha::Keep).map_err(ProbeError::rejected));
    let cmap = if let Ok(rows) = as_colors {
        Colormap::from_list("probe", rows, n, gamma)
    } else {
        let stops = items.iter().map(stop).collect::<Result<Vec<_>, _>>()?;
        Colormap::from_stops("probe", stops, n, gamma)
    };
    cmap.map_err(ProbeError::rejected)
}

fn from_list(args: Args<'_>) -> Outcome {
    let cmap = apply_extremes(build_from_list(args)?, args)?;
    Ok(Json::Object(table(&cmap)))
}

fn from_list_resampled(args: Args<'_>) -> Outcome {
    let lutsize = lut_size(args, "lutsize")?.unwrap_or(256);
    let cmap = build_from_list(args)?
        .resampled(lutsize)
        .map_err(ProbeError::rejected)?;
    Ok(Json::Object(vec![
        ("N".into(), Json::Integer(cmap.n() as i64)),
        ("lut".into(), lut_rows(&cmap)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::areas::colors::Spec;

    #[test]
    fn spec_pairs_resolve_like_python() {
        let pair = Json::parse(r#"["red", 0.25]"#).unwrap();
        assert_eq!(rgba_arg(&pair).unwrap().a, 0.25);
        let nested = Json::parse(r#"[["red", 0.3], 0.5]"#).unwrap();
        assert!(Spec::parse(&nested).unwrap().resolve(None).is_err());
    }
}
