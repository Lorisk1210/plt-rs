//! The 3-D axes artist: its state, layout pass and drawing (matplotlib's
//! `Axes3D`).

use std::rc::Rc;

use plt_core::{Anchor, Bbox, Path};

use super::art3d::{Art3D, SortKind};
use super::aspect::AspectSettings;
use super::axis_draw::draw_axes;
use super::axis3d::Axis3DState;
use super::camera::{Camera, Projection};
use super::coords::Axis3;
use super::limits::Limits3D;
use super::mouse::MouseButtons;
use super::share::{LimitChange, Shares};
use crate::artist::{Artist, ArtistBase, ArtistId, ArtistStore, DrawContext, LayoutContext};
use crate::axes::AxesState;
use crate::colors::Color;
use crate::figure::FigureState;
use crate::gridspec::SubplotSpec;
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
#[derive(Debug)]
pub(crate) struct Axes3DState {
    base: ArtistBase,
    /// Placement as a fraction of the owning frame, before the box is
    /// made square (matplotlib's original position).
    pub(crate) position: Bbox,
    /// Where the square box sits inside `position`.
    pub(crate) anchor: Anchor,
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
    /// The 3-D geometry of the plane's artists, in insertion order.
    pub(crate) artists: Vec<(ArtistId, Art3D)>,
    /// The mouse buttons that rotate, pan and zoom the axes.
    pub(crate) mouse: MouseButtons,
    /// z-order of the axes artists (`axes.axisbelow`); computed z-orders
    /// start just above it.
    axis_zorder: f64,
    rc: Rc<RcParams>,
}

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
            mouse: MouseButtons::default(),
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
    /// (matplotlib's `Axes3D.apply_aspect`).
    pub(crate) fn active_position(&self, frame: &Bbox) -> Bbox {
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
        self.artists.iter().any(|(_, art)| art.is_data())
    }

    /// The layout pass for an axes of display extent `extent`: ticks for
    /// the axes width, then every 3-D artist projected onto its 2-D
    /// artist and, with computed z-order, collections and patches ordered
    /// by depth above the axes (matplotlib's `Axes3D.draw` before
    /// drawing).
    fn prepare(&mut self, store: &mut ArtistStore, extent: &Bbox, dpi: f64) {
        self.artists.retain(|(id, _)| store.contains(*id));
        let width_points = extent.width().abs() / dpi * 72.0;
        for dir in Axis3::ALL {
            let span = self.limits.span(dir);
            self.axes[dir.index()].update_ticks(&span, width_points, self.limits.automargin);
        }
        let projection = self.projection();
        let mut depths: Vec<(ArtistId, SortKind, f64)> = Vec::new();
        for (id, art) in &self.artists {
            let Some(depth) = art.project(*id, store, &projection) else {
                continue;
            };
            let visible = store.get(*id).is_some_and(|a| a.base().visible);
            if visible && let Some(kind) = art.sort_kind() {
                depths.push((*id, kind, depth));
            }
        }
        if !self.computed_zorder {
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

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let Some(extent) = ctx.layout().axes_extent(self.base.id()) else {
            return;
        };
        match self.facecolor.resolve() {
            Ok(face) => {
                let mut gc = ctx.base_gc(&self.base);
                gc.linewidth = 0.0;
                ctx.renderer().draw_path(
                    &gc,
                    &Path::unit_rectangle(),
                    &unit_to_bbox(&extent),
                    Some(face),
                );
            }
            Err(err) => ctx.report(self.base.id(), err),
        }
        if self.axis_on {
            draw_axes(self, ctx, &extent);
        }
        let Some(plane) = ctx.layout().store().get_as::<AxesState>(self.plane) else {
            return;
        };
        let children = Self::plane_children(plane);
        ctx.draw_children_sorted(&children);
    }

    fn children(&self) -> Vec<ArtistId> {
        vec![self.plane]
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        ctx.axes_extent(self.base.id())
    }
}

/// The display extent of a 3-D axes, for [`FigureState`]'s
/// `axes_bbox`.
pub(crate) fn axes3d_bbox(figure: &FigureState, id: ArtistId) -> Option<Bbox> {
    let axes = figure.store.get_as::<Axes3DState>(id)?;
    let frame = figure.frame_bbox(axes.base().parent?)?;
    Some(
        axes.active_position(&frame)
            .transformed(&unit_to_bbox(&frame)),
    )
}

/// The layout pass of every 3-D axes of a figure (called from
/// [`FigureState::prepare`]).
pub(crate) fn prepare_axes3d(figure: &mut FigureState) {
    let ids: Vec<ArtistId> = figure
        .store
        .ids()
        .filter(|&id| figure.store.get_as::<Axes3DState>(id).is_some())
        .collect();
    let dpi = figure.dpi;
    for id in ids {
        let Some(extent) = figure.axes_bbox(id) else {
            continue;
        };
        figure
            .store
            .with_detached::<Axes3DState, _>(id, |axes, store| axes.prepare(store, &extent, dpi));
    }
}
