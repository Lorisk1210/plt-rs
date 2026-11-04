//! Applying the aspect at layout time (matplotlib's `apply_aspect`): the
//! active position of an axes, or the view interval widened for
//! [`Adjustable::Datalim`], from its data ratio.

use pltrs_core::Bbox;

use super::{Adjustable, AspectError};
use crate::artist::{Artist, ArtistId, DrawError, DrawErrorKind};
use crate::axes::AxesState;
use crate::figure::{FigureState, host_to_figure};
use crate::ticker::AxisName;
use crate::transforms::TransformContext;

impl AxesState {
    /// The `(lower, upper)` view bound of one axis in scaled coordinates.
    fn scaled_bound(&self, name: AxisName) -> (f64, f64) {
        let (v0, v1) = self.view_interval(name);
        let axis = self.axis(name);
        (axis.forward(v0.min(v1)), axis.forward(v0.max(v1)))
    }

    /// matplotlib's `get_data_ratio` (always 1 for polar and geographic
    /// axes, fixed by some custom projections).
    pub(crate) fn data_ratio(&self) -> f64 {
        if let Some(ratio) = self.projection.fixed_data_ratio() {
            return ratio;
        }
        let (x0, x1) = self.scaled_bound(AxisName::X);
        let (y0, y1) = self.scaled_bound(AxisName::Y);
        let xsize = py_max((x1 - x0).abs(), 1e-30);
        let ysize = py_max((y1 - y0).abs(), 1e-30);
        ysize / xsize
    }
}

/// Python's `max(value, floor)`, as matplotlib computes sizes: a NaN
/// `value` stays NaN (Rust's `f64::max` would give `floor`).
fn py_max(value: f64, floor: f64) -> f64 {
    if floor > value { floor } else { value }
}

/// Where an axes is placed before its aspect is applied.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Placement {
    /// Where its divider, inset locator or axes locator puts it.
    Located,
    /// At its original position, without calling its locators
    /// (matplotlib's `get_tightbbox(call_axes_locator=False)`, which
    /// sizes measuring the axes for its own divider use).
    Original,
    /// At a position the program gives, in figure fractions
    /// (matplotlib's `apply_aspect(position)`).
    Given(Bbox),
}

impl FigureState {
    /// Brings one axes up to date for layout: a due layout-engine pass,
    /// pending autoscaling, then the aspect; whether the aspect could be
    /// applied (see [`apply_aspect`](Self::apply_aspect)).
    pub(crate) fn lay_out_axes(&mut self, axes: ArtistId) -> bool {
        self.lay_out_axes_at(axes, Placement::Located)
    }

    /// [`lay_out_axes`](Self::lay_out_axes) with the axes placed by
    /// `placement`.
    pub(crate) fn lay_out_axes_at(&mut self, axes: ArtistId, placement: Placement) -> bool {
        self.run_pending_layout();
        self.unstale_axes(axes);
        self.apply_aspect_reporting(axes, placement)
    }

    /// matplotlib's `apply_aspect`: sets the active position of `axes`
    /// and, for [`Adjustable::Datalim`], widens one view interval; a
    /// problem is reported with the next draw.
    pub(crate) fn apply_aspect(&mut self, axes: ArtistId) {
        self.apply_aspect_reporting(axes, Placement::Located);
    }

    /// [`apply_aspect_at`](Self::apply_aspect_at), its problem reported
    /// with the next draw (matplotlib's draw raises it); whether it
    /// succeeded.
    fn apply_aspect_reporting(&mut self, axes: ArtistId, placement: Placement) -> bool {
        let Err(err) = self.apply_aspect_at(axes, placement) else {
            return true;
        };
        self.pending_errors.push(DrawError {
            artist: axes,
            message: err.to_string(),
            kind: DrawErrorKind::InvalidValue,
        });
        false
    }

