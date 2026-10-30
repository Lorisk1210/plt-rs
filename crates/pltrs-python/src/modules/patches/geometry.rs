//! The geometry of annuli, arrows and polygonal circles: matplotlib's
//! `Annulus` getters and setters (`center`, `radii`, `width`, `angle`),
//! `Arrow.set_data`, `FancyArrow.verts` and `FancyArrow.set_data`, and
//! the `RegularPolygon` attributes of a `CirclePolygon`.

use pltrs::patches::{Annulus, Arrow, CirclePolygon, FancyArrow, Patch};
use pltrs_core::Point;

use super::super::transforms::points_array;
use crate::binding::prelude::*;

/// matplotlib's `ValueError` for a ring thicker than its smaller
/// semi-axis.
pub(super) fn annulus_too_wide() -> PyErr {
    PyErr::value_error("Width of annulus must be less than or equal to semi-minor axis")
}

/// `((x, y), (a, b), width, angle)` of an annulus: its centre, outer
/// semi-axes, thickness and rotation in degrees.
type AnnulusFields = ((f64, f64), (f64, f64), f64, f64);

/// An annulus's geometry.
fn annulus(patch: Patch<Annulus>) -> PyResult<AnnulusFields> {
    let shape = patch.shape();
    let center = shape.center();
    Ok((
        (center.x, center.y),
        shape.semi_axes(),
        shape.width(),
        shape.angle(),
    ))
}

/// Changes the given parts of an annulus (`None` keeps a part), in the
/// order of matplotlib's setters; a thickness larger than the smaller
/// semi-axis raises matplotlib's `ValueError` and changes nothing.
fn set_annulus(
    patch: Patch<Annulus>,
    center: Option<(f64, f64)>,
    radii: Option<(f64, f64)>,
    width: Option<f64>,
    angle: Option<f64>,
) -> PyResult<()> {
    let mut shape = patch.shape();
    if let Some(center) = center {
        shape.set_center(center);
    }
    if let Some(radii) = radii {
        shape.set_radii(radii);
    }
    if let Some(width) = width {
        shape.set_width(width).map_err(|_| annulus_too_wide())?;
    }
    if let Some(angle) = angle {
        shape.set_angle(angle);
    }
    patch.set_shape(shape);
    Ok(())
}

/// `(x, y, dx, dy, width)` of an arrow: its tail, its extent to the tip
/// and its width scale.
fn arrow(patch: Patch<Arrow>) -> PyResult<(f64, f64, f64, f64, f64)> {
    let shape = patch.shape();
    Ok((
        shape.tail.x,
        shape.tail.y,
        shape.delta.x,
        shape.delta.y,
        shape.width,
    ))
}

/// Replaces an arrow's tail, extent and width scale (matplotlib's
/// `Arrow.set_data`).
fn set_arrow(
    patch: Patch<Arrow>,
    (x, y): (f64, f64),
    (dx, dy): (f64, f64),
    width: f64,
) -> PyResult<()> {
    patch.set_shape(Arrow {
        tail: Point::new(x, y),
        delta: Point::new(dx, dy),
        width,
    });
    Ok(())
}

/// The vertices of a fancy arrow's polygon, as an `(N, 2)` array
/// (matplotlib's `FancyArrow.verts`).
fn fancy_arrow_verts(patch: Patch<FancyArrow>) -> PyResult<NdArray> {
    let vertices = patch.shape().vertices();
    points_array(&vertices, vec![vertices.len(), 2])
}

/// Changes the given parts of a fancy arrow (`None` keeps a part): its
/// tail and extent `(x, y, dx, dy)` and its `(width, head_width,
/// head_length)` (matplotlib's `FancyArrow.set_data`).
fn set_fancy_arrow(
    patch: Patch<FancyArrow>,
    (x, y, dx, dy): (Option<f64>, Option<f64>, Option<f64>, Option<f64>),
    (width, head_width, head_length): (Option<f64>, Option<f64>, Option<f64>),
) -> PyResult<()> {
    patch.update_shape(|arrow| {
        arrow.tail = Point::new(x.unwrap_or(arrow.tail.x), y.unwrap_or(arrow.tail.y));
        arrow.delta = Point::new(dx.unwrap_or(arrow.delta.x), dy.unwrap_or(arrow.delta.y));
        arrow.width = width.unwrap_or(arrow.width);
        arrow.head_width = head_width.or(arrow.head_width);
        arrow.head_length = head_length.or(arrow.head_length);
    });
    Ok(())
}

/// `(x, y, numvertices, radius, orientation)` of a polygonal circle, as
/// the regular polygon it is (matplotlib's `CirclePolygon` attributes).
fn circle_polygon(patch: Patch<CirclePolygon>) -> PyResult<(f64, f64, usize, f64, f64)> {
    let shape = patch.shape();
    Ok((
        shape.center.x,
        shape.center.y,
        shape.resolution,
        shape.radius,
        0.0,
    ))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("annulus", annulus)?;
    module.add("set_annulus", set_annulus)?;
    module.add("arrow", arrow)?;
    module.add("circle_polygon", circle_polygon)?;
    module.add("fancy_arrow_verts", fancy_arrow_verts)?;
    module.add("set_fancy_arrow", set_fancy_arrow)?;
    module.add("set_arrow", set_arrow)
}
