//! The route of a fancy arrow patch: its ends or its path (matplotlib's
//! `FancyArrowPatch` `_posA_posB`, `_path_original` and
//! `set_positions`).

use pltrs::patches::{ArrowRoute, FancyArrowPatch, Patch};
use pltrs_core::Point;

use super::super::path::{PathArrays, path_arrays};
use crate::binding::prelude::*;

/// The ends of an arrow between two points.
type ArrowEnds = ((f64, f64), (f64, f64));

/// Where the arrow `patch` runs: `(ends, None)` between two points,
/// `(None, path)` along a path.
fn arrow_route(patch: Patch<FancyArrowPatch>) -> PyResult<(Option<ArrowEnds>, Option<PathArrays>)> {
    Ok(match patch.shape().route {
        ArrowRoute::Between(a, b) => (Some(((a.x, a.y), (b.x, b.y))), None),
        ArrowRoute::Along(path) => (None, Some(path_arrays(&path, false)?)),
    })
}

/// Moves the start (`a`) and the end (`b`) of the arrow `patch`, each
/// kept when `None`. An arrow along a path has no ends to move:
/// `TypeError`, as matplotlib raises.
fn set_arrow_positions(
    patch: Patch<FancyArrowPatch>,
    a: Option<(f64, f64)>,
    b: Option<(f64, f64)>,
) -> PyResult<()> {
    let ArrowRoute::Between(start, end) = patch.shape().route else {
        return Err(PyErr::type_error(
            "'NoneType' object does not support item assignment",
        ));
    };
    let point =
        |given: Option<(f64, f64)>, kept: Point| given.map_or(kept, |(x, y)| Point::new(x, y));
    patch.update_shape(|shape| {
        shape.route = ArrowRoute::Between(point(a, start), point(b, end));
    });
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("arrow_route", arrow_route)?;
    module.add("set_arrow_positions", set_arrow_positions)
}
