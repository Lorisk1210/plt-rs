//! Contexts handed to artists while measuring and drawing.

use std::cell::{OnceCell, Ref};

use pltrs_core::{Bbox, Point};
use pltrs_render::{ClipPath, GraphicsContext, Renderer, SnapMode};

use super::{Artist, ArtistBase, ArtistId, ArtistStore, ClipPathSpec, GraphicsState};
use crate::figure::{FigureCell, FigureState};
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

    /// Whether artist `id` is measured as drawn (see
    /// [`with_drawn`](Self::with_drawn)).
    pub(crate) fn counts_as_drawn(&self, id: ArtistId) -> bool {
        self.drawn.contains(&id)
    }

    /// Every artist of the figure.
    pub fn store(&self) -> &'a ArtistStore {
        &self.figure.store
    }

    /// The figure's state.
    pub(crate) fn figure_state(&self) -> &'a FigureState {
        self.figure
    }

    /// Whether artist `id` is in no figure: removed from this one, or made
    /// outside any (matplotlib's `get_figure()` is then `None`).
    pub(crate) fn in_no_figure(&self, id: ArtistId) -> bool {
        self.figure.is_removed(id)
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

    /// Whether axis `name` of axes `axes` is left out of a draw as
    /// animated, as an animated child of the axes is (see
    /// [`Axis::set_animated`](crate::axes::Axis::set_animated)).
    pub(crate) fn skips_animated_axis(self, axes: ArtistId, name: AxisName) -> bool {
        let animated = (self.store().get_as::<crate::axes::AxesState>(axes))
            .is_some_and(|state| state.axis(name).animated);
        animated && !self.figure.saving
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

    /// The frame's `bbox` as matplotlib's figures have it: where its
    /// background is drawn, the whole canvas for the root figure while
    /// part of it is saved.
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
        match self.figure.background_outline(axes) {
            Some(outline) => outline
                .path
                .contains_point(at, Some(&outline.transform), 0.0),
            None => self.axes_extent(axes).is_some_and(|bbox| bbox.contains(at)),
        }
    }

    /// Display-space extent of an axes or figure frame used as a clip box
    /// (`None` for polar and geographic axes, which clip to their outline
    /// instead; the box of the rectangle an axes' patch is when it is one
    /// of its own).
    pub fn clip_extent(&self, owner: ArtistId) -> Option<Bbox> {
        if let Some(rect) = self.figure.outline_clip_box(owner) {
            return Some(rect);
        }
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
            Some(spec @ ClipPathSpec::BoundingBox { .. }) => self.bounding_rect(spec),
            _ => base.clip_box.and_then(|owner| self.clip_extent(owner)),
        }
    }

    /// The display rectangle a [`ClipPathSpec::BoundingBox`] clips to;
    /// `None` for the other clip paths.
    pub(crate) fn bounding_rect(&self, spec: &ClipPathSpec) -> Option<Bbox> {
        match spec {
            ClipPathSpec::BoundingBox { path, transform } => {
                Some(self.resolve(transform)?.transform_path(path).extents())
            }
            ClipPathSpec::Path { .. } | ClipPathSpec::Artist(_) => None,
        }
    }

    /// The outline artists clipped to `owner` are clipped to when it is
    /// not a rectangle (the wedge of polar axes, the ellipse of
    /// geographic axes).
    pub fn clip_outline(&self, owner: ArtistId) -> Option<ClipPath> {
        self.figure.axes_outline(owner)
    }

    /// The outline the background of `owner` is drawn along when it is no
    /// rectangle of the axes box (its own outline, the wedge of polar
    /// axes, the ellipse of geographic axes).
    pub(crate) fn background_outline(&self, owner: ArtistId) -> Option<ClipPath> {
        self.figure.background_outline(owner)
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

/// What code laying out part of a figure reads the figure through: a
/// layout's context, which holds it throughout, or a draw's, which lets it
/// go around the program's code that layout runs (a transform of the
/// program's, the text measurer), as matplotlib lets that code change the
/// figure while it is drawn.
pub(crate) trait FigureReader {
    /// The figure, read until [`release_if`](Self::release_if).
    fn read(&self) -> LayoutContext<'_>;

    /// Lets the figure go before code that `runs_program` says runs the
    /// program's code (asked only by a reader that lets it go).
    fn release_if(&mut self, runs_program: impl FnOnce() -> bool);

    /// Lets the figure go before `transform` is applied, when that runs
    /// the program's code.
    fn release_for(&mut self, transform: &ResolvedTransform) {
        self.release_if(|| transform.calls_program());
    }
}

impl FigureReader for LayoutContext<'_> {
    fn read(&self) -> LayoutContext<'_> {
        *self
    }

    /// A layout holds the figure throughout.
    fn release_if(&mut self, _runs_program: impl FnOnce() -> bool) {}
}

