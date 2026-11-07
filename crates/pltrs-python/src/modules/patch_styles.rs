//! `_native.patch_styles`: the geometry of matplotlib's `BoxStyle`,
//! `ConnectionStyle` and `ArrowStyle` (`patches`), done by pltrs (Rust)'s
//! styles.
//!
//! A style of matplotlib's arrives as its description string,
//! `"round,pad=0.2"`, which the Python classes make from their
//! parameters; a style of the program's as the callable pltrs (Rust) calls
//! ([`PyBoxStyle`], [`PyConnectionStyle`], [`PyArrowStyle`]).

use pltrs::patches::{ArrowStyle, BoxShape, BoxStyle, ConnectionStyle, PatchError, StyleError};
use pltrs_core::Point;

use super::archive::{program_arrow, program_box, program_connection, program_route};
use super::path::{PathArrays, PyPath, path_arrays};
use crate::binding::prelude::*;

/// matplotlib's `ValueError` for a bad style description.
pub(super) fn style_error(err: StyleError) -> PyErr {
    PyErr::value_error(err.to_string())
}

/// A box style as Python gives it: the description of one of matplotlib's
/// styles, or a style of the program's, `style(x0, y0, width, height,
/// mutation_size)` returning the box's path.
pub(super) struct PyBoxStyle(pub(super) BoxStyle);

impl FromPy<'_> for PyBoxStyle {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        match object.extract::<String>() {
            Ok(spec) => spec.parse().map(Self).map_err(style_error),
            Err(_) => Ok(Self(BoxStyle::new(BoxShape::Program(program_box(object)?)))),
        }
    }
}

/// A connection style as Python gives it: the description of one of
/// matplotlib's styles, a connection of the program's, `connect(posA,
/// posB)` returning the path between the points, or a route of the
/// program's, `("route", route)`, `route(posA, posB, shrinkA, shrinkB)`
/// returning the path drawn.
pub(super) struct PyConnectionStyle(pub(super) ConnectionStyle);

impl FromPy<'_> for PyConnectionStyle {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(spec) = object.extract::<String>() {
            return spec.parse().map(Self).map_err(style_error);
        }
        if let Ok((kind, route)) = object.extract::<(String, Bound<'_>)>()
            && kind == "route"
        {
            return Ok(Self(ConnectionStyle::ProgramRoute(program_route(&route)?)));
        }
        Ok(Self(ConnectionStyle::Program(program_connection(object)?)))
    }
}

/// An arrow style as Python gives it: the description of one of
/// matplotlib's styles, or an arrow of the program's, `arrow(vertices,
/// codes, mutation_size, linewidth, aspect)` returning the pieces of the
/// arrow.
pub(super) struct PyArrowStyle(pub(super) ArrowStyle);

impl FromPy<'_> for PyArrowStyle {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        match object.extract::<String>() {
            Ok(spec) => spec.parse().map(Self).map_err(style_error),
            Err(_) => Ok(Self(ArrowStyle::Program(program_arrow(object)?))),
        }
    }
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
