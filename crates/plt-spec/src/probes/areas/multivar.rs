//! Probe operations for the `multivar` area (multivariate and bivariate
//! colormaps).

use plt::colors::multivar::{
    BivarAxis, BivarColormap, BivarShape, bivar_colormap, multivar_colormap,
};
use plt::colors::{Color, Colormap, Normalized};
use plt_core::Rgba;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, malformed};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("multivar.call", multivar_call),
    Op::new("bivar.call", bivar_call),
    Op::new("bivar.lut", bivar_lut),
    Op::new("bivar.component", bivar_component),
];

/// One variate's values: indices when every value is an integer, else
/// fractions with `null` as NaN (numpy's array dtype rule).
pub(super) fn values(json: &Json) -> Result<Vec<Normalized>, ProbeError> {
    let items = json.as_array().ok_or_else(|| malformed("values"))?;
    if items.iter().all(|item| matches!(item, Json::Integer(_))) {
        return Ok(items
            .iter()
            .filter_map(|item| match item {
                Json::Integer(i) => Some(Normalized::Index(*i)),
                _ => None,
            })
            .collect());
    }
    items
        .iter()
        .map(|item| match item {
            Json::Null => Ok(Normalized::Fraction(f64::NAN)),
            other => other
                .as_f64()
                .map(Normalized::Fraction)
                .ok_or_else(|| malformed("values")),
        })
        .collect()
}

pub(super) fn colors_json(colors: impl IntoIterator<Item = Rgba>) -> Json {
    Json::Array(colors.into_iter().map(color_json).collect())
}

fn color_json(color: Rgba) -> Json {
    Json::Array(color.to_array().into_iter().map(Json::Number).collect())
}

fn opt_color(args: Args<'_>, key: &str) -> Result<Option<Color>, ProbeError> {
    Ok(args.opt_str(key)?.map(Color::from))
}

fn opt_colors(args: Args<'_>, key: &str) -> Result<Option<Vec<Color>>, ProbeError> {
    args.get(key)
        .filter(|json| !matches!(json, Json::Null))
        .map(|json| {
            json.as_array()
                .and_then(|items| items.iter().map(|c| c.as_str().map(Color::from)).collect())
                .ok_or_else(|| malformed(key))
        })
        .transpose()
}

fn multivar_call(args: Args<'_>) -> Outcome {
    let mut cmap = multivar_colormap(args.str("name")?).map_err(ProbeError::rejected)?;
    let (bad_color, under, over) = (
        opt_color(args, "bad")?,
        opt_colors(args, "under")?,
        opt_colors(args, "over")?,
    );
    if bad_color.is_some() || under.is_some() || over.is_some() {
        cmap = cmap
            .with_extremes(bad_color, under, over)
            .map_err(ProbeError::rejected)?;
    }
    let variates = args
        .json("X")?
        .as_array()
        .ok_or_else(|| malformed("X"))?
        .iter()
        .map(values)
        .collect::<Result<Vec<_>, _>>()?;
    let alpha = args.opt_f64("alpha")?;
    let clip = args.opt_bool("clip")?.unwrap_or(true);
    let points = variates.first().map_or(0, Vec::len);
    let mut colors = Vec::with_capacity(points);
    for i in 0..points {
        let point: Vec<Normalized> = variates.iter().map(|v| v[i]).collect();
        colors.push(if clip {
            cmap.lookup(&point, alpha).map_err(ProbeError::rejected)?
        } else {
            let mixed = cmap.mix(&point).map_err(ProbeError::rejected)?;
            let a = alpha.map_or(mixed[3], |alpha| mixed[3] * alpha);
            Rgba::new(mixed[0], mixed[1], mixed[2], a)
        });
    }
    Ok(colors_json(colors))
}

