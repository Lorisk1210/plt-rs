//! Images attached to an axes (matplotlib's `AxesImage`).

use std::cell::RefCell;
use std::rc::Rc;

use pltrs_core::{Bbox, Path, Point};

use super::ahead::MadeAhead;
use super::any::checked_filterrad;
use super::make::{ImageCore, MadeImage, Placement, RenderedImage};
use super::resample::PixelMap;
use super::shared::{axes_image_contains, canvas_bbox, draw_rendered, split_alpha};
use super::unsampled::UnsampledImage;
use super::{
    BboxImageOptions, Extent, ImageAlpha, ImageData, ImageError, Interpolation, InterpolationStage,
    Origin,
};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, ClipPathSpec, DrawContext, LayoutContext,
    LimitPoints, ProgramShaping, RawHandle, TypedHandle,
};
use crate::axes::Axes;
use crate::colors::{Colormap, Norm};
use crate::patches::PatchShape;
use crate::transforms::{ResolvedTransform, Transform, TransformContext};

/// State of an image on a regular grid in data coordinates.
#[derive(Clone, Debug)]
pub(crate) struct AxesImageArtist {
    base: ArtistBase,
    pub(crate) core: ImageCore,
    /// The data rectangle covered, `None` until one is set: then the
    /// pixels of the array are centred on their indices (matplotlib's
    /// `_extent`).
    pub(crate) extent: Option<Extent>,
    /// The image made ahead of its next paint, through a norm of the
    /// program's (see [`super::ahead`]).
    pub(super) made_ahead: RefCell<Option<MadeAhead>>,
}

pltrs_core::archive_struct!(AxesImageArtist { base, core, extent } skip { made_ahead });

impl AxesImageArtist {
    /// An image at z-order 0 covering `extent`.
    pub(crate) fn new(core: ImageCore, extent: Extent) -> Self {
        Self::placed(core, Some(extent))
    }

    /// An image at z-order 0 covering `extent`, or its default extent.
    fn placed(core: ImageCore, extent: Option<Extent>) -> Self {
        let mut base = ArtistBase::with_zorder(0.0);
        // Status bars show the value under the pointer, as for matplotlib's
        // images.
        base.mouseover = true;
        Self {
            base,
            core,
            extent,
            made_ahead: RefCell::new(None),
        }
    }

    /// Whether the image's norm is the program's.
    pub(super) fn norm_calls_program(&self) -> bool {
        self.core.mappable.borrow().norm().calls_program()
    }

    /// Whether the image's norm or colormap is the program's.
    fn maps_through_program(&self) -> bool {
        let mappable = self.core.mappable.borrow();
        mappable.norm().calls_program() || mappable.cmap().calls_program()
    }

