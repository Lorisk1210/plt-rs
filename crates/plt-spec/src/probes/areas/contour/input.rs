//! Decoding the arguments of contour probes into plt-rs inputs, as the
//! adapters of `spec/reference/probes/test_contour.py` build them.

use plt::colors::{Color, Extend};
use plt::contour::{ContourNorm, ContourOptions, Levels, Origin, QuadCoords, QuadField};
use plt::numeric::Grid;
use plt::rc::{self, RcParams};
use plt::ticker::{Locator, LogLocator, MaxNLocator, Nbins, NumTicks};
use plt::tri::Triangulation;

use crate::json::Json;
use crate::probes::{Args, ProbeError};

/// Runs `f` under the reference configuration (`matplotlib.testing.setup()`
/// plus `text.hinting = none`) with the probe's `rc` overrides applied, as
/// `matplotlib.rc_context` does around the contour call.
pub(super) fn with_rc<T>(
    args: Args<'_>,
    f: impl FnOnce() -> Result<T, ProbeError>,
) -> Result<T, ProbeError> {
    rc::context(|| {
        rc::replace(RcParams::matplotlib());
        rc::set("font.family", "DejaVu Sans").map_err(ProbeError::rejected)?;
        rc::set("text.hinting", "none").map_err(ProbeError::rejected)?;
        for (key, value) in args.get("rc").and_then(Json::as_object).unwrap_or(&[]) {
            let text = match value {
                Json::String(text) => text.clone(),
                Json::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
                Json::Number(_) | Json::Integer(_) => value.to_string(),
                other => return Err(ProbeError::BadArgs(format!("unsupported rc value {other}"))),
            };
            rc::set(key, &text).map_err(ProbeError::rejected)?;
        }
        f()
    })
}

/// `np.asarray(value, float64)` of a 2-D list (`bool` dtype: `true` is 1).
fn grid(value: &Json, key: &str) -> Result<Grid<f64>, ProbeError> {
    let rows = value
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a list")))?;
    if rows.iter().any(|row| row.as_array().is_none()) {
        return Err(ProbeError::rejected(format!("{key} must be 2-D")));
    }
    let cell = |v: &Json| {
        v.as_f64()
            .or_else(|| v.as_bool().map(|b| f64::from(u8::from(b))))
            .ok_or_else(|| ProbeError::BadArgs(format!("bad {key} value {v}")))
    };
    let rows: Vec<Vec<f64>> = rows
        .iter()
        .map(|row| row.as_array().unwrap_or(&[]).iter().map(cell).collect())
        .collect::<Result<_, _>>()?;
    Grid::from_rows(rows).map_err(ProbeError::rejected)
}

/// A 2-D boolean list.
fn bool_grid(value: &Json, key: &str) -> Result<Grid<bool>, ProbeError> {
    Ok(grid(value, key)?.map(|&v| v != 0.0))
}

/// The quad field of `ax.contour([x, y,] Z, ...)`.
pub(super) fn quad_field(args: Args<'_>) -> Result<QuadField, ProbeError> {
    let z = grid(args.json("z")?, "z")?;
    let mut field = if args.opt_str("z_dtype")? == Some("bool") {
        QuadField::from_bools(&z.map(|&v| v != 0.0))
    } else {
        QuadField::new(z)
    };
    if let Some(mask) = args.get("mask") {
        field = field.with_mask(bool_grid(mask, "mask")?);
    }
    field.coords = coords(args)?;
    field.corner_mask = match args.get("corner_mask") {
        None => None,
        Some(Json::Bool(b)) => Some(*b),
        Some(other) => {
            return Err(ProbeError::rejected(format!(
                "corner_mask {other} is not a bool"
            )));
        }
    };
    if let Some(nchunk) = args.opt_i64("nchunk")? {
        field.nchunk = usize::try_from(nchunk).map_err(ProbeError::rejected)?;
    }
    if let Some(algorithm) = args.opt_str("algorithm")? {
        field.algorithm = Some(algorithm.parse().map_err(ProbeError::rejected)?);
    }
    Ok(field)
}

/// The coordinates: explicit 1-D or 2-D `x` and `y`, or `origin`/`extent`.
fn coords(args: Args<'_>) -> Result<QuadCoords, ProbeError> {
    let Some(x) = args.get("x") else {
        let origin = match args.opt_str("origin")? {
            None => None,
            Some("lower") => Some(Origin::Lower),
            Some("upper") => Some(Origin::Upper),
            Some("image") => Some(Origin::Image),
            Some(other) => return Err(ProbeError::rejected(format!("invalid origin {other:?}"))),
        };
        let extent = match args.get("extent") {
            None => None,
            Some(value) => match value.as_f64_vec().as_deref() {
                Some(&[x0, x1, y0, y1]) => Some([x0, x1, y0, y1]),
                _ => return Err(ProbeError::rejected("extent must be (x0, x1, y0, y1)")),
            },
        };
        return Ok(QuadCoords::Implicit { origin, extent });
    };
    let y = args.json("y")?;
    let two_d = |v: &Json| {
        v.as_array()
            .is_some_and(|rows| rows.iter().any(|r| r.as_array().is_some()))
    };
    match (two_d(x), two_d(y)) {
        (false, false) => Ok(QuadCoords::Axes {
            x: x.as_f64_vec()
                .ok_or_else(|| ProbeError::BadArgs("bad x".into()))?,
            y: y.as_f64_vec()
                .ok_or_else(|| ProbeError::BadArgs("bad y".into()))?,
        }),
        (true, true) => Ok(QuadCoords::Mesh {
            x: grid(x, "x")?,
            y: grid(y, "y")?,
        }),
        _ => Err(ProbeError::rejected(
            "x and y must have the same number of dimensions",
        )),
    }
}

