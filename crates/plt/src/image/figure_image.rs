//! Unresampled images placed in figure pixels (matplotlib's `FigureImage`
//! and `Figure.figimage`).

use std::cell::RefCell;
use std::rc::Rc;

use plt_core::Bbox;

use super::artist::{canvas_bbox, draw_rendered, split_alpha};
use super::make::{ImageCore, Placement, RenderedImage};
use super::resample::Resampling;
use super::{Extent, ImageAlpha, ImageData, ImageError, Origin, rc_defaults};
use crate::artist::{Artist, ArtistBase, ArtistHandle, DrawContext, RawHandle};
use crate::colors::{ColorizerOptions, impl_colorize};
use crate::figure::Figure;

/// Options of [`Figure::figimage`].
#[derive(Debug, Default)]
pub struct FigimageOptions {
    /// Horizontal offset of the image's left edge, in pixels.
    pub xo: f64,
    /// Vertical offset of the image's bottom edge, in pixels.
    pub yo: f64,
    /// Colormap, norm and limits for scalar data.
    pub colorizer: ColorizerOptions,
    /// Where row 0 of the array is drawn (`None` = `image.origin`).
    pub origin: Option<Origin>,
    /// Opacity: uniform or per pixel.
    pub alpha: Option<ImageAlpha>,
    /// Resize the figure to fit the image.
    pub resize: bool,
}

impl_colorize!(FigimageOptions);

/// State of a figure image.
#[derive(Debug)]
pub(crate) struct FigureImageArtist {
    base: ArtistBase,
    pub(super) core: ImageCore,
    /// Pixel offset of the bottom-left corner.
    offset: (f64, f64),
}

impl Artist for FigureImageArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let magnification = ctx.renderer().image_magnification();
        let rendered = self.render(ctx, magnification);
        draw_rendered(ctx, &self.base, rendered);
    }
}

impl FigureImageArtist {
    /// matplotlib's `FigureImage.make_image` at `magnification`: one
    /// image pixel per figure pixel at the figure's resolution, scaled for
    /// other resolutions.
    pub(super) fn render(
        &self,
        ctx: &mut DrawContext<'_>,
        magnification: f64,
    ) -> Result<Option<RenderedImage>, ImageError> {
        let canvas = canvas_bbox(ctx);
        let fac = ctx.renderer().dpi() / ctx.layout().dpi();
        let (rows, cols) = self.core.data.shape();
        let (x0, y0) = (self.offset.0 / fac, self.offset.1 / fac);
        let (x1, y1) = (x0 + cols as f64, y0 + rows as f64);
        // Like the image, the clip box is the figure at the renderer's
        // resolution rather than at the 72 dpi vector canvases measure in.
        let clip = Bbox::from_extents(0.0, 0.0, canvas.width() * fac, canvas.height() * fac);
        let placement = Placement {
            extent: Extent::new(x0, x1, y0, y1),
            transform: None,
            out_bbox: Bbox::from_extents(x0, y0, x1, y1),
            clip,
            round_to_pixels: false,
            magnification: magnification / fac,
        };
        let alpha = self.core.alpha(self.base.alpha);
        self.core.make_image(&placement, alpha.as_ref())
    }
}

/// A handle to an image drawn in figure pixels (matplotlib's
/// `FigureImage`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FigureImage {
    raw: RawHandle,
}

impl ArtistHandle for FigureImage {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl FigureImage {
    /// The pixel offset of the image's bottom-left corner.
    pub fn offset(&self) -> (f64, f64) {
        self.raw.with::<FigureImageArtist, _>(|image| image.offset)
    }

    /// `(rows, columns)` of the image array.
    pub fn shape(&self) -> (usize, usize) {
        self.raw
            .with::<FigureImageArtist, _>(|image| image.core.data.shape())
    }
}

impl Figure {
    /// Draws an image directly in figure pixels, without resampling, with
    /// its bottom-left corner `(xo, yo)` pixels from the figure's
    /// (matplotlib's `figimage`). Scalar data is colour-mapped; the norm
    /// autoscales when the image is first drawn.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::image::FigimageOptions;
    /// use plt::numeric::Grid;
    /// let fig = Figure::new();
    /// let data = Grid::from_fn(20, 30, |r, c| (r * c) as f64);
    /// let image = fig.figimage(data, FigimageOptions { xo: 10.0, ..Default::default() })?;
    /// assert_eq!(image.offset(), (10.0, 0.0));
    /// # Ok::<(), plt::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the colormap or norm options are invalid.
    pub fn figimage(
        &self,
        data: impl Into<ImageData>,
        options: FigimageOptions,
    ) -> Result<FigureImage, ImageError> {
        let rc = crate::rc::params();
        let data = data.into().normalized();
        if options.resize {
            let (rows, cols) = data.shape();
            let dpi = self.dpi();
            // At a degenerate resolution the size is invalid: the figure
            // keeps its size.
            let _ = self.set_size_inches(cols as f64 / dpi, rows as f64 / dpi);
        }
        let (interpolation, stage, rc_origin) = rc_defaults(&rc);
        let resampling = Resampling {
            interpolation,
            resample: rc.image_resample,
            filternorm: true,
            filterrad: 4.0,
            origin: options.origin.unwrap_or(rc_origin),
        };
        let (uniform, alpha_grid) = split_alpha(options.alpha);
        let core = ImageCore {
            data,
            mappable: RefCell::new(options.colorizer.into_mappable(&rc)?),
            alpha_grid,
            stage,
            resampling,
        };
        let mut base = ArtistBase::with_zorder(0.0);
        base.alpha = uniform;
        let artist = FigureImageArtist {
            base,
            core,
            offset: (options.xo, options.yo),
        };
        let figure = self.raw().figure_state();
        let id = figure.borrow_mut().adopt(self.id(), Box::new(artist));
        Ok(FigureImage {
            raw: RawHandle::new(Rc::clone(figure), id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::Grid;

    #[test]
    fn resize_fits_the_figure_to_the_image() {
        let fig = Figure::with_size(6.4, 4.8, 100.0);
        let image = fig
            .figimage(
                Grid::filled(50, 80, 1.0),
                FigimageOptions {
                    resize: true,
                    ..FigimageOptions::default()
                },
            )
            .unwrap();
        assert_eq!(fig.size_pixels(), (80.0, 50.0));
        assert_eq!(image.shape(), (50, 80));
    }

    #[test]
    fn vector_output_keeps_images_reaching_into_the_upper_half() {
        // 200 × 200 pixels: the image covers the right half in full,
        // though at 72 dpi its offset of 100 pixels is 72 points.
        let fig = Figure::with_size(2.0, 2.0, 100.0);
        fig.set_suppress_composite(Some(true));
        let options = FigimageOptions {
            xo: 100.0,
            ..FigimageOptions::default()
        };
        fig.figimage(Grid::filled(100, 100, 1.0), options).unwrap();
        let svg = fig.to_svg().unwrap();
        let image = &svg[svg.find("<image").unwrap()..];
        assert!(image.contains("width=\"72\""), "{image:.120}");
    }
}
