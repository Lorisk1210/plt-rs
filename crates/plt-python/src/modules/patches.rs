//! `_native.patches`: shapes with a face and an edge (matplotlib's
//! `patches`: the rectangles `bar` makes).

use plt::patches::{Patch, Rectangle};

use crate::binding::prelude::*;

artist_handle!(Patch<Rectangle>);

/// `(x, y, width, height)` of a rectangle.
fn get_bounds(rectangle: Patch<Rectangle>) -> PyResult<(f64, f64, f64, f64)> {
    let shape = rectangle.shape();
    Ok((shape.xy.x, shape.xy.y, shape.width, shape.height))
}

/// The face colour as an RGBA tuple.
fn get_facecolor(rectangle: Patch<Rectangle>) -> PyResult<(f64, f64, f64, f64)> {
    let face = rectangle.colors()?.face;
    Ok((face.r, face.g, face.b, face.a))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_bounds", get_bounds)?;
    module.add("get_facecolor", get_facecolor)
}
