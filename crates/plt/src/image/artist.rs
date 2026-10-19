//! Images attached to an axes (matplotlib's `AxesImage`).

use std::rc::Rc;

use plt_core::{Bbox, Path, Point};

use super::make::{ImageCore, Placement, RenderedImage};
use super::resample::PixelMap;
use super::unsampled::UnsampledImage;
use super::{Extent, ImageAlpha, ImageError, Interpolation, InterpolationStage, Origin};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, ArtistStore, ClipPathSpec, DrawContext,
    LayoutContext, LimitPoints, RawHandle,
};
use crate::axes::Axes;
use crate::colorizer::{ColorMapped, MappingChange, change_mapping};
use crate::colors::{Colormap, Norm, NormError, ScalarMappable};
use crate::numeric::Grid;
use crate::patches::PatchShape;
use crate::transforms::{ResolvedTransform, Transform};

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
        let mut base = ArtistBase::with_zorder(0.0);
        // Status bars show the value under the pointer, as for matplotlib's
        // images.
        base.mouseover = true;
        Self { base, core, extent }
    }

    /// The `(row, column)` of the array element under display point `at`
    /// (matplotlib's `AxesImage.get_cursor_data`, which truncates towards
    /// zero).
    fn element_at(&self, ctx: LayoutContext<'_>, at: Point) -> Option<(usize, usize)> {
        let data = ctx
            .resolve(&self.transform())?
            .inverted()?
            .transform_point(at);
        let Extent {
            left,
            right,
            mut bottom,
            mut top,
        } = self.extent;
        if self.core.resampling.origin == Origin::Upper {
            std::mem::swap(&mut bottom, &mut top);
        }
        let (rows, columns) = self.core.data.shape();
        let row = (data.y - bottom) / (top - bottom) * rows as f64;
        let column = (data.x - left) / (right - left) * columns as f64;
        index_below(row, rows).zip(index_below(column, columns))
    }

    pub(super) fn transform(&self) -> Transform {
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

    /// The display box the image is clipped to (matplotlib's clip box,
    /// axes box or figure box).
    fn clip_bbox(&self, ctx: &mut DrawContext<'_>) -> Bbox {
        let canvas = canvas_bbox(ctx);
        let layout = ctx.layout();
        let clip = if self.base.clip_on {
            layout.clip_rect(&self.base).or_else(|| {
                self.base
                    .parent
                    .filter(|_| self.base.clip_box.is_none())
                    .and_then(|owner| layout.clip_extent(owner))
            })
        } else {
            None
        };
        clip.unwrap_or(canvas)
    }

    /// matplotlib's `AxesImage.make_image` at `magnification`.
    pub(super) fn render(
        &self,
        ctx: &mut DrawContext<'_>,
        magnification: f64,
    ) -> Result<Option<RenderedImage>, ImageError> {
        let clip = self.clip_bbox(ctx);
        let layout = ctx.layout();
        let (Some(resolved), Some(out_bbox)) =
            (ctx.resolve(&self.transform()), self.out_bbox(*layout))
        else {
            return Ok(None);
        };
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

impl AxesImageArtist {
    /// `make_image(unsampled=True)`, or `None` when the image's map is
    /// not affine.
    fn unsampled(
        &self,
        ctx: &mut DrawContext<'_>,
    ) -> Option<Result<Option<UnsampledImage>, ImageError>> {
        let clip = self.clip_bbox(ctx);
        let resolved = ctx
            .resolve(&self.transform())
            .filter(ResolvedTransform::is_affine)?;
        let out_bbox = self.out_bbox(*ctx.layout())?;
        Some(
            self.core
                .unsampled(self.extent, resolved.affine_part(), out_bbox, clip),
        )
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

    /// Images with interpolation `none` go unsampled to renderers that
    /// scale images themselves, when their map is affine (matplotlib's
    /// `_check_unsampled_image`).
    fn draw(&self, ctx: &mut DrawContext<'_>) {
        if self.core.resampling.interpolation == Interpolation::None
            && ctx.renderer().option_scale_image()
            && let Some(unsampled) = self.unsampled(ctx)
        {
            match unsampled {
                Ok(Some(image)) => {
                    let mut gc = ctx.base_gc(&self.base);
                    gc.alpha = self.base.alpha;
                    ctx.renderer().draw_image(
                        &gc,
                        image.origin,
                        &image.pixels,
                        Some(&image.transform),
                    );
                }
                Ok(None) => {}
                Err(err) => ctx.report(self.base.id(), err),
            }
            return;
        }
        let magnification = ctx.renderer().image_magnification();
        let rendered = self.render(ctx, magnification);
        draw_rendered(ctx, &self.base, rendered);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        self.out_bbox(*ctx)
    }

    /// The corners of the extent (matplotlib's `_update_image_limits`).
    fn limit_points(&self, _axes: ArtistId) -> Option<LimitPoints> {
        let Extent {
            left,
            right,
            bottom,
            top,
        } = self.extent;
        Some(LimitPoints {
            points: vec![Point::new(left, bottom), Point::new(right, top)],
            directions: (true, true),
        })
    }

    /// matplotlib's `AxesImage.contains`: over the axes background and
    /// inside the extent.
    fn contains(&self, ctx: &LayoutContext<'_>, point: Point) -> bool {
        self.base
            .parent
            .is_some_and(|axes| ctx.axes_contains(axes, point))
            && self.out_bbox(*ctx).is_some_and(|bbox| bbox.contains(point))
    }

    fn cursor_text(&self, ctx: &LayoutContext<'_>, at: Point) -> Option<String> {
        let element = self.element_at(*ctx, at)?;
        Some(
            self.core
                .data
                .cursor_text(element, &self.core.mappable.borrow()),
        )
    }
}

/// `value` truncated towards zero as an index below `len` (numpy's
/// `astype(int)` followed by a bounds check).
fn index_below(value: f64, len: usize) -> Option<usize> {
    let index = value.trunc();
    // The range check rejects NaN; the cast is exact below `len`.
    (0.0..len as f64).contains(&index).then_some(index as usize)
}

impl ColorMapped for AxesImageArtist {
    fn mapping_mut(&mut self) -> Option<&mut ScalarMappable> {
        Some(self.core.mappable.get_mut())
    }

    /// Images map their data when drawn; a new norm takes its unset limits
    /// from the whole array, as matplotlib's `_make_image` does.
    fn remap(&mut self, _others: &mut ArtistStore) -> Result<(), NormError> {
        if let Some(field) = self.core.data.scalars() {
            let masked = field.masked_values();
            self.core
                .mappable
                .get_mut()
                .norm_mut()
                .autoscale_none(&masked);
        }
        Ok(())
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

    /// Moves the image to cover `extent` (matplotlib's `set_extent`): the
    /// axes' data limits grow to include it and the view limits of the
    /// axes that autoscale become it.
    pub fn set_extent(&self, extent: Extent) -> &Self {
        match self.raw.with_base(|base| base.parent) {
            Some(axes) => {
                let figure = Rc::clone(self.raw.figure_state());
                Axes::from_raw(RawHandle::new(figure, axes)).fit_image(self, extent);
            }
            None => {
                self.update(|image| image.extent = extent);
            }
        }
        self
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

    /// Replaces the colormap; the image's colorbars follow.
    pub fn set_cmap(&self, cmap: Colormap) -> &Self {
        change_mapping::<AxesImageArtist>(&self.raw, MappingChange::Cmap(Box::new(cmap)));
        self
    }

    /// Replaces the norm, whose unset limits come from the data; the
    /// image's colorbars follow with the norm's scale and ticks.
    pub fn set_norm(&self, norm: Box<dyn Norm>) -> &Self {
        change_mapping::<AxesImageArtist>(&self.raw, MappingChange::Norm(norm));
        self
    }

    /// Sets the data values mapped to the ends of the colormap
    /// (matplotlib's `set_clim`); `None` leaves a limit unchanged. The
    /// image's colorbars follow.
    pub fn set_clim(&self, vmin: Option<f64>, vmax: Option<f64>) -> &Self {
        change_mapping::<AxesImageArtist>(&self.raw, MappingChange::Clim(vmin, vmax));
        self
    }

    /// The data values mapped to the ends of the colormap.
    pub fn clim(&self) -> (Option<f64>, Option<f64>) {
        self.read(|image| image.core.mappable.borrow().clim())
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
    /// `transform` coordinates (matplotlib's `set_clip_path(patch)`); a
    /// rectangle replaces the clip box with its bounding box instead.
    pub fn set_clip_patch(&self, shape: &impl PatchShape, transform: Transform) -> &Self {
        let placed = Transform::Affine(shape.patch_transform()).then(transform);
        let path = shape.path();
        let spec = if shape.clips_to_bounding_box() {
            ClipPathSpec::BoundingBox {
                path,
                transform: placed,
            }
        } else {
            ClipPathSpec::Path {
                path,
                transform: placed,
            }
        };
        self.raw.with_base_mut(|base| base.clip_path = Some(spec));
        self
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::ImshowOptions;
    use crate::figure::Figure;
    use crate::patches::{Circle, Rectangle, RotationPoint};

    #[test]
    fn rectangles_clip_to_their_bounding_box_and_other_shapes_to_their_outline() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
        let image = ax
            .imshow(Grid::filled(4, 4, 1.0), ImshowOptions::default())
            .unwrap();
        ax.set_xlim(0.0, 4.0).set_ylim(0.0, 4.0);
        // A 2 × 1 rectangle at (1, 1) turned by 90°: its box spans x in
        // [0, 1] and y in [1, 3] (one data unit is 100 pixels).
        let rect = Rectangle::new((1.0, 1.0), 2.0, 1.0).rotated(90.0, RotationPoint::Xy);
        image.set_clip_patch(&rect, ax.trans_data());
        let base = image.raw().with_base(Clone::clone);
        assert!(matches!(
            base.clip_path,
            Some(ClipPathSpec::BoundingBox { .. })
        ));
        let state = fig.raw().figure_state();
        let clip = LayoutContext::new(&state.borrow())
            .clip_rect(&base)
            .expect("a clip box");
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(close(clip.x0(), 0.0) && close(clip.x1(), 100.0));
        assert!(close(clip.y0(), 100.0) && close(clip.y1(), 300.0));
        image.set_clip_patch(&Circle::new((2.0, 2.0), 1.0), ax.trans_data());
        let base = image.raw().with_base(Clone::clone);
        assert!(matches!(base.clip_path, Some(ClipPathSpec::Path { .. })));
    }
}
