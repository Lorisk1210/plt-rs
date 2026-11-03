//! Contexts handed to artists while measuring and drawing.

use pltrs_core::{Bbox, Point};
use pltrs_render::{ClipPath, GraphicsContext, Renderer, SnapMode};

use super::{Artist, ArtistBase, ArtistId, ArtistLookup, ArtistStore, ClipPathSpec};
use crate::figure::FigureState;
use crate::layout::Aligned;
use crate::ticker::AxisName;
use crate::transforms::{ResolvedTransform, Transform, TransformContext};

/// Read-only access to figure geometry, for measuring and hit testing.
#[derive(Clone, Copy, Debug)]
pub struct LayoutContext<'a> {
    figure: &'a FigureState,
    /// Artists measured as drawn at the canvas resolution: those that
    /// draw before what is being placed for the coming draw.
    drawn: &'a [ArtistId],
}

impl<'a> LayoutContext<'a> {
    pub(crate) fn new(figure: &'a FigureState) -> Self {
        Self { figure, drawn: &[] }
    }

    /// The same context, measuring the arrows of `drawn` as drawn at the
    /// canvas resolution: what matplotlib measures while drawing sees the
    /// artists drawn earlier in the same draw that way.
    pub(crate) fn with_drawn(self, drawn: &'a [ArtistId]) -> Self {
        Self { drawn, ..self }
    }

    /// Every artist of the figure.
    pub fn store(&self) -> &'a ArtistStore {
        &self.figure.store
    }

    /// The figure's state.
    pub(crate) fn figure_state(&self) -> &'a FigureState {
        self.figure
    }

    /// The frame of the root figure.
    pub(crate) fn root(&self) -> ArtistId {
        self.figure.root
    }

    /// Canvas resolution.
    pub fn dpi(&self) -> f64 {
        self.figure.dpi()
    }

    /// Canvas size in pixels.
    pub fn canvas_size(&self) -> (f64, f64) {
        self.figure.size_pixels()
    }

    /// Whether the figure keeps adjacent images apart or composites them
    /// (`None` = as the renderer prefers; matplotlib's `suppressComposite`).
    pub(crate) fn suppress_composite(self) -> Option<bool> {
        self.figure.suppress_composite
    }

    /// Whether child `id` is left out of a draw as animated: figures
    /// never draw animated artists, axes only while the figure is saved
    /// (matplotlib's `Figure._get_draw_artists` and `_AxesBase.draw`).
    pub(crate) fn skips_animated(self, id: ArtistId, in_axes: bool) -> bool {
        let animated = self.store().get(id).is_some_and(|a| a.base().animated);
        animated && !(in_axes && self.figure.saving)
    }

    /// Converts points to pixels at the canvas resolution.
    pub fn points_to_pixels(&self, points: f64) -> f64 {
        points * self.figure.dpi() / 72.0
    }

    /// Pixels per point of the sizes of an arrow (its head, width and
    /// shrinking) when it is measured rather than drawn. matplotlib's
    /// `FancyArrowPatch` scales them by a `_dpi_cor` that only drawing
    /// sets, so an arrow measured before the figure was first drawn (by
    /// the layout engine, or by an anchored box laid out in the first
    /// draw) is measured at one pixel per point; afterwards at the
    /// resolution of the last draw.
    pub fn measured_dpi_cor(&self) -> f64 {
        self.figure.last_draw_dpi.map_or(1.0, |dpi| dpi / 72.0)
    }

    /// Resolves a transform; `None` if it refers to removed artists.
    pub fn resolve(&self, transform: &Transform) -> Option<ResolvedTransform> {
        transform.resolve(self.figure).ok()
    }

    /// Display-space extent of a figure or subfigure frame.
    pub fn frame_extent(&self, frame: ArtistId) -> Option<Bbox> {
        self.figure.frame_bbox(frame)
    }

    /// Where the background of a frame is drawn (the whole canvas for the
    /// root figure while part of it is saved).
    pub fn frame_patch_extent(&self, frame: ArtistId) -> Option<Bbox> {
        self.figure.frame_patch_bbox(frame)
    }

    /// Display-space extent of an axes.
    pub fn axes_extent(&self, axes: ArtistId) -> Option<Bbox> {
        self.figure.axes_bbox(axes)
    }

    /// Whether display point `at` is over the background of `axes`: its
    /// box, or the wedge of polar axes (matplotlib's `Axes.contains`).
    pub fn axes_contains(&self, axes: ArtistId, at: Point) -> bool {
        match self.figure.axes_outline(axes) {
            Some(outline) => outline
                .path
                .contains_point(at, Some(&outline.transform), 0.0),
            None => self.axes_extent(axes).is_some_and(|bbox| bbox.contains(at)),
        }
    }

    /// Display-space extent of an axes or figure frame used as a clip box
    /// (`None` for polar and geographic axes, which clip to their outline
    /// instead).
    pub fn clip_extent(&self, owner: ArtistId) -> Option<Bbox> {
        if self.figure.axes_outline(owner).is_some() {
            return None;
        }
        self.axes_extent(owner).or_else(|| self.frame_extent(owner))
    }

    /// The display rectangle an artist is clipped to (matplotlib's clip
    /// box): the bounding box of a [`ClipPathSpec::BoundingBox`], else
    /// the extent of the frame in `base.clip_box`. Clipping being on is
    /// not checked.
    pub fn clip_rect(&self, base: &ArtistBase) -> Option<Bbox> {
        match &base.clip_path {
            Some(ClipPathSpec::BoundingBox { path, transform }) => {
                Some(self.resolve(transform)?.transform_path(path).extents())
            }
            _ => base.clip_box.and_then(|owner| self.clip_extent(owner)),
        }
    }

    /// The outline artists clipped to `owner` are clipped to when it is
    /// not a rectangle (the wedge of polar axes, the ellipse of
    /// geographic axes).
    pub fn clip_outline(&self, owner: ArtistId) -> Option<ClipPath> {
        self.figure.axes_outline(owner)
    }

    /// The axes whose `axis` labels align with those of `axes` in its
    /// figure, `axes` included (see
    /// [`Figure::align_labels`](crate::figure::Figure::align_labels)).
    pub(crate) fn label_siblings(self, axes: ArtistId, axis: AxisName) -> Vec<ArtistId> {
        match self.store().get(axes).and_then(|a| a.base().parent) {
            Some(frame) => {
                self.figure
                    .layout
                    .label_groups
                    .siblings(frame, Aligned::Labels(axis), axes)
            }
            None => vec![axes],
        }
    }
}

