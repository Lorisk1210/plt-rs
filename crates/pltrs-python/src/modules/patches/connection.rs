//! Connection patches (matplotlib's `ConnectionPatch`): arrows between two
//! points, each in its own coordinate system and axes.
//!
//! An end crosses as `((x, y), coords, axes)`: the coordinates as
//! annotations take them ([`PyCoords`]) or `("transform", transform)` for
//! a transform, and the native axes of data and axes coordinates (`None`:
//! the axes the patch is added to).

use pltrs::artist::RawHandle;
use pltrs::coords::Coords;
use pltrs::patches::{ConnectionEnd, ConnectionPatch, Patch, PatchProps};
use pltrs::transforms::Transform;

use super::super::annotation::PyCoords;
use super::super::patch_styles::{PyArrowStyle, PyConnectionStyle};
use crate::binding::prelude::*;

/// The coordinates of an end (see the module documentation).
struct EndCoords(Coords);

impl FromPy<'_> for EndCoords {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok((tag, transform)) = object.extract::<(String, Transform)>()
            && tag == "transform"
        {
            return Ok(Self(Coords::Transform(transform)));
        }
        Ok(Self(object.extract::<PyCoords>()?.0))
    }
}

/// An end of a connection patch (see the module documentation).
struct End(ConnectionEnd);

impl FromPy<'_> for End {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (xy, coords, axes): ((f64, f64), EndCoords, Option<RawHandle>) = object.extract()?;
        Ok(Self(ConnectionEnd::new(
            xy,
            coords.0,
            axes.map(|axes| axes.id()),
        )))
    }
}

/// The styles of a connection patch: `(arrowstyle, connectionstyle,
/// shrinkA, shrinkB, mutation_scale, mutation_aspect)`, the styles as
/// description strings.
type ConnectionStyles = (PyArrowStyle, PyConnectionStyle, f64, f64, f64, f64);

/// A loose connection patch from end `a` to end `b` (matplotlib's
/// `ConnectionPatch(xyA, xyB, coordsA, coordsB, axesA=, axesB=, ...)`).
fn new_connection_patch(
    a: End,
    b: End,
    (arrowstyle, connectionstyle, shrink_a, shrink_b, scale, aspect): ConnectionStyles,
) -> PyResult<Patch<ConnectionPatch>> {
    let mut shape = ConnectionPatch::new(a.0, b.0);
    shape.spec.arrowstyle = arrowstyle.0;
    shape.spec.connectionstyle = connectionstyle.0;
    shape.spec.shrink_a = shrink_a;
    shape.spec.shrink_b = shrink_b;
    shape.spec.mutation_scale = scale;
    shape.spec.mutation_aspect = aspect;
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// The points of the two ends, each in its own coordinates (matplotlib's
/// `xy1` and `xy2`).
fn connection_points(patch: Patch<ConnectionPatch>) -> PyResult<((f64, f64), (f64, f64))> {
    let shape = patch.shape();
    Ok(((shape.a.xy.x, shape.a.xy.y), (shape.b.xy.x, shape.b.xy.y)))
}

/// Replaces the start (`second` false) or the end of a connection patch.
fn set_connection_end(patch: Patch<ConnectionPatch>, second: bool, end: End) -> PyResult<()> {
    patch.update_shape(|shape| {
        if second {
            shape.b = end.0;
        } else {
            shape.a = end.0;
        }
    });
    Ok(())
}

/// Whether the patch is drawn only when its ends are inside their axes:
/// always (`True`), never (`False`) or for ends in data coordinates
/// (`None`; matplotlib's `get_annotation_clip`).
fn get_connection_clip(patch: Patch<ConnectionPatch>) -> PyResult<Option<bool>> {
    Ok(patch.shape().annotation_clip)
}

/// Sets [`get_connection_clip`] (matplotlib's `set_annotation_clip`).
fn set_connection_clip(patch: Patch<ConnectionPatch>, clip: Option<bool>) -> PyResult<()> {
    patch.update_shape(|shape| shape.annotation_clip = clip);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new_connection_patch", new_connection_patch)?;
    module.add("connection_points", connection_points)?;
    module.add("set_connection_end", set_connection_end)?;
    module.add("get_connection_clip", get_connection_clip)?;
    module.add("set_connection_clip", set_connection_clip)
}
