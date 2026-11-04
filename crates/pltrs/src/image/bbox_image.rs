//! Images stretched over a box of the canvas (matplotlib's `BboxImage`),
//! as offset boxes draw them or as added to an axes.

use std::rc::Rc;

use pltrs_core::archive::{External, ProgramFn};
use pltrs_core::{Affine2D, Bbox, Point};

use super::make::{CoreSettings, ImageCore, MadeImage, Placement, RenderedImage};
use super::resample::PixelMap;
use super::shared::{canvas_bbox, draw_rendered};
use super::{Extent, ImageAlpha, ImageData, ImageError, Interpolation, InterpolationStage, Origin};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, LayoutContext, RawHandle, TypedHandle,
};
use crate::colors::{ColorizerOptions, impl_colorize};
use crate::offsetbox::transformed_bbox;
use crate::rc::RcParams;
use crate::transforms::Transform;

/// Options of an image drawn into a box of the canvas (matplotlib's
/// `BboxImage` keyword arguments, as `OffsetImage` passes them on), which
/// are also those of an image made for an axes outside any figure
/// ([`AxesImage::loose`](super::AxesImage::loose)); unset ones follow the
/// `image.*` rc parameters.
#[derive(Debug, Default)]
pub struct BboxImageOptions {
    /// Colormap, norm and limits for scalar data.
    pub colorizer: ColorizerOptions,
    /// Interpolation when resampling (`None` = `image.interpolation`).
    pub interpolation: Option<Interpolation>,
    /// Whether data or colours are resampled
    /// (`None` = `image.interpolation_stage`).
    pub interpolation_stage: Option<InterpolationStage>,
    /// Where row 0 of the array is drawn (`None` = `image.origin`).
    pub origin: Option<Origin>,
    /// Normalise the kernel weights (`None` = `true`).
    pub filternorm: Option<bool>,
    /// Kernel radius of `sinc`, `lanczos` and `blackman` (`None` = 4).
    pub filterrad: Option<f64>,
    /// Agg's full resampling when downsampling (off unless set, unlike
    /// `imshow`).
    pub resample: bool,
    /// Opacity: uniform or per pixel.
    pub alpha: Option<ImageAlpha>,
}

impl_colorize!(BboxImageOptions);

impl BboxImageOptions {
    /// The settings of the image core these options make.
    pub(super) fn core_settings(self) -> CoreSettings {
        CoreSettings {
            colorizer: self.colorizer,
            interpolation: self.interpolation,
            stage: self.interpolation_stage,
            origin: self.origin,
            filternorm: self.filternorm,
            filterrad: self.filterrad,
            resample: self.resample,
            alpha: self.alpha,
        }
    }
}

/// State of an image filling a box given in display pixels.
#[derive(Clone, Debug)]
pub(crate) struct BboxImageArtist {
    base: ArtistBase,
    pub(super) core: ImageCore,
    /// The display box the image fills, once its owner placed it.
    pub(crate) bbox: Option<Bbox>,
    /// The box the image fills instead, found when it is drawn.
    pub(crate) region: Option<Region>,
}

pltrs_core::archive_struct!(BboxImageArtist {
    base,
    core,
    bbox,
    region
});

/// A box an image added to an axes fills, found when the image is drawn.
#[derive(Clone, Debug)]
pub(crate) enum Region {
    /// A box in other coordinates (matplotlib's `TransformedBbox`).
    Transformed(Bbox, Transform),
    /// The display box of another artist (a `BboxImage` given that
    /// artist's `get_window_extent`).
    ExtentOf(ArtistId),
    /// The display box a function gives for the canvas.
    Function(ProgramFn<BboxFn>),
}

pltrs_core::archive_enum!(Region {
    Transformed(v0, v1),
    ExtentOf(v0),
    Function(v0),
});

/// What a box function of a [`BboxImage`] is told of the canvas the image
/// is drawn on or measured for, as matplotlib's renderer tells it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Canvas {
    /// The width in pixels.
    pub width: f64,
    /// The height in pixels.
    pub height: f64,
    /// The resolution, in dots per inch.
    pub dpi: f64,
}

/// A function giving the display box a [`BboxImage`] fills on a canvas,
/// `None` to leave the image out (matplotlib's `BboxImage(bbox)` with a
/// callable of the renderer).
pub type BboxFn = dyn Fn(Canvas) -> Option<Bbox>;