/// The registered bivariate colormap of the probe, with its `shape`,
/// `origin`, `bad` and `outside` arguments applied.
fn bivar(args: Args<'_>) -> Result<BivarColormap, ProbeError> {
    let cmap = bivar_colormap(args.str("name")?).map_err(ProbeError::rejected)?;
    let shape = args
        .opt_str("shape")?
        .map(str::parse::<BivarShape>)
        .transpose()
        .map_err(|_| malformed("shape"))?;
    let origin = match args.get("origin") {
        None | Some(Json::Null) => None,
        Some(_) => match args.f64_vec("origin")?.as_slice() {
            &[a, b] => Some((a, b)),
            _ => return Err(malformed("origin")),
        },
    };
    cmap.with_extremes(
        opt_color(args, "bad")?,
        opt_color(args, "outside")?,
        shape,
        origin,
    )
    .map_err(ProbeError::rejected)
}

fn bivar_call(args: Args<'_>) -> Outcome {
    let cmap = bivar(args)?;
    let (x0, x1) = (values(args.json("X0")?)?, values(args.json("X1")?)?);
    let alpha = args.opt_f64("alpha")?;
    let colors = x0
        .into_iter()
        .zip(x1)
        .map(|(a, b)| cmap.lookup(a, b, alpha))
        .collect::<Result<Vec<_>, _>>()
        .map_err(ProbeError::rejected)?;
    Ok(colors_json(colors))
}

/// A `resampled` size: `null` keeps the axis.
pub(super) fn size(json: &Json) -> Result<Option<isize>, ProbeError> {
    match json {
        Json::Null => Ok(None),
        Json::Integer(n) => isize::try_from(*n)
            .map(Some)
            .map_err(|_| malformed("resampled")),
        _ => Err(malformed("resampled")),
    }
}

/// A pair of sizes.
fn sizes(json: &Json) -> Result<(Option<isize>, Option<isize>), ProbeError> {
    match json.as_array() {
        Some([a, b]) => Ok((size(a)?, size(b)?)),
        _ => Err(malformed("resampled")),
    }
}

fn bivar_lut(args: Args<'_>) -> Outcome {
    let mut cmap = bivar(args)?;
    if let Some(base) = args.get("base").filter(|b| !matches!(b, Json::Null)) {
        cmap = cmap
            .resampled(sizes(base)?, false)
            .map_err(ProbeError::rejected)?;
    }
    if let Some(reversed) = args.get("reversed").and_then(Json::as_array) {
        let flag = |i: usize| {
            reversed
                .get(i)
                .and_then(Json::as_bool)
                .ok_or_else(|| malformed("reversed"))
        };
        cmap = cmap
            .reversed(flag(0)?, flag(1)?)
            .map_err(ProbeError::rejected)?;
    }
    let transposed = args.opt_bool("transposed")?.unwrap_or(false);
    let lut = cmap
        .resampled(sizes(args.json("resampled")?)?, transposed)
        .map_err(ProbeError::rejected)?
        .lut();
    let (rows, cols) = lut.shape();
    Ok(Json::Array(
        (0..rows)
            .map(|i| colors_json((0..cols).map(|j| Rgba::from(lut[(i, j)]))))
            .collect(),
    ))
}

fn component_json(part: &Colormap) -> Json {
    let lut = part.lut().iter().step_by(15).copied();
    Json::Object(vec![
        ("name".into(), Json::String(part.name().to_owned())),
        ("lut".into(), colors_json(lut)),
        ("under".into(), color_json(part.under())),
        ("over".into(), color_json(part.over())),
        ("bad".into(), color_json(part.bad())),
    ])
}

fn bivar_component(args: Args<'_>) -> Outcome {
    let cmap = bivar(args)?;
    let axis = match args.json("axis")? {
        Json::Integer(0) => BivarAxis::First,
        Json::Integer(1) => BivarAxis::Second,
        _ => return Err(malformed("axis")),
    };
    let part = cmap.component(axis).map_err(ProbeError::rejected)?;
    Ok(component_json(&part))
}
