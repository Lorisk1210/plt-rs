//! The 3-D axes artist: its state, layout pass and drawing (matplotlib's
//! `Axes3D`).

use std::rc::Rc;

use pltrs_core::{Anchor, Bbox, Path};

use super::art3d::{Art3D, Artist3D, SortKind};
use super::aspect::AspectSettings;
use super::axis_draw::{axis_extents, draw_axes, draw_panes};
use super::axis3d::Axis3DState;
use super::camera::{Camera, Projection};
use super::coords::Axis3;
use super::drawn::DrawnMatrices;
use super::failures::DrawFailures;
use super::limits::Limits3D;
use super::mouse::MouseButtons;
use super::share::{LimitChange, Shares};
use crate::artist::{
    Artist, ArtistBase, ArtistId, ArtistLookup, ArtistStore, DrawContext, LayoutContext,
};
use crate::axes::{AxesState, collapse_x};
use crate::collections::CollectionArtist;
use crate::figure::{FigureState, Painting};
use crate::gridspec::{GridSpecError, SubplotParams, SubplotSpec};
use crate::patches::PatchPaint;
use crate::rc::RcParams;
use crate::ticker::{AxisName, TickSpace};
use crate::transforms::{TransformContext, unit_to_bbox};

/// State of one 3-D axes.
///
/// The axes owns a 2-D *projection plane*: a hidden [`AxesState`] filling
/// the same box whose data coordinates are matplotlib's projected
/// coordinates (the 2-D `transData` of `Axes3D`). Every 3-D artist is a
/// 2-D artist of that plane plus a 3-D geometry kept in `artists`; the
/// layout pass projects each geometry onto its 2-D artist, which then
/// draws like any other artist.
/// How the box of a 3-D axes fits its position: matplotlib makes it square
/// there in `apply_aspect`, which drawing, laying out and `get_position`
/// run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoxFit {
    /// At its position, as placed there.
    Placed,
    /// The largest square box in its position.
    Squared,
}

pltrs_core::archive_enum!(BoxFit { Placed, Squared });

/// State of one 3-D axes (matplotlib's `Axes3D`).
#[derive(Clone, Debug)]
pub(crate) struct Axes3DState {
    base: ArtistBase,
    /// Placement as a fraction of the owning frame, before the box is
    /// made square (matplotlib's original position).
    pub(crate) position: Bbox,
    /// Where the square box sits inside `position`.
    pub(crate) anchor: Anchor,
    /// Whether the box was made square in `position` since the axes was
    /// placed there: until then the axes is at `position`.
    pub(crate) fit: BoxFit,
    /// The grid cell the axes occupies, if placed on a grid.
    pub(crate) subplot_spec: Option<SubplotSpec>,
    /// The 2-D axes the projected artists live in.
    pub(crate) plane: ArtistId,
    pub(crate) camera: Camera,
    pub(crate) limits: Limits3D,
    /// Which axes have equal data scales and how.
    pub(crate) aspect: AspectSettings,
    /// The axes sharing an axis or the view with this one.
    pub(crate) shares: Shares,
    /// Limit changes the axes sharing them have yet to follow.
    pub(crate) emitted: Vec<LimitChange>,
    /// The x, y and z axes.
    pub(crate) axes: [Axis3DState; 3],
    /// Whether gridlines are drawn on the panes.
    pub(crate) grid_on: bool,
    /// Whether panes, gridlines and axes are drawn.
    pub(crate) axis_on: bool,
    /// Whether collections and patches are ordered by depth.
    pub(crate) computed_zorder: bool,
    /// The plane's artists with their 3-D geometry, in insertion order.
    pub(crate) artists: Vec<Artist3D>,
    /// The 3-D geometry of the artists removed from the plane, which they
    /// take back when they are added to it again (see `readd`).
    pub(crate) removed: Vec<Artist3D>,
    /// The visible collections and patches the last draw ordered by
    /// depth, with their depths, in insertion order (empty unless it
    /// computed their z-order).
    pub(crate) depth_order: Vec<(ArtistId, SortKind, f64)>,
    /// The problems drawing the plane's artists meets, recorded in
    /// advance.
    pub(crate) failures: DrawFailures,
    /// The mouse buttons that rotate, pan and zoom the axes.
    pub(crate) mouse: MouseButtons,
    /// The projection of the last draw (matplotlib's `M` and `invM`).
    pub(crate) drawn: DrawnMatrices,
    /// z-order of the axes artists (`axes.axisbelow`); computed z-orders
    /// start just above it.
    axis_zorder: f64,
    rc: Rc<RcParams>,
}

