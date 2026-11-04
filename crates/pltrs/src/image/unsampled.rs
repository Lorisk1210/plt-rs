//! Images drawn at their own resolution (matplotlib's `make_image` with
//! `unsampled=True`): renderers that scale images themselves (the vector
//! backends) receive the coloured array and the affine map placing it,
//! for images with interpolation `none`.

use pltrs_core::{Affine2D, Bbox, Point};
use pltrs_render::RgbaImage;

use super::make::{ImageCore, MadeImage, Placement, mapping_copy};
use super::pcolor_image::cell_colors;
use super::resample::PixelMap;
use super::{Extent, ImageError, Origin};

/// The visible part of an image's coloured array (first row at the top,
/// as every [`RgbaImage`]), the lower left corner it is drawn from, the
/// map of its unit square relative to that corner, and the map of its
/// cells (column and row indices of the visible part) to the display.
#[derive(Debug)]
pub(crate) struct UnsampledImage {
    pub(crate) pixels: RgbaImage,
    pub(crate) origin: Point,
    pub(crate) transform: Affine2D,
    pub(crate) cells: Affine2D,
}

impl UnsampledImage {
    /// The image as `make_image(..., unsampled=True)` returns it at
    /// `magnification`: its pixels, their corner and the map of its cells
    /// to the magnified output, counted from that corner.
    pub(crate) fn made(self, magnification: f64) -> MadeImage {
        MadeImage {
            transform: Some(self.cells.scale(magnification, magnification).translate(
                -self.origin.x * magnification,
                -self.origin.y * magnification,
            )),
            pixels: self.pixels,
            origin: self.origin,
        }
    }
}

/// The cells of an image's coloured array the clip box shows, coloured,
/// and where they are drawn from.
struct VisibleCells {
    /// Their colours, first row at the top.
    pixels: RgbaImage,
    /// The column and row of the first of them.
    first: (usize, usize),
    /// The display position the pixels are drawn from (the clipped box's
    /// lower left corner).
    origin: Point,
}

impl ImageCore {
    /// [`ImageCore::unsampled`] for an image placed as `placement` says,
    /// as `make_image(..., unsampled=True)` returns it; through a
    /// non-affine map, the pixels without the map (the image's data
    /// transform places them, as for resampled images).
    ///
    /// # Errors
    ///
    /// When the image cannot be colour-mapped.
    pub(crate) fn make_unsampled(
        &self,
        placement: &Placement,
    ) -> Result<Option<MadeImage>, ImageError> {
        let data_to_display = placement
            .transform
            .clone()
            .unwrap_or(PixelMap::Affine(Affine2D::IDENTITY));
        if let PixelMap::Affine(affine) = data_to_display {
            return Ok(self
                .unsampled(placement.extent, affine, placement.out_bbox, placement.clip)?
                .map(|made| made.made(placement.magnification)));
        }
        let array_to_display = data_to_display.after(self.array_to_data(placement.extent));
        Ok(self
            .visible_cells(&array_to_display, placement.out_bbox, placement.clip)?
            .map(|visible| MadeImage {
                pixels: visible.pixels,
                origin: visible.origin,
                transform: None,
            }))
    }

