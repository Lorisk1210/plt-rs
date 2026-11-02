//! Where a legend is anchored and how it is dragged (matplotlib's
//! `Legend.get_bbox_to_anchor`, `set_bbox_to_anchor`, `set_draggable` and
//! `DraggableLegend`).

use pltrs::legend::{Legend, LegendDragUpdate};
use pltrs::offsetbox::BboxToAnchor;
use pltrs::transforms::Transform;

use crate::binding::prelude::*;

/// A box as `(x, y, width, height)`.
type Bounds = (f64, f64, f64, f64);

/// The box the legend is anchored to as `(x, y, width, height)` in the
/// coordinates of its transform, `None` for its parent's box.
fn get_bbox_to_anchor(legend: Legend) -> PyResult<Option<(Bounds, Transform)>> {
    Ok(legend.bbox_to_anchor().map(|(bbox, transform)| {
        (
            (bbox.x0(), bbox.y0(), bbox.width(), bbox.height()),
            transform,
        )
    }))
}

/// Anchors the legend to `bounds` (`(x, y, width, height)`) in the
/// coordinates `transform` (by default fractions of the parent), or to its
/// parent's box with `None`.
fn set_bbox_to_anchor(
    legend: Legend,
    bounds: Option<Bounds>,
    transform: Option<Transform>,
) -> PyResult<()> {
    let anchor = bounds.map(|(x, y, width, height)| BboxToAnchor::Bounds(x, y, width, height));
    legend.set_bbox_to_anchor(anchor, transform);
    Ok(())
}

/// Makes the legend draggable or not, its drag changing its `"loc"` or its
/// `"bbox"` (to anchor).
fn set_draggable(legend: Legend, draggable: bool, update: String) -> PyResult<()> {
    let update = match update.as_str() {
        "loc" => LegendDragUpdate::Loc,
        "bbox" => LegendDragUpdate::Bbox,
        other => {
            return Err(PyErr::value_error(format!(
                "{other:?} is not a valid value for update; supported values are 'loc', 'bbox'"
            )));
        }
    };
    legend.set_draggable_by(draggable, update);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_bbox_to_anchor", get_bbox_to_anchor)?;
    module.add("set_bbox_to_anchor", set_bbox_to_anchor)?;
    module.add("set_draggable", set_draggable)
}