impl FigureReader for DrawContext<'_> {
    fn read(&self) -> LayoutContext<'_> {
        self.layout()
    }

    fn release_if(&mut self, runs_program: impl FnOnce() -> bool) {
        if runs_program() {
            self.free();
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
    /// An infinite value where a whole number was needed, such as an
    /// infinite log limit a locator rounds (Python's `OverflowError`).
    Overflow,
    /// What matplotlib does not support yet, such as the logit scale on
    /// the radial axis of polar axes (matplotlib's `NotImplementedError`).
    Unsupported,
    /// An index beyond the values it picks from, such as the depth order
    /// of 3-D markers indexing fewer colours than markers (numpy's
    /// `IndexError`).
    OutOfRange,
    /// Memory the system refused, such as for the canvas filtered artists
    /// are drawn on (matplotlib's `MemoryError`).
    OutOfMemory,
    /// An attribute the artist's class lacks, such as the 3-D projection
    /// of a 2-D collection a 3-D axes draws (Python's `AttributeError`).
    MissingAttribute,
    /// A file the program named that does not exist, such as the font
    /// file of a text (Python's `FileNotFoundError`).
    FileNotFound,
    /// A matrix without inverse, such as the projection of a 3-D axes
    /// through an infinite limit (numpy's `LinAlgError`).
    Singular,
    /// A library matplotlib draws with refusing what it is asked, such as
    /// `FreeType` refusing a font size, which matplotlib raises as a
    /// `RuntimeError` with the library's message.
    Library,
    /// Any other problem (matplotlib's `RuntimeError`).
    #[default]
    Other,
}

pltrs_core::archive_enum!(DrawErrorKind {
    InvalidValue,
    InvalidType,
    Overflow,
    Unsupported,
    OutOfRange,
    OutOfMemory,
    MissingAttribute,
    FileNotFound,
    Singular,
    Other,
    Library
});

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
///
/// A draw runs the program's code (a path effect, a norm, a transform),
/// which matplotlib lets read and change the figure while it is drawn,
/// and the rest of the draw sees the change. So the context does not keep
/// the figure borrowed throughout: it borrows it for reading when first
/// asked about it ([`layout`](Self::layout)), and lets it go whenever it
/// hands its renderer out ([`renderer`](Self::renderer)) or is told to
/// ([`free`](Self::free)), which borrowing it mutably ensures nothing
/// read from the figure is held meanwhile. The artist drawn is not
/// borrowed from the figure either (see
/// [`ArtistStore::shared`]): code the renderer runs, or code run after
/// [`free`](Self::free), finds the figure free and may change it at once.
pub struct DrawContext<'a> {
    /// The cell of the figure drawn.
    figure: &'a FigureCell,
    /// The figure's state, while the context holds it borrowed.
    held: OnceCell<Ref<'a, FigureState>>,
    renderer: &'a mut dyn Renderer,
    errors: Vec<DrawError>,
    rasterization: Rasterization,
    /// Whether the draw ends at its first problem (see
    /// [`set_draws_stop_at_first_problem`](super::set_draws_stop_at_first_problem)).
    stops_at_problem: bool,
    /// The state the artists' graphics contexts start from (see
    /// [`drawing_with_state`](super::drawing_with_state)).
    graphics: GraphicsState,
    /// The artists pltrs is drawing as parts of others, outermost first
    /// (see [`DrawContext::draw_child`]).
    drawing: Vec<ArtistId>,
}

