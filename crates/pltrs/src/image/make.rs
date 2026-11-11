//! Turning an image into screen pixels: matplotlib's
//! `_ImageBase._make_image` geometry and stage selection.

use std::cell::RefCell;

use pltrs_core::{Affine2D, Bbox, Point};
use pltrs_render::RgbaImage;

use super::any::checked_filterrad;
use super::resample::{PixelMap, Resampling};
use super::shared::split_alpha;
use super::stages::{data_stage, rgba_stage};
use super::{
    Extent, ImageAlpha, ImageData, ImageError, Interpolation, InterpolationStage, Origin,
    rc_defaults,
};
use crate::colors::{ColorizerOptions, ScalarMappable};
use crate::numeric::Grid;
use crate::rc::RcParams;

/// How an image is coloured, resampled and made transparent; unset
/// settings follow the `image.*` rc parameters.
#[derive(Debug, Default)]
pub(crate) struct CoreSettings {
    /// Colormap, norm and limits for scalar data.
    pub(crate) colorizer: ColorizerOptions,
    /// Interpolation when resampling.
    pub(crate) interpolation: Option<Interpolation>,
    /// Whether data or colours are resampled.
    pub(crate) stage: Option<InterpolationStage>,
    /// Where row 0 of the array is drawn.
    pub(crate) origin: Option<Origin>,
    /// Normalise the kernel weights (`None` = `true`).
    pub(crate) filternorm: Option<bool>,
    /// Kernel radius of the sinc family (`None` = 4).
    pub(crate) filterrad: Option<f64>,
    /// Agg's full resampling when downsampling.
    pub(crate) resample: bool,
    /// Opacity: uniform or per pixel.
    pub(crate) alpha: Option<ImageAlpha>,
}

/// An image resampled for the screen: pixels (top row first) and the
/// display position of their bottom-left corner.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RenderedImage {
    /// The pixels.
    pub(crate) pixels: RgbaImage,
    /// Bottom-left corner in display coordinates.
    pub(crate) origin: Point,
}

/// An image resampled and colour-mapped for display (what matplotlib's
/// `make_image` returns).
#[derive(Clone, Debug, PartialEq)]
pub struct MadeImage {
    /// The RGBA pixels, top row first.
    pub pixels: RgbaImage,
    /// The display position of the pixels' bottom-left corner.
    pub origin: Point,
    /// The map from array indices (column, row) to the pixels, counted
    /// from `origin`, when it is affine; images on non-affine axes (log
    /// scales, projections) map through their data transform instead.
    pub transform: Option<Affine2D>,
}

impl MadeImage {
    /// The image `rendered` with the map from array indices to its pixels.
    pub(crate) fn mapped((rendered, map): (RenderedImage, PixelMap)) -> Self {
        Self {
            pixels: rendered.pixels,
            origin: rendered.origin,
            transform: map.affine(),
        }
    }
}

/// The colouring and resampling state every image kind shares.
#[derive(Clone, Debug)]
pub(crate) struct ImageCore {
    /// The image array.
    pub(crate) data: ImageData,
    /// Norm and colormap; mutable while drawing because matplotlib's norms
    /// autoscale on first use.
    pub(crate) mappable: RefCell<ScalarMappable>,
    /// Per-pixel opacity (a uniform opacity lives on the artist).
    pub(crate) alpha_grid: Option<Grid<f64>>,
    /// Interpolation stage as set on the image.
    pub(crate) stage: InterpolationStage,
    /// Kernel and orientation settings.
    pub(crate) resampling: Resampling,
}

pltrs_core::archive_struct!(ImageCore {
    data,
    mappable,
    alpha_grid,
    stage,
    resampling
});

/// A copy of the norm and colormap of `mappable` to map an image's data
/// with, which a norm autoscales on first use; the caller puts it back
/// after (`RefCell::replace`). A norm of the program's may change the
/// image it maps (a draw lets the figure go meanwhile), which copies the
/// image, so the image's cell is not borrowed meanwhile.
pub(crate) fn mapping_copy(mappable: &RefCell<ScalarMappable>) -> ScalarMappable {
    mappable.borrow().clone()
}