/// What kind of problem a [`DrawError`] is, after the exception
/// matplotlib raises for it, so that a binding can raise it the same way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DrawErrorKind {
    /// A value the program gave that cannot be drawn, such as malformed
    /// mathtext or a line through two identical points (matplotlib's
    /// `ValueError`).
    InvalidValue,
    /// A value of the wrong type the program gave, which matplotlib's
    /// renderers refuse when they draw it, such as a single path given to a
    /// collection as its paths (matplotlib's `TypeError`).
    InvalidType,
    /// Any other problem (matplotlib's `RuntimeError`).
    #[default]
    Other,
}

/// A problem that kept (part of) an artist from being drawn, such as an
/// invalid colour string given to a setter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawError {
    /// The artist concerned.
    pub artist: ArtistId,
    /// What went wrong.
    pub message: String,
    /// What kind of problem it is.
    pub kind: DrawErrorKind,
}

impl DrawError {
    /// A problem of [`DrawErrorKind::Other`] with `artist`.
    pub fn new(artist: ArtistId, message: impl Into<String>) -> Self {
        Self {
            artist,
            message: message.into(),
            kind: DrawErrorKind::Other,
        }
    }
}

impl std::fmt::Display for DrawError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "artist {}: {}", self.artist, self.message)
    }
}

impl std::error::Error for DrawError {}

