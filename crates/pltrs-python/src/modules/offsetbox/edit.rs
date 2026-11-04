//! Changes to the boxes of an added box (matplotlib's `OffsetImage.set_zoom`
//! and the size attributes of a `DrawingArea` once its box joined a
//! figure): a leaf is reached through the box added with its tree and its
//! index among the leaves of its kind there, in tree order
//! (`pltrs::offsetbox::edit_tree`).

use pltrs::artist::{ArtistId, RawHandle};
use pltrs::offsetbox::{OffsetBox, edit_tree};

use crate::binding::prelude::*;

/// Runs `f` on the tree of the added box `owner`; `what` names the leaf `f`
/// looks for, which it gives back when it found it.
fn edit_leaf(
    owner: &RawHandle,
    what: &str,
    f: impl FnOnce(&mut OffsetBox<ArtistId>) -> Option<()>,
) -> PyResult<()> {
    match edit_tree(owner, f) {
        Some(Some(())) => Ok(()),
        Some(None) => Err(PyErr::value_error(format!("the box has no such {what}"))),
        None => Err(PyErr::type_error(format!(
            "a {what} belongs to an added offset box"
        ))),
    }
}

/// Sets the zoom of the offset image at `image` (in tree order) of the
/// added box `owner` (`OffsetImage.set_zoom`).
fn set_image_zoom(owner: RawHandle, image: usize, zoom: f64) -> PyResult<()> {
    edit_leaf(&owner, "offset image", |tree| {
        tree.image_mut(image).map(|found| found.zoom = zoom)
    })
}

/// Sets the `(width, height, xdescent, ydescent)` of the drawing area at
/// `area` (in tree order) of the added box `owner`, in points (the
/// attributes of a `DrawingArea`).
fn set_area_size(owner: RawHandle, area: usize, size: (f64, f64, f64, f64)) -> PyResult<()> {
    edit_leaf(&owner, "drawing area", |tree| {
        tree.drawing_area_mut(area).map(|found| {
            (found.width, found.height, found.xdescent, found.ydescent) = size;
        })
    })
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_image_zoom", set_image_zoom)?;
    module.add("set_area_size", set_area_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::artist::ArtistHandle;
    use pltrs::figure::Figure;
    use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, DrawingArea};

    #[test]
    fn leaves_are_reached_through_their_box() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let area = DrawingArea::new(10.0, 10.0, 0.0, 0.0);
        let options = AnchoredOptions::new(AnchorLoc::Center);
        let anchored = ax.add_anchored_box(AnchoredBox::new(area, options));
        let owner = anchored.raw().clone();
        assert!(set_area_size(owner.clone(), 0, (40.0, 5.0, 1.0, 2.0)).is_ok());
        let width = edit_tree(&owner, |tree| tree.drawing_area_mut(0).map(|a| a.width));
        assert_eq!(width, Some(Some(40.0)));
        assert!(set_image_zoom(owner, 0, 2.0).is_err(), "no image");
        assert!(set_area_size(ax.raw().clone(), 0, (1.0, 1.0, 0.0, 0.0)).is_err());
    }
}
