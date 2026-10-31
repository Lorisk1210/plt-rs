//! The boxes a `BboxImage` given a callable fills (matplotlib's
//! `BboxImage(bbox)` with a function of the renderer): the box of an
//! artist of its figure, or what the callable gives when pltrs (Rust)
//! draws or measures the image.

use pltrs::artist::RawHandle;
use pltrs::image::{BboxImage, Canvas};
use pltrs::transforms::Bbox;

use crate::binding::prelude::*;

/// Stretches `image` over the display box `function(width, height, dpi)`
/// gives for the canvas, `(x0, y0, x1, y1)` or `None` to leave the image
/// out; a call that raises leaves it out too, and its exception is
/// raised by the native call drawing or measuring the image.
fn set_bbox_function(image: BboxImage, function: Callback) -> PyResult<()> {
    image.set_bbox_fn(move |canvas: Canvas| {
        function
            .call::<Option<(f64, f64, f64, f64)>>((canvas.width, canvas.height, canvas.dpi))
            .flatten()
            .map(|(x0, y0, x1, y1)| Bbox::from_extents(x0, y0, x1, y1))
    });
    Ok(())
}

/// Stretches `image` over the display box of `artist`, an artist of its
/// figure, wherever it is when the image is drawn.
fn set_bbox_extent_of(image: BboxImage, artist: RawHandle) -> PyResult<()> {
    image.set_bbox_extent_of(&artist);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_bbox_function", set_bbox_function)?;
    module.add("set_bbox_extent_of", set_bbox_extent_of)
}
