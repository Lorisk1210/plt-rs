//! What a 3-D axes keeps of its last draw (`pltrs::mplot3d`'s drawn
//! projection): matplotlib's `Axes3D.M` and `invM`, and the mask of the
//! 2-D data a draw gives a `Line3D`.

use pltrs::lines::Line2D;
use pltrs::mplot3d::Axes3D;

use super::art3d::{array_matrix, matrix_array};
use crate::binding::prelude::*;

/// The projection matrix of the last draw, `None` before it (`M`).
fn drawn_proj(axes: Axes3D) -> PyResult<Option<NdArray>> {
    axes.drawn_proj().as_ref().map(matrix_array).transpose()
}

/// Replaces the projection matrix of the last draw (`ax.M = ...`).
fn set_drawn_proj(axes: Axes3D, matrix: Option<NdArray>) -> PyResult<()> {
    axes.set_drawn_proj(matrix.as_ref().map(array_matrix).transpose()?);
    Ok(())
}

/// The inverse of the projection matrix of the last draw (`invM`).
fn drawn_proj_inverse(axes: Axes3D) -> PyResult<Option<NdArray>> {
    axes.drawn_proj_inverse()
        .as_ref()
        .map(matrix_array)
        .transpose()
}

/// Replaces the inverse of the projection matrix of the last draw
/// (`ax.invM = ...`).
fn set_drawn_proj_inverse(axes: Axes3D, inverse: Option<NdArray>) -> PyResult<()> {
    axes.set_drawn_proj_inverse(inverse.as_ref().map(array_matrix).transpose()?);
    Ok(())
}

/// Which points of the 3-D line `line` its last draw masked, `None` until
/// a draw projected it (`Line3D.draw`'s masked data).
fn projected_line_mask(axes: Axes3D, line: Line2D) -> PyResult<Option<Vec<bool>>> {
    Ok(axes.projected_line_mask(&line)?)
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("drawn_proj", drawn_proj)?;
    module.add("set_drawn_proj", set_drawn_proj)?;
    module.add("drawn_proj_inverse", drawn_proj_inverse)?;
    module.add("set_drawn_proj_inverse", set_drawn_proj_inverse)?;
    module.add("projected_line_mask", projected_line_mask)
}
