//! Images attached to an axes (matplotlib's `AxesImage`).

use std::rc::Rc;

use pltrs_core::{Bbox, Path, Point};

use super::make::{ImageCore, MadeImage, Placement, RenderedImage};
use super::resample::PixelMap;
use super::shared::{axes_image_contains, canvas_bbox, draw_rendered, split_alpha};
use super::unsampled::UnsampledImage;
use super::{Extent, ImageAlpha, ImageData, ImageError, Interpolation, InterpolationStage, Origin};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, ArtistStore, ClipPathSpec, DrawContext,
    LayoutContext, LimitPoints, RawHandle, TypedHandle,
};
use crate::axes::Axes;
use crate::colorizer::{ColorMapped, MappingChange, change_mapping};
use crate::colors::{Colormap, Norm, NormError, ScalarMappable};
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
    /// axes box or figure box, the `canvas`).
    fn clip_bbox(&self, layout: LayoutContext<'_>, canvas: Bbox) -> Bbox {
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

    /// Where the image goes at `magnification` on a figure of size
    /// `canvas` (matplotlib's `AxesImage.make_image`); `None` when its
    /// transform refers to removed artists.
    fn placement(
        &self,
        layout: LayoutContext<'_>,
        canvas: Bbox,
        magnification: f64,
    ) -> Option<Placement> {
        Some(Placement {
            extent: self.extent,
            transform: Some(PixelMap::from_resolved(layout.resolve(&self.transform())?)),
            out_bbox: self.out_bbox(layout)?,
            clip: self.clip_bbox(layout, canvas),
            round_to_pixels: true,
            magnification,
        })
    }

    /// matplotlib's `AxesImage.make_image` at `magnification`.
    pub(super) fn render(
        &self,
        ctx: &mut DrawContext<'_>,
        magnification: f64,
    ) -> Result<Option<RenderedImage>, ImageError> {
        let canvas = canvas_bbox(ctx);
        let Some(placement) = self.placement(*ctx.layout(), canvas, magnification) else {
            return Ok(None);
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
        let canvas = canvas_bbox(ctx);
        let clip = self.clip_bbox(*ctx.layout(), canvas);
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
        axes_image_contains(&self.base, ctx, point, || self.out_bbox(*ctx))
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
            field.autoscale(self.core.mappable.get_mut().norm_mut());
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

impl TypedHandle for AxesImage {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<AxesImageArtist>()
            .then(|| Self::from_raw(raw.clone()))
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

    /// Whether data or colours are resampled, as set (matplotlib's
    /// `get_interpolation_stage`).
    pub fn interpolation_stage(&self) -> InterpolationStage {
        self.read(|image| image.core.stage)
    }

    /// Sets whether the weights of the resampling kernel are normalised
    /// (matplotlib's `set_filternorm`).
    pub fn set_filternorm(&self, filternorm: bool) -> &Self {
        self.update(|image| image.core.resampling.filternorm = filternorm)
    }

    /// Whether the weights of the resampling kernel are normalised
    /// (matplotlib's `get_filternorm`).
    pub fn filternorm(&self) -> bool {
        self.read(|image| image.core.resampling.filternorm)
    }

    /// Sets the radius of the sinc-family kernels (matplotlib's
    /// `set_filterrad`).
    ///
    /// # Errors
    ///
    /// When the radius is not positive.
    pub fn set_filterrad(&self, radius: f64) -> Result<&Self, ImageError> {
        if radius.is_nan() || radius <= 0.0 {
            return Err(ImageError::FilterRadius(radius));
        }
        Ok(self.update(|image| image.core.resampling.filterrad = radius))
    }

    /// The radius of the sinc-family kernels (matplotlib's
    /// `get_filterrad`).
    pub fn filterrad(&self) -> f64 {
        self.read(|image| image.core.resampling.filterrad)
    }

    /// Sets whether downsampling resamples fully (matplotlib's
    /// `set_resample`).
    pub fn set_resample(&self, resample: bool) -> &Self {
        self.update(|image| image.core.resampling.resample = resample)
    }

    /// Whether downsampling resamples fully (matplotlib's `get_resample`).
    pub fn resample(&self) -> bool {
        self.read(|image| image.core.resampling.resample)
    }

    /// Changes the opacity: uniform, or one value per pixel.
    pub fn set_image_alpha(&self, alpha: Option<ImageAlpha>) -> &Self {
        self.update(|image| {
            let (uniform, grid) = split_alpha(alpha);
            image.base.alpha = uniform;
            image.core.alpha_grid = grid;
        })
    }

    /// The opacity: uniform, one value per pixel, or none (matplotlib's
    /// `get_alpha`).
    pub fn image_alpha(&self) -> Option<ImageAlpha> {
        self.read(|image| image.core.alpha(image.base.alpha))
    }

    /// The image resampled and colour-mapped for display at
    /// `magnification`, as the figure is laid out now (matplotlib's
    /// `make_image`): its pixels, top row first, where they go and how the
    /// array maps onto them; `None` when nothing of it is visible.
    ///
    /// ```
    /// use pltrs::axes::ImshowOptions;
    /// use pltrs::numeric::Grid;
    /// use pltrs::prelude::*;
    /// let fig = Figure::with_size(2.0, 2.0, 100.0);
    /// let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    /// let image = ax.imshow(Grid::filled(2, 2, 1.0), ImshowOptions::default())?;
    /// let made = image.make_image(1.0)?.expect("the image is visible");
    /// assert_eq!((made.pixels.width(), made.pixels.height()), (200, 200));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the image's colormap or norm cannot map its data.
    pub fn make_image(&self, magnification: f64) -> Result<Option<MadeImage>, ImageError> {
        let state = self.raw.figure_state().borrow();
        let layout = LayoutContext::new(&state);
        let (width, height) = state.size_pixels();
        let canvas = Bbox::from_extents(0.0, 0.0, width, height);
        self.read(|image| {
            let Some(placement) = image.placement(layout, canvas, magnification) else {
                return Ok(None);
            };
            let alpha = image.core.alpha(image.base.alpha);
            let made = image.core.make_mapped(&placement, alpha.as_ref())?;
            Ok(made.map(|(rendered, map)| MadeImage {
                pixels: rendered.pixels,
                origin: rendered.origin,
                transform: map.affine(),
            }))
        })
    }

    /// The image array (matplotlib's `get_array`).
    pub fn array(&self) -> ImageData {
        self.read(|image| image.core.data.clone())
    }

    /// Replaces the image array, keeping the extent; unset limits of the
    /// norm come from new scalar data (matplotlib's `set_data`).
    ///
    /// ```
    /// use pltrs::axes::ImshowOptions;
    /// use pltrs::numeric::Grid;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let image = ax.imshow(Grid::filled(2, 2, 1.0), ImshowOptions::default())?;
    /// let extent = image.extent();
    /// image.set_data(Grid::filled(3, 4, 2.0));
    /// assert_eq!((image.shape(), image.extent()), ((3, 4), extent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_data(&self, data: impl Into<ImageData>) -> &Self {
        let data = data.into().normalized();
        self.update(|image| image.core.replace_data(data))
    }

    /// The colormap.
    pub fn cmap(&self) -> Colormap {
        self.read(|image| image.core.mappable.borrow().cmap().clone())
    }

    /// The norm (matplotlib's `norm`).
    pub fn norm(&self) -> Box<dyn Norm> {
        self.read(|image| image.core.mappable.borrow().norm().clone_box())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::ImshowOptions;
    use crate::figure::Figure;
    use crate::numeric::Grid;
    use crate::patches::{Circle, Rectangle, RotationPoint};

    #[test]
    fn resampling_settings_read_back_and_reject_bad_radii() {
        let ax = Figure::new().subplot();
        let image = ax
            .imshow(Grid::filled(2, 2, 1.0), ImshowOptions::default())
            .unwrap();
        assert_eq!((image.filternorm(), image.filterrad()), (true, 4.0));
        assert_eq!(image.interpolation_stage(), InterpolationStage::Auto);
        image
            .set_filternorm(false)
            .set_resample(true)
            .set_interpolation_stage(InterpolationStage::Rgba);
        assert_eq!(
            image.set_filterrad(2.5).ok().map(AxesImage::filterrad),
            Some(2.5)
        );
        for bad in [0.0, -1.0, f64::NAN] {
            assert!(image.set_filterrad(bad).is_err(), "{bad}");
        }
        assert_eq!(image.filterrad(), 2.5);
        assert!(!image.filternorm() && image.resample());
        assert_eq!(image.interpolation_stage(), InterpolationStage::Rgba);
    }

    #[test]
    fn new_data_keep_the_extent_and_fill_unset_limits() {
        let ax = Figure::new().subplot();
        let image = ax
            .imshow(Grid::filled(2, 2, 1.0), ImshowOptions::default())
            .unwrap();
        assert_eq!(image.norm().vmin(), Some(1.0));
        assert_eq!(image.image_alpha(), None);
        image.set_clim(None, Some(4.0));
        let extent = image.extent();
        image.set_data(Grid::from_fn(3, 3, |r, c| (r + c) as f64));
        assert_eq!((image.shape(), image.extent()), ((3, 3), extent));
        assert_eq!(image.clim(), (Some(1.0), Some(4.0)), "set limits stay");
        assert_eq!(image.array().shape(), (3, 3));
        image.set_image_alpha(Some(ImageAlpha::Uniform(0.5)));
        assert_eq!(image.image_alpha(), Some(ImageAlpha::Uniform(0.5)));
    }

    #[test]
    fn rectangles_clip_to_their_bounding_box_and_other_shapes_to_their_outline() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        let image = ax
            .imshow(Grid::filled(4, 4, 1.0), ImshowOptions::default())
            .unwrap();
        ax.set_xlim(0.0, 4.0)
            .and_then(|ax| ax.set_ylim(0.0, 4.0))
            .expect("finite limits");
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