impl BboxImageArtist {
    /// An image of `data` made as `options` say, at z-order 0 and not
    /// placed yet; fails on an invalid colormap, norm or filter radius.
    pub(crate) fn new(
        data: impl Into<ImageData>,
        options: BboxImageOptions,
        rc: &RcParams,
    ) -> Result<Self, ImageError> {
        let (core, uniform) = ImageCore::build(data, options.core_settings(), rc)?;
        let mut base = ArtistBase::with_zorder(0.0);
        base.alpha = uniform;
        Ok(Self {
            base,
            core,
            bbox: None,
            region: None,
        })
    }

    /// The array's rows and columns.
    pub(crate) fn shape(&self) -> (usize, usize) {
        self.core.data.shape()
    }

    /// The display box the image fills (matplotlib's `get_window_extent`),
    /// inverted where the box is.
    fn display_bbox(&self, ctx: LayoutContext<'_>) -> Option<Bbox> {
        match &self.region {
            Some(Region::Transformed(bbox, transform)) => {
                Some(transformed_bbox(bbox, &ctx.resolve(transform)?))
            }
            Some(Region::ExtentOf(id)) => ctx.store().get(*id)?.window_extent(&ctx),
            Some(Region::Function(bbox)) => bbox.get()(canvas_of(ctx)),
            None => self.bbox,
        }
    }

    /// [`display_bbox`](Self::display_bbox) for a draw, which lets the
    /// figure go while a function of the program's gives the box.
    fn drawn_bbox(&self, ctx: &mut DrawContext<'_>) -> Option<Bbox> {
        let Some(Region::Function(bbox)) = &self.region else {
            return self.display_bbox(ctx.layout());
        };
        let canvas = canvas_of(ctx.layout());
        ctx.free();
        bbox.get()(canvas)
    }

    /// matplotlib's `BboxImage.make_image`: the box, as fractions of the
    /// canvas, mapped back onto the canvas, clipped to it, at
    /// `magnification`; nothing while the image is not placed.
    pub(super) fn render(
        &self,
        ctx: &mut DrawContext<'_>,
        magnification: f64,
    ) -> Result<Option<RenderedImage>, ImageError> {
        let canvas = canvas_bbox(ctx);
        let Some(out_bbox) = self.drawn_bbox(ctx) else {
            return Ok(None);
        };
        let placement = Self::placement(out_bbox, canvas, magnification);
        // A norm of the program's maps the data with the figure free.
        ctx.free();
        let alpha = self.core.alpha(self.base.alpha);
        self.core.make_image(&placement, alpha.as_ref())
    }

    /// Where the image filling `out_bbox` goes on `canvas`, magnified by
    /// `magnification`.
    fn placement(out_bbox: Bbox, canvas: Bbox, magnification: f64) -> Placement {
        let (width, height) = (canvas.width(), canvas.height());
        Placement {
            extent: Extent::new(
                out_bbox.x0() / width,
                out_bbox.x1() / width,
                out_bbox.y0() / height,
                out_bbox.y1() / height,
            ),
            transform: Some(PixelMap::Affine(Affine2D::scaling(width, height))),
            out_bbox,
            clip: canvas,
            round_to_pixels: true,
            magnification,
        }
    }
}

impl Artist for BboxImageArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    /// Drawn at the renderer's image magnification, as `_ImageBase.draw`
    /// asks.
    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let magnification = ctx.renderer().image_magnification();
        let rendered = self.render(ctx, magnification);
        draw_rendered(ctx, &self.base, rendered);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        self.display_bbox(*ctx)
    }

    /// matplotlib's `BboxImage.contains`: a shown image's box.
    fn contains(&self, ctx: &LayoutContext<'_>, point: Point) -> bool {
        self.base.visible
            && self
                .display_bbox(*ctx)
                .is_some_and(|bbox| bbox.contains(point))
    }
}

/// A handle to an image stretched over a box (matplotlib's `BboxImage`),
/// as [`Axes::add_bbox_image`](crate::axes::Axes::add_bbox_image) adds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BboxImage {
    raw: RawHandle,
}

impl ArtistHandle for BboxImage {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl TypedHandle for BboxImage {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<BboxImageArtist>()
            .then(|| Self::from_raw(raw.clone()))
    }
}

