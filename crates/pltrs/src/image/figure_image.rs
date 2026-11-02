//! Unresampled images placed in figure pixels (matplotlib's `FigureImage`
//! and `Figure.figimage`).

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use pltrs_core::Bbox;

use super::make::{ImageCore, MadeImage, Placement, RenderedImage};
use super::resample::Resampling;
use super::shared::{canvas_bbox, draw_rendered, split_alpha};
use super::{Extent, ImageAlpha, ImageData, ImageError, Origin, rc_defaults};
use crate::artist::{Artist, ArtistBase, ArtistHandle, DrawContext, RawHandle, TypedHandle};
use crate::colors::{ColorizerOptions, impl_colorize};
use crate::figure::Figure;
use crate::rc::RcParams;

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

pltrs_core::archive_struct!(FigureImageArtist { base, core, offset });

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

super::impl_color_mapped!(FigureImageArtist, core.data, core.mappable);

impl FigureImageArtist {
    /// An image of `data` (normalized) placed and colour-mapped as
    /// `options` say, resampled as the rc parameters say.
    fn new(data: ImageData, options: FigimageOptions, rc: &RcParams) -> Result<Self, ImageError> {
        let (interpolation, stage, rc_origin) = rc_defaults(rc);
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
            mappable: RefCell::new(options.colorizer.into_mappable(rc)?),
            alpha_grid,
            stage,
            resampling,
        };
        let mut base = ArtistBase::with_zorder(0.0);
        base.alpha = uniform;
        Ok(Self {
            base,
            core,
            offset: (options.xo, options.yo),
        })
    }

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
        let placement = self.placement(canvas, fac, magnification);
        let alpha = self.core.alpha(self.base.alpha);
        self.core.make_image(&placement, alpha.as_ref())
    }

    /// Where the image goes on `canvas`, drawn at `fac` times the
    /// figure's resolution and magnified by `magnification`.
    fn placement(&self, canvas: Bbox, fac: f64, magnification: f64) -> Placement {
        let (rows, cols) = self.core.data.shape();
        let (x0, y0) = (self.offset.0 / fac, self.offset.1 / fac);
        let (x1, y1) = (x0 + cols as f64, y0 + rows as f64);
        // Like the image, the clip box is the figure at the renderer's
        // resolution rather than at the 72 dpi vector canvases measure in.
        let clip = Bbox::from_extents(0.0, 0.0, canvas.width() * fac, canvas.height() * fac);
        Placement {
            extent: Extent::new(x0, x1, y0, y1),
            transform: None,
            out_bbox: Bbox::from_extents(x0, y0, x1, y1),
            clip,
            round_to_pixels: false,
            magnification: magnification / fac,
        }
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

impl TypedHandle for FigureImage {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<FigureImageArtist>()
            .then(|| Self { raw: raw.clone() })
    }
}

super::impl_mapping_changes!(FigureImage, FigureImageArtist);

impl FigureImage {
    /// An image of `data` placed and colour-mapped as `options` say (but
    /// for `resize`, which only `figimage` does), outside any figure until
    /// a figure adopts it (matplotlib's `FigureImage(fig, ...)`; see
    /// [`Figure::adopt_artist`]).
    ///
    /// ```
    /// use pltrs::image::{FigimageOptions, FigureImage, ImageData};
    /// use pltrs::numeric::Grid;
    /// let image = FigureImage::loose(ImageData::empty(), FigimageOptions::default())?;
    /// image.set_data(Grid::filled(4, 3, 0.0));
    /// assert_eq!(image.shape(), (4, 3));
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the colormap or norm options are invalid.
    pub fn loose(data: impl Into<ImageData>, options: FigimageOptions) -> Result<Self, ImageError> {
        let artist =
            FigureImageArtist::new(data.into().normalized(), options, &crate::rc::params())?;
        Ok(Self {
            raw: RawHandle::loose(Box::new(artist)),
        })
    }

    /// Changes the opacity: uniform, or one value per pixel.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::image::{FigimageOptions, ImageAlpha};
    /// use pltrs::numeric::Grid;
    /// let fig = Figure::new();
    /// let image = fig.figimage(Grid::filled(2, 2, 1.0), FigimageOptions::default())?;
    /// image.set_image_alpha(Some(ImageAlpha::Uniform(0.5)));
    /// assert_eq!(image.image_alpha(), Some(ImageAlpha::Uniform(0.5)));
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    pub fn set_image_alpha(&self, alpha: Option<ImageAlpha>) -> &Self {
        self.raw.with_mut::<FigureImageArtist, _>(|image| {
            let (uniform, grid) = split_alpha(alpha);
            image.base.alpha = uniform;
            image.core.alpha_grid = grid;
        });
        self
    }

    /// The opacity: uniform, one value per pixel, or none.
    pub fn image_alpha(&self) -> Option<ImageAlpha> {
        self.raw
            .with::<FigureImageArtist, _>(|image| image.core.alpha(image.base.alpha))
    }

    /// The pixel offset of the image's bottom-left corner.
    pub fn offset(&self) -> (f64, f64) {
        self.raw.with::<FigureImageArtist, _>(|image| image.offset)
    }

