//! Where a legend is anchored and where its box is placed (matplotlib's
//! `Legend.get_bbox_to_anchor`, `set_bbox_to_anchor`, `_findoffset` and
//! `_legend_box.set_offset`, which `DraggableLegend` drags it with).

use pltrs::legend::Legend;
use pltrs::offsetbox::BboxToAnchor;
use pltrs::transforms::{Point, Transform};

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

/// Pins the origin of the legend box at the display point `offset`, or
/// places the legend at its location again with `None` (matplotlib's
/// `_legend_box.set_offset`).
fn set_box_offset(legend: Legend, offset: Option<(f64, f64)>) -> PyResult<()> {
    legend.set_box_offset(offset.map(|(x, y)| Point::new(x, y)));
    Ok(())
}

/// The display point the origin of the legend box is pinned at, if any.
fn box_offset(legend: Legend) -> PyResult<Option<(f64, f64)>> {
    Ok(legend.box_offset().map(|point| (point.x, point.y)))
}

/// Where the location of the legend puts the origin of its box now
/// (matplotlib's `_findoffset`), `None` for a legend its figure does not
/// lay out.
fn located_box_offset(legend: Legend) -> PyResult<Option<(f64, f64)>> {
    Ok(legend.located_box_offset().map(|point| (point.x, point.y)))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_bbox_to_anchor", get_bbox_to_anchor)?;
    module.add("set_bbox_to_anchor", set_bbox_to_anchor)?;
    module.add("set_box_offset", set_box_offset)?;
    module.add("box_offset", box_offset)?;
    module.add("located_box_offset", located_box_offset)
}