pltrs_core::archive_struct!(Axes3DState {
    base,
    position,
    anchor,
    fit,
    subplot_spec,
    plane,
    camera,
    limits,
    aspect,
    shares,
    emitted,
    axes,
    grid_on,
    axis_on,
    computed_zorder,
    artists,
    removed,
    depth_order,
    failures,
    mouse,
    drawn,
    axis_zorder,
    rc
});

impl Axes3DState {
    /// A 3-D axes at `position` (frame fractions) drawing into `plane`.
    pub(crate) fn new(position: Bbox, plane: ArtistId, camera: Camera, rc: Rc<RcParams>) -> Self {
        let zmargin = if camera.is_ortho() {
            rc.axes_zmargin
        } else {
            0.0
        };
        // matplotlib makes each axis cleared (`clear=True`), then
        // `Axes3D.__init__` clears the axes, which clears each axis again:
        // two linear scales, with their default formatters, per axis.
        for _ in 0..2 * Axis3::ALL.len() {
            crate::scale::note_default_formatter(crate::scale::ScaleName::Linear);
        }
        Self {
            base: ArtistBase::default(),
            position,
            anchor: Anchor::C,
            fit: BoxFit::Placed,
            subplot_spec: None,
            plane,
            camera,
            limits: Limits3D::new(zmargin, &rc),
            aspect: AspectSettings::default(),
            shares: Shares::default(),
            emitted: Vec::new(),
            axes: Axis3::ALL.map(|dir| Axis3DState::new(dir, &rc)),
            grid_on: rc.axes3d_grid,
            axis_on: true,
            computed_zorder: true,
            artists: Vec::new(),
            removed: Vec::new(),
            depth_order: Vec::new(),
            failures: DrawFailures::default(),
            mouse: MouseButtons::default(),
            drawn: DrawnMatrices::default(),
            axis_zorder: rc.axes_axisbelow.zorder(),
            rc,
        }
    }

    /// The axes' own id.
    pub(crate) fn base_id(&self) -> ArtistId {
        self.base.id()
    }

    /// The rc parameters the axes was created with.
    pub(crate) fn rc(&self) -> &Rc<RcParams> {
        &self.rc
    }

    /// The axis along `dir`.
    pub(crate) fn axis(&self, dir: Axis3) -> &Axis3DState {
        &self.axes[dir.index()]
    }

    /// Mutable access to the axis along `dir`.
    pub(crate) fn axis_mut(&mut self, dir: Axis3) -> &mut Axis3DState {
        &mut self.axes[dir.index()]
    }

    /// The position the axes is drawn at inside a frame of extent
    /// `frame`: the largest square box in `position`, anchored
    /// (matplotlib's `Axes3D.apply_aspect`), once it was
    /// [`Squared`](BoxFit::Squared), `position` before.
    pub(crate) fn active_position(&self, frame: &Bbox) -> Bbox {
        if self.fit == BoxFit::Placed {
            return self.position;
        }
        let fig_aspect = frame.height() / frame.width();
        self.position
            .shrunk_to_aspect(1.0, Some(&self.position), fig_aspect)
            .map_or(self.position, |square| {
                square.anchored(self.anchor, &self.position)
            })
    }

    /// The projection of data points for the current scales, limits and
    /// camera (matplotlib's `get_proj` after the scale transforms).
    pub(crate) fn projection(&self) -> Projection {
        self.camera
            .projection(self.scaled_limits())
            .with_scales(self.axes.each_ref().map(|axis| axis.scale.clone()))
    }

