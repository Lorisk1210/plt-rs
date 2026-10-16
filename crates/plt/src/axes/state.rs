//! The axes artist: a data coordinate system placed inside a figure frame.

use std::rc::Rc;

use plt_core::{Bbox, Path};

use super::limits::{AxisAutoscale, DataLimits, autoscale_axis, oriented};
use crate::artist::{Artist, ArtistBase, ArtistId, DrawContext, LayoutContext};
use crate::colors::Color;
use crate::rc::{Cycler, RcParams};
use crate::transforms::{AxisMap, ResolvedTransform, Step, bbox_to_unit, unit_to_bbox};

/// Which axis of an axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AxisName {
    /// The horizontal axis.
    X,
    /// The vertical axis.
    Y,
}

/// Per-axis autoscaling configuration and state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AutoscaleState {
    /// Whether the view follows the data.
    pub(crate) enabled: bool,
    /// Padding added around the data, as a fraction of its span.
    pub(crate) margin: f64,
    /// Set when data changed and the view must be recomputed before use.
    pub(crate) stale: bool,
}

/// State of one axes (matplotlib's `_AxesBase`).
#[derive(Debug)]
pub(crate) struct AxesState {
    base: ArtistBase,
    /// Placement as a fraction of the owning frame.
    pub(crate) position: Bbox,
    /// Visible data range; `x0 > x1` means an inverted axis.
    pub(crate) view_lim: Bbox,
    /// Extent of all data added so far.
    pub(crate) data_lim: DataLimits,
    /// Autoscaling of the x axis.
    pub(crate) autoscale_x: AutoscaleState,
    /// Autoscaling of the y axis.
    pub(crate) autoscale_y: AutoscaleState,
    /// Whether sticky edges limit the margins.
    pub(crate) use_sticky_edges: bool,
    /// Skip view-limit rounding when autoscaling.
    pub(crate) tight: bool,
    /// Non-linear scale of the x axis (`None` = linear).
    pub(crate) x_map: Option<Rc<dyn AxisMap>>,
    /// Non-linear scale of the y axis (`None` = linear).
    pub(crate) y_map: Option<Rc<dyn AxisMap>>,
    /// Background colour.
    pub(crate) facecolor: Color,
    /// Whether the background patch is drawn.
    pub(crate) frame_on: bool,
    /// Property cycle for lines and patches.
    pub(crate) prop_cycle: Cycler,
    /// Next property-cycle entry for lines.
    pub(crate) line_cycle: usize,
    /// Plotted artists in insertion order.
    pub(crate) children: Vec<ArtistId>,
}

impl AxesState {
    /// Axes at `position` (frame fractions) configured from `rc`.
    pub(crate) fn new(position: Bbox, rc: &RcParams) -> Self {
        let autoscale = |margin| AutoscaleState { enabled: true, margin, stale: false };
        Self {
            base: ArtistBase::default(),
            position,
            view_lim: Bbox::UNIT,
            data_lim: DataLimits::default(),
            autoscale_x: autoscale(rc.axes_xmargin),
            autoscale_y: autoscale(rc.axes_ymargin),
            use_sticky_edges: true,
            tight: false,
            x_map: None,
            y_map: None,
            facecolor: rc.axes_facecolor.clone(),
            frame_on: true,
            prop_cycle: rc.axes_prop_cycle.clone(),
            line_cycle: 0,
            children: Vec::new(),
        }
    }

    /// The full data → display transform for the axes placed at `display`:
    /// scales, then view limits onto the unit square, then onto `display`.
    pub(crate) fn data_transform(&self, display: &Bbox) -> ResolvedTransform {
        let forward = |map: &Option<Rc<dyn AxisMap>>, v: f64| map.as_ref().map_or(v, |m| m.forward(v));
        let scaled_limits = Bbox::from_extents(
            forward(&self.x_map, self.view_lim.x0()),
            forward(&self.y_map, self.view_lim.y0()),
            forward(&self.x_map, self.view_lim.x1()),
            forward(&self.y_map, self.view_lim.y1()),
        );
        let affine = bbox_to_unit(&scaled_limits).then(unit_to_bbox(display));
        if self.x_map.is_none() && self.y_map.is_none() {
            ResolvedTransform::affine(affine)
        } else {
            ResolvedTransform::with_step(
                Step::Separable(self.x_map.clone(), self.y_map.clone()),
                affine,
            )
        }
    }

