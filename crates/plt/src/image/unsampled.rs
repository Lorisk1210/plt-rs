//! Images drawn at their own resolution (matplotlib's `make_image` with
//! `unsampled=True`): renderers that scale images themselves (the vector
//! backends) receive the coloured array and the affine map placing it,
//! for images with interpolation `none`.

use plt_core::{Affine2D, Bbox, Point};
use plt_render::RgbaImage;

use super::make::ImageCore;
use super::pcolor_image::cell_colors;
use super::{Extent, ImageError, Origin};

/// The visible part of an image's coloured array (first row at the top,
/// as every [`RgbaImage`]), the lower left corner it is drawn from and the
/// map of its unit square relative to that corner.
#[derive(Debug)]
pub(crate) struct UnsampledImage {
    pub(crate) pixels: RgbaImage,
    pub(crate) origin: Point,
    pub(crate) transform: Affine2D,
}

impl ImageCore {
    /// `_make_image(..., unsampled=True)` for an image covering `extent`
    /// in data coordinates mapped to display by `data_to_display`, whose
    /// display box `out_bbox` is clipped to `clip`.
    pub(crate) fn unsampled(
        &self,
        extent: Extent,
        data_to_display: Affine2D,
        out_bbox: Bbox,
        clip: Bbox,
    ) -> Result<Option<UnsampledImage>, ImageError> {
        if self.data.is_empty() {
            return Ok(None);
        }
        let Some(clipped) = out_bbox.intersection(&clip) else {
            return Ok(None);
        };
        if clipped.width() == 0.0 || clipped.height() == 0.0 {
            return Ok(None);
        }
        let (rows, cols) = self.data.shape();
        let (rows_f, cols_f) = (rows as f64, cols as f64);
        let flip = match self.resampling.origin {
            Origin::Upper => Affine2D::translation(0.0, -rows_f).scale(1.0, -1.0),
            Origin::Lower => Affine2D::IDENTITY,
        };
        let to_data = Affine2D::scaling(
            (extent.right - extent.left) / cols_f,
            (extent.top - extent.bottom) / rows_f,
        )
        .translate(extent.left, extent.bottom);
        let array_to_display = flip.then(to_data).then(data_to_display);
        let Some(display_to_array) = array_to_display.inverted() else {
            return Ok(None);
        };
        let subset = clip.transformed(&display_to_array);
        let (x0, y0, x1, y1) = subset.extents();
        let (lo_x, lo_y, hi_x, hi_y) = (x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1));
        let first_col = lo_x.max(0.0) as usize;
        let first_row = lo_y.max(0.0) as usize;
        let end_col = ((hi_x + 1.0).min(cols_f) as usize).max(first_col);
        let end_row = ((hi_y + 1.0).min(rows_f) as usize).max(first_row);
        let colors = cell_colors(&self.data, &mut self.mappable.borrow_mut())?;
        // matplotlib passes the array as is, first row at the bottom of
        // the unit square; plt-rs images list their top row first.
        let data: Vec<u8> = (first_row..end_row)
            .rev()
            .flat_map(|row| (first_col..end_col).map(move |col| (row, col)))
            .flat_map(|cell| colors[cell])
            .collect();
        let (width, height) = (end_col - first_col, end_row - first_row);
        if width == 0 || height == 0 {
            return Ok(None);
        }
        let pixels = RgbaImage::new(width, height, data).expect("one colour per pixel");
        let transform = Affine2D::scaling(width as f64, height as f64)
            .translate(first_col as f64, first_row as f64)
            .then(array_to_display)
            .translate(-clipped.x0(), -clipped.y0());
        Ok(Some(UnsampledImage {
            pixels,
            origin: Point::new(clipped.x0(), clipped.y0()),
            transform,
        }))
    }
}
