//! Images composited into one (matplotlib's `image.composite_images`).

use pltrs::image::{CompositePart, RgbaImage, composite_images as composite};
use pltrs::transforms::Point;

use super::pixels;
use crate::binding::prelude::*;

/// The rows of an `(M, N, 4)` byte image in the other order: matplotlib's
/// arrays start with their bottom row, pltrs (Rust)'s images with their
/// top row.
fn flipped(width: usize, data: &[u8]) -> Vec<u8> {
    data.chunks_exact((width * 4).max(1))
        .rev()
        .flatten()
        .copied()
        .collect()
}

/// Composites `parts`, each `(pixels, x, y, alpha)` with the `(M, N, 4)`
/// byte pixels `make_image` gives (bottom row first) and their lower left
/// corner, at `magnification`: `(pixels, x, y)` of the composite, the
/// same way round, or `None` when nothing has pixels.
fn composite_images(
    parts: Vec<(NdArray<u8>, f64, f64, f64)>,
    magnification: f64,
) -> PyResult<Option<(NdArray<u8>, f64, f64)>> {
    let parts = parts
        .into_iter()
        .map(|(array, x, y, alpha)| {
            let grid = pixels::<u8, 4>(&array)?;
            let (width, height) = (grid.cols(), grid.rows());
            let pixels = RgbaImage::new(width, height, flipped(width, array.data.as_slice()))
                .map_err(|err| PyErr::value_error(err.to_string()))?;
            Ok(CompositePart {
                pixels,
                origin: Point::new(x, y),
                alpha,
            })
        })
        .collect::<PyResult<Vec<_>>>()?;
    let Some((image, origin)) = composite(&parts, magnification)? else {
        return Ok(None);
    };
    let (width, height) = (image.width(), image.height());
    let pixels = NdArray::with_shape(flipped(width, image.data()), vec![height, width, 4])?;
    Ok(Some((pixels, origin.x, origin.y)))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("composite_images", composite_images)
}