    /// Whether data artists were added (matplotlib's `has_data`).
    pub(crate) fn has_data(&self) -> bool {
        self.artists.iter().any(|artist| artist.art.is_data())
    }

    /// The layout pass of the axes once its axes are ticked (see
    /// [`prepare_one`]): every 3-D artist projected onto its 2-D artist
    /// (hiding the points outside the view limits for the artists
    /// clipped to them) and, for a draw with computed z-order,
    /// collections and patches ordered by depth above the axes
    /// (matplotlib's
    /// `Axes3D.draw` before drawing). Lines are projected as matplotlib
    /// projects them when it draws them: animated ones only while the
    /// figure is `saving`, none for a draw stopped before its artists.
    /// Artists the draw fails on (see [`Axes3D::set_draw_failure`]) are
    /// not projected, nor are the lines and texts of a draw that ends
    /// projecting the collections and patches, nor anything when the
    /// projection itself fails. A measure projects none: matplotlib
    /// projects an artist only as it draws it, so its 2-D artist keeps the
    /// projection of the last draw, or the 2-D data it was made with, and
    /// what a program set on it since (offsets).
    ///
    /// [`Axes3D::set_draw_failure`]: super::Axes3D::set_draw_failure
    pub(crate) fn prepare(
        &mut self,
        store: &mut ArtistStore,
        (saving, painting): (bool, Option<Painting>),
    ) {
        self.failures.retain_in(store);
        if self.projection_problem().is_some() {
            // matplotlib's draw fails computing the projection, before it
            // updates the ticks or projects an artist.
            return;
        }
        let Some(painting) = painting else {
            self.depth_order.clear();
            return;
        };
        let projection = self.projection();
        let hiding = projection
            .clone()
            .hiding_outside(Axis3::ALL.map(|axis| self.limits.bound(axis)));
        let mut depths: Vec<(ArtistId, SortKind, f64)> = Vec::new();
        let stops = self.stops_projecting(store);
        for Artist3D {
            id,
            art,
            axlim_clip,
            ..
        } in &self.artists
        {
            let animated = store.get(*id).is_some_and(|a| a.base().animated);
            if art.projected_when_drawn()
                && ((animated && !saving) || painting == Painting::Stopped || stops)
            {
                continue;
            }
            if self.failures.of(*id).is_some() {
                // matplotlib's draw fails on it before it projects it.
                continue;
            }
            let projection = if *axlim_clip { &hiding } else { &projection };
            let Some(depth) = art.project(*id, store, projection) else {
                continue;
            };
            let visible = store.get(*id).is_some_and(|a| a.base().visible);
            if visible && let Some(kind) = art.sort_kind() {
                depths.push((*id, kind, depth));
            }
        }
        if self.computed_zorder {
            self.depth_order = depths;
            self.order_by_depth(store);
        } else {
            self.depth_order.clear();
        }
    }

    /// Gives the collections and patches of `depth_order` their computed
    /// z-order: the deepest first (ties keep their insertion order), above
    /// the axes, collections and patches numbered apart (matplotlib's
    /// `Axes3D.draw`).
    pub(super) fn order_by_depth(&self, store: &mut ArtistStore) {
        let mut depths = self.depth_order.clone();
        depths.sort_by(|a, b| b.2.total_cmp(&a.2));
        let start = self.axis_zorder + 1.0;
        let (mut collection_z, mut patch_z) = (start, start);
        for (id, kind, _) in depths {
            let counter = match kind {
                SortKind::Patch => &mut patch_z,
                SortKind::Collection => &mut collection_z,
            };
            if let Some(artist) = store.get_mut(id) {
                artist.base_mut().zorder = *counter;
            }
            *counter += 1.0;
        }
    }