/// Where an image goes: its data extent, the map from data to display
/// (`None` = the identity), its display box and the clip box.
#[derive(Clone, Debug)]
pub(crate) struct Placement {
    /// The data rectangle the image covers (matplotlib's `in_bbox`).
    pub(crate) extent: Extent,
    /// Data → display; `None` for images placed in display pixels.
    pub(crate) transform: Option<PixelMap>,
    /// The image's display box (matplotlib's `out_bbox`).
    pub(crate) out_bbox: Bbox,
    /// The display box the output is clipped to.
    pub(crate) clip: Bbox,
    /// Round the output to whole pixels (all images but figure images).
    pub(crate) round_to_pixels: bool,
    /// Extra scaling of the output (renderer resolution over figure
    /// resolution for figure images).
    pub(crate) magnification: f64,
}

/// Sets the unset limits of `mappable`'s norm from `data`, when it holds
/// scalars (matplotlib's `autoscale_None` on new image data). A norm of the
/// program's is left alone: matplotlib's `set_data` does not autoscale it
/// (its plotting functions' `_scale_norm` and its draws do).
pub(crate) fn autoscale_unset(mappable: &mut ScalarMappable, data: &ImageData) {
    if let Some(field) = data.scalars()
        && !mappable.norm().calls_program()
    {
        mappable.norm_mut().autoscale_none(&field.masked_values());
    }
}

impl ImageCore {
    /// Replaces the image array; unset limits of the norm come from new
    /// scalar data (matplotlib's `set_data`).
    pub(crate) fn replace_data(&mut self, data: ImageData) {
        autoscale_unset(self.mappable.get_mut(), &data);
        self.data = data;
    }

    /// Replaces the image array, the norm's limits as they are: an
    /// unscaled norm takes them when the image is first made, from the
    /// values it resamples (matplotlib's `set_data` and `_make_image`).
    pub(crate) fn set_data_as_given(&mut self, data: ImageData) {
        self.data = data;
    }

    /// The image of `data` made as `settings` say, with the uniform
    /// opacity, which lives on the artist; scalar data autoscale the norm's
    /// unset limits.
    ///
    /// # Errors
    ///
    /// When the filter radius is not positive, or the colormap or norm is
    /// invalid.
    pub(crate) fn build(
        data: impl Into<ImageData>,
        settings: CoreSettings,
        rc: &RcParams,
    ) -> Result<(Self, Option<f64>), ImageError> {
        let data = data.into().normalized();
        let (interpolation, stage, origin) = rc_defaults(rc);
        let filterrad = checked_filterrad(settings.filterrad.unwrap_or(4.0))?;
        let resampling = Resampling {
            interpolation: settings.interpolation.unwrap_or(interpolation),
            resample: settings.resample,
            filternorm: settings.filternorm.unwrap_or(true),
            filterrad,
            origin: settings.origin.unwrap_or(origin),
        };
        let mut mappable = settings.colorizer.into_mappable(rc)?;
        if let Some(field) = data.scalars() {
            field.autoscale(mappable.norm_mut());
        }
        let (uniform, alpha_grid) = split_alpha(settings.alpha);
        let core = Self {
            data,
            mappable: RefCell::new(mappable),
            alpha_grid,
            stage: settings.stage.unwrap_or(stage),
            resampling,
        };
        Ok((core, uniform))
    }

    /// The opacity of the image: per-pixel, else the artist's `alpha`.
    pub(crate) fn alpha(&self, uniform: Option<f64>) -> Option<ImageAlpha> {
        self.alpha_grid
            .clone()
            .map(ImageAlpha::Each)
            .or(uniform.map(ImageAlpha::Uniform))
    }

    /// matplotlib's `_make_image`: the clipped, pixel-rounded output box,
    /// the array-to-output transform, and the pixels from the data or
    /// RGBA stage; `None` when nothing is visible.
    pub(crate) fn make_image(
        &self,
        placement: &Placement,
        alpha: Option<&ImageAlpha>,
    ) -> Result<Option<RenderedImage>, ImageError> {
        Ok(self.make_mapped(placement, alpha)?.map(|(image, _)| image))
    }

    /// The image placed as `placement` says, as `make_image` returns it:
    /// resampled with `alpha`, or at its own resolution when `unsampled`.
    pub(crate) fn made(
        &self,
        placement: &Placement,
        alpha: Option<&ImageAlpha>,
        unsampled: bool,
    ) -> Result<Option<MadeImage>, ImageError> {
        if unsampled {
            self.make_unsampled(placement)
        } else {
            Ok(self.make_mapped(placement, alpha)?.map(MadeImage::mapped))
        }
    }

