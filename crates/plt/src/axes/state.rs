//! The axes artist: a data coordinate system placed inside a figure frame.

use std::rc::Rc;

use plt_core::{Bbox, Path, Point};

use super::DataLimits;
use super::aspect::AspectState;
use super::axis::{AxisGeometry, AxisState};
use super::current_image::CurrentImage;
use super::limits::{AxisAutoscale, autoscale_axis};
use super::projection::Projection;
use super::secondary::Secondary;
use crate::artist::{Artist, ArtistBase, ArtistId, DrawContext, LayoutContext};
use crate::axes_grid1::Parasite;
use crate::colors::Color;
use crate::geo::{self, GeoState};
use crate::gridspec::SubplotSpec;
use crate::patches::PatchShape;
use crate::polar::{PolarState, PolarView};
use crate::rc::{Cycler, RcParams};
use crate::ticker::{AxisInfo, AxisName, TickContext, TickerError};
use crate::transforms::{ResolvedTransform, Step, bbox_to_unit, unit_to_bbox};

/// State of one axes (matplotlib's `_AxesBase`).
#[derive(Debug)]
pub(crate) struct AxesState {
    base: ArtistBase,
    /// Placement as a fraction of the owning frame, before any aspect
    /// adjustment (matplotlib's original position).
    pub(crate) position: Bbox,
    /// The aspect settings and the active position they produce.
    pub(crate) aspect: AspectState,
    /// The grid cell the axes occupies, if placed on a grid.
    pub(crate) subplot_spec: Option<SubplotSpec>,
    /// The coordinate system.
    pub(crate) projection: Projection,
    /// Visible data range; `x0 > x1` means an inverted axis.
    pub(crate) view_lim: Bbox,
    /// Whether the view limits were ever changed from the initial unit box.
    pub(crate) view_mutated: (bool, bool),
    /// Extent of all data added so far.
    pub(crate) data_lim: DataLimits,
    /// The horizontal axis.
    pub(crate) xaxis: AxisState,
    /// The vertical axis.
    pub(crate) yaxis: AxisState,
    /// Whether sticky edges limit the margins.
    pub(crate) use_sticky_edges: bool,
    /// Skip view-limit rounding when autoscaling.
    pub(crate) tight: bool,
    /// The background rectangle.
    pub(crate) patch: AxesPatch,
    /// Which decorations are drawn.
    pub(crate) decorations: Decorations,
    /// The axes this one shares its x and y axis with, if any
    /// (matplotlib's `_sharex`, `_sharey`).
    pub(crate) shares: (Option<ArtistId>, Option<ArtistId>),
    /// Set when this is a secondary axis of another axes.
    pub(crate) secondary: Option<Secondary>,
    /// Set when this is a parasite axes drawn by its host (`axes_grid1`).
    pub(crate) parasite: Option<Parasite>,
    /// Property cycle for lines and patches.
    pub(crate) prop_cycle: Cycler,
    /// Next property-cycle entry for lines.
    pub(crate) line_cycle: usize,
    /// Next property-cycle entry for patches and collections.
    pub(crate) patch_cycle: usize,
    /// Plotted artists in insertion order.
    pub(crate) children: Vec<ArtistId>,
    /// Containers of the plotting calls that made them (`ax.containers`).
    pub(crate) containers: Vec<crate::container::ContainerIds>,
    /// The spines, in `SpineSide::ALL` order once created.
    pub(crate) spines: Vec<ArtistId>,
    /// The left, centre and right titles once created.
    pub(crate) titles: Vec<ArtistId>,
    /// Whether the navigation toolbar acts on this axes (matplotlib's
    /// `get_navigate`).
    pub(crate) navigate: bool,
    /// How the titles' height is chosen (matplotlib's `_autotitlepos`).
    pub(crate) title_placement: TitlePlacement,
    /// z-order of the axis artists (`axes.axisbelow`).
    pub(crate) axis_zorder: f64,
    /// Children at or below this z-order are rasterized together in
    /// vector output (matplotlib's `rasterization_zorder`).
    pub(crate) rasterization_zorder: Option<f64>,
    /// The colour-mapped artist pyplot's colormap functions act on
    /// (matplotlib's `_current_image`).
    pub(crate) current_image: Option<CurrentImage>,
    /// The rc parameters the axes was created with.
    pub(crate) rc: Rc<RcParams>,
}

/// How the height of an axes' titles is chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TitlePlacement {
    /// Lifted above the decorations on top of the axes.
    Auto,
    /// Where `axes.titley` or `Axes::set_title_y` put them.
    Fixed,
}

