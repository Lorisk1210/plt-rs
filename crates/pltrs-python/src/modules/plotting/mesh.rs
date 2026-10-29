//! Pseudocolour plots: `pcolor` and `pcolormesh`.

use pltrs::axes::{Axes, Mesh, MeshCoords, MeshValues, PcolorOptions, Shading};
use pltrs::collections::Collection;
use pltrs::image::{ImageError, ScalarField};
use pltrs::numeric::Grid;

use super::mapping::{Mapping, colorizer};
use crate::binding::prelude::*;

/// matplotlib's errors for mesh data: `TypeError` for shapes that do
/// not fit, else `ValueError`.
fn mesh_error(err: ImageError) -> PyErr {
    let message = err.to_string();
    if message.contains("Dimensions of C") || message.contains("shape") {
        PyErr::type_error(message)
    } else {
        PyErr::value_error(message)
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
        colorizer: colorizer(mapping)?,
        shading: shading
            .map(|name| {
                name.parse::<Shading>()
                    .map_err(|err| PyErr::value_error(err.to_string()))
            })
            .transpose()?,
        ..PcolorOptions::default()
    };
    if polygons {
        axes.pcolor(mesh, options)
    } else {
        axes.pcolormesh(mesh, options)
    }
    .map_err(mesh_error)
}
