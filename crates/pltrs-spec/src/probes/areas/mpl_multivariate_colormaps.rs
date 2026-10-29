//! Probe operations for the `mpl_multivariate_colormaps` area:
//! matplotlib's own non-image tests of multivariate and bivariate
//! colormaps that the `multivar` area leaves out (see
//! `spec/reference/probes/test_mpl_multivariate_colormaps.py`).

use pltrs::colors::multivar::{
    BivarAxis, BivarColormap, BivarShape, MultiColormap, MultivarColormap, bivar_colormap,
    multivar_colormap,
};
use pltrs::colors::{Color, Normalized, colormaps};
use pltrs::numeric::{Grid, linspace};
use pltrs_core::Rgba;

use super::mpl_tests::in_test_style;
use super::multivar::{colors_json, size, values};
use crate::json::Json;
use crate::probes::encode::{numbers, object};
use crate::probes::{Args, Op, Outcome, ProbeError, made, malformed};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("MultivarColormap", |args| {
        in_test_style(args, || multivar(args))
    }),
    Op::new("MultivarColormap.__call__", |args| {
        in_test_style(args, || multivar_call(args))
    }),
    Op::new("MultivarColormap.resampled", |args| {
        in_test_style(args, || multivar_resampled(args))
    }),
    Op::new("BivarColormap.__call__", |args| {
        in_test_style(args, || bivar_call(args))
    }),
    Op::new("BivarColormap.lut", |args| {
        in_test_style(args, || lut(args))
    }),
    Op::new("BivarColormap.__getitem__", |args| {
        in_test_style(args, || getitem(args))
    }),
    Op::new("BivarColormap.resampled", |args| {
        in_test_style(args, || bivar_resampled(args))
    }),
    Op::new("BivarColormap.with_extremes", |args| {
        in_test_style(args, || bad_shape(args))
    }),
    Op::new("Colormap.__eq__", |args| {
        in_test_style(args, || equal(args))
    }),
];

/// A colour given by name or as an RGBA list.
fn color(json: &Json) -> Result<Color, ProbeError> {
    match json {
        Json::String(name) => Ok(Color::from(name.as_str())),
        other => match other.as_f64_vec().as_deref() {
            Some(&[r, g, b, a]) => Ok(Color::from(Rgba::new(r, g, b, a))),
            _ => Err(malformed("colour")),
        },
    }
}

/// The values of every variate, one list each.
fn variates(json: &Json) -> Result<Vec<Vec<Normalized>>, ProbeError> {
    json.as_array()
        .ok_or_else(|| malformed("values"))?
        .iter()
        .map(values)
        .collect()
}

/// The values as points: one value per variate each.
fn points(variates: &[Vec<Normalized>]) -> impl Iterator<Item = Vec<Normalized>> + '_ {
    let count = variates.first().map_or(0, Vec::len);
    (0..count).map(move |i| variates.iter().map(|v| v[i]).collect())
}

/// RGBA bytes as JSON.
fn bytes_json(colors: Vec<[u8; 4]>) -> Json {
    Json::Array(
        colors
            .into_iter()
            .map(|rgba| Json::Array(rgba.iter().map(|&b| Json::Integer(i64::from(b))).collect()))
            .collect(),
    )
}