    /// matplotlib's `apply_aspect` called on its own, between draws: no
    /// layout-engine pass, and pending autoscaling only when a fixed
    /// aspect reads the view limits (through `get_data_ratio`). A
    /// secondary axis takes its host's limits first, as matplotlib's
    /// `SecondaryAxis.apply_aspect` does.
    pub(crate) fn apply_aspect_now(
        &mut self,
        axes: ArtistId,
        placement: Placement,
    ) -> Result<(), AspectError> {
        self.sync_secondary(axes);
        let fixed = self
            .store
            .get_as::<AxesState>(axes)
            .is_some_and(|state| state.aspect.ratio.is_some());
        if fixed {
            self.unstale_axes(axes);
        }
        self.apply_aspect_at(axes, placement)
    }

    /// [`apply_aspect`](Self::apply_aspect) with the axes placed by
    /// `placement`. The active position is set before the data limits are
    /// adjusted, which fails when they would not be finite (as
    /// matplotlib's `set_xbound` refuses them there).
    fn apply_aspect_at(&mut self, axes: ArtistId, placement: Placement) -> Result<(), AspectError> {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return Ok(());
        };
        let original = state.position;
        let settings = state.aspect;
        let custom = state.axes_locator.clone();
        let located = match (placement, custom) {
            (Placement::Located, Some(locator)) => locator.locate(original),
            (Placement::Located, None) => {
                let divided = settings
                    .divider
                    .and_then(|cell| self.locate_in_divider(cell))
                    .or_else(|| settings.inset.and_then(|id| self.locate_inset(axes, id)))
                    .unwrap_or(original);
                settings
                    .locator
                    .map_or(divided, |locator| locator.locate(&divided))
            }
            (Placement::Original, _) => original,
            (Placement::Given(position), _) => position,
        };
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return Ok(());
        };
        let box_aspect = settings
            .locator
            .and_then(|locator| locator.box_aspect)
            .or(settings.box_aspect);
        let data_ratio = state.data_ratio();
        let parent = state.base().parent;
        // Insets and secondary axes are placed in their host axes, in the
        // fractions of the host's figure, as matplotlib's locator places
        // them there before it applies the aspect; axes covering their host
        // stay in its fractions.
        let hosted = parent
            .filter(|&p| original != Bbox::UNIT && self.store.get_as::<AxesState>(p).is_some())
            .and_then(|p| {
                let (host, figure) = (self.axes_bbox(p)?, self.figure_bbox_of(p)?);
                Some(((host, figure), host_to_figure(&host, &figure)?))
            });
        let (located, frame) = match hosted {
            // A position the program gives is in figure fractions already.
            Some(((_, figure), _)) if matches!(placement, Placement::Given(_)) => {
                (located, Some(figure))
            }
            Some(((_, figure), to_figure)) => (located.transformed(&to_figure), Some(figure)),
            None => (
                located,
                parent.and_then(|f| self.frame_bbox(f).or_else(|| self.axes_bbox(f))),
            ),
        };
        let mut adjusted = Ok(());
        let active = if settings.ratio.is_none() && box_aspect.is_none() {
            Some(located)
        } else if let Some(frame) = frame {
            let fig_aspect = frame.height() / frame.width();
            let shrunk = |aspect: f64| {
                located
                    .shrunk_to_aspect(aspect, Some(&located), fig_aspect)
                    .map(|b| b.anchored(settings.anchor(), &located))
            };
            match (settings.adjustable, box_aspect) {
                (Adjustable::Box, _) => shrunk(settings.ratio.unwrap_or(1.0) * data_ratio),
                (Adjustable::Datalim, Some(box_aspect)) => shrunk(box_aspect),
                (Adjustable::Datalim, None) => Some(located),
            }
            .inspect(|position| {
                if let (Adjustable::Datalim, Some(ratio)) = (settings.adjustable, settings.ratio) {
                    let box_aspect = fig_aspect * (position.height() / position.width());
                    adjusted = self.adjust_datalim(axes, ratio, box_aspect);
                }
            })
        } else {
            None
        };
        let Some(active) = active else {
            return adjusted;
        };
        let in_host = hosted.map(|(boxes, to_figure)| {
            let in_host = to_figure
                .inverted()
                .map_or(active, |back| active.transformed(&back));
            (boxes, in_host)
        });
        if let Some(state) = self.store.get_as_mut::<AxesState>(axes) {
            // A colorbar's locator sets the box aspect of the axes it
            // places (matplotlib's `_ColorbarAxesLocator.__call__`).
            if let (Placement::Located, Some(placed)) = (
                placement,
                settings.locator.and_then(|locator| locator.box_aspect),
            ) {
                state.aspect.box_aspect = Some(placed);
            }
            match in_host {
                Some((boxes, in_host)) => {
                    state
                        .aspect
                        .set_active_in_figure(original, boxes, active, in_host);
                }
                None => state.aspect.set_active(original, active),
            }
        }
        adjusted
    }

    /// The `adjustable='datalim'` half of `apply_aspect`: widens the x or
    /// the y view interval about its centre to reach `box_aspect / ratio`,
    /// as matplotlib computes it (NaN where a scale cannot map the view);
    /// fails, leaving the limits alone, when they would not be finite.
    fn adjust_datalim(
        &mut self,
        axes: ArtistId,
        ratio: f64,
        box_aspect: f64,
    ) -> Result<(), AspectError> {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return Ok(());
        };
        let (xmin, xmax) = state.scaled_bound(AxisName::X);
        let (ymin, ymax) = state.scaled_bound(AxisName::Y);
        let xsize = py_max((xmax - xmin).abs(), 1e-30);
        let ysize = py_max((ymax - ymin).abs(), 1e-30);
        let data_ratio = box_aspect / ratio;
        let y_expander = data_ratio * xsize / ysize - 1.0;
        if y_expander.abs() < 0.005 {
            return Ok(());
        }
        let limits = state.data_lim.bbox;
        let (x0, x1) = (
            state.xaxis.forward(limits.x0()),
            state.xaxis.forward(limits.x1()),
        );
        let (y0, y1) = (
            state.yaxis.forward(limits.y0()),
            state.yaxis.forward(limits.y1()),
        );
        let xr = 1.05 * (x1 - x0);
        let yr = 1.05 * (y1 - y0);
        let xmarg = xsize - xr;
        let ymarg = ysize - yr;
        let ysize_target = data_ratio * xsize;
        let xsize_target = ysize / data_ratio;
        let shared_x = self.siblings(AxisName::X, axes).len() > 1;
        let shared_y = self.siblings(AxisName::Y, axes).len() > 1;
        if shared_x && shared_y {
            self.pending_errors.push(DrawError::new(
                axes,
                "set_aspect(..., adjustable='datalim') is not allowed when both axes are \
                 shared; use adjustable='box'",
            ));
            return Ok(());
        }
        let adjust_y = !shared_y && {
            let adjy = if xmarg > 0.0 && ymarg > 0.0 {
                (ysize_target - yr > 0.0 && y_expander < 0.0)
                    || (xsize_target - xr < 0.0 && y_expander > 0.0)
            } else {
                y_expander > 0.0
            };
            shared_x || adjy
        };
        let (name, centre, size) = if adjust_y {
            (AxisName::Y, f64::midpoint(ymin, ymax), ysize_target)
        } else {
            (AxisName::X, f64::midpoint(xmin, xmax), xsize_target)
        };
        let axis = state.axis(name);
        if !axis.autoscale.enabled {
            crate::warnings::log(
                "axes._base",
                &format!(
                    "Ignoring fixed {} limits to fulfill fixed data aspect with adjustable \
                     data limits.",
                    name.name()
                ),
            );
        }
        let inverse = |v: f64| axis.map().map_or(v, |m| m.inverse(v));
        let (low, high) = (inverse(centre - size / 2.0), inverse(centre + size / 2.0));
        if !(low.is_finite() && high.is_finite()) {
            return Err(AspectError::NonFiniteLimits);
        }
        let (v0, v1) = state.view_interval(name);
        let bound = if v0 > v1 { (high, low) } else { (low, high) };
        self.set_axis_limits(axes, name, bound, None, true);
        Ok(())
    }
}
