//! The axes artist: a data coordinate system placed inside a figure frame.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use pltrs_core::{Bbox, Path, Point};
use pltrs_render::ClipPath;

use super::DataLimits;
use super::aspect::{AspectState, AxesLocatorFn};
use super::axis::{AxisGeometry, AxisState};
use super::current_image::CurrentImage;
use super::limits::{AxisAutoscale, autoscale_axis};
use super::projection::ProjectionState;
use super::secondary::Secondary;
use super::transform_sites::ScaleBox;
use crate::artist::{Artist, ArtistBase, ArtistId, DrawContext, LayoutContext};
use crate::axes_grid1::Parasite;
use crate::colors::Color;
use crate::geo::GeoState;
use crate::gridspec::SubplotSpec;
use crate::patches::PatchStyle;
use crate::polar::{PolarState, PolarView};
use crate::projections::SharedOptions;
use crate::rc::{AxisBelow, Cycler, RcParams};
use crate::scale::ScaleName;
use crate::spines::SpineSide;
use crate::ticker::{AxisInfo, AxisName, Formatter, TickContext, TickerError};
use crate::transforms::{ResolvedTransform, Transform, unit_to_bbox};

/// The formatters of the x and y coordinates a status bar shows, unset
/// where the major formatters do (see [`AxesState::fmt_data`]).
pub(crate) type DataFormatters = (Option<Rc<dyn Formatter>>, Option<Rc<dyn Formatter>>);

/// State of one axes (matplotlib's `_AxesBase`).
#[derive(Clone, Debug)]
pub(crate) struct AxesState {
    base: ArtistBase,
    /// Placement as a fraction of the owning frame, before any aspect
    /// adjustment (matplotlib's original position).
    pub(crate) position: Bbox,
    /// The aspect settings and the active position they produce.
    pub(crate) aspect: AspectState,
    /// The function placing the axes at layout time, if one was set
    /// (matplotlib's `_axes_locator`).
    pub(crate) axes_locator: Option<AxesLocatorFn>,
    /// The grid cell the axes occupies, if placed on a grid.
    pub(crate) subplot_spec: Option<SubplotSpec>,
    /// The (sub)figure an axes made outside its figure is for, until the
    /// (sub)figure adds it (matplotlib's `_parent_figure` of an
    /// `Axes(fig, rect)`): its position is in that (sub)figure's
    /// fractions, and only it can add the axes.
    pub(crate) home: Option<ArtistId>,
    /// The coordinate system.
    pub(crate) projection: ProjectionState,
    /// The options a registered projection was made with, if any
    /// (matplotlib's `_projection_init`).
    pub(crate) projection_options: Option<SharedOptions>,
    /// Visible data range; `x0 > x1` means an inverted axis.
    pub(crate) view_lim: Bbox,
    /// Whether the view limits differ from the initial unit box
    /// (matplotlib's `viewLim.mutatedx()` and `mutatedy()`).
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
    /// Set for the auxiliary axes of an axisartist host: data coordinates
    /// go through this transform, then the host's data transform
    /// (matplotlib's parasite `transAux`).
    pub(crate) aux_transform: Option<ResolvedTransform>,
    /// A background outline replacing the projection's (matplotlib's
    /// `_gen_axes_patch` of floating axes or of an Axes class of the
    /// program's).
    pub(crate) patch_outline: Option<PatchOutline>,
    /// Property cycle for lines and patches.
    pub(crate) prop_cycle: Cycler,
    /// Next property-cycle entry for lines.
    pub(crate) line_cycle: usize,
    /// Next property-cycle entry for patches and collections.
    pub(crate) patch_cycle: usize,
    /// The axes whose line property cycle this one draws from, when a
    /// program pointed it at another's (see [`PropCycle`](super::PropCycle)).
    pub(crate) line_cycle_from: Option<ArtistId>,
    /// The axes whose patch property cycle this one draws from.
    pub(crate) patch_cycle_from: Option<ArtistId>,
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
    /// Whether pan and zoom also reach the axes below (matplotlib's
    /// `forward_navigation_events`; `None` is `"auto"`).
    pub(crate) forward_navigation: Option<bool>,
    /// The formatters of the x and y coordinates a status bar shows, in
    /// place of the major formatters (matplotlib's `fmt_xdata` and
    /// `fmt_ydata`).
    pub(crate) fmt_data: DataFormatters,
    /// How the titles' height is chosen (matplotlib's `_autotitlepos`).
    pub(crate) title_placement: TitlePlacement,
    /// The gap between the axes top and its titles in points (matplotlib's
    /// `titleOffsetTrans`).
    pub(crate) title_pad: f64,
    /// Where the axis artists sit in the z-order (`axes.axisbelow`).
    pub(crate) axisbelow: AxisBelow,
    /// Children at or below this z-order are rasterized together in
    /// vector output (matplotlib's `rasterization_zorder`).
    pub(crate) rasterization_zorder: Option<f64>,
    /// The colour-mapped artist pyplot's colormap functions act on
    /// (matplotlib's `_current_image`).
    pub(crate) current_image: Option<CurrentImage>,
    /// The rc parameters the axes was created with.
    pub(crate) rc: Rc<RcParams>,
    /// The view limits as matplotlib last mapped them through the
    /// program's scales of the axes (see `transform_sites`).
    pub(crate) scale_box: RefCell<Option<ScaleBox>>,
}