/// Where drawing stands with respect to rasterized artists (the
/// `_raster_depth` and `_rasterizing` attributes matplotlib keeps on the
/// renderer).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rasterization {
    /// How many rasterized artists are being drawn, one inside the other.
    depth: usize,
    /// Whether the renderer draws into a raster canvas.
    active: bool,
    /// The figure's resolution to restore when rasterizing stops, while
    /// the figure is drawn at the raster canvas's.
    figure_dpi: Option<f64>,
    /// The figure's resolution to restore when the rasterizing a program
    /// asked for stops (see [`DrawContext::start_program_rasterizing`]).
    program_figure_dpi: Option<f64>,
}

/// Everything an artist needs while drawing: the renderer plus read-only
/// access to the rest of the figure.
pub struct DrawContext<'a> {
    layout: LayoutContext<'a>,
    renderer: &'a mut dyn Renderer,
    errors: Vec<DrawError>,
    rasterization: Rasterization,
}

impl std::fmt::Debug for DrawContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DrawContext")
            .field("layout", &self.layout)
            .finish_non_exhaustive()
    }
}

impl<'a> DrawContext<'a> {
    pub(crate) fn new(figure: &'a FigureState, renderer: &'a mut dyn Renderer) -> Self {
        Self {
            layout: LayoutContext::new(figure),
            renderer,
            errors: Vec::new(),
            rasterization: Rasterization::default(),
        }
    }

    /// Records a problem; drawing continues with the remaining artists.
    pub fn report(&mut self, artist: ArtistId, problem: impl std::fmt::Display) {
        self.report_kind(artist, DrawErrorKind::Other, problem);
    }

    /// Records a problem of `kind`; drawing continues with the remaining
    /// artists.
    pub fn report_kind(
        &mut self,
        artist: ArtistId,
        kind: DrawErrorKind,
        problem: impl std::fmt::Display,
    ) {
        self.errors.push(DrawError {
            artist,
            message: problem.to_string(),
            kind,
        });
    }

    /// The problems recorded while drawing.
    pub(crate) fn into_errors(self) -> Vec<DrawError> {
        self.errors
    }

