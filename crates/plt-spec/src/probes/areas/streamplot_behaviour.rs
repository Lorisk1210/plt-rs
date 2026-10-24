//! Probe operations for the `streamplot_behaviour` area: the grids and
//! settings streamplots accept, and how many streamlines they draw, ported
//! from matplotlib's `test_streamplot.py` (see
//! `spec/reference/probes/test_streamplot_behaviour.py`). Every op runs
//! under the reference configuration.

use plt::axes::Axes;
use plt::figure::Figure;
use plt::numeric::Grid;
use plt::streamplot::{
    StreamColorMap, StreamColors, StreamField, StreamplotOptions, StreamplotSet,
};

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::{bbox, parse_points};
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("streamplot.field", |args| with_rc(args, || field(args))),
    Op::new("streamplot.datalim", |args| with_rc(args, || datalim(args))),
];

fn made<T>(result: Result<T, impl std::fmt::Display>) -> Result<T, ProbeError> {
    result.map_err(ProbeError::rejected)
}

/// A 2-D list as a grid.
fn grid(value: &Json) -> Result<Grid<f64>, ProbeError> {
    let rows: Vec<Vec<f64>> = value
        .as_array()
        .and_then(|rows| rows.iter().map(Json::as_f64_vec).collect())
        .ok_or_else(|| ProbeError::BadArgs(format!("not a 2-D list: {value}")))?;
    made(Grid::from_rows(rows))
}

/// The grid of `x, y`: 1-D coordinates, or a mesh.
fn stream_field(args: Args<'_>) -> Result<StreamField, ProbeError> {
    let (u, v) = (grid(args.json("u")?)?, grid(args.json("v")?)?);
    let (x, y) = (args.json("x")?, args.json("y")?);
    match (x.as_f64_vec(), y.as_f64_vec()) {
        (Some(x), Some(y)) => Ok(StreamField::new(x, y, u, v)),
        _ => made(StreamField::from_mesh(&grid(x)?, &grid(y)?, u, v)),
    }
}

fn options(value: &Json) -> Result<StreamplotOptions, ProbeError> {
    let mut options = StreamplotOptions::default();
    for (key, item) in value.as_object().unwrap_or(&[]) {
        let number = || {
            item.as_f64()
                .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a number")))
        };
        match key.as_str() {
            "integration_max_step_scale" => options.integration_max_step_scale = number()?,
            "integration_max_error_scale" => options.integration_max_error_scale = number()?,
            "density" => {
                let density = number()?;
                options.density = (density, density);
            }
            "start_points" => options.start_points = Some(parse_points(item)?),
            "color" => {
                options.color = Some(StreamColors::Mapped(Box::new(StreamColorMap::new(grid(
                    item,
                )?))));
            }
            other => return Err(ProbeError::BadArgs(format!("unknown option {other}"))),
        }
    }
    Ok(options)
}

/// A streamplot on a fresh axes.
fn streamplot(args: Args<'_>) -> Result<(Axes, StreamplotSet), ProbeError> {
    let field = stream_field(args)?;
    let ax = Figure::new().subplot();
    let set = made(ax.streamplot(&field, options(args.json("options")?)?))?;
    Ok((ax, set))
}

/// The number of streamlines of a streamplot.
fn field(args: Args<'_>) -> Outcome {
    let (_, set) = streamplot(args)?;
    Ok(Json::Integer(
        i64::try_from(set.lines.paths().len()).expect("few lines"),
    ))
}

/// The data limits a streamplot leaves.
fn datalim(args: Args<'_>) -> Outcome {
    let (ax, _) = streamplot(args)?;
    Ok(bbox(&ax.data_lim().bbox))
}