pltrs_core::archive_struct!(AxesState {
    base,
    position,
    aspect,
    axes_locator,
    subplot_spec,
    home,
    projection,
    projection_options,
    view_lim,
    view_mutated,
    data_lim,
    xaxis,
    yaxis,
    use_sticky_edges,
    tight,
    patch,
    decorations,
    shares,
    secondary,
    parasite,
    aux_transform,
    patch_outline,
    prop_cycle,
    line_cycle,
    patch_cycle,
    line_cycle_from,
    patch_cycle_from,
    children,
    containers,
    spines,
    titles,
    navigate,
    forward_navigation,
    fmt_data,
    title_placement,
    title_pad,
    axisbelow,
    rasterization_zorder,
    current_image,
    rc
} skip { scale_box });

/// How the height of an axes' titles is chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TitlePlacement {
    /// Lifted above the decorations on top of the axes.
    Auto,
    /// Where `axes.titley` or `Axes::set_title_y` put them.
    Fixed,
}

pltrs_core::archive_enum!(TitlePlacement { Auto, Fixed });

/// Which decorations an axes draws (matplotlib's `axison` and
/// `_frameon`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Decorations {
    /// The patch, the spines and both axes.
    pub(crate) axis_on: bool,
    /// The patch and the spines.
    pub(crate) frame_on: bool,
}

pltrs_core::archive_struct!(Decorations { axis_on, frame_on });

impl Decorations {
    /// Whether the patch and spines are drawn.
    pub(crate) fn framed(self) -> bool {
        self.axis_on && self.frame_on
    }
}

/// The background rectangle of an axes (matplotlib's `ax.patch`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AxesPatch {
    /// Its face (`axes.facecolor`), edge (`"none"` unless set), edge
    /// width (0 unless set), line style, hatch, caps, joins and
    /// anti-aliasing, as a patch has them.
    pub(crate) style: PatchStyle,
    /// The opacity replacing that of its colours (matplotlib's
    /// `ax.patch.set_alpha`).
    pub(crate) alpha: Option<f64>,
    /// Whether it is drawn.
    pub(crate) visible: bool,
    /// Its z-order where it is drawn among children (a parasite sharing
    /// it); as the background it is always drawn first.
    pub(crate) zorder: f64,
    /// The id of its group in SVG output (matplotlib's `Patch.get_gid`).
    pub(crate) gid: Option<String>,
    /// Whether this parasite's patch is its host's (matplotlib's
    /// `aux.patch = ax.patch`), which the host then draws again among the
    /// parasite's children.
    pub(crate) host_owned: bool,
}

/// The outline of a background replacing the projection's: the shape the
/// background is drawn in and what the axes clips is clipped to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PatchOutline {
    /// The closed outline, in the coordinates of `transform`.
    pub(crate) path: Path,
    /// Its coordinates.
    pub(crate) transform: Transform,
    /// Whether the background drawn along it is clipped to the axes box
    /// (floating axes' `patch.set_clip_path(orig_patch)`).
    pub(crate) clipped_to_box: bool,
    /// Whether what the axes clips is clipped to the outline's bounding
    /// box rather than to the outline (matplotlib's clip box of a
    /// `Rectangle` patch).
    pub(crate) clips_as_box: bool,
}

