//! What the image artists share: the canvas box, matplotlib's
//! `AxesImage.contains`, the extent and cells of images on rectilinear
//! grids, sending a rendered image to the renderer and splitting an alpha
//! argument.

use plt_core::{Bbox, Point};

use super::make::RenderedImage;
use super::{ImageAlpha, ImageError};
use crate::artist::{ArtistBase, DrawContext, LayoutContext, LimitPoints};
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

/// The first and last points of the grid along `x` and `y`: the corners
/// of the extent of an image on it.
pub(crate) fn grid_corners(x: &[f64], y: &[f64]) -> Option<[Point; 2]> {
    Some([
        Point::new(*x.first()?, *y.first()?),
        Point::new(*x.last()?, *y.last()?),
    ])
}

/// The window extent of an image on the grid along `x` and `y`.
pub(crate) fn grid_extent(
    base: &ArtistBase,
    ctx: &LayoutContext<'_>,
    x: &[f64],
    y: &[f64],
) -> Option<Bbox> {
    let resolved = ctx.resolve(base.transform.as_ref()?)?;
    Some(Bbox::from_points(
        grid_corners(x, y)?.map(|corner| resolved.transform_point(corner)),
    ))
}

/// The corners of the grid along `x` and `y`, which autoscaling takes into
/// account (matplotlib's `_update_image_limits`).
pub(crate) fn grid_limits(x: &[f64], y: &[f64]) -> Option<LimitPoints> {
    Some(LimitPoints {
        points: grid_corners(x, y)?.to_vec(),
        directions: (true, true),
    })
}

/// The `(row, column)` cell of the grid between the edges `x` and `y` of
/// an image under the display point `at`.
pub(crate) fn grid_cell(
    base: &ArtistBase,
    ctx: &LayoutContext<'_>,
    at: Point,
    x: &[f64],
    y: &[f64],
) -> Option<(usize, usize)> {
    let data = ctx
        .resolve(base.transform.as_ref()?)?
        .inverted()?
        .transform_point(at);
    Some((cell_of(y, data.y)?, cell_of(x, data.x)?))
}

/// The cell between `edges` holding `value`: the lower one on an inner
/// edge, and the last one on the first edge, where matplotlib's
/// `searchsorted(value) - 1` wraps around as a negative index.
fn cell_of(edges: &[f64], value: f64) -> Option<usize> {
    let (&first, &last) = (edges.first()?, edges.last()?);
    if !(first..=last).contains(&value) {
        return None;
    }
    let cells = edges.len() - 1;
    let below = edges.partition_point(|&edge| edge < value);
    Some((below + cells - 1) % cells)
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