    /// Reports the projections of the visible animated collections that
    /// failed (sorting their sizes or widths, which leaves them without
    /// their drawing order): matplotlib's draw projects the axes' visible
    /// collections, the animated ones too, and fails with them; the
    /// others report their problems as they are drawn. A problem recorded
    /// for a collection's projection comes first, and is reported with the
    /// others recorded.
    fn report_animated_projections(&self, ctx: &mut DrawContext<'_>) {
        let store = ctx.layout().store();
        let failed: Vec<_> = (self.artists.iter())
            // The colours of patches fail as they are drawn.
            .filter(|artist| matches!(&artist.art, Art3D::Points(points) if points.depth_sorted))
            .filter(|artist| self.failure_of(artist.id).is_none())
            .filter_map(|artist| {
                let collection = store.get_as::<CollectionArtist>(artist.id)?;
                let base = collection.base();
                let sorted = collection.depth_sorted.as_ref()?;
                (base.visible && base.animated && sorted.offsets.is_none())
                    .then_some((artist.id, sorted.problem?))
            })
            .collect();
        for (id, problem) in failed {
            ctx.report_kind(id, problem.kind(), problem);
        }
    }

    /// The plane's artists in matplotlib's drawing order: data artists,
    /// then the titles (the spines are hidden).
    fn plane_children(plane: &AxesState) -> Vec<ArtistId> {
        plane
            .children
            .iter()
            .filter(|id| !plane.spines.contains(id) && !plane.titles.contains(id))
            .chain(&plane.titles)
            .copied()
            .collect()
    }
}

impl Artist for Axes3DState {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    /// Draws the background and the axes, then the plane's artists in the
    /// axes' SVG group: matplotlib's `Axes3D.draw` draws the first two
    /// before the base class opens the group.
    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let Some(extent) = ctx.layout().axes_extent(self.base.id()) else {
            return;
        };
        self.draw_background(ctx, &extent);
        if let Some(problem) = self.projection_problem() {
            // matplotlib's draw computes the projection and its inverse
            // once the background is drawn, and fails there.
            ctx.report_kind(self.base.id(), problem.kind, problem);
            return;
        }
        self.warn_drawn_tick_sites(ctx.layout().store());
        self.report_animated_projections(ctx);
        let children = ctx
            .layout()
            .store()
            .get_as::<AxesState>(self.plane)
            .map(Self::plane_children)
            .unwrap_or_default();
        if self.report_projection_failures(ctx, &children) && ctx.ended() {
            // matplotlib's draw raises projecting the collections and
            // patches, before it draws the panes.
            return;
        }
        // matplotlib's draw fails drawing the axes, before it draws the
        // plane's artists: at the pane of an axis it cannot draw, once
        // those before it are drawn, else once every pane is.
        let failing = match (self.pane_failure(), self.axis_failure()) {
            (Some((axis, problem)), _) => Some((Some(*axis), problem)),
            (None, Some(problem)) => Some((None, problem)),
            (None, None) => None,
        };
        if let Some((failing_pane, problem)) = failing {
            let panes: Vec<Axis3> = (Axis3::ALL.into_iter())
                .take_while(|&dir| Some(dir) != failing_pane)
                .collect();
            draw_panes(self, ctx, &extent, &panes);
            ctx.report_kind(self.base.id(), problem.kind, problem.clone());
            if ctx.ended() {
                return;
            }
        } else if self.axis_on {
            draw_axes(self, ctx, &extent);
        }
        if let Some(problem) = self.failures.of_children() {
            // matplotlib's draw fails ordering the children, once the axes
            // are drawn.
            ctx.report_kind(self.base.id(), problem.kind, problem.clone());
            if ctx.ended() {
                return;
            }
        }
        ctx.renderer().open_group("axes", self.base.gid.as_deref());
        self.draw_plane_children(ctx);
        ctx.renderer().close_group("axes");
    }

    fn children(&self) -> Vec<ArtistId> {
        vec![self.plane]
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        ctx.axes_extent(self.base.id())
    }
}