pltrs_core::archive_struct!(PatchOutline {
    path,
    transform,
    clipped_to_box,
    clips_as_box
});

pltrs_core::archive_struct!(AxesPatch {
    style,
    alpha,
    visible,
    zorder,
    gid,
    host_owned
});

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

/// How long an axis is and how many ticks fit along it, which
/// round-number autoscaling needs (matplotlib's `get_tick_space`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct AxisSpace {
    /// The length of the axis in points.
    pub(crate) length_points: f64,
    /// How many ticks fit along the axis of another axes the locator is
    /// bound to (`None` measures this axis).
    pub(crate) tick_space: Option<usize>,
}

impl AxesState {
    /// Axes at `position` (frame fractions) configured from `rc`.
    pub(crate) fn new(position: Bbox, rc: Rc<RcParams>) -> Self {
        Self {
            base: ArtistBase::default(),
            position,
            aspect: AspectState::default(),
            axes_locator: None,
            subplot_spec: None,
            home: None,
            projection: ProjectionState::Rectilinear,
            projection_options: None,
            view_lim: Bbox::UNIT,
            view_mutated: (false, false),
            data_lim: DataLimits::default(),
            xaxis: AxisState::new(AxisName::X, &rc),
            yaxis: AxisState::new(AxisName::Y, &rc),
            use_sticky_edges: true,
            tight: false,
            patch: AxesPatch {
                style: PatchStyle {
                    facecolor: Some(rc.axes_facecolor.clone()),
                    edgecolor: Some(Color::from("none")),
                    linewidth: Some(0.0),
                    ..PatchStyle::from_rc(&rc)
                },
                alpha: None,
                visible: true,
                zorder: 1.0,
                gid: None,
                host_owned: false,
            },
            decorations: Decorations {
                axis_on: true,
                frame_on: true,
            },
            shares: (None, None),
            secondary: None,
            parasite: None,
            aux_transform: None,
            patch_outline: None,
            prop_cycle: rc.axes_prop_cycle.clone(),
            line_cycle: 0,
            patch_cycle: 0,
            line_cycle_from: None,
            patch_cycle_from: None,
            children: Vec::new(),
            containers: Vec::new(),
            spines: Vec::new(),
            titles: Vec::new(),
            navigate: true,
            forward_navigation: None,
            fmt_data: (None, None),
            title_placement: if rc.axes_titley.is_some() {
                TitlePlacement::Fixed
            } else {
                TitlePlacement::Auto
            },
            title_pad: rc.axes_titlepad,
            axisbelow: rc.axes_axisbelow,
            rasterization_zorder: None,
            current_image: None,
            rc,
            scale_box: RefCell::new(None),
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

    /// The titles in matplotlib's order (`title`, `_left_title`,
    /// `_right_title`): the centre one first, then the left and the right.
    pub(crate) fn titles_centre_first(&self) -> impl Iterator<Item = ArtistId> + '_ {
        [1, 0, 2]
            .into_iter()
            .filter_map(|i| self.titles.get(i).copied())
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
        self.projection.polar()
    }

    /// The geographic settings, if this is a geographic axes.
    pub(crate) fn geo(&self) -> Option<&GeoState> {
        let projection: &dyn Any = self.projection.custom()?;
        projection.downcast_ref()
    }

    /// The polar geometry at the current view, if this is a polar axes.
    pub(crate) fn polar_view(&self) -> Option<PolarView<'_>> {
        let view = PolarView::new(self.polar()?, self.view_lim, self.yaxis.map());
        Some(match self.projection.polar_program() {
            Some(program) => view.with_program(program, self.projection_view()),
            None => view,
        })
    }

