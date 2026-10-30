//! Matrices drawn as images: `pcolorfast`, `spy` and `matshow`'s axis
//! styling.

use pltrs::axes::{Axes, MeshCoords, PcolorFast, PcolorfastOptions, Spy, SpyOptions};
use pltrs::collections::Collection;
use pltrs::image::{AxesImage, Origin, PcolorImage};
use pltrs::lines::Line2D;
use pltrs::numeric::Grid;
use pltrs::rc::Aspect;
use pltrs::spines::SpineSide;

use super::super::image::ImageArray;
use super::super::lines::{LineKwargs, PyMarker};
use super::mapping::{Mapping, colorizer};
use crate::binding::prelude::*;

artist_handle!(PcolorImage);

/// A 2-D float array as a grid.
fn grid(array: &NdArray) -> PyResult<Grid<f64>> {
    match array.shape[..] {
        [rows, cols] => Grid::from_vec(rows, cols, array.data.clone())
            .map_err(|err| PyErr::value_error(err.to_string())),
        _ => Err(PyErr::value_error(format!(
            "expected a 2-D array, got shape {:?}",
            array.shape
        ))),
    }
}

/// Mesh coordinates: one per column or row (1-D), or one per grid point.
fn coords(array: NdArray) -> PyResult<MeshCoords> {
    Ok(if array.shape.len() == 2 {
        MeshCoords::Grid(grid(&array)?)
    } else {
        MeshCoords::Vector(array.into_vec()?)
    })
}

/// What `pcolorfast` made: an image, a `PcolorImage` or a quad mesh (the
/// other two `None`).
type FastArtist = (Option<AxesImage>, Option<PcolorImage>, Option<Collection>);

/// The fastest pseudocolour plot of `values` for its grid (matplotlib's
/// `pcolorfast`): an image for a regular grid, a `PcolorImage` for a
/// rectangular one, a `QuadMesh` for quadrilaterals; `coords` are the
/// optional `(X, Y)`.
pub(super) fn pcolorfast(
    axes: Axes,
    coords_xy: Option<(NdArray, NdArray)>,
    values: ImageArray,
    mapping: Mapping,
    alpha: Option<f64>,
) -> PyResult<FastArtist> {
    let coords_xy = coords_xy
        .map(|(x, y)| Ok::<_, PyErr>((coords(x)?, coords(y)?)))
        .transpose()?;
    let options = PcolorfastOptions {
        colorizer: colorizer(mapping),
        alpha,
        ..PcolorfastOptions::default()
    };
    Ok(match axes.pcolorfast(coords_xy, values.0, options)? {
        PcolorFast::Image(image) => (Some(image), None, None),
        PcolorFast::PcolorImage(image) => (None, Some(image), None),
        PcolorFast::QuadMesh(mesh) => (None, None, Some(mesh)),
    })
}

/// The look of a sparsity plot: `(marker, markersize, aspect, origin,
/// line keywords)`; no marker draws an image.
type SpyLook = (Option<PyMarker>, Option<f64>, Owned, String, LineKwargs);

/// The non-zero entries of `z` (beyond `precision`; matplotlib's `spy`):
/// an image or markers (the other `None`).
pub(super) fn spy(
    axes: Axes,
    z: NdArray,
    precision: f64,
    mapping: Mapping,
    (marker, markersize, aspect_value, origin, line): SpyLook,
) -> PyResult<(Option<AxesImage>, Option<Line2D>)> {
    let aspect = Python::with_gil(|py| {
        let aspect = aspect_value.bind(py);
        if aspect.is_none() {
            return Ok::<_, PyErr>(Aspect::Equal);
        }
        Ok(match aspect.extract::<String>().ok().as_deref() {
            Some("auto") => Aspect::Auto,
            Some(_) => Aspect::Equal,
            None => Aspect::Ratio(aspect.extract()?),
        })
    })?;
    let options = SpyOptions {
        precision,
        marker: marker.map(|marker| marker.marker()),
        markersize,
        aspect,
        origin: if origin == "lower" {
            Origin::Lower
        } else {
            Origin::Upper
        },
        colorizer: colorizer(mapping),
        line: line.0,
    };
    Ok(match axes.spy(&grid(&z)?, options)? {
        Spy::Image(image) => (Some(image), None),
        Spy::Markers(line) => (None, Some(line)),
    })
}

/// Styles the axes for a matrix (matplotlib's `matshow` after its
/// `imshow`): the title nudged up, x ticks labelled on top with marks on
/// both sides, integer tick locators.
pub(super) fn style_matrix_axes(axes: Axes) -> PyResult<()> {
    axes.style_matrix_axes(SpineSide::Top);
    Ok(())
}