impl Axes3DState {
    /// Draws the background patch over the display box `extent`, as
    /// matplotlib's `Axes3D.draw` draws `self.patch`: the projection
    /// plane's patch, in its style (a face made transparent, say) and in
    /// the SVG group named after its gid, unless it is hidden.
    fn draw_background(&self, ctx: &mut DrawContext<'_>, extent: &Bbox) {
        let Some((paint, gid)) = (ctx.layout().store().get_as::<AxesState>(self.plane))
            .filter(|plane| plane.patch.visible)
            .map(|plane| {
                let patch = &plane.patch;
                let paint = PatchPaint::new(&patch.style, &plane.rc, patch.alpha, false);
                (paint, patch.gid.clone())
            })
        else {
            return;
        };
        ctx.renderer().open_part_group("patch", gid.as_deref());
        match paint {
            Ok(paint) => paint.paint(ctx.base_gc(&self.base), |gc, face| {
                ctx.renderer()
                    .draw_path(gc, &Path::unit_rectangle(), &unit_to_bbox(extent), face);
            }),
            Err(err) => ctx.report(self.base.id(), err),
        }
        ctx.renderer().close_part_group("patch");
    }

    /// Draws the plane's artists sorted by z-order, but those whose
    /// recorded problem it reports instead.
    fn draw_plane_children(&self, ctx: &mut DrawContext<'_>) {
        let Some(plane) = ctx.layout().store().get_as::<AxesState>(self.plane) else {
            return;
        };
        let mut children = Self::plane_children(plane);
        children.retain(|&id| !ctx.layout().skips_animated(id, true));
        for id in ctx.sorted_children(&children) {
            self.draw_plane_child(ctx, id);
        }
    }
}

/// The display extent of a 3-D axes, for [`FigureState`]'s
/// `axes_bbox`.
pub(crate) fn axes3d_bbox(figure: &FigureState, id: ArtistId) -> Option<Bbox> {
    let axes = figure.artist_as::<Axes3DState>(id)?;
    // A removed axes is placed in the figure, as a removed 2-D axes is.
    let frame = figure.frame_bbox(axes.base().parent.unwrap_or(figure.root))?;
    Some(
        axes.active_position(&frame)
            .transformed(&unit_to_bbox(&frame)),
    )
}

/// Whether artist `id` is a 3-D axes.
pub(crate) fn is_axes3d(store: &ArtistStore, id: ArtistId) -> bool {
    store.get_as::<Axes3DState>(id).is_some()
}

/// The grid cell of a 3-D axes, if it was placed on a grid.
pub(crate) fn axes3d_subplot_spec(store: &ArtistStore, id: ArtistId) -> Option<&SubplotSpec> {
    store.get_as::<Axes3DState>(id)?.subplot_spec.as_ref()
}

/// Moves artist `id`, if it is a 3-D axes placed on a grid, to its cell
/// under the subplot parameters `params`.
pub(crate) fn place_axes3d(
    store: &mut ArtistStore,
    id: ArtistId,
    params: &SubplotParams,
) -> Result<(), GridSpecError> {
    if let Some(axes) = store.get_as_mut::<Axes3DState>(id)
        && let Some(spec) = &axes.subplot_spec
    {
        axes.position = spec.position(params)?;
    }
    Ok(())
}

/// The 3-D axes of a figure, in creation order.
pub(crate) fn axes3d_ids(store: &ArtistStore) -> Vec<ArtistId> {
    store.ids().filter(|&id| is_axes3d(store, id)).collect()
}

/// The extent of a visible 3-D axes for tight boxes: its box, the lines
/// and texts of its visible axes when shown (without their labels when
/// `for_layout_only`), and its titles (matplotlib's
/// `Axes3D.get_tightbbox`).
fn axes3d_tight_bbox(figure: &FigureState, id: ArtistId, for_layout_only: bool) -> Option<Bbox> {
    let axes = figure.store.get_as::<Axes3DState>(id)?;
    if !axes.base.visible {
        return None;
    }
    let extent = figure.axes_bbox(id)?;
    let layout = LayoutContext::new(figure);
    let mut boxes = vec![extent];
    if axes.axis_on {
        let shown: Vec<Axis3> = (Axis3::ALL.into_iter())
            .filter(|&dir| axes.axis(dir).visible)
            .collect();
        boxes.extend(axis_extents(axes, layout, &extent, for_layout_only, &shown));
    }
    if let Some(plane) = figure.store.get_as::<AxesState>(axes.plane) {
        let narrowed = |bbox: Bbox| {
            if for_layout_only && bbox.width() > 0.0 {
                collapse_x(&bbox)
            } else {
                bbox
            }
        };
        boxes.extend(
            plane
                .titles
                .iter()
                .filter_map(|&title| figure.store.get(title))
                .filter(|title| title.base().visible)
                .filter_map(|title| title.window_extent(&layout))
                .map(narrowed)
                .filter(|bbox| bbox.width() != 0.0 || bbox.height() != 0.0),
        );
    }
    boxes.into_iter().reduce(|a, b| a.union(&b))
}

