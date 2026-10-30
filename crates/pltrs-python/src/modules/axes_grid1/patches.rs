//! The patches of `inset_locator` (matplotlib's `BboxPatch`,
//! `BboxConnector`, `BboxConnectorPatch` and `mark_inset`): boxes worked
//! out at every draw, outlined and joined.

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::axes::Axes;
use pltrs::axes_grid1::{
    BboxConnector, BboxConnectorPatch, BboxPatch, BboxSource, Corner, mark_inset,
};
use pltrs::patches::{Patch, PatchProps};
use pltrs::transforms::{Bbox, Transform};

use super::super::patches::PatchKwargs;
use crate::binding::prelude::*;

artist_handle!(
    Patch<BboxPatch>,
    Patch<BboxConnector>,
    Patch<BboxConnectorPatch>
);

/// A box as the Python side describes it: `("axes", axes)` for an axes'
/// box, `("transformed", (x0, y0, x1, y1), transform or None)` for a box in
/// the coordinates of a transform (None: display pixels), or
/// `("view_limits", axes, transform)` for an axes' view limits in the
/// coordinates of a transform.
struct PySource(BboxSource);

impl FromPy<'_> for PySource {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let tag: String = object
            .call_method("__getitem__", &[0_i64.into_py(object.py())?])?
            .extract()?;
        Ok(Self(match tag.as_str() {
            "axes" => BboxSource::axes(&object.extract::<(String, Axes)>()?.1),
            "transformed" => {
                let (_, (x0, y0, x1, y1), transform): (
                    String,
                    (f64, f64, f64, f64),
                    Option<Transform>,
                ) = object.extract()?;
                BboxSource::transformed(
                    Bbox::from_extents(x0, y0, x1, y1),
                    transform.unwrap_or(Transform::Display),
                )
            }
            "view_limits" => {
                let (_, axes, transform): (String, Axes, Transform) = object.extract()?;
                BboxSource::view_limits(&axes, transform)
            }
            _ => return Err(PyErr::type_error(format!("unknown box {tag:?}"))),
        }))
    }
}

/// The corner of matplotlib's code `code` (1 upper right, 2 upper left, 3
/// lower left, 4 lower right).
fn corner(code: u8) -> PyResult<Corner> {
    Corner::from_code(code).ok_or_else(|| {
        PyErr::value_error(format!(
            "{code} is not a valid corner; supported values are 1, 2, 3, 4"
        ))
    })
}

/// A loose outline of `bbox` (`BboxPatch(bbox)`).
fn bbox_patch(bbox: PySource) -> PyResult<Patch<BboxPatch>> {
    Ok(Patch::loose(BboxPatch::new(bbox.0), PatchProps::default()))
}

/// A loose line from corner `loc1` of `bbox1` to corner `loc2` of `bbox2`
/// (`BboxConnector(bbox1, bbox2, loc1, loc2)`).
fn bbox_connector(
    bbox1: PySource,
    bbox2: PySource,
    loc1: u8,
    loc2: Option<u8>,
) -> PyResult<Patch<BboxConnector>> {
    let shape = BboxConnector::new(
        bbox1.0,
        bbox2.0,
        corner(loc1)?,
        loc2.map(corner).transpose()?,
    );
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// A loose quadrilateral between `bbox1` and `bbox2`
/// (`BboxConnectorPatch(bbox1, bbox2, loc1a, loc2a, loc1b, loc2b)`).
fn bbox_connector_patch(
    bbox1: PySource,
    bbox2: PySource,
    (loc1a, loc2a, loc1b, loc2b): (u8, u8, u8, u8),
) -> PyResult<Patch<BboxConnectorPatch>> {
    let shape = BboxConnectorPatch {
        bbox1: bbox1.0,
        bbox2: bbox2.0,
        loc1a: corner(loc1a)?,
        loc2a: corner(loc2a)?,
        loc1b: corner(loc1b)?,
        loc2b: corner(loc2b)?,
    };
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// Outlines the area of `parent` that `inset` shows and joins corners
/// `loc1` and `loc2` of the inset to it (`mark_inset`); returns the
/// outline and the two lines.
fn mark(
    parent: Axes,
    inset: Axes,
    (loc1, loc2): (u8, u8),
    props: PatchKwargs,
) -> PyResult<(RawHandle, RawHandle, RawHandle)> {
    let marks = mark_inset(&parent, &inset, corner(loc1)?, corner(loc2)?, props.0);
    let [first, second] = marks.connectors;
    Ok((
        marks.area.raw().clone(),
        first.raw().clone(),
        second.raw().clone(),
    ))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("bbox_patch", bbox_patch)?;
    module.add("bbox_connector", bbox_connector)?;
    module.add("bbox_connector_patch", bbox_connector_patch)?;
    module.add("mark_inset", mark)
}
