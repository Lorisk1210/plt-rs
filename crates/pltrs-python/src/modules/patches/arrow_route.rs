//! The ends of a fancy arrow patch (matplotlib's
//! `FancyArrowPatch.set_positions`).

use pltrs::patches::{ArrowRoute, FancyArrowPatch, Patch};
use pltrs_core::Point;

use crate::binding::prelude::*;

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
    module.add("set_arrow_positions", set_arrow_positions)
}