/// [`axes3d_tight_bbox`] after a layout pass of the axes where it is now,
/// as matplotlib computes the ticks afresh when it measures an axes, and
/// its titles placed (matplotlib's `get_tightbbox` calls
/// `_update_title_position`: a title matplotlib's `Axes3D.set_title`
/// lowered to 0.92 is measured at the top of the box, where it is drawn).
pub(crate) fn axes3d_fresh_tight_bbox(
    figure: &mut FigureState,
    id: ArtistId,
    for_layout_only: bool,
) -> Option<Bbox> {
    // matplotlib's `get_tightbbox` applies the aspect.
    square(figure, id);
    prepare_one(figure, id, None);
    super::drawn::note_measure(figure, id);
    let axes = figure.store.get_as::<Axes3DState>(id)?;
    let plane = axes.plane;
    if axes.base.visible {
        axes.warn_measured_tick_sites(&figure.store);
    }
    figure.update_title_position(plane);
    axes3d_tight_bbox(figure, id, for_layout_only)
}

/// The layout pass of every 3-D axes of a figure (called from
/// [`FigureState::prepare`]), for a draw with `painting` (`None` for a
/// measure), the box of each made square for a draw (matplotlib's
/// `Figure.draw` applies the aspect of every axes, animated ones too,
/// though it leaves those out of what it draws); between draws an axes
/// stays where it stands.
pub(crate) fn prepare_axes3d(figure: &mut FigureState, painting: Option<Painting>) {
    for id in axes3d_ids(&figure.store) {
        match painting {
            Some(painting) => prepare_drawn(figure, id, painting),
            None => prepare_one(figure, id, None),
        }
    }
}

/// The pass of the 3-D axes `id` before it is drawn with `painting`, in
/// the figure's draw or alone (`draw_artist`): its box made square, its
/// view, ticks and the depth order of its artists computed as
/// matplotlib's `Axes3D.draw` computes them as it draws, in the view of
/// that moment (one `view_init` changed since the figure's last draw).
pub(crate) fn prepare_drawn(figure: &mut FigureState, id: ArtistId, painting: Painting) {
    super::pending::unstale_for_draw(figure, id);
    square(figure, id);
    prepare_one(figure, id, Some(painting));
    super::drawn::note_draw(figure, id, painting);
}

/// Makes the box of the 3-D axes `id` square in its position (matplotlib's
/// `apply_aspect`).
fn square(figure: &mut FigureState, id: ArtistId) {
    if let Some(axes) = figure.store.get_as_mut::<Axes3DState>(id) {
        axes.fit = BoxFit::Squared;
    }
}

/// The layout pass of the 3-D axes `id`, in the figure or removed from
/// it, for a draw with `painting` (matplotlib orders the axes'
/// collections and patches by depth as it draws them), `None` for a
/// measure.
pub(crate) fn prepare_one(figure: &mut FigureState, id: ArtistId, painting: Option<Painting>) {
    let Some(extent) = figure.axes_bbox(id) else {
        return;
    };
    let saving = figure.saving;
    let plane_minpos = plane_minpos(figure, id);
    let owner_spaces = owner_tick_spaces(figure, id);
    let removed = super::readd::removed_artists(figure, id);
    figure.with_others::<Axes3DState, _>(id, |axes, store| {
        axes.set_aside_removed(store, &removed);
        axes.limits.plane_minpos = plane_minpos;
        for (axis, space) in axes.axes.iter_mut().zip(owner_spaces) {
            axis.owner_tick_space = space;
        }
    });
    update_ticks(figure, id, &extent);
    figure.with_others::<Axes3DState, _>(id, |axes, store| {
        axes.prepare(store, (saving, painting));
    });
}