impl std::fmt::Debug for DrawContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DrawContext")
            .field("rasterization", &self.rasterization)
            .finish_non_exhaustive()
    }
}

impl<'a> DrawContext<'a> {
    pub(crate) fn new(figure: &'a FigureCell, renderer: &'a mut dyn Renderer) -> Self {
        Self {
            figure,
            held: OnceCell::new(),
            renderer,
            errors: Vec::new(),
            rasterization: Rasterization::default(),
            stops_at_problem: super::stopping::draws_stop_at_first_problem(),
            graphics: super::graphics_state::current(),
            drawing: Vec::new(),
        }
    }

    /// Whether the draw has ended: it ends at its first problem, met
    /// drawing or a failure of the program's code, when draws do (see
    /// [`set_draws_stop_at_first_problem`](super::set_draws_stop_at_first_problem)),
    /// as matplotlib's ends at its first exception. The artists still to
    /// draw are then left undrawn.
    pub fn ended(&self) -> bool {
        self.stops_at_problem && (!self.errors.is_empty() || crate::layout::failures::failed())
    }

    /// Whether this context draws the figure of `cell`.
    pub(crate) fn draws(&self, cell: &FigureCell) -> bool {
        std::ptr::eq(self.figure, cell)
    }

    /// Lets the figure go: drops the borrow of its state the context
    /// holds, so that the code run next (the program's) finds the figure
    /// free; the context borrows it again when next asked about it.
    pub fn free(&mut self) {
        self.held.take();
    }