/// Which decorations an axes draws (matplotlib's `axison` and
/// `_frameon`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Decorations {
    /// The patch, the spines and both axes.
    pub(crate) axis_on: bool,
    /// The patch and the spines.
    pub(crate) frame_on: bool,
}

impl Decorations {
    /// Whether the patch and spines are drawn.
    pub(crate) fn framed(self) -> bool {
        self.axis_on && self.frame_on
    }
}

/// The background rectangle of an axes (matplotlib's `ax.patch`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AxesPatch {
    /// Fill colour.
    pub(crate) facecolor: Color,
    /// Whether it is drawn.
    pub(crate) visible: bool,
}

/// What the axes sharing one axis contribute to its autoscaling.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SharedData {
    /// The span of their finite data limits, if any is finite.
    pub(crate) interval: Option<(f64, f64)>,
    /// Their smallest positive datum.
    pub(crate) minpos: f64,
    /// Their children's sorted sticky edges.
    pub(crate) stickies: Vec<f64>,
}

impl AxesState {
    /// Axes at `position` (frame fractions) configured from `rc`.
    pub(crate) fn new(position: Bbox, rc: Rc<RcParams>) -> Self {
        Self {
            base: ArtistBase::default(),
            position,
            aspect: AspectState::default(),
            subplot_spec: None,
            projection: Projection::Rectilinear,
            view_lim: Bbox::UNIT,
            view_mutated: (false, false),
            data_lim: DataLimits::default(),
            xaxis: AxisState::new(AxisName::X, &rc),
            yaxis: AxisState::new(AxisName::Y, &rc),
            use_sticky_edges: true,
            tight: false,
            patch: AxesPatch {
                facecolor: rc.axes_facecolor.clone(),
                visible: true,
            },
            decorations: Decorations {
                axis_on: true,
                frame_on: true,
            },
            shares: (None, None),
            secondary: None,
            parasite: None,
            prop_cycle: rc.axes_prop_cycle.clone(),
            line_cycle: 0,
            patch_cycle: 0,
            children: Vec::new(),
            containers: Vec::new(),
            spines: Vec::new(),
            titles: Vec::new(),
            navigate: true,
            title_placement: if rc.axes_titley.is_some() {
                TitlePlacement::Fixed
            } else {
                TitlePlacement::Auto
            },
            axis_zorder: rc.axes_axisbelow.zorder(),
            rasterization_zorder: None,
            current_image: None,
            rc,
        }
    }

    /// The axis called `name`.
    pub(crate) fn axis(&self, name: AxisName) -> &AxisState {
        match name {
            AxisName::X => &self.xaxis,
            AxisName::Y => &self.yaxis,
        }
    }

    /// Mutable access to the axis called `name`.
    pub(crate) fn axis_mut(&mut self, name: AxisName) -> &mut AxisState {
        match name {
            AxisName::X => &mut self.xaxis,
            AxisName::Y => &mut self.yaxis,
        }
    }

    /// The view interval of one axis.
    pub(crate) fn view_interval(&self, name: AxisName) -> (f64, f64) {
        match name {
            AxisName::X => self.view_lim.interval_x(),
            AxisName::Y => self.view_lim.interval_y(),
        }
    }

    /// The polar settings, if this is a polar axes.
    pub(crate) fn polar(&self) -> Option<&PolarState> {
        match &self.projection {
            Projection::Polar(settings) => Some(settings),
            Projection::Rectilinear | Projection::Geo(_) => None,
        }
    }

    /// The geographic settings, if this is a geographic axes.
    pub(crate) fn geo(&self) -> Option<&GeoState> {
        match &self.projection {
            Projection::Geo(settings) => Some(settings),
            Projection::Rectilinear | Projection::Polar(_) => None,
        }
    }