/// `MultivarColormap(colormaps, mode)` of registered colormaps for the
/// probe's values.
fn multivar(args: Args<'_>) -> Outcome {
    let mode = made(args.str("mode")?.parse())?;
    let components = args
        .json("colormaps")?
        .as_array()
        .ok_or_else(|| malformed("colormaps"))?
        .iter()
        .map(|name| {
            made(colormaps::get(
                name.as_str().ok_or_else(|| malformed("colormap"))?,
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let cmap = made(MultivarColormap::new(
        "multivariate colormap",
        components,
        mode,
    ))?;
    call_multivar(&cmap, args)
}

/// `cmap(values, alpha, bytes)`.
fn call_multivar(cmap: &MultivarColormap, args: Args<'_>) -> Outcome {
    let variates = variates(args.json("values")?)?;
    if variates.len() != cmap.n_variates() {
        return Err(ProbeError::rejected(format!(
            "{} variates for a colormap of {}",
            variates.len(),
            cmap.n_variates()
        )));
    }
    let alpha = args.opt_f64("alpha")?;
    if args.opt_bool("bytes")?.unwrap_or(false) {
        let colors = points(&variates)
            .map(|point| made(cmap.lookup_bytes(&point, alpha)))
            .collect::<Result<_, _>>()?;
        return Ok(bytes_json(colors));
    }
    let colors = points(&variates)
        .map(|point| made(cmap.lookup(&point, alpha)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(colors_json(colors))
}

/// A list of colours, one per variate.
fn color_list(json: &Json) -> Result<Vec<Color>, ProbeError> {
    json.as_array()
        .ok_or_else(|| malformed("colours"))?
        .iter()
        .map(color)
        .collect()
}

fn multivar_call(args: Args<'_>) -> Outcome {
    let mut cmap = made(multivar_colormap(args.str("name")?))?;
    if let Some(extremes) = args.get("extremes").filter(|e| !matches!(e, Json::Null)) {
        let extremes = Args::new(extremes);
        cmap = made(cmap.with_extremes(
            extremes.get("bad").map(color).transpose()?,
            extremes.get("under").map(color_list).transpose()?,
            extremes.get("over").map(color_list).transpose()?,
        ))?;
    }
    call_multivar(&cmap, args)
}

/// `cmap.resampled(lutshape)` called on the values, and one of its
/// colormaps at one value.
fn multivar_resampled(args: Args<'_>) -> Outcome {
    let cmap = made(multivar_colormap(args.str("name")?))?;
    let lutshape = match args.json("lutshape")? {
        Json::Array(sizes) => sizes
            .iter()
            .map(|s| {
                size(s)?
                    .map(|n| usize::try_from(n).map_err(|_| malformed("lutshape")))
                    .transpose()
            })
            .collect::<Result<Vec<_>, _>>()?,
        single => vec![size(single)?.map(isize::unsigned_abs)],
    };
    let resampled = made(cmap.resampled(&lutshape))?;
    let called = call_multivar(&resampled, args)?;
    let component = args
        .json("component")?
        .as_array()
        .ok_or_else(|| malformed("component"))?;
    let [index, x] = component else {
        return Err(malformed("component"));
    };
    let index = index
        .as_i64()
        .and_then(|i| usize::try_from(i).ok())
        .ok_or_else(|| malformed("component"))?;
    let part = resampled
        .colormaps()
        .get(index)
        .ok_or_else(|| ProbeError::rejected("no such variate"))?;
    let x = x.as_f64().ok_or_else(|| malformed("component"))?;
    Ok(object([
        ("call", called),
        (
            "component",
            numbers(part.lookup(Normalized::Fraction(x), None).to_array()),
        ),
    ]))
}

/// The tables of `BivarColormapFromImage` the tests build.
fn image(name: &str) -> Result<BivarColormap, ProbeError> {
    let (square, origin) = (BivarShape::Square, (0.0, 0.0));
    made(match name {
        "ramp" => {
            let (red, green) = (linspace(0.0, 1.0, 10), linspace(0.0, 1.0, 12));
            let lut = Grid::from_fn(10, 12, |r, c| [red[r], green[c], 1.0, 1.0]);
            BivarColormap::from_image("from image", lut, square, origin)
        }
        "rgb" => {
            let lut = Grid::from_fn(10, 12, |r, c| [r as f64 / 10.0, c as f64 / 12.0, 1.0]);
            BivarColormap::from_image("from image", lut, square, origin)
        }
        "rgb_u8" => {
            let byte = |fraction: f64| (fraction * 255.0) as u8;
            let lut = Grid::from_fn(10, 12, |r, c| {
                [byte(r as f64 / 10.0), byte(c as f64 / 12.0), 255]
            });
            BivarColormap::from_image("from image", lut, square, origin)
        }
        registered => bivar_colormap(registered),
    })
}

/// A shape name.
fn shape(name: &str) -> Result<BivarShape, ProbeError> {
    made(name.parse::<BivarShape>())
}

/// An `[a, b]` pair.
fn pair(json: &Json) -> Result<(f64, f64), ProbeError> {
    match json.as_f64_vec().as_deref() {
        Some(&[a, b]) => Ok((a, b)),
        _ => Err(malformed("pair")),
    }
}

/// `cmap.with_extremes(**extremes)` for the keys `bad`, `outside`,
/// `shape` and `origin` present.
fn with_extremes(
    cmap: BivarColormap,
    extremes: Option<&Json>,
) -> Result<BivarColormap, ProbeError> {
    let Some(extremes) = extremes.filter(|e| !matches!(e, Json::Null)) else {
        return Ok(cmap);
    };
    let extremes = Args::new(extremes);
    made(cmap.with_extremes(
        extremes.get("bad").map(color).transpose()?,
        extremes.get("outside").map(color).transpose()?,
        extremes.opt_str("shape")?.map(shape).transpose()?,
        extremes.get("origin").map(pair).transpose()?,
    ))
}

/// `cmap(values, alpha, bytes)` of a bivariate colormap.
fn call_bivar(cmap: &BivarColormap, variates: &[Vec<Normalized>], args: Args<'_>) -> Outcome {
    let [x0, x1] = variates else {
        return Err(ProbeError::rejected(format!(
            "{} variates for a bivariate colormap",
            variates.len()
        )));
    };
    let alpha = args.opt_f64("alpha")?;
    let pairs = x0.iter().copied().zip(x1.iter().copied());
    if args.opt_bool("bytes")?.unwrap_or(false) {
        let colors = pairs
            .map(|(a, b)| made(cmap.lookup_bytes(a, b, alpha)))
            .collect::<Result<_, _>>()?;
        return Ok(bytes_json(colors));
    }
    let colors = pairs
        .map(|(a, b)| made(cmap.lookup(a, b, alpha)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(colors_json(colors))
}

fn bivar_call(args: Args<'_>) -> Outcome {
    let cmap = with_extremes(image(args.str("cmap")?)?, args.get("extremes"))?;
    call_bivar(&cmap, &variates(args.json("values")?)?, args)
}

/// Ten slices of the table of `test_bivar_cmap_lut_smooth`.
fn lut(args: Args<'_>) -> Outcome {
    let lut = made(bivar_colormap(args.str("name")?))?.lut();
    let column = |col: usize, channel: usize| numbers((0..256).map(|r| lut[(r, col)][channel]));
    let row = |r: usize, channel: usize| numbers((0..256).map(|col| lut[(r, col)][channel]));
    Ok(Json::Array(vec![
        column(0, 0),
        column(255, 0),
        column(0, 1),
        column(153, 1),
        column(255, 1),
        row(0, 1),
        row(102, 1),
        row(255, 1),
        row(0, 2),
        row(255, 2),
    ]))
}

/// `cmap[axis].colors`, or `[cmap(values), cmap[axis](values[axis])]`.
fn getitem(args: Args<'_>) -> Outcome {
    let mut cmap = made(bivar_colormap(args.str("name")?))?;
    let origin = args.get("origin").filter(|o| !matches!(o, Json::Null));
    cmap = made(cmap.with_extremes(
        None,
        None,
        args.opt_str("shape")?.map(shape).transpose()?,
        origin.map(pair).transpose()?,
    ))?;
    let (axis, index) = match args.i64("axis")? {
        0 => (BivarAxis::First, 0),
        1 => (BivarAxis::Second, 1),
        _ => return Err(malformed("axis")),
    };
    let part = made(cmap.component(axis))?;
    let Some(given) = args.get("values").filter(|v| !matches!(v, Json::Null)) else {
        return Ok(colors_json(part.lut().iter().copied()));
    };
    let variates = variates(given)?;
    let whole = call_bivar(&cmap, &variates, args)?;
    let along = colors_json(variates[index].iter().map(|&v| part.lookup(v, None)));
    Ok(Json::Array(vec![whole, along]))
}

/// The colours of a resampled, reversed or transposed colormap.
fn bivar_resampled(args: Args<'_>) -> Outcome {
    let cmap = made(bivar_colormap(args.str("name")?))?;
    let changed = match args.str("method")? {
        "resampled" => match args.json("lutshape")?.as_array() {
            Some([a, b]) => cmap.resampled((size(a)?, size(b)?), false),
            _ => return Err(malformed("lutshape")),
        },
        "reversed" => cmap.reversed(true, true),
        "transposed" => cmap.transposed(),
        other => return Err(ProbeError::BadArgs(format!("unknown method {other:?}"))),
    };
    call_bivar(&made(changed)?, &variates(args.json("values")?)?, args)
}

/// An unknown shape given to `with_extremes` or to an image colormap.
fn bad_shape(args: Args<'_>) -> Outcome {
    let shape = shape(args.str("shape")?)?;
    if args.bool("image")? {
        made(BivarColormap::from_image(
            "from image",
            Grid::filled(3, 3, [1.0; 4]),
            shape,
            (0.0, 0.0),
        ))?;
    } else {
        made(made(bivar_colormap("BiCone"))?.with_extremes(None, None, Some(shape), None))?;
    }
    Ok(Json::Null)
}

/// A colormap of either kind, as the equality tests describe them.
fn described(json: &Json) -> Result<MultiColormap, ProbeError> {
    let spec = Args::new(json);
    if let Some(components) = spec.get("colormaps") {
        let components = components
            .as_array()
            .ok_or_else(|| malformed("colormaps"))?
            .iter()
            .map(|c| match c.as_array() {
                Some([Json::String(name), index]) => {
                    let index = index
                        .as_i64()
                        .and_then(|i| usize::try_from(i).ok())
                        .ok_or_else(|| malformed("index"))?;
                    made(multivar_colormap(name))?
                        .colormaps()
                        .get(index)
                        .cloned()
                        .ok_or_else(|| malformed("index"))
                }
                _ => made(colormaps::get(
                    c.as_str().ok_or_else(|| malformed("colormap"))?,
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mode = made(spec.str("mode")?.parse())?;
        return Ok(MultiColormap::Multivariate(made(MultivarColormap::new(
            "multivariate colormap",
            components,
            mode,
        ))?));
    }
    let name = spec.str("name")?;
    let bad_color = spec.get("bad").map(color).transpose()?;
    if let Ok(cmap) = multivar_colormap(name) {
        return Ok(MultiColormap::Multivariate(made(
            cmap.with_extremes(bad_color, None, None),
        )?));
    }
    let mut cmap = made(made(bivar_colormap(name))?.with_extremes(
        bad_color,
        spec.get("outside").map(color).transpose()?,
        spec.opt_str("shape")?.map(shape).transpose()?,
        None,
    ))?;
    if let Some(scale) = spec.opt_f64("lut_scale")? {
        let scaled = cmap.lut().map(|rgba| rgba.map(|channel| channel * scale));
        cmap = made(BivarColormap::from_image(
            name,
            scaled,
            cmap.shape(),
            cmap.origin(),
        ))?;
    }
    Ok(MultiColormap::Bivariate(cmap))
}

fn equal(args: Args<'_>) -> Outcome {
    Ok(Json::Bool(
        described(args.json("a")?)? == described(args.json("b")?)?,
    ))
}