    /// Lets the figure go (see [`free`](Self::free)) before `transform`
    /// is applied, when that runs code of the program's (see
    /// [`ResolvedTransform::calls_program`]): the program's transform then
    /// finds the figure free, as matplotlib's does.
    pub fn free_for(&mut self, transform: &ResolvedTransform) {
        if transform.calls_program() {
            self.free();
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

    /// Read-only geometry access, borrowing the figure until the context
    /// next lets it go.
    pub fn layout(&self) -> LayoutContext<'_> {
        LayoutContext::new(self.held.get_or_init(|| self.figure.borrow()))
    }

    /// How many rasterized artists are being drawn, one inside the other
    /// (matplotlib's `renderer._raster_depth`).
    pub fn raster_depth(&self) -> usize {
        self.rasterization.depth
    }

    /// The output backend; the figure is free while it is used (see
    /// [`free`](Self::free)).
    pub fn renderer(&mut self) -> &mut dyn Renderer {
        self.free();
        self.renderer
    }

    /// Resolves a transform; `None` if it refers to removed artists.
    pub fn resolve(&self, transform: &Transform) -> Option<ResolvedTransform> {
        self.layout().resolve(transform)
    }

    /// Converts points to pixels at the canvas resolution.
    pub fn points_to_pixels(&self, points: f64) -> f64 {
        self.layout().points_to_pixels(points)
    }

    /// Runs `f` with every drawing call going through the renderer that
    /// `wrap` builds around this context's renderer (matplotlib swaps in a
    /// `PathEffectRenderer` this way).
    pub fn with_wrapped_renderer<R>(
        &mut self,
        wrap: impl for<'r> FnOnce(&'r mut dyn Renderer) -> Box<dyn Renderer + 'r>,
        f: impl FnOnce(&mut DrawContext<'_>) -> R,
    ) -> R {
        self.free();
        let mut wrapped = wrap(&mut *self.renderer);
        let mut inner = DrawContext {
            figure: self.figure,
            held: OnceCell::new(),
            renderer: &mut *wrapped,
            errors: Vec::new(),
            rasterization: self.rasterization,
            stops_at_problem: self.stops_at_problem,
            graphics: self.graphics,
            drawing: std::mem::take(&mut self.drawing),
        };
        let result = f(&mut inner);
        let DrawContext {
            errors,
            rasterization,
            drawing,
            ..
        } = inner;
        self.errors.extend(errors);
        self.rasterization = rasterization;
        self.drawing = drawing;
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
        if self.rasterization.active && self.layout().suppress_composite() == Some(true) {
            // The figure's `suppressComposite` keeps what each artist
            // rasterized in an image of its own.
            self.renderer().stop_rasterizing();
            self.renderer().start_rasterizing();
        }
        result
    }

    /// Sends the drawing to the renderer's raster canvas, if it has one,
    /// and draws the figure at the canvas's resolution meanwhile
    /// (matplotlib's `MixedModeRenderer.start_rasterizing`).
    fn start_rasterizing(&mut self) {
        self.renderer().start_rasterizing();
        self.rasterization.active = true;
        if let Some(dpi) = self.renderer.raster_dpi() {
            self.rasterization.figure_dpi = Some(self.layout().dpi());
            self.layout().figure_state().set_dpi(dpi);
        }
    }

    /// Places what is being rasterized, if anything, and restores the
    /// figure's resolution (matplotlib's `stop_rasterizing`, also its
    /// `_finalize_rasterization` once the figure is drawn).
    pub(crate) fn finish_rasterizing(&mut self) {
        if !self.rasterization.active {
            return;
        }
        self.renderer().stop_rasterizing();
        self.rasterization.active = false;
        if let Some(dpi) = self.rasterization.figure_dpi.take() {
            self.layout().figure_state().set_dpi(dpi);
        }
    }

    /// Sends the following drawing to the renderer's raster canvas, if it
    /// has one, because the program asked (matplotlib's
    /// `renderer.start_rasterizing()` called by an artist's `draw`): the
    /// figure is drawn at the canvas's resolution meanwhile, but whether
    /// artists count as rasterizing stays as it is.
    pub(crate) fn start_program_rasterizing(&mut self) {
        self.renderer().start_rasterizing();
        if let Some(dpi) = self.renderer.raster_dpi() {
            let before = self.layout().dpi();
            self.rasterization.program_figure_dpi.get_or_insert(before);
            self.layout().figure_state().set_dpi(dpi);
        }
    }

    /// Ends the rasterizing the program asked for (matplotlib's
    /// `renderer.stop_rasterizing()` called by an artist's `draw`).
    pub(crate) fn stop_program_rasterizing(&mut self) {
        self.renderer().stop_rasterizing();
        if let Some(dpi) = self.rasterization.program_figure_dpi.take() {
            self.layout().figure_state().set_dpi(dpi);
        }
    }

    /// Draws a child artist if it exists: through its painter if it has
    /// one (see [`ArtistPainter`](super::ArtistPainter)), else as
    /// [`draw_unpainted`](Self::draw_unpainted) draws it.
    /// Nothing is drawn once the draw has [`ended`](Self::ended).
    ///
    /// An artist pltrs draws is never one of its own parts in a figure a
    /// program makes; one read back from a corrupted archive can be (its
    /// frame among the titles of its axes, say). Drawing it again inside
    /// its own draw is then a problem, as matplotlib's `RecursionError`,
    /// instead of a recursion without end. What the program draws (a
    /// painter) starts afresh, and recurses as the program makes it.
    pub fn draw_child(&mut self, id: ArtistId) {
        if self.ended() {
            return;
        }
        let Some(artist) = self.layout().store().shared(id) else {
            return;
        };
        match &artist.base().painter {
            Some(painter) => {
                let outer = std::mem::take(&mut self.drawing);
                self.draw_painted(id, &*artist, painter);
                self.drawing = outer;
            }
            None if self.drawing.contains(&id) => {
                self.report(
                    id,
                    "maximum recursion depth exceeded: the artist is its own part",
                );
            }
            None => {
                self.drawing.push(id);
                self.draw_own(&*artist, Some(id));
                self.drawing.pop();
            }
        }
    }

    /// Draws artist `id`, in the figure or removed from it, as pltrs
    /// draws it, its painter left out (what a painter draws its own
    /// artist with, matplotlib's `super().draw(renderer)`): through its
    /// image filter if it has one. An invisible artist draws nothing but
    /// still ends rasterizing like any artist that is not rasterized;
    /// nothing is drawn once the draw has [`ended`](Self::ended).
    pub fn draw_unpainted(&mut self, id: ArtistId) {
        let artist = self.layout().figure_state().shared_artist(id);
        if let Some(artist) = artist {
            self.draw_own(&*artist, Some(id));
        }
    }

    /// Draws `artist` itself (see [`DrawContext::draw_unpainted`]), which
    /// then counts as drawn when it is artist `id` of the figure drawn.
    pub(super) fn draw_own(&mut self, artist: &dyn Artist, id: Option<ArtistId>) {
        self.draw_own_filtered(artist, id, true);
    }

    /// Draws `artist` itself as [`DrawContext::draw_own`] does, through
    /// its image filter only with `own_filter`.
    pub(super) fn draw_own_filtered(
        &mut self,
        artist: &dyn Artist,
        id: Option<ArtistId>,
        own_filter: bool,
    ) {
        if self.ended() {
            return;
        }
        let base = artist.base();
        self.draw_artist(base.rasterized, |ctx| {
            if !base.visible && !artist.draws_hidden() {
                return;
            }
            match base.agg_filter.as_ref().filter(|_| own_filter) {
                Some(filter) => {
                    ctx.renderer().start_filter();
                    artist.draw(ctx);
                    ctx.renderer().stop_shared_filter(filter.shared());
                }
                None => artist.draw(ctx),
            }
            if let Some(id) = id {
                let layout = ctx.layout();
                let fresh = artist.freshened_by_draw(&layout);
                layout.figure_state().mark_drawn(id, fresh);
            }
        });
    }

    /// `children` in z-order (stable for equal z-orders), removed ones
    /// left out.
    pub fn sorted_children(&self, children: &[ArtistId]) -> Vec<ArtistId> {
        let layout = self.layout();
        let store = layout.store();
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
    /// [`GraphicsContext`]'s defaults (black, 1 point wide, solid, round
    /// joins; mitred ones in [`GraphicsState::Cairo`]; the program's when
    /// it makes them, see [`GcSource`](super::GcSource)), for the artist
    /// to set from properties of its own: its public fields `foreground`,
    /// `linewidth`, `dashes`, ...
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
            ..super::graphics_state::new_gc(self.graphics)
        };
        if base.clip_on {
            gc.clip_rect = self.layout().clip_rect(base);
            gc.clip_path = match &base.clip_path {
                Some(ClipPathSpec::BoundingBox { .. }) => None,
                Some(spec) => self.clip_path(spec),
                None => base
                    .clip_box
                    .and_then(|owner| self.layout().clip_outline(owner)),
            };
        }
        gc
    }