    /// The data rectangle covered: the one set, else the array's pixels
    /// centred on their indices (matplotlib's `get_extent`).
    pub(crate) fn extent(&self) -> Extent {
        self.extent.unwrap_or_else(|| {
            let (rows, cols) = self.core.data.shape();
            Extent::default_for(rows, cols, self.core.resampling.origin)
        })
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
        } = self.extent();
        if self.core.resampling.origin == Origin::Upper {
            std::mem::swap(&mut bottom, &mut top);
        }
        let (rows, columns) = self.core.data.shape();
        let row = (data.y - bottom) / (top - bottom) * rows as f64;
        let column = (data.x - left) / (right - left) * columns as f64;
        index_below(row, rows).zip(index_below(column, columns))
    }

    /// The transform the image is placed with: display coordinates
    /// unless it was given one.
    pub(super) fn transform(&self) -> Transform {
        self.base.transform.clone().unwrap_or(Transform::Display)
    }

    /// The display box of the extent (matplotlib's `TransformedBbox` of
    /// the extent corners).
    fn out_bbox(&self, ctx: LayoutContext<'_>) -> Option<Bbox> {
        Self::out_bbox_over(ctx, self.extent(), &self.transform())
    }

    /// The display box of `extent` through `transform`.
    fn out_bbox_over(
        ctx: LayoutContext<'_>,
        extent: Extent,
        transform: &Transform,
    ) -> Option<Bbox> {
        let resolved = ctx.resolve(transform)?;
        let Extent {
            left,
            right,
            bottom,
            top,
        } = extent;
        // The corners as matplotlib's `TransformedBbox` bounds them: a
        // transform that is not finite leaves a box at NaN, which
        // intersects nothing, so that nothing of the image is made.
        let enclosing = Bbox::from_extents(left, bottom, right, top)
            .mapped_enclosing(|corner| resolved.transform_point(corner));
        Some(Bbox::from_extents(
            enclosing.xmin(),
            enclosing.ymin(),
            enclosing.xmax(),
            enclosing.ymax(),
        ))
    }

    /// The display box the image is clipped to (matplotlib's clip box,
    /// axes box or figure box, the `canvas`): the box of the axes clipping
    /// it to its outline rather than to its box, too (matplotlib's
    /// `get_clip_box() or self.axes.bbox`).
    fn clip_bbox(&self, layout: LayoutContext<'_>, canvas: Bbox) -> Bbox {
        let clip = if self.base.clip_on {
            layout.clip_rect(&self.base).or_else(|| {
                let owner = self.base.clip_box.or(self.base.parent)?;
                layout
                    .clip_extent(owner)
                    .or_else(|| layout.axes_extent(owner))
            })
        } else {
            None
        };
        clip.unwrap_or(canvas)
    }

    /// Where the image goes at `magnification` on a figure of size
    /// `canvas` (matplotlib's `AxesImage.make_image`); `None` when its
    /// transform refers to removed artists.
    pub(super) fn placement(
        &self,
        layout: LayoutContext<'_>,
        canvas: Bbox,
        magnification: f64,
    ) -> Option<Placement> {
        self.placement_over(
            layout,
            canvas,
            magnification,
            self.extent(),
            &self.transform(),
        )
    }

    /// [`placement`](Self::placement) of the image stretched over `extent`
    /// through `transform`.
    fn placement_over(
        &self,
        layout: LayoutContext<'_>,
        canvas: Bbox,
        magnification: f64,
        extent: Extent,
        transform: &Transform,
    ) -> Option<Placement> {
        Some(Placement {
            extent,
            transform: Some(PixelMap::from_resolved(layout.resolve(transform)?)),
            out_bbox: Self::out_bbox_over(layout, extent, transform)?,
            clip: self.clip_bbox(layout, canvas),
            round_to_pixels: true,
            magnification,
        })
    }

    /// matplotlib's `AxesImage.make_image` at `magnification`: the image
    /// made ahead of this paint, if it was for the same boxes, else made
    /// now.
    pub(super) fn render(
        &self,
        ctx: &mut DrawContext<'_>,
        magnification: f64,
    ) -> Result<Option<RenderedImage>, ImageError> {
        let ahead = self.made_ahead.take();
        let canvas = canvas_bbox(ctx);
        let Some(placement) = self.placement(ctx.layout(), canvas, magnification) else {
            return Ok(None);
        };
        if let Some(made) = ahead.and_then(|ahead| ahead.for_placement(&placement)) {
            return made;
        }
        // A norm or a transform of the program's maps the data with the
        // figure free.
        ctx.free();
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
        let clip = self.clip_bbox(ctx.layout(), canvas);
        let resolved = ctx
            .resolve(&self.transform())
            .filter(ResolvedTransform::is_affine)?;
        let out_bbox = self.out_bbox(ctx.layout())?;
        // A norm of the program's maps the data with the figure free.
        ctx.free();
        Some(
            self.core
                .unsampled(self.extent(), resolved.affine_part(), out_bbox, clip),
        )
    }
}

