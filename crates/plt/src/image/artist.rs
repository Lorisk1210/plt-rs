//! Images attached to an axes (matplotlib's `AxesImage`).

use plt_core::{Bbox, Path, Point};

use super::make::{ImageCore, Placement, RenderedImage};
use super::resample::PixelMap;
use super::{Extent, ImageAlpha, ImageError, Interpolation, InterpolationStage, Origin};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ClipPathSpec, DrawContext, LayoutContext, RawHandle,
};
use crate::colors::{Colormap, Norm};
use crate::numeric::Grid;
use crate::patches::PatchShape;
use crate::transforms::Transform;

/// State of an image on a regular grid in data coordinates.
#[derive(Debug)]
pub(crate) struct AxesImageArtist {
    base: ArtistBase,
    pub(crate) core: ImageCore,
    pub(crate) extent: Extent,
}

impl AxesImageArtist {
    /// An image at z-order 0 covering `extent`.
    pub(crate) fn new(core: ImageCore, extent: Extent) -> Self {
        Self {
            base: ArtistBase::with_zorder(0.0),
            core,
            extent,
        }
    }

    fn transform(&self) -> Transform {
        self.base.transform.clone().unwrap_or(Transform::Display)
    }

    /// The display box of the extent (matplotlib's `TransformedBbox` of
    /// the extent corners).
    fn out_bbox(&self, ctx: LayoutContext<'_>) -> Option<Bbox> {
        let resolved = ctx.resolve(&self.transform())?;
        let Extent {
            left,
            right,
            bottom,
            top,
        } = self.extent;
        Some(Bbox::from_points(
            [(left, bottom), (right, bottom), (left, top), (right, top)]
                .map(|(x, y)| resolved.transform_point(Point::new(x, y))),
        ))
    }

    fn render(&self, ctx: &mut DrawContext<'_>) -> Result<Option<RenderedImage>, ImageError> {
        let canvas = canvas_bbox(ctx);
        let magnification = ctx.renderer().image_magnification();
        let layout = ctx.layout();
        let (Some(resolved), Some(out_bbox)) =
            (ctx.resolve(&self.transform()), self.out_bbox(*layout))
        else {
            return Ok(None);
        };
        let clip = if self.base.clip_on {
            self.base
                .clip_box
                .or(self.base.parent)
                .and_then(|owner| layout.clip_extent(owner))
        } else {
            None
        };
        let clip = clip.unwrap_or(canvas);
        let placement = Placement {
            extent: self.extent,
            transform: Some(PixelMap::from_resolved(resolved)),
            out_bbox,
            clip,
            round_to_pixels: true,
            magnification,
        };
        let alpha = self.core.alpha(self.base.alpha);
        self.core.make_image(&placement, alpha.as_ref())
    }
}

/// The figure canvas in display pixels (matplotlib's `figure.bbox`).
pub(crate) fn canvas_bbox(ctx: &mut DrawContext<'_>) -> Bbox {
    let (width, height) = ctx.renderer().canvas_size();
    Bbox::from_extents(0.0, 0.0, width, height)
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

impl Artist for AxesImageArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let rendered = self.render(ctx);
        draw_rendered(ctx, &self.base, rendered);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        self.out_bbox(*ctx)
    }
}

/// A handle to an image in an axes (matplotlib's `AxesImage`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxesImage {
    raw: RawHandle,
}

impl ArtistHandle for AxesImage {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl AxesImage {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    fn read<R>(&self, f: impl FnOnce(&AxesImageArtist) -> R) -> R {
        self.raw.with::<AxesImageArtist, _>(f)
    }

    fn update(&self, f: impl FnOnce(&mut AxesImageArtist)) -> &Self {
        self.raw.with_mut::<AxesImageArtist, _>(f);
        self
    }

    /// The data rectangle the image covers.
    pub fn extent(&self) -> Extent {
        self.read(|image| image.extent)
    }

    /// `(rows, columns)` of the image array.
    pub fn shape(&self) -> (usize, usize) {
        self.read(|image| image.core.data.shape())
    }

    /// Where row 0 of the array is drawn.
    pub fn origin(&self) -> Origin {
        self.read(|image| image.core.resampling.origin)
    }

    /// The interpolation used when resampling.
    pub fn interpolation(&self) -> Interpolation {
        self.read(|image| image.core.resampling.interpolation)
    }

    /// Changes the interpolation.
    pub fn set_interpolation(&self, interpolation: Interpolation) -> &Self {
        self.update(|image| image.core.resampling.interpolation = interpolation)
    }

    /// Changes whether data or colours are resampled.
    pub fn set_interpolation_stage(&self, stage: InterpolationStage) -> &Self {
        self.update(|image| image.core.stage = stage)
    }

    /// Changes the opacity: uniform, or one value per pixel.
    pub fn set_image_alpha(&self, alpha: Option<ImageAlpha>) -> &Self {
        self.update(|image| {
            let (uniform, grid) = split_alpha(alpha);
            image.base.alpha = uniform;
            image.core.alpha_grid = grid;
        })
    }

    /// The colormap.
    pub fn cmap(&self) -> Colormap {
        self.read(|image| image.core.mappable.borrow().cmap().clone())
    }

    /// Replaces the colormap.
    pub fn set_cmap(&self, cmap: Colormap) -> &Self {
        self.update(|image| image.core.mappable.get_mut().set_cmap(cmap))
    }

    /// Replaces the norm.
    pub fn set_norm(&self, norm: Box<dyn Norm>) -> &Self {
        self.update(|image| image.core.mappable.get_mut().set_norm(norm))
    }

    /// Sets the data values mapped to the ends of the colormap
    /// (matplotlib's `set_clim`); `None` leaves a limit unchanged.
    pub fn set_clim(&self, vmin: Option<f64>, vmax: Option<f64>) -> &Self {
        self.update(|image| {
            let norm = image.core.mappable.get_mut().norm_mut();
            if vmin.is_some() {
                norm.set_vmin(vmin);
            }
            if vmax.is_some() {
                norm.set_vmax(vmax);
            }
        })
    }

    /// The data values mapped to the ends of the colormap.
    pub fn clim(&self) -> (Option<f64>, Option<f64>) {
        self.read(|image| {
            let mappable = image.core.mappable.borrow();
            (mappable.norm().vmin(), mappable.norm().vmax())
        })
    }

    /// Clips the image to `path` given in `transform` coordinates, in
    /// addition to its clip box (matplotlib's `set_clip_path`).
    pub fn set_clip_path(&self, path: Path, transform: Transform) -> &Self {
        self.raw.with_base_mut(|base| {
            base.clip_path = Some(ClipPathSpec::Path { path, transform });
        });
        self
    }

    /// Clips the image to the outline of a patch shape placed in
    /// `transform` coordinates (matplotlib's `set_clip_path(patch)`).
    pub fn set_clip_patch(&self, shape: &impl PatchShape, transform: Transform) -> &Self {
        let placed = Transform::Affine(shape.patch_transform()).then(transform);
        self.set_clip_path(shape.path(), placed)
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
