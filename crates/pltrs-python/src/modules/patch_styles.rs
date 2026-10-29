//! `_native.patch_styles`: the geometry of matplotlib's `BoxStyle`,
//! `ConnectionStyle` and `ArrowStyle` (`patches`), done by pltrs (Rust)'s
//! styles.
//!
//! A style arrives as matplotlib's description string, `"round,pad=0.2"`,
//! which the Python classes make from their parameters.

use pltrs::patches::{ArrowStyle, BoxStyle, ConnectionStyle, PatchError, StyleError};
use pltrs_core::Point;

use super::path::{PathArrays, PyPath, path_arrays};
use crate::binding::prelude::*;

/// matplotlib's `ValueError` for a bad style description.
pub(super) fn style_error(err: StyleError) -> PyErr {
    PyErr::value_error(err.to_string())
}

/// `ValueError` for geometry a style cannot build.
fn geometry_error(err: PatchError) -> PyErr {
    PyErr::value_error(err.to_string())
}

/// The outline the box style `spec` draws around the rectangle `(x0,
/// y0, width, height)`, scaled by `mutation_size`.
fn box_path(
    spec: String,
    (x0, y0, width, height): (f64, f64, f64, f64),
    mutation_size: f64,
) -> PyResult<PathArrays> {
    let style: BoxStyle = spec.parse().map_err(style_error)?;
    path_arrays(&style.path(x0, y0, width, height, mutation_size), false)
}

/// The path the connection style `spec` draws from `a` to `b`.
fn connection_path(
    spec: String,
    (xa, ya): (f64, f64),
    (xb, yb): (f64, f64),
) -> PyResult<PathArrays> {
    let style: ConnectionStyle = spec.parse().map_err(style_error)?;
    path_arrays(
        &style
            .connect(Point::new(xa, ya), Point::new(xb, yb))
            .map_err(geometry_error)?,
        false,
    )
}

/// The pieces the arrow style `spec` draws along `path`, each with
/// whether it is filled.
fn arrow_paths(
    spec: String,
    path: PyPath,
    mutation_size: f64,
    linewidth: f64,
    aspect: f64,
) -> PyResult<Vec<(PathArrays, bool)>> {
    let style: ArrowStyle = spec.parse().map_err(style_error)?;
    style
        .paths(&path.path, mutation_size, linewidth, aspect)
        .map_err(geometry_error)?
        .iter()
        .map(|(piece, filled)| Ok((path_arrays(piece, false)?, *filled)))
        .collect()
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("box_path", box_path)?;
    module.add("connection_path", connection_path)?;
    module.add("arrow_paths", arrow_paths)
}
