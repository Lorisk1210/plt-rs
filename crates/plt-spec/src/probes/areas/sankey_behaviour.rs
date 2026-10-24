//! Probe operations for the `sankey_behaviour` area: Sankey diagrams'
//! flows, arrow directions, tips and labels, and the settings they reject,
//! ported from matplotlib's `test_sankey.py` (see
//! `spec/reference/probes/test_sankey_behaviour.py`). Every op runs under
//! the reference configuration.
//!
//! A negative prior or connection index and an orientation other than
//! `-1`, `0` and `1` cannot be expressed with plt-rs's types; the probes
//! giving them are rejected when their arguments are read.

use plt::figure::Figure;
use plt::sankey::{FlowOrientation, PathLengths, Sankey, SankeyOptions, Subdiagram};

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::{object, points};
use crate::probes::{Args, Op, Outcome, ProbeError};
use plt::ticker::LabelFormat;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("sankey.diagram", |args| {
    with_rc(args, || diagram(args))
})];

fn made<T>(result: Result<T, impl std::fmt::Display>) -> Result<T, ProbeError> {
    result.map_err(ProbeError::rejected)
}

fn number(value: &Json, key: &str) -> Result<f64, ProbeError> {
    value
        .as_f64()
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a number: {value}")))
}

/// An index; negative ones are not representable.
fn index(value: &Json) -> Result<usize, ProbeError> {
    value
        .as_i64()
        .and_then(|i| usize::try_from(i).ok())
        .ok_or_else(|| ProbeError::rejected(format!("{value} is not an index")))
}

fn options(spec: &Json) -> Result<SankeyOptions, ProbeError> {
    let mut options = SankeyOptions::default();
    for (key, value) in spec.as_object().unwrap_or(&[]) {
        match key.as_str() {
            "gap" => options.gap = number(value, key)?,
            "radius" => options.radius = number(value, key)?,
            "head_angle" => options.head_angle = number(value, key)?,
            "tolerance" => options.tolerance = number(value, key)?,
            "unit" => options.unit = value.as_str().map(str::to_owned),
            "format" => {
                let template = value.as_str().unwrap_or("%G").to_owned();
                options.format = LabelFormat::Template(template);
            }
            other => return Err(ProbeError::BadArgs(format!("unknown option {other}"))),
        }
    }
    Ok(options)
}

fn orientation(value: &Json) -> Result<FlowOrientation, ProbeError> {
    match value.as_i64() {
        Some(-1) => Ok(FlowOrientation::Bottom),
        Some(0) => Ok(FlowOrientation::Side),
        Some(1) => Ok(FlowOrientation::Top),
        _ => Err(ProbeError::rejected(format!(
            "{value} is not an orientation"
        ))),
    }
}

fn subdiagram(spec: &Json) -> Result<Subdiagram, ProbeError> {
    let mut sub = Subdiagram::default();
    for (key, value) in spec.as_object().unwrap_or(&[]) {
        let list = || {
            value
                .as_array()
                .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a list")))
        };
        match key.as_str() {
            "flows" => {
                sub.flows = value
                    .as_f64_vec()
                    .ok_or_else(|| ProbeError::BadArgs("bad flows".into()))?;
            }
            "labels" => {
                sub.labels = list()?
                    .iter()
                    .map(|label| label.as_str().map(str::to_owned))
                    .collect();
            }
            "orientations" => {
                sub.orientations = list()?.iter().map(orientation).collect::<Result<_, _>>()?;
            }
            "trunklength" => sub.trunklength = number(value, key)?,
            "pathlengths" => {
                sub.pathlengths = match value.as_f64_vec() {
                    Some(lengths) => PathLengths::PerFlow(lengths),
                    None => PathLengths::Inner(number(value, key)?),
                };
            }
            "prior" => sub.prior = Some(index(value)?),
            "connect" => match list()? {
                [from, to] => sub.connect = (index(from)?, index(to)?),
                _ => return Err(ProbeError::BadArgs("connect needs two indices".into())),
            },
            other => return Err(ProbeError::BadArgs(format!("unknown argument {other}"))),
        }
    }
    Ok(sub)
}

/// The diagrams of a Sankey built from `options` and `subdiagrams`.
fn diagram(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let mut sankey = made(Sankey::new(&fig.subplot(), options(args.json("options")?)?))?;
    for spec in args.json("subdiagrams")?.as_array().unwrap_or(&[]) {
        made(sankey.add(subdiagram(spec)?))?;
    }
    let diagrams = made(sankey.finish())?
        .iter()
        .map(|d| {
            let angles = d
                .angles
                .iter()
                .map(|angle| angle.map_or(Json::Null, Json::from))
                .collect();
            object([
                ("flows", Json::from(d.flows.clone())),
                ("angles", Json::Array(angles)),
                ("tips", points(&d.tips)),
                ("text", d.text.text().into()),
                (
                    "texts",
                    Json::Array(d.texts.iter().map(|t| t.text().into()).collect()),
                ),
            ])
        })
        .collect();
    Ok(Json::Array(diagrams))
}