    /// Read-only geometry access.
    pub fn layout(&self) -> &LayoutContext<'a> {
        &self.layout
    }

    /// How many rasterized artists are being drawn, one inside the other
    /// (matplotlib's `renderer._raster_depth`).
    pub fn raster_depth(&self) -> usize {
        self.rasterization.depth
    }

    /// The output backend.
    pub fn renderer(&mut self) -> &mut dyn Renderer {
        self.renderer
    }

    /// Resolves a transform; `None` if it refers to removed artists.
    pub fn resolve(&self, transform: &Transform) -> Option<ResolvedTransform> {
        self.layout.resolve(transform)
    }

    /// Converts points to pixels at the canvas resolution.
    pub fn points_to_pixels(&self, points: f64) -> f64 {
        self.layout.points_to_pixels(points)
    }

    /// Runs `f` with every drawing call going through the renderer that
    /// `wrap` builds around this context's renderer (matplotlib swaps in a
    /// `PathEffectRenderer` this way).
    pub fn with_wrapped_renderer<R>(
        &mut self,
        wrap: impl for<'r> FnOnce(&'r mut dyn Renderer) -> Box<dyn Renderer + 'r>,
        f: impl FnOnce(&mut DrawContext<'_>) -> R,
    ) -> R {
        let mut wrapped = wrap(&mut *self.renderer);
        let mut inner = DrawContext {
            layout: self.layout,
            renderer: &mut *wrapped,
            errors: Vec::new(),
            rasterization: self.rasterization,
        };
        let result = f(&mut inner);
        let DrawContext {
            errors,
            rasterization,
            ..
        } = inner;
        self.errors.extend(errors);
        self.rasterization = rasterization;
        result
    }

    /// Runs `draw`, the draw call of an artist with the given `rasterized`
    /// property, under matplotlib's rasterization bookkeeping
    /// (`allow_rasterization`): a rasterized artist outside any other
    /// starts rasterizing, an artist that is not rasterized outside any
    /// rasterized one stops it, so that consecutive rasterized artists
    /// share one image, unless the figure suppresses compositing.
    pub fn draw_artist<R>(&mut self, rasterized: bool, draw: impl FnOnce(&mut Self) -> R) -> R {
        let outermost = self.rasterization.depth == 0;
        if rasterized {
            if outermost && !self.rasterization.active {
                self.start_rasterizing();
            }
            self.rasterization.depth += 1;
        } else if outermost {
            self.finish_rasterizing();
        }
        let result = draw(self);
        if rasterized {
            self.rasterization.depth -= 1;
        }
        if self.rasterization.active && self.layout.suppress_composite() == Some(true) {
            // The figure's `suppressComposite` keeps what each artist
            // rasterized in an image of its own.
            self.renderer.stop_rasterizing();
            self.renderer.start_rasterizing();
        }
        result
    }

    /// Sends the drawing to the renderer's raster canvas, if it has one,
    /// and draws the figure at the canvas's resolution meanwhile
    /// (matplotlib's `MixedModeRenderer.start_rasterizing`).
    fn start_rasterizing(&mut self) {
        self.renderer.start_rasterizing();
        self.rasterization.active = true;
        if let Some(dpi) = self.renderer.raster_dpi() {
            let figure = self.layout.figure;
            self.rasterization.figure_dpi = Some(figure.dpi());
            figure.set_dpi(dpi);
        }
    }

    /// Places what is being rasterized, if anything, and restores the
    /// figure's resolution (matplotlib's `stop_rasterizing`, also its
    /// `_finalize_rasterization` once the figure is drawn).
    pub(crate) fn finish_rasterizing(&mut self) {
        if !self.rasterization.active {
            return;
        }
        self.renderer.stop_rasterizing();
        self.rasterization.active = false;
        if let Some(dpi) = self.rasterization.figure_dpi.take() {
            self.layout.figure.set_dpi(dpi);
        }
    }

    /// Sends the following drawing to the renderer's raster canvas, if it
    /// has one, because the program asked (matplotlib's
    /// `renderer.start_rasterizing()` called by an artist's `draw`): the
    /// figure is drawn at the canvas's resolution meanwhile, but whether
    /// artists count as rasterizing stays as it is.
    pub(crate) fn start_program_rasterizing(&mut self) {
        self.renderer.start_rasterizing();
        if let Some(dpi) = self.renderer.raster_dpi() {
            let figure = self.layout.figure;
            self.rasterization
                .program_figure_dpi
                .get_or_insert(figure.dpi());
            figure.set_dpi(dpi);
        }
    }

    /// Ends the rasterizing the program asked for (matplotlib's
    /// `renderer.stop_rasterizing()` called by an artist's `draw`).
    pub(crate) fn stop_program_rasterizing(&mut self) {
        self.renderer.stop_rasterizing();
        if let Some(dpi) = self.rasterization.program_figure_dpi.take() {
            self.layout.figure.set_dpi(dpi);
        }
    }

    /// Draws a child artist if it exists: through its painter if it has
    /// one (see [`ArtistPainter`](super::ArtistPainter)), else as
    /// [`draw_unpainted`](Self::draw_unpainted) draws it.
    pub fn draw_child(&mut self, id: ArtistId) {
        let Some(artist) = self.layout.store().get(id) else {
            return;
        };
        match &artist.base().painter {
            Some(painter) => self.draw_painted(id, artist, painter),
            None => self.draw_own(artist, Some(id)),
        }
    }

    /// Draws artist `id`, in the figure or removed from it, as pltrs
    /// draws it, its painter left out (what a painter draws its own
    /// artist with, matplotlib's `super().draw(renderer)`): through its
    /// image filter if it has one. An invisible artist draws nothing but
    /// still ends rasterizing like any artist that is not rasterized.
    pub fn draw_unpainted(&mut self, id: ArtistId) {
        let figure = self.layout.figure;
        if let Some(artist) = figure.artist(id) {
            self.draw_own(artist, Some(id));
        }
    }

    /// Draws `artist` itself (see [`DrawContext::draw_unpainted`]), which
    /// then counts as drawn when it is artist `id` of the figure drawn.
    pub(super) fn draw_own(&mut self, artist: &dyn Artist, id: Option<ArtistId>) {
        let base = artist.base();
        self.draw_artist(base.rasterized, |ctx| {
            if !base.visible {
                return;
            }
            match &base.agg_filter {
                Some(filter) => {
                    ctx.renderer().start_filter();
                    artist.draw(ctx);
                    ctx.renderer().stop_shared_filter(filter.shared());
                }
                None => artist.draw(ctx),
            }
            if let Some(id) = id {
                ctx.layout.figure.mark_drawn(id);
            }
        });
    }

    /// `children` in z-order (stable for equal z-orders), removed ones
    /// left out.
    pub fn sorted_children(&self, children: &[ArtistId]) -> Vec<ArtistId> {
        let store = self.layout.store();
        let mut ordered: Vec<(f64, ArtistId)> = children
            .iter()
            .filter_map(|&id| Some((store.get(id)?.base().zorder, id)))
            .collect();
        ordered.sort_by(|a, b| a.0.total_cmp(&b.0));
        ordered.into_iter().map(|(_, id)| id).collect()
    }

    /// Draws children in z-order (stable for equal z-orders).
    pub fn draw_children_sorted(&mut self, children: &[ArtistId]) {
        for id in self.sorted_children(children) {
            self.draw_child(id);
        }
    }

    /// A graphics context initialised from an artist's common properties:
    /// clipping, snapping (never when `path.snap` is off, as matplotlib's
    /// `get_snap`), sketch, alpha override, url and gid. The rest keeps
    /// [`GraphicsContext`]'s defaults (black, 1 point wide, solid), for
    /// the artist to set from properties of its own: its public fields
    /// `foreground`, `linewidth`, `dashes`, ...
    pub fn base_gc(&self, base: &ArtistBase) -> GraphicsContext {
        let snap = if crate::rc::params().path_snap {
            base.snap
        } else {
            Some(false)
        };
        let mut gc = GraphicsContext {
            alpha: base.alpha,
            snap: match snap {
                None => SnapMode::Auto,
                Some(true) => SnapMode::Always,
                Some(false) => SnapMode::Never,
            },
            sketch: base.sketch,
            url: base.url.clone(),
            gid: base.gid.clone(),
            ..GraphicsContext::default()
        };
        if base.clip_on {
            gc.clip_rect = self.layout.clip_rect(base);
            gc.clip_path = match &base.clip_path {
                Some(ClipPathSpec::BoundingBox { .. }) => None,
                Some(spec) => self.clip_path(spec),
                None => base
                    .clip_box
                    .and_then(|owner| self.layout.clip_outline(owner)),
            };
        }
        gc
    }

    fn clip_path(&self, spec: &ClipPathSpec) -> Option<ClipPath> {
        match spec {
            ClipPathSpec::Path { path, transform } => {
                let resolved = self.resolve(transform)?;
                let (path, transform) = resolved.split_path(path);
                Some(ClipPath {
                    path: path.into_owned(),
                    transform,
                })
            }
            ClipPathSpec::Artist(id) => self.layout.store().get(*id)?.outline(&self.layout),
            ClipPathSpec::BoundingBox { .. } => None,
        }
    }
}