impl Artist for AxesImageArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    /// A norm or a colormap of the program's mapping the image as the draw
    /// a print makes before it renders makes it (`make_image`, at the
    /// layout of that draw); where the figure's draws are walked for the
    /// program's tickers, the image is made ahead there instead (see
    /// `super::ahead`).
    fn program_shaping(&self, layout: &LayoutContext<'_>) -> Option<ProgramShaping> {
        let state = layout.figure_state();
        if !self.maps_through_program() || state.ticks_by_program() || !self.base.visible {
            return None;
        }
        // That draw renders the whole figure, before it is cut.
        let placement = self.placement(*layout, layout.figure_box(), 1.0)?;
        let alpha = self.core.alpha(self.base.alpha);
        let core = self.core.clone();
        Some(Box::new(move || {
            drop(core.make_image(&placement, alpha.as_ref()));
        }))
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
            // Unsampled, the image made ahead is not drawn.
            drop(self.made_ahead.take());
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
                Err(err) => ctx.report_kind(self.base.id(), err.draw_kind(), err),
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
    fn limit_points(&self, _axes: ArtistId, _ctx: &dyn TransformContext) -> Option<LimitPoints> {
        let Extent {
            left,
            right,
            bottom,
            top,
        } = self.extent();
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

super::impl_color_mapped!(AxesImageArtist, core.data, core.mappable);

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

super::impl_mapping_changes!(AxesImage, AxesImageArtist);

/// Fits the axes holding the image `raw` to its new `extent` (see
/// [`Axes::fit_image`]); an image in no axes moves no limits.
pub(crate) fn fit_axes_to(raw: &RawHandle, extent: Extent) {
    if let Some(axes) = raw.with_base(|base| base.parent) {
        let figure = Rc::clone(raw.figure_state());
        Axes::from_raw(RawHandle::new(figure, axes)).fit_image(raw, extent);
    }
}

impl AxesImage {
    /// The handle of the image behind `raw`.
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// An image of `data` made as `options` say, covering `extent` (by
    /// default its pixels centred on their indices, following the data),
    /// outside any figure until an axes adopts it (matplotlib's
    /// `AxesImage(ax, ...)` before `ax.add_image`; see
    /// [`Axes::adopt_artist`]). Unlike `imshow`, adopting it leaves the
    /// axes' limits and aspect alone.
    ///
    /// ```
    /// use pltrs::artist::{ArtistHandle, TypedHandle};
    /// use pltrs::axes::AddAs;
    /// use pltrs::image::{AxesImage, BboxImageOptions, Extent, ImageData};
    /// use pltrs::numeric::Grid;
    /// use pltrs::prelude::*;
    /// let image = AxesImage::loose(ImageData::empty(), BboxImageOptions::default(), None)?;
    /// image.set_data(Grid::filled(2, 3, 1.0));
    /// assert_eq!(image.extent(), Extent::new(-0.5, 2.5, 1.5, -0.5));
    /// let ax = Figure::new().subplot();
    /// let added = ax.adopt_artist(image.raw(), AddAs::Image)?;
    /// assert_eq!(ax.images(), [AxesImage::from_raw_handle(&added).unwrap()]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the colormap, norm or filter radius of `options` is invalid.
    pub fn loose(
        data: impl Into<ImageData>,
        options: BboxImageOptions,
        extent: Option<Extent>,
    ) -> Result<Self, ImageError> {
        let (core, uniform) =
            ImageCore::build(data, options.core_settings(), &crate::rc::params())?;
        let mut artist = AxesImageArtist::placed(core, extent);
        artist.base_mut().alpha = uniform;
        Ok(Self::from_raw(RawHandle::loose(Box::new(artist))))
    }

    fn read<R>(&self, f: impl FnOnce(&AxesImageArtist) -> R) -> R {
        self.raw.with::<AxesImageArtist, _>(f)
    }

    fn update(&self, f: impl FnOnce(&mut AxesImageArtist)) -> &Self {
        self.raw.with_mut::<AxesImageArtist, _>(f);
        self
    }

    /// The data rectangle the image covers: the one set, else its pixels
    /// centred on their indices.
    pub fn extent(&self) -> Extent {
        self.read(AxesImageArtist::extent)
    }

    /// Moves the image to cover `extent` (matplotlib's `set_extent`): the
    /// axes' data limits grow to include it and the view limits of the
    /// axes that autoscale become it.
    pub fn set_extent(&self, extent: Extent) -> &Self {
        self.update(|image| image.extent = Some(extent));
        fit_axes_to(&self.raw, extent);
        self
    }

    /// Moves the image to cover `extent`, or back to its pixels centred on
    /// their indices for `None`, leaving the axes' limits as they are (a
    /// program assigning matplotlib's `_extent`).
    pub fn assign_extent(&self, extent: Option<Extent>) -> &Self {
        self.update(|image| image.extent = extent)
    }

    /// `(rows, columns)` of the image array.
    pub fn shape(&self) -> (usize, usize) {
        self.read(|image| image.core.data.shape())
    }

    /// The `(row, column)` of the array element under display point `at`
    /// in the figure's current layout (laid out first), `None` outside the
    /// image (matplotlib's `AxesImage.get_cursor_data`, which truncates
    /// the indices towards zero: less than an element before the first
    /// row or column still gives it).
    ///
    /// ```
    /// use pltrs::axes::ImshowOptions;
    /// use pltrs::numeric::Grid;
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Point;
    /// let fig = Figure::with_size(2.0, 2.0, 100.0);
    /// let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    /// let image = ax.imshow(Grid::filled(2, 2, 1.0), ImshowOptions::default())?;
    /// // Row 0 is drawn at the top, column 1 on the right.
    /// assert_eq!(image.element_at(Point::new(150.0, 150.0)), Some((0, 1)));
    /// assert_eq!(image.element_at(Point::new(-150.0, 150.0)), None);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn element_at(&self, at: Point) -> Option<(usize, usize)> {
        let state = self.raw.figure_state().prepared();
        let ctx = LayoutContext::new(&state);
        ctx.store()
            .get_as::<AxesImageArtist>(self.raw.id())?
            .element_at(ctx, at)
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
        let radius = checked_filterrad(radius)?;
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
        let (extent, transform) = self.read(|image| (image.extent(), image.transform()));
        self.make_image_over(magnification, false, extent, &transform)
    }

    /// [`make_image`](Self::make_image), or with `unsampled`
    /// [`make_image_unsampled`](Self::make_image_unsampled), of the image
    /// stretched over `extent` through `transform` in place of its own
    /// (matplotlib's `make_image` reads `get_extent` and `get_transform`,
    /// which a subclass of the program's may override).
    ///
    /// ```
    /// use pltrs::axes::ImshowOptions;
    /// use pltrs::image::Extent;
    /// use pltrs::numeric::Grid;
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Transform;
    /// let fig = Figure::with_size(2.0, 2.0, 100.0);
    /// let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    /// let image = ax.imshow(Grid::filled(2, 2, 1.0), ImshowOptions::default())?;
    /// let corner = Extent::new(0.0, 50.0, 0.0, 50.0);
    /// let made = image.make_image_over(1.0, false, corner, &Transform::Display)?.expect("shown");
    /// assert_eq!((made.pixels.width(), made.pixels.height()), (50, 50));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When its colormap or norm cannot map its data.
    pub fn make_image_over(
        &self,
        magnification: f64,
        unsampled: bool,
        extent: Extent,
        transform: &Transform,
    ) -> Result<Option<MadeImage>, ImageError> {
        let state = self.raw.figure_state().borrow();
        let layout = LayoutContext::new(&state);
        let (width, height) = state.size_pixels();
        let canvas = Bbox::from_extents(0.0, 0.0, width, height);
        self.read(|image| {
            let Some(placement) =
                image.placement_over(layout, canvas, magnification, extent, transform)
            else {
                return Ok(None);
            };
            let alpha = image.core.alpha(image.base.alpha);
            image.core.made(&placement, alpha.as_ref(), unsampled)
        })
    }

    /// The image colour-mapped at its own resolution, as renderers that
    /// scale images themselves receive it (matplotlib's `make_image(...,
    /// unsampled=True)`): the visible part of the coloured array, top row
    /// first, the display position of its bottom-left corner, and the map
    /// from its cells (column, row) to the display, counted from that
    /// corner and magnified by `magnification` (when the image's map is
    /// affine; its data transform maps it otherwise, as for
    /// [`AxesImage::make_image`]); `None` when nothing of it is visible.
    ///
    /// ```
    /// use pltrs::axes::ImshowOptions;
    /// use pltrs::numeric::Grid;
    /// use pltrs::prelude::*;
    /// let fig = Figure::with_size(2.0, 2.0, 100.0);
    /// let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    /// let image = ax.imshow(Grid::filled(2, 2, 1.0), ImshowOptions::default())?;
    /// let made = image.make_image_unsampled(1.0)?.expect("the image is visible");
    /// assert_eq!((made.pixels.width(), made.pixels.height()), (2, 2));
    /// let cell = made.transform.expect("an affine map").transform_point((1.0, 1.0).into());
    /// assert_eq!((cell.x, cell.y), (100.0, 100.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When its colormap or norm cannot map its data.
    pub fn make_image_unsampled(
        &self,
        magnification: f64,
    ) -> Result<Option<MadeImage>, ImageError> {
        let (extent, transform) = self.read(|image| (image.extent(), image.transform()));
        self.make_image_over(magnification, true, extent, &transform)
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
    fn an_image_through_a_transform_that_is_not_finite_is_not_made() {
        // Beyond 1 the logit scale maps the view onto one point: the data
        // transform is at NaN, and so is the box of the image, which
        // matplotlib then finds outside whatever clips it.
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_axis_off();
        let empty = fig.to_png().expect("drawn");
        let image = ax
            .imshow(Grid::filled(2, 2, 1.0), ImshowOptions::default())
            .unwrap();
        ax.set_ylim(0.5, 11.0).expect("limits");
        ax.set_yscale("logit").expect("a scale");
        for clipped in [true, false] {
            image.set_clip_on(clipped);
            assert_eq!(fig.to_png().expect("drawn"), empty, "clipped: {clipped}");
        }
    }

    #[test]
    fn resampling_settings_read_back_and_reject_radii_at_most_zero() {
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
        for bad in [0.0, -1.0] {
            assert!(image.set_filterrad(bad).is_err(), "{bad}");
        }
        assert_eq!(image.filterrad(), 2.5);
        image
            .set_filterrad(f64::NAN)
            .expect("NaN is kept, as matplotlib's `set_filterrad` keeps it");
        assert!(image.filterrad().is_nan());
        image.set_filterrad(2.5).expect("a positive radius");
        assert!(!image.filternorm() && image.resample());
        assert_eq!(image.interpolation_stage(), InterpolationStage::Rgba);
    }

    #[test]
    fn an_assigned_extent_leaves_the_limits() {
        let ax = Figure::new().subplot();
        let image = ax
            .imshow(Grid::filled(2, 3, 1.0), ImshowOptions::default())
            .unwrap();
        let (centred, limits) = (image.extent(), ax.xlim());
        let moved = Extent {
            left: 0.0,
            right: 1.0,
            bottom: 0.0,
            top: 1.0,
        };
        image.assign_extent(Some(moved));
        assert_eq!((image.extent(), ax.xlim()), (moved, limits));
        image.assign_extent(None);
        assert_eq!(image.extent(), centred);
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
