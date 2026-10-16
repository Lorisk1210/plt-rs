//! The axes artist: a data coordinate system placed inside a figure frame.

use plt_core::{Bbox, Path, Point};

use super::axis::{AxisGeometry, AxisState};
use super::limits::{AxisAutoscale, DataLimits, autoscale_axis, oriented};
use crate::artist::{Artist, ArtistBase, ArtistId, DrawContext, LayoutContext};
use crate::colors::Color;
use crate::gridspec::SubplotSpec;
use crate::rc::{Cycler, RcParams};
use crate::ticker::{AxisInfo, AxisName};
use crate::transforms::{ResolvedTransform, Step, bbox_to_unit, unit_to_bbox};

/// State of one axes (matplotlib's `_AxesBase`).
#[derive(Debug)]
pub(crate) struct AxesState {
    base: ArtistBase,
    /// Placement as a fraction of the owning frame.
    pub(crate) position: Bbox,
    /// The grid cell the axes occupies, if placed on a grid.
    pub(crate) subplot_spec: Option<SubplotSpec>,
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
    /// The spines, in `SpineSide::ALL` order once created.
    pub(crate) spines: Vec<ArtistId>,
}

impl AxesState {
    /// Axes at `position` (frame fractions) configured from `rc`.
    pub(crate) fn new(position: Bbox, rc: &RcParams) -> Self {
        Self {
            base: ArtistBase::default(),
            position,
            subplot_spec: None,
            view_lim: Bbox::UNIT,
            view_mutated: (false, false),
            data_lim: DataLimits::default(),
            xaxis: AxisState::new(AxisName::X, rc),
            yaxis: AxisState::new(AxisName::Y, rc),
            use_sticky_edges: true,
            tight: false,
            facecolor: rc.axes_facecolor.clone(),
            frame_on: true,
            prop_cycle: rc.axes_prop_cycle.clone(),
            line_cycle: 0,
            children: Vec::new(),
            spines: Vec::new(),
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

    /// The full data → display transform for the axes placed at `display`:
    /// scales, then view limits onto the unit square, then onto `display`.
    pub(crate) fn data_transform(&self, display: &Bbox) -> ResolvedTransform {
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

    /// Sets the view interval of one axis; explicit limits switch that
    /// axis's autoscaling off (matplotlib's `set_xlim(..., auto=False)`).
    pub(crate) fn set_limits(&mut self, name: AxisName, lower: f64, upper: f64) {
        self.set_view_interval(name, (lower, upper));
        let autoscale = &mut self.axis_mut(name).autoscale;
        autoscale.enabled = false;
        autoscale.stale = false;
    }

    fn set_view_interval(&mut self, name: AxisName, (lower, upper): (f64, f64)) {
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

    /// Records new data and asks for the view to follow it.
    pub(crate) fn add_data(&mut self, points: &[Point], update_x: bool, update_y: bool) {
        self.data_lim.update(points, update_x, update_y);
        self.xaxis.autoscale.stale |= update_x;
        self.yaxis.autoscale.stale |= update_y;
    }

    /// Data interval and minimum positive value of one axis.
    fn data_interval(&self, name: AxisName) -> ((f64, f64), f64) {
        let bbox = self.data_lim.bbox;
        match name {
            AxisName::X => ((bbox.x0(), bbox.x1()), self.data_lim.minpos.x),
            AxisName::Y => ((bbox.y0(), bbox.y1()), self.data_lim.minpos.y),
        }
    }

    /// The data interval autoscaling starts from: the finite data limits,
    /// `(-inf, inf)` without data, or `None` when there is no data but the
    /// view was set explicitly (matplotlib keeps such limits).
    fn autoscale_source(&self, name: AxisName) -> Option<(f64, f64)> {
        let ((lo, hi), _) = self.data_interval(name);
        let finite: Vec<f64> = [lo, hi].into_iter().filter(|v| v.is_finite()).collect();
        let mutated = match name {
            AxisName::X => self.view_mutated.0,
            AxisName::Y => self.view_mutated.1,
        };
        match finite.as_slice() {
            [] if mutated => None,
            [] => Some((f64::NEG_INFINITY, f64::INFINITY)),
            values => Some((
                values.iter().copied().fold(f64::INFINITY, f64::min),
                values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            )),
        }
    }

    /// Recomputes stale autoscaled view limits (matplotlib's
    /// `_unstale_viewLim` / `autoscale_view`); `stickies` are the sorted
    /// sticky edges of the axes' children per axis.
    pub(crate) fn unstale_view(&mut self, stickies: (&[f64], &[f64])) -> Result<(), String> {
        for (name, sticky) in [(AxisName::X, stickies.0), (AxisName::Y, stickies.1)] {
            let autoscale = self.axis(name).autoscale;
            if !(autoscale.stale && autoscale.enabled) {
                continue;
            }
            self.axis_mut(name).autoscale.stale = false;
            let Some(data) = self.autoscale_source(name) else {
                continue;
            };
            let view = self.view_interval(name);
            let ((_, _), minpos) = self.data_interval(name);
            let info = AxisInfo {
                data_interval: data,
                ..self.axis(name).info(&AxisGeometry {
                    view,
                    data,
                    minpos,
                    length_points: 0.0,
                })
            };
            let usable = if self.use_sticky_edges && autoscale.margin != 0.0 {
                sticky
            } else {
                &[]
            };
            let bound = autoscale_axis(&AxisAutoscale {
                axis: self.axis(name),
                info: &info,
                margin: autoscale.margin,
                stickies: usable,
                tight: self.tight,
            })
            .map_err(|err| err.to_string())?;
            self.set_view_interval(name, oriented(bound, view.0 > view.1));
        }
        Ok(())
    }

    /// Whether any axis waits for autoscaling.
    pub(crate) fn needs_autoscale(&self) -> bool {
        [&self.xaxis, &self.yaxis]
            .iter()
            .any(|axis| axis.autoscale.stale && axis.autoscale.enabled)
    }

    /// Recomputes the ticks of both axes for an axes `extent` in display
    /// pixels at `dpi`.
    pub(crate) fn update_ticks(&mut self, extent: &Bbox, dpi: f64) {
        for name in [AxisName::X, AxisName::Y] {
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
            };
            self.axis_mut(name).update_ticks(&geometry);
        }
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
                    ctx.renderer().draw_path(
                        &gc,
                        &Path::unit_rectangle(),
                        &unit_to_bbox(&extent),
                        Some(face),
                    );
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