impl BboxImage {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// An image of `data` made as `options` say, outside any figure until
    /// an offset box shows it (the `BboxImage` of matplotlib's
    /// `OffsetImage`; see
    /// [`BoxItem::from_loose`](crate::offsetbox::BoxItem::from_loose)).
    ///
    /// ```
    /// use pltrs::image::{BboxImage, BboxImageOptions};
    /// use pltrs::numeric::Grid;
    /// let image = BboxImage::loose(Grid::filled(2, 3, 0.5), BboxImageOptions::default())?;
    /// assert_eq!(image.shape(), (2, 3));
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the colormap, norm or filter radius of `options` is invalid.
    pub fn loose(
        data: impl Into<ImageData>,
        options: BboxImageOptions,
    ) -> Result<Self, ImageError> {
        let artist = BboxImageArtist::new(data, options, &crate::rc::params())?;
        Ok(Self::from_raw(RawHandle::loose(Box::new(artist))))
    }

    /// The image as the figure draws it in its current layout, magnified
    /// by `magnification` (matplotlib's `BboxImage.make_image`): stretched
    /// over its box, clipped to the canvas, with the map from array
    /// indices to the pixels; `None` while it has no box or nothing of it
    /// shows.
    ///
    /// ```
    /// use pltrs::image::{BboxImage, BboxImageOptions};
    /// use pltrs::numeric::Grid;
    /// use pltrs::transforms::{Bbox, Transform};
    /// let image = BboxImage::loose(Grid::filled(2, 3, 0.5), BboxImageOptions::default())?;
    /// assert!(image.make_image(1.0)?.is_none());
    /// image.set_bbox(Bbox::from_bounds(0.0, 0.0, 30.0, 20.0), Transform::Display);
    /// let made = image.make_image(1.0)?.expect("a box");
    /// assert_eq!((made.pixels.width(), made.pixels.height()), (30, 20));
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the image cannot be colour-mapped.
    pub fn make_image(&self, magnification: f64) -> Result<Option<MadeImage>, ImageError> {
        self.made(magnification, false)
    }

    /// [`BboxImage::make_image`] at the image's own resolution, as
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
        let layout = LayoutContext::new(&state);
        let (width, height) = state.size_pixels();
        let canvas = Bbox::from_extents(0.0, 0.0, width, height);
        self.raw.with::<BboxImageArtist, _>(|image| {
            let Some(out_bbox) = image.display_bbox(layout) else {
                return Ok(None);
            };
            let placement = BboxImageArtist::placement(out_bbox, canvas, magnification);
            let alpha = image.core.alpha(image.base.alpha);
            image.core.made(&placement, alpha.as_ref(), unsampled)
        })
    }

    /// The array's rows and columns.
    pub fn shape(&self) -> (usize, usize) {
        self.raw.with::<BboxImageArtist, _>(BboxImageArtist::shape)
    }

    /// Replaces the image array; unset limits of the norm come from new
    /// scalar data (matplotlib's `set_data`).
    ///
    /// ```
    /// use pltrs::image::{BboxImage, BboxImageOptions, ImageData};
    /// use pltrs::numeric::Grid;
    /// let image = BboxImage::loose(ImageData::empty(), BboxImageOptions::default())?;
    /// image.set_data(Grid::filled(4, 5, 0.5));
    /// assert_eq!(image.shape(), (4, 5));
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    pub fn set_data(&self, data: impl Into<ImageData>) -> &Self {
        let data = data.into().normalized();
        self.raw
            .with_mut::<BboxImageArtist, _>(|image| image.core.replace_data(data));
        self
    }

    /// Stretches the image over `bbox`, a box in the coordinates of
    /// `coords`, found again whenever the image is drawn (matplotlib's
    /// `BboxImage(TransformedBbox(bbox, coords))`); the image is flipped
    /// along the directions in which the box is inverted.
    pub fn set_bbox(&self, bbox: Bbox, coords: Transform) -> &Self {
        self.set_region(Region::Transformed(bbox, coords))
    }

    /// Stretches the image over the display box `bbox` gives for the
    /// canvas, asked again whenever the image is drawn or measured; the
    /// image is left out while it gives `None` (matplotlib's
    /// `BboxImage(bbox)` with a function of the renderer). It runs while
    /// the figure is drawn or laid out: it is `Send`, so it cannot use
    /// the figure's handles. A figure with such an image is archived only
    /// if the function has an origin ([`set_bbox_fn_with_origin`](Self::set_bbox_fn_with_origin)).
    ///
    /// ```
    /// use pltrs::axes::AddAs;
    /// use pltrs::image::{BboxImage, BboxImageOptions};
    /// use pltrs::numeric::Grid;
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Bbox;
    /// let ax = Figure::with_size(4.0, 3.0, 100.0).subplot();
    /// let image = BboxImage::loose(Grid::filled(2, 2, 0.5), BboxImageOptions::default())?;
    /// // The upper right quarter of the canvas, whatever its size.
    /// image.set_bbox_fn(|canvas| {
    ///     let (w, h) = (canvas.width, canvas.height);
    ///     Some(Bbox::from_extents(w / 2.0, h / 2.0, w, h))
    /// });
    /// let image = ax.adopt_artist(image.raw(), AddAs::Artist)?;
    /// assert_eq!(image.window_extent(), Some(Bbox::from_extents(200.0, 150.0, 400.0, 300.0)));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_bbox_fn(&self, bbox: impl Fn(Canvas) -> Option<Bbox> + Send + 'static) -> &Self {
        self.set_region(Region::Function(ProgramFn::new(Rc::new(bbox))))
    }

    /// [`set_bbox_fn`](Self::set_bbox_fn), archived as `origin`, what the
    /// program made `bbox` from (see [`ProgramFn`]).
    pub fn set_bbox_fn_with_origin(
        &self,
        bbox: impl Fn(Canvas) -> Option<Bbox> + Send + 'static,
        origin: External,
    ) -> &Self {
        self.set_region(Region::Function(ProgramFn::with_origin(
            Rc::new(bbox),
            origin,
        )))
    }

    /// Stretches the image over the display box of `artist`, an artist of
    /// the same figure, wherever it is when the image is drawn
    /// (matplotlib's `BboxImage(artist.get_window_extent)`); the image is
    /// left out once `artist` is removed.
    pub fn set_bbox_extent_of(&self, artist: &impl ArtistHandle) -> &Self {
        self.set_region(Region::ExtentOf(artist.id()))
    }

    /// Fills `region` from now on.
    fn set_region(&self, region: Region) -> &Self {
        self.raw
            .with_mut::<BboxImageArtist, _>(|image| image.region = Some(region));
        self
    }
}

