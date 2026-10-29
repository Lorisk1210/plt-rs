//! `_native.patheffects`: the geometry of path effects (matplotlib's
//! `patheffects`), computed by `pltrs::patheffects`.

use pltrs::patheffects::TickedStroke;

use super::path::{PathArrays, path_arrays};
use super::transforms::points;
use crate::binding::prelude::*;

/// The ticks `TickedStroke` draws along a polyline (display pixels):
/// `spacing` points apart (`spacing_px` pixels), at `angle` degrees to the
/// line, `length` times the spacing long; `None` for a single point.
fn ticks(
    polyline: NdArray,
    (spacing, angle, length): (f64, f64, f64),
    spacing_px: f64,
) -> PyResult<Option<PathArrays>> {
    let stroke = TickedStroke {
        spacing,
        angle,
        length,
        ..TickedStroke::default()
    };
    stroke
        .ticks(&points(&polyline)?, spacing_px)
        .map(|path| path_arrays(&path, false))
        .transpose()
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("ticks", ticks)
}