/// The pixels per point an arrow was last drawn at (matplotlib's
/// `FancyArrowPatch._dpi_cor`, which each draw of the arrow sets): an
/// arrow measured after drawing, even earlier in the same draw, uses it.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DrawnDpiCor(std::cell::Cell<Option<f64>>);

pltrs_core::archive_struct!(DrawnDpiCor(v0));

impl DrawnDpiCor {
    /// Records a draw at `dpi_cor` pixels per point.
    pub(crate) fn record(&self, dpi_cor: f64) {
        self.0.set(Some(dpi_cor));
    }

    /// The pixels per point the arrow of artist `owner` is measured at:
    /// the canvas resolution when `layout` counts the artist as drawn
    /// (see [`LayoutContext::with_drawn`]), else that of its last draw, or
    /// until it draws [`LayoutContext::measured_dpi_cor`].
    pub(crate) fn measured(&self, layout: LayoutContext<'_>, owner: ArtistId) -> f64 {
        if layout.drawn.contains(&owner) {
            return layout.points_to_pixels(1.0);
        }
        self.0.get().unwrap_or_else(|| layout.measured_dpi_cor())
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::{Affine2D, Path, Point, Rgba};
    use pltrs_render::{GraphicsContext, Renderer, RgbaImage};

    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    /// A page in points whose rasterized parts go to a canvas at twice
    /// its resolution; records where every path starts, whether it was
    /// rasterized, and how many raster canvases were placed.
    #[derive(Debug, Default)]
    struct Page {
        rasterizing: bool,
        starts: Vec<(bool, Point)>,
        canvases: usize,
    }

    impl Renderer for Page {
        fn canvas_size(&self) -> (f64, f64) {
            if self.rasterizing {
                (288.0, 144.0)
            } else {
                (144.0, 72.0)
            }
        }

        fn dpi(&self) -> f64 {
            144.0
        }

        fn points_to_pixels(&self, points: f64) -> f64 {
            if self.rasterizing {
                points * 2.0
            } else {
                points
            }
        }

        fn draw_path(&mut self, _: &GraphicsContext, path: &Path, t: &Affine2D, _: Option<Rgba>) {
            let start = t.transform_point(path.vertices()[0]);
            self.starts.push((self.rasterizing, start));
        }

        fn draw_gouraud_triangles(
            &mut self,
            _: &GraphicsContext,
            _: &[[Point; 3]],
            _: &[[Rgba; 3]],
            _: &Affine2D,
        ) {
        }

        fn draw_image(
            &mut self,
            _: &GraphicsContext,
            _: Point,
            _: &RgbaImage,
            _: Option<&Affine2D>,
        ) {
        }

        fn start_rasterizing(&mut self) {
            self.rasterizing = true;
        }

        fn stop_rasterizing(&mut self) {
            self.rasterizing = false;
            self.canvases += 1;
        }

        fn raster_dpi(&self) -> Option<f64> {
            self.rasterizing.then_some(144.0)
        }
    }

    #[test]
    fn rasterized_artists_are_drawn_at_the_raster_resolution() {
        let fig = Figure::with_size(2.0, 1.0, 72.0);
        let ax = fig.subplot();
        let line = ax.plot(vec![0.25, 0.75], vec![0.25, 0.75]).unwrap();
        line.set_rasterized(true);
        let mut page = Page::default();
        fig.draw(&mut page).unwrap();
        let at_72_dpi = fig
            .resolve_transform(&ax.trans_data())
            .unwrap()
            .transform_point(Point::new(0.25, 0.25));
        let rasterized: Vec<Point> = page
            .starts
            .iter()
            .filter_map(|&(rasterized, start)| rasterized.then_some(start))
            .collect();
        assert_eq!(
            rasterized,
            [Point::new(at_72_dpi.x * 2.0, at_72_dpi.y * 2.0)]
        );
        assert!(page.starts.iter().any(|&(rasterized, _)| !rasterized));
        assert_eq!(fig.dpi(), 72.0, "the figure's resolution is restored");
    }

    #[test]
    fn suppressing_compositing_gives_each_rasterized_artist_its_canvas() {
        let canvases = |suppress: Option<bool>| {
            let fig = Figure::with_size(2.0, 1.0, 72.0);
            fig.set_suppress_composite(suppress);
            let ax = fig.subplot();
            ax.set_axis_off();
            for y in [0.25, 0.75] {
                let line = ax.plot(vec![0.0, 1.0], vec![y, y]).unwrap();
                line.set_rasterized(true);
            }
            let mut page = Page::default();
            fig.draw(&mut page).unwrap();
            let rasterized = page.starts.iter().filter(|(rasterized, _)| *rasterized);
            (rasterized.count(), page.canvases)
        };
        // Two rasterized paths in one canvas, or in one canvas each (plus
        // the empty one restarted after the last).
        assert_eq!(canvases(None), (2, 1));
        assert_eq!(canvases(Some(true)), (2, 3));
    }
}
