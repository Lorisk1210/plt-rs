//! The `lines.markevery` probe: which data points of a laid-out line get
//! markers.

use plt::figure::Figure;
use plt::lines::{DrawStyle, LineProps, MarkEvery};
use plt::markers::{Marker, MarkerSymbol};

use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// Plots `x`, `y` with circle markers in an axes at `rect`, applies the
/// selection, scales and limits, and reports the selected data indices.
pub(super) fn marker_indices(args: Args<'_>) -> Outcome {
    let selection = decode(args.json("markevery")?)?;
    let [width, height] = numbers(args, "figsize")?;
    let fig = Figure::with_size(width, height, args.f64("dpi")?);
    let ax = fig
        .add_axes(numbers(args, "rect")?)
        .map_err(ProbeError::rejected)?;
    let drawstyle: DrawStyle = args
        .str("drawstyle")?
        .parse()
        .map_err(ProbeError::rejected)?;
    let props = LineProps::default()
        .marker(Marker::Symbol(MarkerSymbol::Circle))
        .drawstyle(drawstyle);
    let line = ax
        .plot_with(args.f64_vec("x")?, args.f64_vec("y")?, props)
        .map_err(ProbeError::rejected)?;
    line.set_markevery(selection);
    ax.set_xscale(args.str("xscale")?)
        .map_err(ProbeError::rejected)?;
    ax.set_yscale(args.str("yscale")?)
        .map_err(ProbeError::rejected)?;
    let [left, right] = numbers(args, "xlim")?;
    let [bottom, top] = numbers(args, "ylim")?;
    ax.set_xlim(left, right)
        .and_then(|ax| ax.set_ylim(bottom, top))
        .map_err(ProbeError::rejected)?;
    fig.layout();
    let indices = line.marker_indices().map_err(ProbeError::rejected)?;
    Ok(Json::Array(
        indices
            .into_iter()
            .map(|i| Json::Integer(i as i64))
            .collect(),
    ))
}

/// A fixed-length list of numbers.
fn numbers<const N: usize>(args: Args<'_>, key: &str) -> Result<[f64; N], ProbeError> {
    args.f64_vec(key)?
        .try_into()
        .map_err(|_| ProbeError::BadArgs(format!("{key} must hold {N} numbers")))
}

/// The tagged `markevery` encoding of the reference suite (JSON cannot
/// tell a tuple from a list); `None` selects every point.
fn decode(spec: &Json) -> Result<Option<MarkEvery>, ProbeError> {
    let kind = spec
        .get("kind")
        .and_then(Json::as_str)
        .ok_or_else(|| bad(spec))?;
    let value = || spec.get("value").ok_or_else(|| bad(spec));
    let selection = match kind {
        "none" => return Ok(None),
        "bool" => every(0, i64::from(value()?.as_bool().ok_or_else(|| bad(spec))?)),
        "int" => every(0, integer(value()?)?),
        "float" => MarkEvery::Distance {
            start: 0.0,
            step: number(value()?)?,
        },
        "start_step" => match value()?.as_array() {
            Some([start, step]) => every(integer(start)?, integer(step)?),
            _ => return Err(bad(spec)),
        },
        "float_pair" => match value()?.as_array() {
            Some([start, step]) => MarkEvery::Distance {
                start: number(start)?,
                step: number(step)?,
            },
            _ => return Err(bad(spec)),
        },
        "tuple" => tuple(value()?, spec.get("types").ok_or_else(|| bad(spec))?)?,
        "slice" => {
            let bound = |key: &str| {
                spec.get(key)
                    .filter(|v| !v.is_null())
                    .map(integer)
                    .transpose()
            };
            MarkEvery::Slice {
                start: bound("start")?,
                stop: bound("stop")?,
                step: bound("step")?,
            }
        }
        "indices" => MarkEvery::Indices(list(value()?, integer)?),
        "mask" => MarkEvery::Mask(list(value()?, |v| v.as_bool().ok_or_else(|| bad(v)))?),
        _ => return Err(bad(spec)),
    };
    Ok(Some(selection))
}

/// matplotlib's tuple rules: `(int, int)` is a stride, `(int | float,
/// float)` a distance; anything else is invalid.
fn tuple(values: &Json, types: &Json) -> Result<MarkEvery, ProbeError> {
    let values = values.as_array().ok_or_else(|| bad(values))?;
    let types = list(types, |t| t.as_str().ok_or_else(|| bad(t)))?;
    match (values, types.as_slice()) {
        ([start, step], ["int", "int"]) => Ok(every(integer(start)?, integer(step)?)),
        ([start, step], [_, "float"]) => Ok(MarkEvery::Distance {
            start: number(start)?,
            step: number(step)?,
        }),
        _ => Err(ProbeError::rejected(format!(
            "markevery tuple {values:?} of types {types:?} is not valid"
        ))),
    }
}

fn every(start: i64, step: i64) -> MarkEvery {
    MarkEvery::Every { start, step }
}

fn list<'a, T>(
    value: &'a Json,
    item: impl Fn(&'a Json) -> Result<T, ProbeError>,
) -> Result<Vec<T>, ProbeError> {
    value
        .as_array()
        .ok_or_else(|| bad(value))?
        .iter()
        .map(item)
        .collect()
}

fn integer(value: &Json) -> Result<i64, ProbeError> {
    value.as_i64().ok_or_else(|| bad(value))
}

fn number(value: &Json) -> Result<f64, ProbeError> {
    value.as_f64().ok_or_else(|| bad(value))
}

fn bad(value: &Json) -> ProbeError {
    ProbeError::BadArgs(format!("malformed markevery encoding {value}"))
}