    /// The image as the figure draws it at its own resolution, magnified
    /// by `magnification` (matplotlib's `FigureImage.make_image`): one
    /// image pixel per figure pixel from its offset, clipped to the
    /// figure, with the map from array indices to the pixels; `None` when
    /// nothing of it shows.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::image::FigimageOptions;
    /// use pltrs::numeric::Grid;
    /// let fig = Figure::with_size(2.0, 2.0, 50.0);
    /// let image = fig.figimage(Grid::filled(3, 4, 0.5), FigimageOptions { xo: 10.0, ..FigimageOptions::default() })?;
    /// let made = image.make_image(1.0)?.expect("on the figure");
    /// assert_eq!((made.pixels.width(), made.pixels.height()), (4, 3));
    /// assert_eq!(made.origin.x, 10.0);
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the image cannot be colour-mapped.
    pub fn make_image(&self, magnification: f64) -> Result<Option<MadeImage>, ImageError> {
        self.made(magnification, false)
    }

    /// [`FigureImage::make_image`] at the image's own resolution, as
    /// renderers that scale images themselves receive it (matplotlib's
    /// `make_image(..., unsampled=True)`).
    ///
    /// # Errors
    ///
    /// When the image cannot be colour-mapped.
    pub fn make_image_unsampled(
        &self,
        magnification: f64,
    ) -> Result<Option<MadeImage>, ImageError> {
        self.made(magnification, true)
    }

    fn made(&self, magnification: f64, unsampled: bool) -> Result<Option<MadeImage>, ImageError> {
        let state = self.raw.figure_state().borrow();
        let (width, height) = state.size_pixels();
        let canvas = Bbox::from_extents(0.0, 0.0, width, height);
        self.raw.with::<FigureImageArtist, _>(|image| {
            let placement = image.placement(canvas, 1.0, magnification);
            let alpha = image.core.alpha(image.base.alpha);
            image.core.made(&placement, alpha.as_ref(), unsampled)
        })
    }

    /// `(rows, columns)` of the image array.
    pub fn shape(&self) -> (usize, usize) {
        self.raw
            .with::<FigureImageArtist, _>(|image| image.core.data.shape())
    }

    /// The figure pixels the image covers, with pixel centres on whole
    /// numbers (matplotlib's `FigureImage.get_extent`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::image::{Extent, FigimageOptions};
    /// use pltrs::numeric::Grid;
    /// let fig = Figure::new();
    /// let options = FigimageOptions { xo: 10.0, yo: 20.0, ..Default::default() };
    /// let image = fig.figimage(Grid::filled(2, 3, 1.0), options)?;
    /// assert_eq!(image.extent(), Extent::new(9.5, 12.5, 19.5, 21.5));
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    pub fn extent(&self) -> Extent {
        let (rows, cols) = self.shape();
        let (xo, yo) = self.offset();
        Extent::new(
            xo - 0.5,
            xo + cols as f64 - 0.5,
            yo - 0.5,
            yo + rows as f64 - 0.5,
        )
    }

    /// The image array (matplotlib's `get_array`).
    pub fn array(&self) -> ImageData {
        self.raw
            .with::<FigureImageArtist, _>(|image| image.core.data.clone())
    }

    /// Replaces the image array; unset limits of the norm come from new
    /// scalar data (matplotlib's `FigureImage.set_data`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::image::FigimageOptions;
    /// use pltrs::numeric::Grid;
    /// let fig = Figure::new();
    /// let image = fig.figimage(Grid::filled(2, 3, 1.0), FigimageOptions::default())?;
    /// image.set_data(Grid::filled(4, 5, 2.0));
    /// assert_eq!(image.shape(), (4, 5));
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    pub fn set_data(&self, data: impl Into<ImageData>) -> &Self {
        let data = data.into().normalized();
        self.raw
            .with_mut::<FigureImageArtist, _>(|image| image.core.replace_data(data));
        self
    }
}

impl Figure {
    /// Draws an image directly in figure pixels, without resampling, with
    /// its bottom-left corner `(xo, yo)` pixels from the figure's
    /// (matplotlib's `figimage`). Scalar data is colour-mapped; the norm
    /// autoscales when the image is first drawn.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::image::FigimageOptions;
    /// use pltrs::numeric::Grid;
    /// let fig = Figure::new();
    /// let data = Grid::from_fn(20, 30, |r, c| (r * c) as f64);
    /// let image = fig.figimage(data, FigimageOptions { xo: 10.0, ..Default::default() })?;
    /// assert_eq!(image.offset(), (10.0, 0.0));
    /// # Ok::<(), pltrs::image::ImageError>(())
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
        let artist = FigureImageArtist::new(data, options, &rc)?;
        let figure = self.raw().figure_state();
        let id = figure.borrow_mut().adopt(self.id(), Box::new(artist));
        Ok(FigureImage {
            raw: RawHandle::new(Rc::clone(figure), id),
        })
    }

    /// The images [`Figure::figimage`] drew in this figure (or subfigure)
    /// itself, in the order they were made (matplotlib's `Figure.images`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::image::FigimageOptions;
    /// use pltrs::numeric::Grid;
    /// let fig = Figure::new();
    /// assert!(fig.images().is_empty());
    /// let image = fig.figimage(Grid::filled(2, 2, 1.0), FigimageOptions::default())?;
    /// assert_eq!(fig.images(), [image]);
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    pub fn images(&self) -> Vec<FigureImage> {
        self.own_artists_where(|artist| (artist as &dyn Any).is::<FigureImageArtist>())
            .into_iter()
            .map(|raw| FigureImage { raw })
            .collect()
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