    /// The polar geometry at the current view, if this is a polar axes.
    pub(crate) fn polar_view(&self) -> Option<PolarView<'_>> {
        self.polar()
            .map(|settings| PolarView::new(settings, self.view_lim, self.yaxis.map()))
    }

    /// The full data → display transform for the axes placed at `display`:
    /// scales, then view limits onto the unit square, then onto `display`
    /// (or the polar or a geographic projection).
    pub(crate) fn data_transform(&self, display: &Bbox) -> ResolvedTransform {
        if let Some(polar) = self.polar_view() {
            return polar.data_transform(display);
        }
        if let Some(geo) = self.geo() {
            return geo.data_transform(display);
        }
        let (x0, x1) = self.view_lim.interval_x();
        let (y0, y1) = self.view_lim.interval_y();
        let scaled_limits = Bbox::from_extents(
            self.xaxis.forward(x0),
            self.yaxis.forward(y0),
            self.xaxis.forward(x1),
            self.yaxis.forward(y1),
        );
        let affine = bbox_to_unit(&scaled_limits).then(unit_to_bbox(display));
        let (x_map, y_map) = (self.xaxis.map().cloned(), self.yaxis.map().cloned());
        if x_map.is_none() && y_map.is_none() {
            ResolvedTransform::affine(affine)
        } else {
            ResolvedTransform::with_step(Step::Separable(x_map, y_map), affine)
        }
    }

    /// Whether the view limits of one axis were ever changed from the
    /// initial unit interval.
    pub(crate) fn view_mutated(&self, name: AxisName) -> bool {
        match name {
            AxisName::X => self.view_mutated.0,
            AxisName::Y => self.view_mutated.1,
        }
    }

    /// The limits `(v0, v1)` become once validated for one axis
    /// (matplotlib's `Axis._set_lim`): non-positive limits of a log axis
    /// keep the current value, singular intervals are expanded by the
    /// major locator, and the scale clamps the range; `v0 > v1` keeps the
    /// axis inverted.
    pub(crate) fn validated_limits(&self, name: AxisName, v0: f64, v1: f64) -> (f64, f64) {
        let axis = self.axis(name);
        let (mut v0, mut v1) = (v0, v1);
        if axis.scale().name() == "log" && (v0 <= 0.0 || v1 <= 0.0) {
            let (old0, old1) = self.view_interval(name);
            if v0 <= 0.0 {
                v0 = old0;
            }
            if v1 <= 0.0 {
                v1 = old1;
            }
        }
        let reverse = v0 > v1;
        let (data, minpos) = self.data_interval(name);
        let info = axis.info(&AxisGeometry {
            view: self.view_interval(name),
            data,
            minpos,
            length_points: 0.0,
            tick_space: None,
            tick_floor: None,
        });
        let ctx = TickContext::new(&info, &axis.settings);
        let (v0, v1) = axis.major_locator.nonsingular(v0, v1, &ctx);
        let (v0, v1) = axis.scale().limit_range_for_scale(v0, v1, minpos);
        let (low, high) = if v0 <= v1 { (v0, v1) } else { (v1, v0) };
        if reverse { (high, low) } else { (low, high) }
    }

    /// Widens the view interval of one axis to include `low..=high`,
    /// keeping its direction (matplotlib's `Axis.set_view_interval`).
    pub(crate) fn include_in_view(&mut self, name: AxisName, low: f64, high: f64) {
        let (old_low, old_high) = self.view_interval(name);
        let (start, end) = (low.min(high), low.max(high));
        let widened = if old_low < old_high {
            (start.min(old_low), end.max(old_high))
        } else {
            (end.max(old_low), start.min(old_high))
        };
        self.set_view_interval(name, widened);
    }

    /// Replaces the view interval of one axis.
    pub(crate) fn set_view_interval(&mut self, name: AxisName, (lower, upper): (f64, f64)) {
        let v = self.view_lim;
        match name {
            AxisName::X => {
                self.view_lim = Bbox::from_extents(lower, v.y0(), upper, v.y1());
                self.view_mutated.0 = true;
            }
            AxisName::Y => {
                self.view_lim = Bbox::from_extents(v.x0(), lower, v.x1(), upper);
                self.view_mutated.1 = true;
            }
        }
    }

    /// Marks the view limits of the given axes for re-autoscaling at the
    /// next layout (matplotlib's `_request_autoscale_view`).
    pub(crate) fn request_autoscale(&mut self, x: bool, y: bool) {
        self.xaxis.autoscale.stale |= x;
        self.yaxis.autoscale.stale |= y;
    }

    /// Data interval and minimum positive value of one axis.
    pub(crate) fn data_interval(&self, name: AxisName) -> ((f64, f64), f64) {
        let bbox = self.data_lim.bbox;
        match name {
            AxisName::X => ((bbox.x0(), bbox.x1()), self.data_lim.minpos.x),
            AxisName::Y => ((bbox.y0(), bbox.y1()), self.data_lim.minpos.y),
        }
    }

    /// Widens the data limits of one axis to include `low..=high`
    /// without touching the smallest positive datum (matplotlib's
    /// assignment to `dataLim.intervalx`).
    pub(crate) fn extend_data_interval(&mut self, name: AxisName, low: f64, high: f64) {
        let b = self.data_lim.bbox;
        self.data_lim.bbox = match name {
            AxisName::X => Bbox::from_extents(b.x0().min(low), b.y0(), b.x1().max(high), b.y1()),
            AxisName::Y => Bbox::from_extents(b.x0(), b.y0().min(low), b.x1(), b.y1().max(high)),
        };
    }

    /// Whether the margins of one axis stop at sticky edges.
    pub(crate) fn uses_stickies(&self, name: AxisName) -> bool {
        let autoscale = self.axis(name).autoscale;
        self.use_sticky_edges && autoscale.margin != 0.0 && autoscale.enabled
    }

    /// The autoscaled `(lower, upper)` bound of one axis (matplotlib's
    /// `handle_single_axis` in `autoscale_view`), or `None` when there is
    /// no data and the view was set explicitly (matplotlib keeps it).
    pub(crate) fn autoscale_bound(
        &self,
        name: AxisName,
        shared: &SharedData,
    ) -> Result<Option<(f64, f64)>, TickerError> {
        let data = match shared.interval {
            Some(interval) => interval,
            None if self.view_mutated(name) => return Ok(None),
            None => (f64::NEG_INFINITY, f64::INFINITY),
        };
        let axis = self.axis(name);
        let (_, minpos) = self.data_interval(name);
        let info = AxisInfo {
            data_interval: data,
            ..axis.info(&AxisGeometry {
                view: self.view_interval(name),
                data,
                minpos,
                length_points: 0.0,
                tick_space: None,
                tick_floor: None,
            })
        };
        let log = axis.scale().name() == "log";
        let stickies: Vec<f64> = if self.uses_stickies(name) {
            shared
                .stickies
                .iter()
                .copied()
                .filter(|&s| !log || s > 0.0)
                .collect()
        } else {
            Vec::new()
        };
        autoscale_axis(&AxisAutoscale {
            axis,
            info: &info,
            minpos: shared.minpos,
            margin: axis.autoscale.margin,
            stickies: &stickies,
            tight: self.tight,
        })
        .map(Some)
    }

    /// Recomputes the ticks of both axes for an axes `extent` in display
    /// pixels at `dpi`.
    pub(crate) fn update_ticks(&mut self, extent: &Bbox, dpi: f64, shared: [Option<usize>; 2]) {
        for (name, tick_space) in [AxisName::X, AxisName::Y].into_iter().zip(shared) {
            let ((lo, hi), minpos) = self.data_interval(name);
            let length_pixels = match name {
                AxisName::X => extent.width().abs(),
                AxisName::Y => extent.height().abs(),
            };
            let geometry = AxisGeometry {
                view: self.view_interval(name),
                data: (lo, hi),
                minpos,
                length_points: length_pixels / dpi * 72.0,
                tick_space,
                tick_floor: self.tick_floor(name),
            };
            self.axis_mut(name).update_ticks(&geometry);
        }
    }
}