    /// Sets the view interval of one axis; explicit limits switch that
    /// axis's autoscaling off (matplotlib's `set_xlim(..., auto=False)`).
    pub(crate) fn set_limits(&mut self, axis: AxisName, lower: f64, upper: f64) {
        let v = self.view_lim;
        match axis {
            AxisName::X => {
                self.view_lim = Bbox::from_extents(lower, v.y0(), upper, v.y1());
                self.autoscale_x = AutoscaleState { enabled: false, stale: false, ..self.autoscale_x };
            }
            AxisName::Y => {
                self.view_lim = Bbox::from_extents(v.x0(), lower, v.x1(), upper);
                self.autoscale_y = AutoscaleState { enabled: false, stale: false, ..self.autoscale_y };
            }
        }
    }

    /// Records new data and asks for the view to follow it.
    pub(crate) fn add_data(&mut self, points: &[plt_core::Point], update_x: bool, update_y: bool) {
        self.data_lim.update(points, update_x, update_y);
        self.autoscale_x.stale |= update_x;
        self.autoscale_y.stale |= update_y;
    }

    /// Recomputes stale autoscaled view limits (matplotlib's
    /// `_unstale_viewLim`); `stickies` are the sorted sticky edges of the
    /// axes' children per axis.
    pub(crate) fn unstale_view(&mut self, stickies: (&[f64], &[f64])) {
        let identity = |lo, hi, _| (lo, hi);
        for (axis, sticky) in [(AxisName::X, stickies.0), (AxisName::Y, stickies.1)] {
            let (state, interval, minpos, map, inverted) = match axis {
                AxisName::X => (
                    self.autoscale_x,
                    (self.data_lim.bbox.x0(), self.data_lim.bbox.x1()),
                    self.data_lim.minpos.x,
                    self.x_map.as_ref(),
                    self.view_lim.x0() > self.view_lim.x1(),
                ),
                AxisName::Y => (
                    self.autoscale_y,
                    (self.data_lim.bbox.y0(), self.data_lim.bbox.y1()),
                    self.data_lim.minpos.y,
                    self.y_map.as_ref(),
                    self.view_lim.y0() > self.view_lim.y1(),
                ),
            };
            if !(state.stale && state.enabled) {
                continue;
            }
            let usable = |s: &[f64]| if self.use_sticky_edges && state.margin != 0.0 { s.to_vec() } else { Vec::new() };
            let bound = autoscale_axis(&AxisAutoscale {
                data: interval,
                minpos,
                margin: state.margin,
                stickies: &usable(sticky),
                scale: map,
                limit_range: &identity,
                tight: self.tight,
            });
            let (lower, upper) = oriented(bound, inverted);
            let v = self.view_lim;
            match axis {
                AxisName::X => {
                    self.view_lim = Bbox::from_extents(lower, v.y0(), upper, v.y1());
                    self.autoscale_x.stale = false;
                }
                AxisName::Y => {
                    self.view_lim = Bbox::from_extents(v.x0(), lower, v.x1(), upper);
                    self.autoscale_y.stale = false;
                }
            }
        }
    }

    /// Whether any axis waits for autoscaling.
    pub(crate) fn needs_autoscale(&self) -> bool {
        (self.autoscale_x.stale && self.autoscale_x.enabled) || (self.autoscale_y.stale && self.autoscale_y.enabled)
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
        let Some(extent) = ctx.layout().axes_extent(self.base.id()) else {
            return;
        };
        if self.frame_on {
            match self.facecolor.resolve() {
                Ok(face) => {
                    let mut gc = ctx.base_gc(&self.base);
                    gc.linewidth = 0.0;
                    ctx.renderer()
                        .draw_path(&gc, &Path::unit_rectangle(), &unit_to_bbox(&extent), Some(face));
                }
                Err(err) => ctx.report(self.base.id(), err),
            }
        }
        ctx.draw_children_sorted(&self.children);
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
}