    /// [`ImageCore::make_image`] with the map from array indices to the
    /// output pixels (matplotlib's `t`).
    pub(crate) fn make_mapped(
        &self,
        placement: &Placement,
        alpha: Option<&ImageAlpha>,
    ) -> Result<Option<(RenderedImage, PixelMap)>, ImageError> {
        if self.data.is_empty() {
            return Ok(None);
        }
        let Some(clipped) = placement.out_bbox.intersection(&placement.clip) else {
            return Ok(None);
        };
        let magnification = placement.magnification;
        let (x0, y0, x1, y1) = clipped.extents();
        let (x0, y0, x1, y1) = (
            x0 * magnification,
            y0 * magnification,
            x1 * magnification,
            y1 * magnification,
        );
        let output = if placement.round_to_pixels {
            // The fudge accounts for precision loss on the sides away from
            // Agg's anchor point (x0, y1).
            Bbox::from_extents(
                (x0 + 0.5).floor(),
                (y0 - 0.5 - 1e-8).ceil(),
                (x1 + 0.5 + 1e-8).floor(),
                (y1 - 0.5).ceil(),
            )
        } else {
            Bbox::from_extents(x0, y0, x1, y1)
        };
        if output.width() == 0.0 || output.height() == 0.0 {
            return Ok(None);
        }
        let map = self.pixel_map(placement, &output);
        let out_shape = (output.height() as usize, output.width() as usize);
        if out_shape.0 == 0 || out_shape.1 == 0 {
            return Ok(None);
        }
        let mut mappable = mapping_copy(&self.mappable);
        let pixels = match (&self.data, self.resolved_stage(&map)) {
            (ImageData::Scalar(field), InterpolationStage::Data) => data_stage(
                field,
                alpha,
                &mut mappable,
                &self.resampling,
                out_shape,
                &map,
            ),
            (data, _) => rgba_stage(
                data,
                alpha,
                &mut mappable,
                &self.resampling,
                out_shape,
                &map,
            ),
        };
        self.mappable.replace(mappable);
        let pixels = pixels?;
        let rendered = RenderedImage {
            pixels: top_down(&pixels),
            origin: Point::new(output.x0() / magnification, output.y0() / magnification),
        };
        Ok(Some((rendered, map)))
    }

    /// The array-index → output-pixel map `t`, composed in matplotlib's
    /// order: the origin flip, the extent scaling, the data transform, then
    /// the shift to the output box.
    fn pixel_map(&self, placement: &Placement, output: &Bbox) -> PixelMap {
        let (rows, cols) = self.data.shape();
        let (rows, cols) = (rows as f64, cols as f64);
        let extent = placement.extent;
        let scale = Affine2D::identity()
            .scale(
                (extent.right - extent.left) / cols,
                (extent.top - extent.bottom) / rows,
            )
            .translate(extent.left, extent.bottom);
        let placed = match &placement.transform {
            Some(transform) => transform.clone().after(scale),
            None => PixelMap::Affine(scale),
        };
        let oriented = match self.resampling.origin {
            Origin::Upper => {
                placed.after(Affine2D::identity().translate(0.0, -rows).scale(1.0, -1.0))
            }
            Origin::Lower => placed,
        };
        let magnification = placement.magnification;
        oriented.then(
            Affine2D::identity()
                .scale(magnification, magnification)
                .translate(-output.x0(), -output.y0()),
        )
    }

    /// The interpolation stage, resolving `Auto` by the magnification:
    /// colours when either direction is magnified less than three times.
    fn resolved_stage(&self, map: &PixelMap) -> InterpolationStage {
        if self.stage != InterpolationStage::Auto {
            return self.stage;
        }
        let (rows, cols) = self.data.shape();
        let a = map.apply(Point::ORIGIN);
        let b = map.apply(Point::new(cols as f64, rows as f64));
        let dispx = (b.x - a.x).abs() / cols as f64;
        let dispy = (b.y - a.y).abs() / rows as f64;
        if dispx < 3.0 || dispy < 3.0 {
            InterpolationStage::Rgba
        } else {
            InterpolationStage::Data
        }
    }
}

