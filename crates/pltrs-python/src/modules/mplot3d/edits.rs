//! Changes to the 3-D artists of a 3-D axes after they joined it
//! (matplotlib's `set_sort_zpos`, `Poly3DCollection.set_zsort`,
//! `set_verts_and_codes` and `set_3d_properties`, `collection_2d_to_3d` of
//! a collection in the axes) and the contour sets it places in 3-D
//! (`Axes3D.add_contour_set` and `add_contourf_set`).

use std::num::NonZeroUsize;

use pltrs::artist::RawHandle;
use pltrs::collections::Collection;
use pltrs::contour::ContourSet;
use pltrs::mplot3d::{Axes3D, ContourPlacement};
use pltrs_core::PathCode;

use super::art3d::{vertices, zsort};
use super::zdir_axis;
use crate::binding::prelude::*;

/// The position a collection of the axes is ordered by, `None` to order
/// it by its own depth (`set_sort_zpos`).
fn set_sort_zpos(axes: Axes3D, collection: Collection, position: Option<f64>) -> PyResult<()> {
    axes.set_sort_zpos(&collection, position)?;
    Ok(())
}

/// The position a collection of the axes is ordered by, `None` where it
/// is ordered by its own depth (matplotlib's `_sort_zpos`).
fn sort_zpos(axes: Axes3D, collection: Collection) -> PyResult<Option<f64>> {
    Ok(axes.sort_zpos(&collection)?)
}

/// How the polygons of a polygon collection of the axes are ordered:
/// `"average"`, `"min"` or `"max"` (`Poly3DCollection.set_zsort`).
fn set_zsort(axes: Axes3D, collection: Collection, name: String) -> PyResult<()> {
    axes.set_zsort(&collection, zsort(&name)?)?;
    Ok(())
}

/// Replaces the polylines of a line collection of the axes by the `(N, 3)`
/// arrays `segments` (`Line3DCollection.set_segments`).
fn set_segments3d(axes: Axes3D, collection: Collection, segments: Vec<NdArray>) -> PyResult<()> {
    axes.set_segments(&collection, vertices(&segments)?)?;
    Ok(())
}

/// Replaces the polygons of a polygon collection of the axes by the
/// `(N, 3)` arrays `verts`, `closed` or not (`Poly3DCollection.set_verts`).
fn set_poly_verts(
    axes: Axes3D,
    collection: Collection,
    verts: Vec<NdArray>,
    closed: bool,
) -> PyResult<()> {
    axes.set_poly_verts(&collection, vertices(&verts)?, closed)?;
    Ok(())
}

/// Replaces the polygons of a polygon collection of the axes by paths
/// through the `(N, 3)` arrays `verts` with the path codes `codes`
/// (`Poly3DCollection.set_verts_and_codes`).
fn set_poly_verts_and_codes(
    axes: Axes3D,
    collection: Collection,
    verts: Vec<NdArray>,
    codes: Vec<Vec<u8>>,
) -> PyResult<()> {
    let codes = codes
        .into_iter()
        .map(|codes| {
            codes
                .into_iter()
                .map(|code| {
                    PathCode::try_from(code)
                        .map_err(|_| PyErr::value_error(format!("invalid path code {code}")))
                })
                .collect::<PyResult<Vec<_>>>()
        })
        .collect::<PyResult<_>>()?;
    axes.set_poly_verts_and_codes(&collection, vertices(&verts)?, codes)?;
    Ok(())
}

/// Takes the colours a polygon collection of the axes has now as its
/// colours in 3-D, ordered by mean depth
/// (`Poly3DCollection.set_3d_properties`).
fn set_poly_3d_properties(axes: Axes3D, collection: Collection) -> PyResult<()> {
    axes.set_poly_3d_properties(&collection)?;
    Ok(())
}

/// Places the paths of a collection of the axes' projection plane at the
/// 3-D points `verts`, one `(N, 3)` array per path
/// (`collection_2d_to_3d`).
fn lift_paths(
    axes: Axes3D,
    collection: Collection,
    verts: Vec<NdArray>,
    axlim_clip: bool,
) -> PyResult<()> {
    axes.lift_paths(&collection, vertices(&verts)?, axlim_clip)?;
    Ok(())
}

/// Whether a 3-D artist of the axes hides its parts outside the view
/// limits (the `axlim_clip` of `set_3d_properties`).
fn set_axlim_clip(axes: Axes3D, artist: RawHandle, axlim_clip: bool) -> PyResult<()> {
    axes.set_axlim_clip(&artist, axlim_clip)?;
    Ok(())
}

/// The placement of a contour set: `(zdir, offset, axlim_clip)`.
type Placement = (String, Option<f64>, bool);

/// The placement `(zdir, offset, axlim_clip)` describes.
fn placement((zdir, offset, axlim_clip): Placement) -> PyResult<ContourPlacement> {
    Ok(ContourPlacement {
        zdir: zdir_axis(&zdir)?,
        offset,
        axlim_clip,
    })
}

/// Places the line contour set `set` of the axes' projection plane in
/// 3-D, or extends it into ribbons every `stride` points
/// (`Axes3D.add_contour_set`).
fn add_contour_set(
    axes: Axes3D,
    set: ContourSet,
    place: Placement,
    stride: Option<usize>,
) -> PyResult<()> {
    let extend3d = stride
        .map(|stride| {
            NonZeroUsize::new(stride)
                .ok_or_else(|| PyErr::value_error("stride must be a positive integer"))
        })
        .transpose()?;
    axes.add_contour_set(&set, placement(place)?, extend3d)?;
    Ok(())
}

/// Places the paths of the contour set `set` of the axes' projection plane
/// in 3-D, path `i` at `positions[i]` along `zdir` (`collection_2d_to_3d`
/// of `add_contour_set` and `add_contourf_set`).
fn lift_contour_paths(
    axes: Axes3D,
    set: ContourSet,
    (zdir, axlim_clip): (String, bool),
    positions: Vec<f64>,
) -> PyResult<()> {
    axes.lift_contour_paths(&set, zdir_axis(&zdir)?, &positions, axlim_clip)?;
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("lift_contour_paths", lift_contour_paths)?;
    module.add("set_sort_zpos", set_sort_zpos)?;
    module.add("sort_zpos", sort_zpos)?;
    module.add("set_zsort", set_zsort)?;
    module.add("set_poly_verts", set_poly_verts)?;
    module.add("set_segments3d", set_segments3d)?;
    module.add("set_poly_verts_and_codes", set_poly_verts_and_codes)?;
    module.add("set_poly_3d_properties", set_poly_3d_properties)?;
    module.add("lift_paths", lift_paths)?;
    module.add("set_axlim_clip", set_axlim_clip)?;
    module.add("add_contour_set", add_contour_set)
}
