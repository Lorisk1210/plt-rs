//! The boxes of an added box as Python has them now
//! (`pltrs::offsetbox::rebuild_tree`): matplotlib reads them again at
//! every draw, so Python hands a changed tree again (`rebuild_tree`), and
//! a box inside the tree answers matplotlib's `get_bbox` and `get_offset`
//! through the box added with it and its index in its tree, in tree order
//! (`node_extent`, `node_origin`).

use pltrs::artist::RawHandle;
use pltrs::offsetbox::{
    OffsetBoxError, node_extent, node_origin, rebuild_tree, text_area_baseline_shift,
    text_area_bbox,
};
use pltrs::transforms::Bbox;

use super::super::figure::draw_exception;
use super::tree::PyOffsetBox;
use crate::binding::prelude::*;

/// The error for `owner`, which is no box added with a tree.
fn not_an_owner() -> PyErr {
    PyErr::type_error("not an offset box added to an axes or a figure")
}

/// The bounds `(x0, y0, width, height)` of `bbox`.
fn bounds(bbox: Bbox) -> (f64, f64, f64, f64) {
    (bbox.x0(), bbox.y0(), bbox.width(), bbox.height())
}

/// Hands the added box `owner` its tree `tree` again (see the module
/// documentation): the handles of the tree's artists in tree order.
fn rebuild(owner: RawHandle, tree: PyOffsetBox) -> PyResult<Vec<RawHandle>> {
    Ok(rebuild_tree(&owner, tree.0).ok_or_else(not_an_owner)??)
}

/// The exception matplotlib raises measuring a box that fails with `err`:
/// the kind its draw would meet, else a `ValueError`.
fn measure_error(err: &OffsetBoxError) -> PyErr {
    let kind = draw_exception(err.draw_kind()).unwrap_or(ExceptionKind::ValueError);
    PyErr::new(kind, err.to_string())
}

/// The bounds of the box `index` of the tree of the added box `owner`,
/// relative to its origin (matplotlib's `get_bbox`).
fn extent(owner: RawHandle, index: usize) -> PyResult<(f64, f64, f64, f64)> {
    let extent = node_extent(&owner, index)
        .ok_or_else(|| PyErr::value_error(format!("the box has no box {index}")))?
        .map_err(|err| measure_error(&err))?;
    Ok(bounds(extent))
}

/// Where the box `index` of the tree of the added box `owner` has its
/// origin, in display pixels, as its owner lays it out now (matplotlib's
/// `get_offset` once the parent placed it); None when it is not placed.
fn origin(owner: RawHandle, index: usize) -> PyResult<Option<(f64, f64)>> {
    let origin = node_origin(&owner, index)
        .ok_or_else(|| PyErr::value_error(format!("the box has no box {index}")))?
        .map_err(|err| PyErr::value_error(err.to_string()))?;
    Ok(origin.map(|origin| (origin.x, origin.y)))
}

/// The bounds of a text area holding `text`, relative to its origin
/// (matplotlib's `TextArea.get_bbox`), at the resolution `dpi`, else that
/// of the text's figure.
fn text_area_extent(
    text: RawHandle,
    multilinebaseline: bool,
    dpi: Option<f64>,
) -> PyResult<(f64, f64, f64, f64)> {
    let extent = text_area_bbox(&text, multilinebaseline, dpi)
        .map_err(|err| PyErr::value_error(err.to_string()))?;
    Ok(bounds(extent))
}

/// How far a text area holding `text` moves it up from its origin
/// (`TextArea._baseline_transform`), at the resolution `dpi`, else that of
/// the text's figure.
fn text_area_baseline(text: RawHandle, multilinebaseline: bool, dpi: Option<f64>) -> PyResult<f64> {
    text_area_baseline_shift(&text, multilinebaseline, dpi)
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("rebuild_tree", rebuild)?;
    module.add("node_extent", extent)?;
    module.add("node_origin", origin)?;
    module.add("text_area_extent", text_area_extent)?;
    module.add("text_area_baseline", text_area_baseline)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, extension, with_python};
    use pltrs::artist::ArtistHandle;
    use pltrs::figure::Figure;
    use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, DrawingArea};

    #[test]
    fn boxes_answer_through_the_box_added_with_them() {
        with_python(|py| {
            extension(py);
            let fig = Figure::with_size(4.0, 3.0, 72.0);
            let ax = fig.subplot();
            let area = DrawingArea::new(10.0, 20.0, 0.0, 5.0);
            let options = AnchoredOptions::new(AnchorLoc::Center);
            let anchored = ax.add_anchored_box(AnchoredBox::new(area, options));
            let owner = anchored.raw().clone();
            assert_eq!(extent(owner.clone(), 0).unwrap(), (0.0, -5.0, 10.0, 20.0));
            assert!(origin(owner.clone(), 0).unwrap().is_some());
            assert!(extent(owner, 1).is_err(), "no second box");
            let description = eval(py, "", "('drawing', True, 1.0, 1.0, 0.0, 0.0, False, [])");
            let tree = description.extract::<PyOffsetBox>().unwrap();
            assert!(
                rebuild(ax.raw().clone(), tree).is_err(),
                "an axes has no tree"
            );
        });
    }
}
