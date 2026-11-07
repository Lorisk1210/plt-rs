//! What a 3-D axes keeps of its last draw (`pltrs::mplot3d`'s drawn
//! projection): matplotlib's `Axes3D.M` and `invM`, the mask of the 2-D
//! data a draw gives a `Line3D`, and the depths that the patches and
//! collections of the program's classes projecting themselves return.

use pltrs::artist::RawHandle;
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

/// Has `artist`, a patch or collection of a class of the program's
/// projecting itself (`do_3d_projection`), ordered by the depths its
/// projection returns.
fn project_by_program(axes: Axes3D, artist: RawHandle) -> PyResult<()> {
    axes.project_by_program(&artist)?;
    Ok(())
}

/// Takes the depth the projection of `artist` returned for the draw
/// coming.
fn set_program_depth(axes: Axes3D, artist: RawHandle, depth: f64) -> PyResult<()> {
    axes.set_program_depth(&artist, depth)?;
    Ok(())
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("project_by_program", project_by_program)?;
    module.add("set_program_depth", set_program_depth)?;
    module.add("drawn_proj", drawn_proj)?;
    module.add("set_drawn_proj", set_drawn_proj)?;
    module.add("drawn_proj_inverse", drawn_proj_inverse)?;
    module.add("set_drawn_proj_inverse", set_drawn_proj_inverse)?;
    module.add("projected_line_mask", projected_line_mask)
}