super::impl_mapping_changes!(BboxImage, BboxImageArtist);
super::impl_color_mapped!(BboxImageArtist, core.data, core.mappable);

/// The canvas a function of the program's places a box image on.
fn canvas_of(layout: LayoutContext<'_>) -> Canvas {
    let (width, height) = layout.canvas_size();
    Canvas {
        width,
        height,
        dpi: layout.dpi(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::numeric::Grid;

    #[test]
    fn images_over_an_artist_follow_it_and_vanish_with_it() {
        let ax = Figure::new().subplot();
        let text = ax.text(0.2, 0.5, "backed");
        let ramp = Grid::from_fn(1, 8, |_, c| c as f64);
        let image = ax
            .add_bbox_image_over(ramp, &text, BboxImageOptions::default())
            .unwrap();
        let before = image.window_extent();
        assert!(before.is_some());
        assert_eq!(before, text.window_extent());
        text.set_x(0.6);
        assert_eq!(image.window_extent(), text.window_extent());
        assert_ne!(image.window_extent(), before, "the image moved along");
        text.remove();
        assert_eq!(image.window_extent(), None);
    }

    #[test]
    fn box_functions_are_asked_for_the_canvas_each_time() {
        let fig = Figure::with_size(2.0, 1.0, 100.0);
        let ax = fig.subplot();
        let image = BboxImage::loose(Grid::filled(1, 1, 0.5), BboxImageOptions::default()).unwrap();
        image.set_bbox_fn(|canvas| {
            (canvas.dpi > 50.0)
                .then(|| Bbox::from_extents(0.0, 0.0, canvas.width / 2.0, canvas.height))
        });
        let image = ax
            .adopt_artist(image.raw(), crate::axes::AddAs::Artist)
            .unwrap();
        assert_eq!(
            image.window_extent(),
            Some(Bbox::from_extents(0.0, 0.0, 100.0, 100.0))
        );
        fig.set_dpi(200.0);
        assert_eq!(
            image.window_extent(),
            Some(Bbox::from_extents(0.0, 0.0, 200.0, 200.0))
        );
        fig.set_dpi(40.0);
        assert_eq!(image.window_extent(), None, "the function left it out");
        fig.to_png().unwrap();
    }
}
