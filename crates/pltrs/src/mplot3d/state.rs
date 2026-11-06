//! The 3-D axes artist: its state, layout pass and drawing (matplotlib's
//! `Axes3D`).

use std::rc::Rc;

use pltrs_core::{Anchor, Bbox, Path, Rgba};
use pltrs_render::JoinStyle;

use super::art3d::{Art3D, Artist3D, SortKind};
use super::aspect::AspectSettings;
use super::axis_draw::{axis_extents, draw_axes};
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
use crate::colors::Color;
use crate::figure::{FigureState, Painting};
use crate::gridspec::{GridSpecError, SubplotParams, SubplotSpec};
use crate::rc::RcParams;
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
    /// Background colour of the axes box.
    pub(crate) facecolor: Color,
    /// The plane's artists with their 3-D geometry, in insertion order.
    pub(crate) artists: Vec<Artist3D>,
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
    facecolor,
    artists,
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
            facecolor: rc.axes_facecolor.clone(),
            artists: Vec::new(),
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

    /// The layout pass for an axes of display extent `extent`: ticks for
    /// the axes width, then every 3-D artist projected onto its 2-D
    /// artist (hiding the points outside the view limits for the artists
    /// clipped to them) and, for a draw with computed z-order,
    /// collections and patches ordered by depth above the axes
    /// (matplotlib's
    /// `Axes3D.draw` before drawing). Lines are projected as matplotlib
    /// projects them when it draws them: animated ones only while the
    /// figure is `saving`, none for a draw stopped before its artists.
    /// Artists the draw fails on (see [`Axes3D::set_draw_failure`]) are
    /// not projected, nor are the lines and texts of a draw that ends
    /// projecting the collections and patches.
    ///
    /// [`Axes3D::set_draw_failure`]: super::Axes3D::set_draw_failure
    pub(crate) fn prepare(
        &mut self,
        store: &mut ArtistStore,
        extent: &Bbox,
        dpi: f64,
        (saving, painting): (bool, Option<Painting>),
    ) {
        self.artists.retain(|artist| store.contains(artist.id));
        self.failures.retain_in(store);
        let width_points = extent.width().abs() / dpi * 72.0;
        // matplotlib reads `axes3d.automargin` when the ticks are drawn.
        let automargin = crate::rc::params().axes3d_automargin;
        for dir in Axis3::ALL {
            let span = self.limits.span(dir);
            self.axes[dir.index()].update_ticks(&span, width_points, automargin);
        }
        let projection = self.projection();
        let hiding = projection
            .clone()
            .hiding_outside(Axis3::ALL.map(|axis| self.limits.bound(axis)));
        let mut depths: Vec<(ArtistId, SortKind, f64)> = Vec::new();
        let stops = painting.is_some() && self.stops_projecting(store);
        for Artist3D {
            id,
            art,
            axlim_clip,
            ..
        } in &self.artists
        {
            let animated = store.get(*id).is_some_and(|a| a.base().animated);
            if art.projected_when_drawn()
                && ((animated && !saving) || painting == Some(Painting::Stopped) || stops)
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
        if !(self.computed_zorder && painting.is_some()) {
            return;
        }
        // Deepest first; ties keep their insertion order.
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
    /// others report their problems as they are drawn.
    fn report_animated_projections(&self, ctx: &mut DrawContext<'_>) {
        let store = ctx.layout().store();
        let failed: Vec<_> = (self.artists.iter())
            // The colours of patches fail as they are drawn.
            .filter(|artist| matches!(&artist.art, Art3D::Points(points) if points.depth_sorted))
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
        if self.axis_on {
            draw_axes(self, ctx, &extent);
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
    /// Draws the background patch over the display box `extent`, in the
    /// SVG group named after the projection plane's patch gid.
    fn draw_background(&self, ctx: &mut DrawContext<'_>, extent: &Bbox) {
        let plane = ctx.layout().store().get_as::<AxesState>(self.plane);
        let gid = plane.and_then(|plane| plane.patch.gid.clone());
        ctx.renderer().open_part_group("patch", gid.as_deref());
        match self.facecolor.resolve() {
            Ok(face) => {
                // The background patch has edge colour 'none' and no width,
                // and is left unfilled when transparent.
                let mut gc = ctx.base_gc(&self.base);
                gc.linewidth = 0.0;
                gc.foreground = Rgba::TRANSPARENT;
                gc.join = JoinStyle::Miter;
                ctx.renderer().draw_path(
                    &gc,
                    &Path::unit_rectangle(),
                    &unit_to_bbox(extent),
                    (!face.is_transparent()).then_some(face),
                );
            }
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
pub(crate) fn axes3d_tight_bbox(
    figure: &FigureState,
    id: ArtistId,
    for_layout_only: bool,
) -> Option<Bbox> {
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
/// as matplotlib computes the ticks afresh when it measures an axes.
pub(crate) fn axes3d_fresh_tight_bbox(
    figure: &mut FigureState,
    id: ArtistId,
    for_layout_only: bool,
) -> Option<Bbox> {
    // matplotlib's `get_tightbbox` applies the aspect.
    square(figure, id, true);
    prepare_one(figure, id, None);
    super::drawn::note_measure(figure, id);
    if let Some(axes) = figure.store.get_as::<Axes3DState>(id)
        && axes.base.visible
    {
        axes.warn_measured_tick_sites(&figure.store);
    }
    axes3d_tight_bbox(figure, id, for_layout_only)
}

/// The layout pass of every 3-D axes of a figure (called from
/// [`FigureState::prepare`]), for a draw with `painting` (`None` for a
/// measure), the box of each made square for a draw unless it is
/// animated (matplotlib applies the aspect when it draws the axes, which
/// the figure's draws leave out when animated); between draws an axes
/// stays where it stands.
pub(crate) fn prepare_axes3d(figure: &mut FigureState, painting: Option<Painting>) {
    for id in axes3d_ids(&figure.store) {
        if painting.is_some() {
            super::pending::unstale_for_draw(figure, id);
            square(figure, id, false);
        }
        prepare_one(figure, id, painting);
        if let Some(painting) = painting {
            super::drawn::note_draw(figure, id, painting);
        }
    }
}

/// Makes the box of the 3-D axes `id` square in its position (matplotlib's
/// `apply_aspect`), an animated axes only when `animated_too`.
fn square(figure: &mut FigureState, id: ArtistId, animated_too: bool) {
    if let Some(axes) = figure.store.get_as_mut::<Axes3DState>(id)
        && (animated_too || !axes.base().animated)
    {
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
    let (dpi, saving) = (figure.dpi(), figure.saving);
    let plane_minpos = plane_minpos(figure, id);
    let owner_spaces = owner_tick_spaces(figure, id);
    figure.with_others::<Axes3DState, _>(id, |axes, store| {
        axes.limits.plane_minpos = plane_minpos;
        for (axis, space) in axes.axes.iter_mut().zip(owner_spaces) {
            axis.owner_tick_space = space;
        }
        axes.prepare(store, &extent, dpi, (saving, painting));
    });
}

/// For each axis of the 3-D axes `id`, how many ticks fit along the axis
/// of the other axes its major locator was set on, if it was (matplotlib
/// computes a locator's ticks for its `axis`: shared locators place the
/// ticks of every axes sharing them as they fit along that one).
fn owner_tick_spaces(figure: &FigureState, id: ArtistId) -> [Option<usize>; 3] {
    let Some(axes) = figure.artist_as::<Axes3DState>(id) else {
        return [None; 3];
    };
    Axis3::ALL.map(|dir| {
        let owner = axes.axis(dir).locator_owner.filter(|&owner| owner != id)?;
        let width = figure.axes_bbox(owner)?.width().abs() / figure.dpi() * 72.0;
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
