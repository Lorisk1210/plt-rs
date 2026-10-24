//! What the image artists share: the canvas box, matplotlib's
//! `AxesImage.contains`, sending a rendered image to the renderer and
//! splitting an alpha argument.

use plt_core::{Bbox, Point};

use super::make::RenderedImage;
use super::{ImageAlpha, ImageError};
use crate::artist::{ArtistBase, DrawContext, LayoutContext};
use crate::numeric::Grid;

/// The figure canvas in display pixels (matplotlib's `figure.bbox`).
pub(crate) fn canvas_bbox(ctx: &mut DrawContext<'_>) -> Bbox {
    let (width, height) = ctx.renderer().canvas_size();
    Bbox::from_extents(0.0, 0.0, width, height)
}

/// matplotlib's `AxesImage.contains`: whether `point` is over the
/// background of the image's axes and inside its `extent` (computed only
/// when needed).
pub(crate) fn axes_image_contains(
    base: &ArtistBase,
    ctx: &LayoutContext<'_>,
    point: Point,
    extent: impl FnOnce() -> Option<Bbox>,
) -> bool {
    base.parent
        .is_some_and(|axes| ctx.axes_contains(axes, point))
        && extent().is_some_and(|bbox| bbox.contains(point))
}

/// Sends a rendered image to the renderer under the artist's clipping;
/// the pixels already carry the opacity.
pub(crate) fn draw_rendered(
    ctx: &mut DrawContext<'_>,
    base: &ArtistBase,
    rendered: Result<Option<RenderedImage>, ImageError>,
) {
    match rendered {
        Ok(Some(image)) => {
            let mut gc = ctx.base_gc(base);
            gc.alpha = None;
            ctx.renderer()
                .draw_image(&gc, image.origin, &image.pixels, None);
        }
        Ok(None) => {}
        Err(err) => ctx.report(base.id(), err),
    }
}

/// A uniform alpha for the artist and a per-pixel alpha for the image.
pub(crate) fn split_alpha(alpha: Option<ImageAlpha>) -> (Option<f64>, Option<Grid<f64>>) {
    match alpha {
        None => (None, None),
        Some(ImageAlpha::Uniform(a)) => (Some(a), None),
        Some(ImageAlpha::Each(grid)) => (None, Some(grid)),
    }
}
