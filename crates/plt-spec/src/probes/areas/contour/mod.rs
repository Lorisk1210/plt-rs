//! Probe operations for the `contour` area: quad and triangular contour
//! sets (levels, bands, colours, styles and canonical geometry), their raw
//! paths, `clabel` placement, the nearest contour point, and labels
//! added and removed after `clabel`.
//!
//! Every operation builds a fresh `Figure()` with one subplot under the
//! reference configuration, as `spec/reference/probes/test_contour.py`
//! documents.

mod canon;
mod clabel;
mod edits;
mod input;

use plt::contour::{ContourError, ContourSet};
use plt::figure::Figure;
use plt::prelude::Axes;

use crate::json::Json;
use crate::probes::encode;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Axes.contour", |args| summary(args, Grid::Quad, false)),
    Op::new("Axes.contourf", |args| summary(args, Grid::Quad, true)),
    Op::new("Axes.tricontour", |args| summary(args, Grid::Tri, false)),
    Op::new("Axes.tricontourf", tricontourf),
    Op::new("ContourSet.get_paths", raw_paths),
    Op::new("ContourSet.clabel", clabel::clabel),
    Op::new(
        "ContourSet.find_nearest_contour",
        edits::find_nearest_contour,
    ),
    Op::new("ContourSet.label_edits", edits::label_edits),
];

/// Which contour function a probe calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Grid {
    Quad,
    Tri,
}

/// The contour set of a probe on a fresh default axes.
fn contour_set(args: Args<'_>, grid: Grid, filled: bool) -> Result<(Axes, ContourSet), ProbeError> {
    input::with_rc(args, || {
        let ax = Figure::new().subplot();
        let options = input::options(args)?;
        let cs = match (grid, filled) {
            (Grid::Quad, false) => ax.contour(&input::quad_field(args)?, options),
            (Grid::Quad, true) => ax.contourf(&input::quad_field(args)?, options),
            (Grid::Tri, _) => {
                let (tri, z) = input::tri_field(args)?;
                if filled {
                    ax.tricontourf(&tri, &z, options)
                } else {
                    ax.tricontour(&tri, &z, options)
                }
            }
        }
        .map_err(|err: ContourError| ProbeError::rejected(err))?;
        Ok((ax, cs))
    })
}

/// The summary dict of a contour set (`_summary` in the spec).
fn summary(args: Args<'_>, grid: Grid, filled: bool) -> Outcome {
    let (ax, cs) = contour_set(args, grid, filled)?;
    let (vmin, vmax) = cs.norm_limits();
    let (x0, x1) = ax.xlim();
    let (y0, y1) = ax.ylim();
    let colors = Json::Array(cs.colors().into_iter().map(encode::rgba).collect());
    let mut entries = vec![
        ("levels", Json::from(cs.levels())),
        ("layers", Json::from(cs.layers())),
        ("zmin", Json::from(cs.zmin())),
        ("zmax", Json::from(cs.zmax())),
        ("logscale", Json::from(cs.logscale())),
        (
            "norm",
            Json::Array(vec![Json::from(vmin), Json::from(vmax)]),
        ),
        ("xlim", Json::from(vec![x0, x1])),
        ("ylim", Json::from(vec![y0, y1])),
        ("colors", colors),
    ];
    if filled {
        let bands = cs
            .bands()
            .into_iter()
            .map(|(lo, hi)| Json::from(vec![lo, hi]))
            .collect();
        entries.push(("bands", Json::Array(bands)));
    } else {
        let styles = cs
            .dash_patterns()
            .into_iter()
            .map(|dashes| Json::Array(vec![Json::from(dashes.offset), Json::from(dashes.pattern)]))
            .collect();
        entries.push(("linewidths", Json::from(cs.linewidths())));
        entries.push(("linestyles", Json::Array(styles)));
    }
    let warnings = cs
        .warnings()
        .iter()
        .map(|w| Json::from(vec![Json::from("UserWarning"), Json::from(w.to_string())]))
        .collect();
    entries.push(("warnings", Json::Array(warnings)));
    entries.push((
        "paths",
        Json::Array(cs.paths().iter().map(canon::canonical).collect()),
    ));
    Ok(Json::Object(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    ))
}

/// The filled contours of a triangulation (`tricontourf_op` in the spec).
pub(crate) fn tricontourf(args: Args<'_>) -> Outcome {
    summary(args, Grid::Tri, true)
}

/// The raw paths of `func(**args)` (`raw_paths_op` in the spec).
pub(crate) fn raw_paths(args: Args<'_>) -> Outcome {
    let (grid, filled) = match args.str("func")? {
        "contour" => (Grid::Quad, false),
        "contourf" => (Grid::Quad, true),
        "tricontour" => (Grid::Tri, false),
        "tricontourf" => (Grid::Tri, true),
        other => return Err(ProbeError::BadArgs(format!("unknown func {other:?}"))),
    };
    let (_, cs) = contour_set(args, grid, filled)?;
    Ok(Json::Array(cs.paths().iter().map(encode::path).collect()))
}
