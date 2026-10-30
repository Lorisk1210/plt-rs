//! Pseudocolour plots: `pcolor` and `pcolormesh`.

use pltrs::axes::{Axes, Mesh, MeshCoords, MeshValues, PcolorOptions, RgbaMesh, Shading};
use pltrs::collections::Collection;
use pltrs::colors::Rgba;
use pltrs::image::{ImageError, ScalarField};
use pltrs::numeric::Grid;

use super::mapping::{Mapping, colorizer};
use crate::binding::prelude::*;

/// matplotlib's errors for mesh data: `TypeError` for shapes that do
/// not fit (the mesh checks of `From<ImageError>`, and arrays whose
/// shapes differ), else `ValueError`.
fn mesh_error(err: ImageError) -> PyErr {
    match err {
        ImageError::ShapeMismatch { .. } => PyErr::type_error(err.to_string()),
        other => other.into(),
    }
}

/// A 2-D float array as a grid.
fn grid(array: &NdArray) -> PyResult<Grid<f64>> {
    match array.shape[..] {
        [rows, cols] => Grid::from_vec(rows, cols, array.data.clone())
            .map_err(|err| PyErr::value_error(err.to_string())),
        _ => Err(PyErr::type_error(format!(
            "Dimensions of C must be 2D, not {}D",
            array.shape.len()
        ))),
    }
}

/// Coordinates: a 1-D array of positions, or a 2-D grid of them.
fn coords(array: NdArray) -> PyResult<MeshCoords> {
    Ok(if array.shape.len() == 2 {
        MeshCoords::Grid(grid(&array)?)
    } else {
        MeshCoords::Vector(array.into_vec()?)
    })
}

/// A shading by name (`None` for `pcolor.shading`).
fn shading(name: Option<String>) -> PyResult<Option<Shading>> {
    name.map(|name| {
        name.parse::<Shading>()
            .map_err(|err| PyErr::value_error(err.to_string()))
    })
    .transpose()
}

/// The colour-mapped mesh of `values` (`NaN` masked) at the optional
/// coordinates `x` and `y` (matplotlib's `pcolormesh`, or `pcolor` when
/// `polygons`), shaded `"flat"`, `"nearest"`, `"gouraud"` or `"auto"`
/// (`None` for `pcolor.shading`).
pub(super) fn pcolor(
    axes: Axes,
    polygons: bool,
    values: NdArray,
    (x, y): (Option<NdArray>, Option<NdArray>),
    shading: Option<String>,
    mapping: Mapping,
) -> PyResult<Collection> {
    let values = MeshValues::Scalar(ScalarField::new(grid(&values)?));
    let mesh = match (x, y) {
        (Some(x), Some(y)) => Mesh::from_coords(coords(x)?, coords(y)?, values),
        _ => Mesh::new(values),
    };
    let options = PcolorOptions {
        colorizer: colorizer(mapping),
        shading: self::shading(shading)?,
        ..PcolorOptions::default()
    };
    if polygons {
        axes.pcolor(mesh, options)
    } else {
        axes.pcolormesh(mesh, options)
    }
    .map_err(mesh_error)
}

/// The mesh painted in the RGBA `colors` (an `(M, N, 4)` array) at the
/// optional coordinates `x` and `y` (matplotlib's `pcolormesh` of an RGB
/// or RGBA array), shaded as [`pcolor`] is.
pub(super) fn pcolormesh_rgba(
    axes: Axes,
    colors: NdArray,
    (x, y): (Option<NdArray>, Option<NdArray>),
    shading: Option<String>,
) -> PyResult<Collection> {
    let [rows, cols, 4] = colors.shape[..] else {
        return Err(PyErr::type_error(format!(
            "an RGBA mesh is an (M, N, 4) array, not one of shape {:?}",
            colors.shape
        )));
    };
    let cells = colors
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, a]| Rgba::new(r, g, b, a))
        .collect();
    let colors =
        Grid::from_vec(rows, cols, cells).map_err(|err| PyErr::value_error(err.to_string()))?;
    let mesh = match (x, y) {
        (Some(x), Some(y)) => RgbaMesh::from_coords(coords(x)?, coords(y)?, colors),
        _ => RgbaMesh::new(colors),
    };
    let options = PcolorOptions {
        shading: self::shading(shading)?,
        ..PcolorOptions::default()
    };
    axes.pcolormesh_rgba(mesh, options).map_err(mesh_error)
}
