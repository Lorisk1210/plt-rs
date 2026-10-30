//! Contours in 3-D (matplotlib's `Axes3D.contour`, `contourf`,
//! `tricontour` and `tricontourf`), whose levels and look the Python side
//! sorted out as for 2-D contours.

use std::num::NonZeroUsize;

use pltrs::contour::ContourSet;
use pltrs::mplot3d::{Axes3D, Contour3DOptions};
use pltrs::tri::Triangulation;

use super::super::contour::{Extras, Look, contour_options};
use super::axis3;
use super::plotting::grid;
use crate::binding::prelude::*;

/// Where the contours go: `(zdir, offset, stride, axlim_clip)`, the
/// stride (`None`: no extension) extending line contours into ribbons
/// (`extend3d=True`).
type Placement = (String, Option<f64>, Option<usize>, bool);

/// The 3-D options of the contours `placement` places, with the 2-D
/// options of the requested levels and look.
fn options(
    (zdir, offset, stride, axlim_clip): Placement,
    requested: Option<NdArray>,
    look: Look,
    extras: Extras,
) -> PyResult<Contour3DOptions> {
    let extend3d = stride
        .map(|stride| {
            NonZeroUsize::new(stride)
                .ok_or_else(|| PyErr::value_error("stride must be a positive integer"))
        })
        .transpose()?;
    Ok(Contour3DOptions {
        zdir: axis3(&zdir)?,
        offset,
        extend3d,
        axlim_clip,
        contour: contour_options(requested, look, extras)?,
    })
}

/// Line (or, `filled`, filled) contours of the surface `(x, y, z)`, 2-D
/// grids of one shape (matplotlib's `Axes3D.contour` and `contourf`).
fn contour(
    axes: Axes3D,
    filled: bool,
    (x, y, z): (NdArray, NdArray, NdArray),
    placement: Placement,
    requested: Option<NdArray>,
    look: Look,
    extras: Extras,
) -> PyResult<ContourSet> {
    let (x, y, z) = (grid(&x, "X")?, grid(&y, "Y")?, grid(&z, "Z")?);
    let options = options(placement, requested, look, extras)?;
    Ok(if filled {
        axes.contourf(&x, &y, &z, options)?
    } else {
        axes.contour(&x, &y, &z, options)?
    })
}

/// Line (or, `filled`, filled) contours of `z` on the triangulation
/// `tri` (matplotlib's `Axes3D.tricontour` and `tricontourf`).
fn tricontour(
    axes: Axes3D,
    filled: bool,
    (tri, z): (Triangulation, NdArray),
    placement: Placement,
    requested: Option<NdArray>,
    look: Look,
    extras: Extras,
) -> PyResult<ContourSet> {
    let z = z.into_vec()?;
    let options = options(placement, requested, look, extras)?;
    Ok(if filled {
        axes.tricontourf(&tri, &z, options)?
    } else {
        axes.tricontour(&tri, &z, options)?
    })
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("contour", contour)?;
    module.add("tricontour", tricontour)
}