    /// The map of the array's cells (column, row) to the data
    /// coordinates of `extent`, flipped for an image whose origin is at
    /// the top.
    fn array_to_data(&self, extent: Extent) -> Affine2D {
        let (rows, cols) = self.data.shape();
        let (rows_f, cols_f) = (rows as f64, cols as f64);
        let flip = match self.resampling.origin {
            Origin::Upper => Affine2D::translation(0.0, -rows_f).scale(1.0, -1.0),
            Origin::Lower => Affine2D::IDENTITY,
        };
        flip.then(
            Affine2D::scaling(
                (extent.right - extent.left) / cols_f,
                (extent.top - extent.bottom) / rows_f,
            )
            .translate(extent.left, extent.bottom),
        )
    }

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
        let array_to_display = self.array_to_data(extent).then(data_to_display);
        let visible = self.visible_cells(&PixelMap::Affine(array_to_display), out_bbox, clip)?;
        Ok(visible.map(|visible| {
            let (first_col, first_row) = visible.first;
            let cells =
                Affine2D::translation(first_col as f64, first_row as f64).then(array_to_display);
            let size = (
                visible.pixels.width() as f64,
                visible.pixels.height() as f64,
            );
            let transform = Affine2D::scaling(size.0, size.1)
                .then(cells)
                .translate(-visible.origin.x, -visible.origin.y);
            UnsampledImage {
                pixels: visible.pixels,
                origin: visible.origin,
                transform,
                cells,
            }
        }))
    }

    /// The cells of the coloured array the clip box `clip` shows of an
    /// image drawn into `out_bbox` through `array_to_display` (matplotlib's
    /// subset of `_make_image(..., unsampled=True)`).
    fn visible_cells(
        &self,
        array_to_display: &PixelMap,
        out_bbox: Bbox,
        clip: Bbox,
    ) -> Result<Option<VisibleCells>, ImageError> {
        if self.data.is_empty() {
            return Ok(None);
        }
        let Some(clipped) = out_bbox.intersection(&clip) else {
            return Ok(None);
        };
        if clipped.width() == 0.0 || clipped.height() == 0.0 {
            return Ok(None);
        }
        // The clip box's corners in the array, all four of them for a
        // rotated or skewed image (matplotlib's `TransformedBbox`).
        let subset = match array_to_display {
            PixelMap::Affine(affine) => affine
                .inverted()
                .map(|display_to_array| clip.transformed_enclosing(&display_to_array)),
            PixelMap::General(map) => map.inverted().map(|display_to_array| {
                clip.mapped_enclosing(|p| display_to_array.transform_point(p))
            }),
        };
        let Some(subset) = subset else {
            return Ok(None);
        };
        let (rows, cols) = self.data.shape();
        let (x0, y0, x1, y1) = subset.extents();
        let (lo_x, lo_y, hi_x, hi_y) = (x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1));
        let first_col = lo_x.max(0.0) as usize;
        let first_row = lo_y.max(0.0) as usize;
        let end_col = ((hi_x + 1.0).min(cols as f64) as usize).max(first_col);
        let end_row = ((hi_y + 1.0).min(rows as f64) as usize).max(first_row);
        let (width, height) = (end_col - first_col, end_row - first_row);
        if width == 0 || height == 0 {
            return Ok(None);
        }
        let mut mappable = mapping_copy(&self.mappable);
        let colors = cell_colors(&self.data, &mut mappable);
        self.mappable.replace(mappable);
        let colors = colors?;
        // matplotlib passes the array as is, first row at the bottom of
        // the unit square; pltrs images list their top row first.
        let data: Vec<u8> = (first_row..end_row)
            .rev()
            .flat_map(|row| (first_col..end_col).map(move |col| (row, col)))
            .flat_map(|cell| colors[cell])
            .collect();
        Ok(Some(VisibleCells {
            pixels: RgbaImage::new(width, height, data).expect("one colour per pixel"),
            first: (first_col, first_row),
            origin: Point::new(clipped.x0(), clipped.y0()),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::make::CoreSettings;
    use crate::numeric::Grid;

    #[test]
    fn rotated_images_keep_every_cell_inside_the_clip_box() {
        // `images/affine_transforms`: a 24 × 24 image rotated by 30° in
        // axes spanning [-5, 5] × [-4, 4], wholly inside them.
        let settings = CoreSettings {
            origin: Some(Origin::Lower),
            ..CoreSettings::default()
        };
        let data = Grid::from_fn(24, 24, |i, j| (i * 24 + j) as f64);
        let (core, _) = ImageCore::build(data, settings, &crate::rc::params()).unwrap();
        let to_display = Affine2D::identity()
            .rotate_deg(30.0)
            .translate(5.0, 4.0)
            .scale(15.12, 15.12)
            .translate(63.163_636, 183.168);
        let extent = Extent::new(-2.0, 4.0, -3.0, 2.0);
        let out_bbox = Bbox::from_extents(-2.0, -3.0, 4.0, 2.0).transformed_enclosing(&to_display);
        let clip = Bbox::from_extents(63.163_636, 183.168, 214.363_636, 304.128);
        let image = core
            .unsampled(extent, to_display, out_bbox, clip)
            .unwrap()
            .expect("visible");
        assert_eq!((image.pixels.width(), image.pixels.height()), (24, 24));
        assert_eq!(image.origin, Point::new(out_bbox.x0(), out_bbox.y0()));
    }

    #[test]
    fn images_on_log_axes_come_unsampled_without_an_affine_map() {
        use crate::axes::ImshowOptions;
        use crate::figure::Figure;

        let fig = Figure::with_size(2.0, 2.0, 100.0);
        let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]).unwrap();
        let options = ImshowOptions {
            extent: Some(Extent::new(1.0, 100.0, 1.0, 100.0)),
            ..ImshowOptions::default()
        };
        let image = ax.imshow(Grid::filled(3, 4, 1.0), options).unwrap();
        ax.set_xscale("log").unwrap();
        let made = image
            .make_image_unsampled(1.0)
            .unwrap()
            .expect("the image is visible");
        assert_eq!((made.pixels.width(), made.pixels.height()), (4, 3));
        assert!(made.transform.is_none(), "the data transform places it");
    }
}
