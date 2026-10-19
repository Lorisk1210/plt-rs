//! Images stretched over a box of the canvas (matplotlib's `BboxImage`),
//! as offset boxes draw them or as added to an axes.

use plt_core::{Affine2D, Bbox, Point};

use super::artist::{canvas_bbox, draw_rendered};
use super::make::{CoreSettings, ImageCore, Placement};
use super::resample::PixelMap;
use super::{
    ColorizerOptions, Extent, ImageAlpha, ImageData, ImageError, Interpolation, InterpolationStage,
    Origin,
};
use crate::artist::{Artist, ArtistBase, ArtistHandle, DrawContext, LayoutContext, RawHandle};
use crate::offsetbox::transformed_bbox;
use crate::rc::RcParams;
use crate::transforms::Transform;

/// Options of an image drawn into a box of the canvas (matplotlib's
/// `BboxImage` keyword arguments, as `OffsetImage` passes them on); unset
/// ones follow the `image.*` rc parameters.
#[derive(Debug, Default)]
pub struct BboxImageOptions {
    /// Colormap, norm and limits for scalar data.
    pub colors: ColorizerOptions,
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

/// State of an image filling a box given in display pixels.
#[derive(Debug)]
pub(crate) struct BboxImageArtist {
    base: ArtistBase,
    core: ImageCore,
    /// The display box the image fills, once its owner placed it.
    pub(crate) bbox: Option<Bbox>,
    /// A box in other coordinates the image fills instead, mapped to the
    /// display when drawn (matplotlib's `TransformedBbox`).
    pub(crate) region: Option<(Bbox, Transform)>,
}

impl BboxImageArtist {
    /// An image of `data` made as `options` say, at z-order 0 and not
    /// placed yet; fails on an invalid colormap, norm or filter radius.
    pub(crate) fn new(
        data: impl Into<ImageData>,
        options: BboxImageOptions,
        rc: &RcParams,
    ) -> Result<Self, ImageError> {
        let settings = CoreSettings {
            colors: options.colors,
            interpolation: options.interpolation,
            stage: options.interpolation_stage,
            origin: options.origin,
            filternorm: options.filternorm,
            filterrad: options.filterrad,
            resample: options.resample,
            alpha: options.alpha,
        };
        let (core, uniform) = ImageCore::build(data, settings, rc)?;
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
            Some((bbox, transform)) => Some(transformed_bbox(bbox, &ctx.resolve(transform)?)),
            None => self.bbox,
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

    /// matplotlib's `BboxImage.make_image`: the box, as fractions of the
    /// canvas, mapped back onto the canvas, clipped to it, at the
    /// renderer's image magnification (as `_ImageBase.draw` asks).
    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let Some(out_bbox) = self.display_bbox(*ctx.layout()) else {
            return;
        };
        let canvas = canvas_bbox(ctx);
        let (width, height) = (canvas.width(), canvas.height());
        let magnification = ctx.renderer().image_magnification();
        let placement = Placement {
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
        };
        let alpha = self.core.alpha(self.base.alpha);
        let rendered = self.core.make_image(&placement, alpha.as_ref());
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

impl BboxImage {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// The array's rows and columns.
    pub fn shape(&self) -> (usize, usize) {
        self.raw.with::<BboxImageArtist, _>(BboxImageArtist::shape)
    }
}