/// `Triangulation(x, y, triangles, mask=tri_mask)` and z.
pub(super) fn tri_field(args: Args<'_>) -> Result<(Triangulation, Vec<f64>), ProbeError> {
    let (x, y) = (args.f64_vec("x")?, args.f64_vec("y")?);
    let triangles = args
        .json("triangles")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("bad triangles".into()))?
        .iter()
        .map(|t| match t.as_f64_vec().as_deref() {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "point indices"
            )]
            Some(&[a, b, c]) => Ok([a as usize, b as usize, c as usize]),
            _ => Err(ProbeError::BadArgs(format!("bad triangle {t}"))),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mask = args
        .get("tri_mask")
        .map(|m| {
            m.as_array()
                .and_then(|items| items.iter().map(Json::as_bool).collect::<Option<Vec<_>>>())
                .ok_or_else(|| ProbeError::BadArgs(format!("bad tri_mask {m}")))
        })
        .transpose()?;
    let tri = Triangulation::new(&x, &y, triangles)
        .and_then(|t| t.with_mask(mask))
        .map_err(ProbeError::rejected)?;
    Ok((tri, args.f64_vec("z")?))
}

/// The keyword arguments passed through to the contour call.
pub(super) fn options(args: Args<'_>) -> Result<ContourOptions, ProbeError> {
    let levels = match args.get("levels") {
        None => Levels::Auto,
        Some(value) if value.is_integer() => Levels::Count(value.as_i64().unwrap_or_default()),
        Some(value) => Levels::Values(
            value
                .as_f64_vec()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad levels {value}")))?,
        ),
    };
    // matplotlib does not validate `extend`: unknown values act as neither.
    let extend = args
        .opt_str("extend")?
        .and_then(|e| e.parse().ok())
        .unwrap_or(Extend::Neither);
    let colors = match args.get("colors") {
        None => None,
        Some(Json::String(color)) => Some(vec![Color::from(color.as_str())]),
        Some(value) => Some(
            value
                .as_array()
                .and_then(|items| {
                    items
                        .iter()
                        .map(|c| c.as_str().map(Color::from))
                        .collect::<Option<Vec<_>>>()
                })
                .ok_or_else(|| ProbeError::BadArgs(format!("bad colors {value}")))?,
        ),
    };
    let norm = match args.opt_str("norm")? {
        None => ContourNorm::Linear,
        Some("log") => ContourNorm::Log,
        Some(other) => return Err(ProbeError::BadArgs(format!("unknown norm {other:?}"))),
    };
    Ok(ContourOptions {
        levels,
        locator: args.get("locator").map(locator).transpose()?,
        extend,
        colors,
        norm,
        vmin: args.opt_f64("vmin")?,
        vmax: args.opt_f64("vmax")?,
        ..ContourOptions::default()
    })
}

/// `ticker.<name>(**kw)` for `{"name": ..., **kw}`.
fn locator(spec: &Json) -> Result<Box<dyn Locator>, ProbeError> {
    let args = Args::new(spec);
    match args.str("name")? {
        "MaxNLocator" => {
            let nbins = args.opt_i64("nbins")?.unwrap_or(10);
            let nbins = usize::try_from(nbins).map_err(ProbeError::rejected)?;
            let mut locator = MaxNLocator::new(Nbins::Fixed(nbins));
            if let Some(steps) = args.get("steps") {
                let steps = steps
                    .as_f64_vec()
                    .ok_or_else(|| ProbeError::BadArgs("bad steps".into()))?;
                locator = locator.with_steps(&steps).map_err(ProbeError::rejected)?;
            }
            if let Some(symmetric) = args.opt_bool("symmetric")? {
                locator = locator.with_symmetric(symmetric);
            }
            Ok(Box::new(locator))
        }
        "LogLocator" => {
            let mut locator = LogLocator::new(10.0).map_err(ProbeError::rejected)?;
            if let Some(numticks) = args.opt_i64("numticks")? {
                let numticks = usize::try_from(numticks).map_err(ProbeError::rejected)?;
                locator = locator.with_numticks(NumTicks::Fixed(numticks));
            }
            Ok(Box::new(locator))
        }
        other => Err(ProbeError::BadArgs(format!("unknown locator {other:?}"))),
    }
}