/// Ticks every axis of the 3-D axes `id` for an axes of display extent
/// `extent` (matplotlib's `Axis._update_ticks` as `Axes3D.draw` runs it),
/// unless computing the projection fails first, as matplotlib's draw
/// does before it ticks. An axis whose helpers run code of the program's
/// is ticked with the figure lent, the axes held apart from it meanwhile:
/// matplotlib lets that code read and change the figure (a locator
/// making a tick to measure its label).
fn update_ticks(figure: &mut FigureState, id: ArtistId, extent: &Bbox) {
    let width_points = crate::axes::axis_length_points(extent, figure.dpi(), AxisName::X);
    // matplotlib reads `axes3d.automargin` when the ticks are drawn.
    let automargin = crate::rc::params().axes3d_automargin;
    for dir in Axis3::ALL {
        let compute = |axes: &Axes3DState| {
            let span = axes.limits.span(dir);
            (axes.projection_problem().is_none())
                .then(|| (axes.axis(dir)).computed_ticks(&span, width_points, automargin))
        };
        let held = (figure.store.shared_as::<Axes3DState>(id))
            .filter(|axes| axes.axis(dir).calls_program());
        let computed = held
            .and_then(|held| figure.lend(|_| compute(&held)))
            .unwrap_or_else(|| figure.artist_as::<Axes3DState>(id).and_then(compute));
        // (The program may have removed the axes meanwhile.)
        if let Some(computed) = computed
            && let Some(axes) = figure.artist_as_mut::<Axes3DState>(id)
        {
            axes.axis_mut(dir).install_ticks(computed);
        }
    }
}

/// For each axis of the 3-D axes `id`, how many ticks fit along the axis
/// of the other axes its major locator was set on, if it was (matplotlib
/// computes a locator's ticks for its `axis`: shared locators place the
/// ticks of every axes sharing them as they fit along that one).
fn owner_tick_spaces(figure: &FigureState, id: ArtistId) -> [Option<TickSpace>; 3] {
    let Some(axes) = figure.artist_as::<Axes3DState>(id) else {
        return [None; 3];
    };
    Axis3::ALL.map(|dir| {
        let owner = axes.axis(dir).locator_owner.filter(|&owner| owner != id)?;
        let width = figure.axis_length_points(owner, AxisName::X)?;
        Some(
            figure
                .artist_as::<Axes3DState>(owner)?
                .axis(dir)
                .tick_space(width),
        )
    })
}

/// The smallest positive x of the 2-D data limits of the projection plane
/// of the 3-D axes `id`, in the figure or removed from it (infinite
/// without data; see `Limits3D::plane_minpos`).
pub(crate) fn plane_minpos(figure: &FigureState, id: ArtistId) -> f64 {
    figure
        .artist_as::<Axes3DState>(id)
        .and_then(|axes| figure.artist_as::<AxesState>(axes.plane))
        .map_or(f64::INFINITY, |plane| plane.data_lim.minpos.x)
}

#[cfg(test)]
mod tests {
    use crate::figure::Figure;

    #[test]
    fn tight_boxes_take_in_the_axes_of_3d_axes() {
        // matplotlib's test_tightbbox_box_aspect: the tick labels of the
        // axes reach beyond the box of the 3-D axes, a label further.
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.subplot_3d();
        ax.set_axis_off();
        let bare = fig.tight_bbox();
        ax.set_axis_on();
        let with_axes = fig.tight_bbox();
        assert!(
            with_axes.width() > bare.width() + 0.1,
            "{with_axes:?} vs {bare:?}"
        );
        ax.set_zlabel("z");
        let labelled = fig.tight_bbox();
        assert!(
            labelled.x1() > with_axes.x1(),
            "{labelled:?} vs {with_axes:?}"
        );
    }
}