impl AxesState {
    /// The value at or below which major ticks of one axis are dropped:
    /// the origin of full-circle polar axes whose lower radial limit
    /// does not exceed it (matplotlib's `RadialLocator.__call__`).
    fn tick_floor(&self, name: AxisName) -> Option<f64> {
        let polar = self.polar_view().filter(|_| name == AxisName::Y)?;
        let origin = polar.rorigin() * polar.rsign();
        let rmin = self.view_lim.y0().min(self.view_lim.y1());
        (polar.is_full_circle_rad() && rmin <= origin).then_some(origin)
    }

    /// The spines in the order of matplotlib's `spines` dictionary: left,
    /// right, bottom, top; for polar axes the outer arc, the start and
    /// end of the sector, then the inner arc.
    pub(super) fn spines_in_children_order(&self) -> impl Iterator<Item = ArtistId> + '_ {
        // `spines` holds the left, right, bottom and top spine, which polar
        // axes use as start, end, inner and outer spine.
        let order = if self.polar_view().is_some() {
            [3, 0, 1, 2]
        } else {
            [0, 1, 2, 3]
        };
        order
            .into_iter()
            .filter_map(|i| self.spines.get(i).copied())
    }
}

impl Artist for AxesState {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        self.paint(ctx);
    }

    fn children(&self) -> Vec<ArtistId> {
        self.children.clone()
    }

    fn remove_child(&mut self, child: ArtistId) {
        self.children.retain(|&id| id != child);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        ctx.axes_extent(self.base.id())
    }

    /// matplotlib's `Axes.contains`: over the background patch.
    fn contains(&self, ctx: &LayoutContext<'_>, point: Point) -> bool {
        ctx.axes_contains(self.base.id(), point)
    }
}

impl AxesState {
    /// The outline of the background patch in axes coordinates: the unit
    /// square, the wedge of polar axes, or the ellipse of geographic axes.
    pub(crate) fn patch_path(&self) -> Path {
        match self.polar_view() {
            Some(polar) => polar.patch().path(),
            None if self.geo().is_some() => geo::outline(),
            None => Path::unit_rectangle(),
        }
    }
}