/// The renderer's image: rows top first, while the resampled grid has the
/// display's bottom row first.
fn top_down(pixels: &Grid<[u8; 4]>) -> RgbaImage {
    let (rows, cols) = pixels.shape();
    let data = (0..rows)
        .rev()
        .flat_map(|row| pixels.row(row).iter().flatten().copied())
        .collect();
    RgbaImage::new(cols, rows, data).expect("four bytes per pixel")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::{Normalize, colormaps};
    use crate::image::Interpolation;

    fn core(data: ImageData, origin: Origin) -> ImageCore {
        ImageCore {
            data,
            mappable: RefCell::new(ScalarMappable::with_norm(
                colormaps::get("gray").unwrap(),
                Box::new(Normalize::new(0.0, 1.0)),
            )),
            alpha_grid: None,
            stage: InterpolationStage::Auto,
            resampling: Resampling {
                interpolation: Interpolation::Nearest,
                resample: true,
                filternorm: true,
                filterrad: 4.0,
                origin,
            },
        }
    }

    fn placement(out_bbox: Bbox, clip: Bbox) -> Placement {
        Placement {
            extent: Extent::new(0.0, 2.0, 0.0, 1.0),
            transform: Some(PixelMap::Affine(Affine2D::scaling(10.0, 10.0))),
            out_bbox,
            clip,
            round_to_pixels: true,
            magnification: 1.0,
        }
    }

    #[test]
    fn upper_origin_puts_row_zero_on_top() {
        let data = Grid::from_vec(1, 2, vec![0.0, 1.0]).unwrap();
        let image = core(ImageData::from(data.clone()), Origin::Upper);
        let bbox = Bbox::from_extents(0.0, 0.0, 20.0, 10.0);
        let rendered = image
            .make_image(&placement(bbox, bbox), None)
            .unwrap()
            .unwrap();
        assert_eq!(rendered.origin, Point::ORIGIN);
        assert_eq!(rendered.pixels.width(), 20);
        assert_eq!(rendered.pixels.height(), 10);
        assert_eq!(rendered.pixels.pixel(0, 0), Some([0, 0, 0, 255]));
        assert_eq!(rendered.pixels.pixel(19, 9), Some([255, 255, 255, 255]));
    }

    #[test]
    fn clipping_and_rounding_shape_the_output() {
        let data = Grid::from_vec(1, 2, vec![0.0, 1.0]).unwrap();
        let image = core(ImageData::from(data), Origin::Lower);
        let bbox = Bbox::from_extents(0.0, 0.0, 20.0, 10.0);
        let clip = Bbox::from_extents(4.6, 2.2, 30.0, 7.5);
        let rendered = image
            .make_image(&placement(bbox, clip), None)
            .unwrap()
            .unwrap();
        assert_eq!(rendered.origin, Point::new(5.0, 2.0));
        assert_eq!((rendered.pixels.width(), rendered.pixels.height()), (15, 5));
        let outside = Bbox::from_extents(50.0, 50.0, 60.0, 60.0);
        assert_eq!(image.make_image(&placement(bbox, outside), None), Ok(None));
        let sliver = Bbox::from_extents(0.0, 0.0, 20.0, 0.2);
        assert_eq!(image.make_image(&placement(bbox, sliver), None), Ok(None));
    }

    #[test]
    fn stage_follows_the_magnification() {
        let data = Grid::from_vec(1, 2, vec![0.0, 1.0]).unwrap();
        let image = core(ImageData::from(data), Origin::Lower);
        let map = |s: f64| PixelMap::Affine(Affine2D::scaling(s, s));
        assert_eq!(image.resolved_stage(&map(2.0)), InterpolationStage::Rgba);
        assert_eq!(image.resolved_stage(&map(3.0)), InterpolationStage::Data);
    }

    #[test]
    fn grid_alpha_wins_over_uniform_alpha() {
        let mut image = core(ImageData::from(Grid::filled(1, 1, 0.0)), Origin::Lower);
        assert_eq!(image.alpha(Some(0.5)), Some(ImageAlpha::Uniform(0.5)));
        image.alpha_grid = Some(Grid::filled(1, 1, 0.25));
        assert!(matches!(image.alpha(Some(0.5)), Some(ImageAlpha::Each(_))));
        assert_eq!(image.alpha(None).map(|a| a.scalar()), Some(1.0));
    }
}