    /// The outline the clip path `spec` clips to; `None` for a bounding
    /// box, which clips as a rectangle.
    pub(crate) fn clip_path(&self, spec: &ClipPathSpec) -> Option<ClipPath> {
        match spec {
            ClipPathSpec::Path { path, transform } => {
                let resolved = self.resolve(transform)?;
                let (path, transform) = resolved.split_path(path);
                Some(ClipPath {
                    path: path.into_owned(),
                    transform,
                })
            }
            ClipPathSpec::Artist(id) => {
                let layout = self.layout();
                layout.store().get(*id)?.outline(&layout)
            }
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

    /// The pixels per point of the last draw, `None` before the first.
    pub(crate) fn last(&self) -> Option<f64> {
        self.0.get()
    }

    /// The pixels per point the arrow of artist `owner` is measured at:
    /// the canvas resolution when `layout` counts the artist as drawn
    /// (see [`LayoutContext::with_drawn`]), else that of its last draw;
    /// until it draws, [`LayoutContext::measured_dpi_cor`], or one for an
    /// animated arrow, which the figure's draws leave out (matplotlib's
    /// initial `_dpi_cor`, which an arrow never drawn keeps).
    pub(crate) fn measured(&self, layout: LayoutContext<'_>, owner: ArtistId) -> f64 {
        if layout.drawn.contains(&owner) {
            return layout.points_to_pixels(1.0);
        }
        let animated = layout
            .store()
            .get(owner)
            .is_some_and(|artist| artist.base().animated);
        self.0.get().unwrap_or_else(|| {
            if animated {
                1.0
            } else {
                layout.measured_dpi_cor()
            }
        })
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
