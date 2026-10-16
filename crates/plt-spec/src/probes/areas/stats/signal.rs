//! Windows, detrending, `_stride_windows` and `Axes.acorr` / `Axes.xcorr`.

use plt::mlab::{self, Correlation, Detrend, MlabError, XcorrOptions};
use plt::numeric::Grid;

use super::{floats, grid_json, number, object};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

pub(super) fn window_hanning(args: Args<'_>) -> Outcome {
    Ok(Json::from(mlab::window_hanning(&args.f64_vec("x")?)))
}

pub(super) fn window_none(args: Args<'_>) -> Outcome {
    Ok(Json::from(mlab::window_none(&args.f64_vec("x")?)))
}

/// An array argument of any dimensionality up to two.
enum Array {
    Scalar(f64),
    Vector(Vec<f64>),
    Matrix(Grid<f64>),
}

impl Array {
    fn parse(value: &Json) -> Result<Self, ProbeError> {
        match value {
            Json::Array(items) if items.iter().any(|v| v.as_array().is_some()) => {
                let rows = items.iter().map(floats).collect::<Result<Vec<_>, _>>()?;
                Grid::from_rows(rows)
                    .map(Self::Matrix)
                    .map_err(ProbeError::rejected)
            }
            Json::Array(_) => floats(value).map(Self::Vector),
            v => number(v).map(Self::Scalar),
        }
    }

    const fn ndim(&self) -> usize {
        match self {
            Self::Scalar(_) => 0,
            Self::Vector(_) => 1,
            Self::Matrix(_) => 2,
        }
    }

    fn to_json(&self) -> Json {
        match self {
            Self::Scalar(v) => Json::from(*v),
            Self::Vector(v) => Json::from(v.clone()),
            Self::Matrix(g) => grid_json(g),
        }
    }
}

/// `mlab.detrend`'s `key`: absent, null and the mean aliases subtract the
/// mean.
fn parse_key(key: Option<&str>) -> Result<Detrend, ProbeError> {
    match key {
        None | Some("default" | "constant" | "mean") => Ok(Detrend::Mean),
        Some("linear") => Ok(Detrend::Linear),
        Some("none") => Ok(Detrend::None),
        Some(other) => Err(ProbeError::rejected(format!(
            "unknown detrend key {other:?}"
        ))),
    }
}

fn axis(args: Args<'_>) -> Result<Option<usize>, ProbeError> {
    args.opt_i64("axis")?
        .map(|a| usize::try_from(a).map_err(ProbeError::rejected))
        .transpose()
}

/// matplotlib's `detrend(x, key, axis)` on an array of dimension 0–2.
fn detrend_array(x: &Array, detrend: &Detrend, axis: Option<usize>) -> Result<Array, ProbeError> {
    if let Some(a) = axis
        && a + 1 > x.ndim()
    {
        return Err(ProbeError::rejected(MlabError::AxisOutOfBounds {
            axis: a,
            ndim: x.ndim(),
        }));
    }
    Ok(match x {
        // A scalar minus its own mean (or trend) is zero.
        Array::Scalar(v) => Array::Scalar(if *detrend == Detrend::None { *v } else { 0.0 }),
        Array::Vector(v) => Array::Vector(detrend.apply(v).map_err(ProbeError::rejected)?),
        Array::Matrix(g) => {
            Array::Matrix(mlab::detrend_grid(g, detrend, axis).map_err(ProbeError::rejected)?)
        }
    })
}

pub(super) fn detrend(args: Args<'_>) -> Outcome {
    let x = Array::parse(args.json("x")?)?;
    let key = parse_key(args.opt_str("key")?)?;
    Ok(detrend_array(&x, &key, axis(args)?)?.to_json())
}

pub(super) fn detrend_mean(args: Args<'_>) -> Outcome {
    let x = Array::parse(args.json("x")?)?;
    Ok(detrend_array(&x, &Detrend::Mean, axis(args)?)?.to_json())
}

pub(super) fn detrend_linear(args: Args<'_>) -> Outcome {
    match Array::parse(args.json("y")?)? {
        Array::Matrix(_) => Err(ProbeError::rejected("y cannot have ndim > 1")),
        y => Ok(detrend_array(&y, &Detrend::Linear, None)?.to_json()),
    }
}

pub(super) fn detrend_none(args: Args<'_>) -> Outcome {
    Ok(Array::parse(args.json("x")?)?.to_json())
}

/// An argument that must be a Python `int`.
fn integer(args: Args<'_>, key: &str, default: i64) -> Result<usize, ProbeError> {
    match args.get(key) {
        None => usize::try_from(default).map_err(ProbeError::rejected),
        Some(Json::Integer(n)) => usize::try_from(*n).map_err(ProbeError::rejected),
        Some(other) => Err(ProbeError::rejected(format!(
            "{key} must be an integer, got {other}"
        ))),
    }
}

pub(super) fn stride_windows(args: Args<'_>) -> Outcome {
    let n = integer(args, "n", 0)?;
    let noverlap = integer(args, "noverlap", 0)?;
    let windows =
        mlab::stride_windows(&args.f64_vec("x")?, n, noverlap).map_err(ProbeError::rejected)?;
    Ok(grid_json(&windows))
}

pub(super) fn acorr(args: Args<'_>) -> Outcome {
    let x = args.f64_vec("x")?;
    correlation(args, &x, &x)
}

pub(super) fn xcorr(args: Args<'_>) -> Outcome {
    correlation(args, &args.f64_vec("x")?, &args.f64_vec("y")?)
}

fn correlation(args: Args<'_>, x: &[f64], y: &[f64]) -> Outcome {
    let detrend = match args.opt_str("detrend")?.unwrap_or("none") {
        "none" => Detrend::None,
        "mean" => Detrend::Mean,
        "linear" => Detrend::Linear,
        other => return Err(ProbeError::BadArgs(format!("unknown detrend {other:?}"))),
    };
    let maxlags = if args.contains("maxlags") {
        args.opt_i64("maxlags")?
            .map(|m| usize::try_from(m).map_err(ProbeError::rejected))
            .transpose()?
    } else {
        XcorrOptions::default().maxlags
    };
    let opts = XcorrOptions {
        normed: args.opt_bool("normed")?.unwrap_or(true),
        detrend,
        maxlags,
    };
    let Correlation { lags, values } = mlab::xcorr(x, y, &opts).map_err(ProbeError::rejected)?;
    let vlines = args.opt_bool("usevlines")?.unwrap_or(true);
    Ok(object([
        (
            "lags",
            Json::Array(lags.into_iter().map(Json::Integer).collect()),
        ),
        ("c", Json::from(values)),
        (
            "artist",
            Json::from(if vlines { "LineCollection" } else { "Line2D" }),
        ),
        ("hline", Json::from(vlines)),
        ("marker", if vlines { Json::Null } else { Json::from("o") }),
        (
            "linestyle",
            if vlines {
                Json::Null
            } else {
                Json::from("None")
            },
        ),
    ]))
}