    /// The full data → display transform for the axes placed at `display`:
    /// scales, then view limits onto the unit square, then onto `display`
    /// (or the polar, a geographic or a custom projection).
    pub(crate) fn data_transform(&self, display: &Bbox) -> ResolvedTransform {
        if let Some(polar) = self.polar_view() {
            return polar.data_transform(display);
        }
        if let Some(custom) = self.custom_data_transform(display) {
            return custom;
        }
        (self.projection_view().rectilinear())
            .then(ResolvedTransform::affine(unit_to_bbox(display)))
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
    /// keep the current value, with matplotlib's warning for each,
    /// singular intervals are expanded by the major locator, and the scale
    /// clamps the range; `v0 > v1` keeps the axis inverted.
    pub(crate) fn validated_limits(&self, name: AxisName, v0: f64, v1: f64) -> (f64, f64) {
        let axis = self.axis(name);
        let (mut v0, mut v1) = (v0, v1);
        if axis.scale().name() == ScaleName::Log && (v0 <= 0.0 || v1 <= 0.0) {
            let (old0, old1) = self.view_interval(name);
            for (limit, old) in [(&mut v0, old0), (&mut v1, old1)] {
                if *limit <= 0.0 {
                    crate::warnings::warn(&format!(
                        "Attempt to set non-positive {}lim on a log-scaled axis will be ignored.",
                        name.name()
                    ));
                    *limit = old;
                }
            }
        }
        let reverse = v0 > v1;
        let (v0, v1) = self.nonsingular(name, v0, v1);
        let (_, minpos) = self.data_interval(name);
        let (v0, v1) = axis.scale().limit_range_for_scale(v0, v1, minpos);
        let (low, high) = if v0 <= v1 { (v0, v1) } else { (v1, v0) };
        if reverse { (high, low) } else { (low, high) }
    }

    /// The interval `(v0, v1)` made safe for one axis by its major locator
    /// (matplotlib's `locator.nonsingular`); `(-inf, inf)` gives the
    /// locator's default limits.
    pub(crate) fn nonsingular(&self, name: AxisName, v0: f64, v1: f64) -> (f64, f64) {
        let axis = self.axis(name);
        let (data, minpos) = self.data_interval(name);
        let info = axis.info(&AxisGeometry {
            view: self.view_interval(name),
            data,
            minpos,
            length_points: 0.0,
            tick_space: None,
            tick_floor: None,
        });
        let settings = axis.settings();
        let ctx = TickContext::new(&info, &settings);
        axis.major_locator.nonsingular(v0, v1, &ctx)
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

    /// Replaces the view interval of one axis; it counts as mutated unless
    /// it is the initial unit interval again (matplotlib compares exactly).
    pub(crate) fn set_view_interval(&mut self, name: AxisName, (lower, upper): (f64, f64)) {
        // matplotlib's view limits invalidate what was computed from them
        // whenever they are set, to the same values or not.
        self.scale_box.replace(None);
        let v = self.view_lim;
        let mutated = (lower, upper) != (0.0, 1.0);
        match name {
            AxisName::X => {
                self.view_lim = Bbox::from_extents(lower, v.y0(), upper, v.y1());
                self.view_mutated.0 = mutated;
            }
            AxisName::Y => {
                self.view_lim = Bbox::from_extents(v.x0(), lower, v.x1(), upper);
                self.view_mutated.1 = mutated;
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

    /// Replaces the data limits of one axis with `low..=high`, keeping the
    /// smallest positive datum (matplotlib's `Axis.set_data_interval` with
    /// `ignore=True`).
    pub(crate) fn set_data_interval(&mut self, name: AxisName, low: f64, high: f64) {
        let b = self.data_lim.bbox;
        self.data_lim.bbox = match name {
            AxisName::X => Bbox::from_extents(low, b.y0(), high, b.y1()),
            AxisName::Y => Bbox::from_extents(b.x0(), low, b.x1(), high),
        };
    }

    /// Whether the margins of one axis stop at sticky edges.
    pub(crate) fn uses_stickies(&self, name: AxisName) -> bool {
        let autoscale = self.axis(name).autoscale;
        self.use_sticky_edges && autoscale.margin != 0.0 && autoscale.enabled
    }

    /// The autoscaled `(lower, upper)` bound of one axis (matplotlib's
    /// `handle_single_axis` in `autoscale_view`) along an axis of `space`,
    /// or `None` when there is no data and the view was set explicitly
    /// (matplotlib keeps it).
    pub(crate) fn autoscale_bound(
        &self,
        name: AxisName,
        shared: &SharedData,
        space: AxisSpace,
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
                length_points: space.length_points,
                tick_space: space.tick_space,
                tick_floor: None,
            })
        };
        let log = axis.scale().name() == ScaleName::Log;
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

    /// Recomputes the ticks of axis `name` alone for an axes `extent` in
    /// display pixels at `dpi`.
    pub(crate) fn update_axis_ticks(
        &mut self,
        name: AxisName,
        extent: &Bbox,
        dpi: f64,
        tick_space: Option<usize>,
    ) {
        let geometry = self.axis_geometry(name, extent, dpi, tick_space);
        self.axis_mut(name).update_ticks(&geometry);
    }

    /// The major (or `minor`) tick locations of axis `name` for an axes
    /// `extent` in display pixels at `dpi`, without keeping them.
    pub(crate) fn level_locs_for(
        &self,
        name: AxisName,
        extent: &Bbox,
        dpi: f64,
        tick_space: Option<usize>,
        minor: bool,
    ) -> Result<Vec<f64>, TickerError> {
        self.axis(name)
            .level_locs(&self.axis_geometry(name, extent, dpi, tick_space), minor)
    }

    /// What the ticks of axis `name` are computed from for an axes
    /// `extent` in display pixels at `dpi`.
    pub(crate) fn axis_geometry(
        &self,
        name: AxisName,
        extent: &Bbox,
        dpi: f64,
        tick_space: Option<usize>,
    ) -> AxisGeometry {
        let ((lo, hi), minpos) = self.data_interval(name);
        let length_pixels = match name {
            AxisName::X => extent.width().abs(),
            AxisName::Y => extent.height().abs(),
        };
        AxisGeometry {
            view: self.tick_interval(name),
            data: (lo, hi),
            minpos,
            length_points: length_pixels / dpi * 72.0,
            tick_space,
            tick_floor: self.tick_floor(name),
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
    /// end of the sector, then the inner arc; for a custom projection the
    /// order it gives, sides it leaves out last. The spines of no side (a
    /// colorbar's outline, see
    /// [`replace_outline_spine`](Self::replace_outline_spine)) come after.
    pub(crate) fn spines_in_children_order(&self) -> Vec<ArtistId> {
        // `spines` holds the left, right, bottom and top spine, which polar
        // axes use as start, end, inner and outer spine.
        let order = match (self.polar_view(), self.projection.custom()) {
            (Some(_), _) => [
                SpineSide::Top,
                SpineSide::Left,
                SpineSide::Right,
                SpineSide::Bottom,
            ],
            (None, Some(custom)) => custom.spine_order(),
            (None, None) => SpineSide::ALL,
        };
        let mut sides: Vec<SpineSide> = Vec::with_capacity(SpineSide::ALL.len());
        for side in order.into_iter().chain(SpineSide::ALL) {
            if !sides.contains(&side) {
                sides.push(side);
            }
        }
        sides
            .into_iter()
            .filter_map(|side| {
                let index = SpineSide::ALL.iter().position(|&s| s == side)?;
                self.spines.get(index).copied()
            })
            .chain(self.spines.iter().skip(SpineSide::ALL.len()).copied())
            .collect()
    }

    /// Makes the child `spine` the spine of the axes that frames no side
    /// (matplotlib's `ax.spines['outline'] = spine`, as a colorbar adds its
    /// outline): drawn with the spines, only while the axes is framed, and
    /// not among the artists added to the axes. Returns the spine it
    /// replaces (that of a colorbar drawn in the axes before), which is no
    /// spine of the axes any more.
    pub(crate) fn replace_outline_spine(&mut self, spine: ArtistId) -> Vec<ArtistId> {
        let sides = SpineSide::ALL.len().min(self.spines.len());
        let replaced = self.spines.split_off(sides);
        self.spines.push(spine);
        replaced
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

    /// The outline of the background patch when it is no rectangle (the
    /// wedge of polar axes, the outline of floating axes), which artists
    /// clipped to the patch (`set_clip_path(ax.patch)`) are clipped to; a
    /// rectangular patch clips as the clip box of the axes, which it is.
    fn outline(&self, ctx: &LayoutContext<'_>) -> Option<ClipPath> {
        ctx.clip_outline(self.base.id())
    }
}

impl AxesState {
    /// The outline of the background patch in axes coordinates: the unit
    /// square, the wedge of polar axes, the ellipse of geographic axes or
    /// the outline of a custom projection.
    pub(crate) fn patch_path(&self) -> Path {
        match self.polar_view() {
            Some(polar) => polar.patch_path(),
            None => self.custom_outline().unwrap_or_else(Path::unit_rectangle),
        }
    }
}
