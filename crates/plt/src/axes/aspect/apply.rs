//! Applying the aspect at layout time (matplotlib's `apply_aspect`): the
//! active position of an axes, or the view interval widened for
//! [`Adjustable::Datalim`], from its data ratio.

use super::Adjustable;
use crate::artist::{Artist, ArtistId, DrawError};
use crate::axes::AxesState;
use crate::figure::FigureState;
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
    /// axes).
    pub(crate) fn data_ratio(&self) -> f64 {
        if !self.projection.is_rectilinear() {
            return 1.0;
        }
        let (x0, x1) = self.scaled_bound(AxisName::X);
        let (y0, y1) = self.scaled_bound(AxisName::Y);
        let xsize = (x1 - x0).abs().max(1e-30);
        let ysize = (y1 - y0).abs().max(1e-30);
        ysize / xsize
    }
}

/// Where an axes is placed before its aspect is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Placement {
    /// Where its divider, inset locator or axes locator puts it.
    Located,
    /// At its original position, without calling its locators
    /// (matplotlib's `get_tightbbox(call_axes_locator=False)`, which
    /// sizes measuring the axes for its own divider use).
    Original,
}

impl FigureState {
    /// Brings one axes up to date for layout: a due layout-engine pass,
    /// pending autoscaling, then the aspect.
    pub(crate) fn lay_out_axes(&mut self, axes: ArtistId) {
        self.lay_out_axes_at(axes, Placement::Located);
    }

    /// [`lay_out_axes`](Self::lay_out_axes) with the axes placed by
    /// `placement`.
    pub(crate) fn lay_out_axes_at(&mut self, axes: ArtistId, placement: Placement) {
        self.run_pending_layout();
        self.unstale_axes(axes);
        self.apply_aspect_at(axes, placement);
    }

    /// matplotlib's `apply_aspect`: sets the active position of `axes`
    /// and, for [`Adjustable::Datalim`], widens one view interval.
    pub(crate) fn apply_aspect(&mut self, axes: ArtistId) {
        self.apply_aspect_at(axes, Placement::Located);
    }

    /// [`apply_aspect`](Self::apply_aspect) with the axes placed by
    /// `placement`.
    fn apply_aspect_at(&mut self, axes: ArtistId, placement: Placement) {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
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
        };
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let box_aspect = settings
            .locator
            .and_then(|locator| locator.box_aspect)
            .or(settings.box_aspect);
        let data_ratio = state.data_ratio();
        // Insets and secondary axes are placed in their host axes.
        let frame = state
            .base()
            .parent
            .and_then(|f| self.frame_bbox(f).or_else(|| self.axes_bbox(f)));
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
                    self.adjust_datalim(axes, ratio, box_aspect);
                }
            })
        } else {
            None
        };
        if let (Some(active), Some(state)) = (active, self.store.get_as_mut::<AxesState>(axes)) {
            state.aspect.active = Some((original, active));
        }
    }

    /// The `adjustable='datalim'` half of `apply_aspect`: widens the x or
    /// the y view interval about its centre to reach `box_aspect / ratio`.
    fn adjust_datalim(&mut self, axes: ArtistId, ratio: f64, box_aspect: f64) {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let (xmin, xmax) = state.scaled_bound(AxisName::X);
        let (ymin, ymax) = state.scaled_bound(AxisName::Y);
        let xsize = (xmax - xmin).abs().max(1e-30);
        let ysize = (ymax - ymin).abs().max(1e-30);
        let data_ratio = box_aspect / ratio;
        let y_expander = data_ratio * xsize / ysize - 1.0;
        if y_expander.abs() < 0.005 {
            return;
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
            self.pending_errors.push(DrawError {
                artist: axes,
                message: "set_aspect(..., adjustable='datalim') is not allowed when both axes \
                          are shared; use adjustable='box'"
                    .to_owned(),
            });
            return;
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
        let inverse = |v: f64| axis.map().map_or(v, |m| m.inverse(v));
        let (low, high) = (inverse(centre - size / 2.0), inverse(centre + size / 2.0));
        let (v0, v1) = state.view_interval(name);
        let bound = if v0 > v1 { (high, low) } else { (low, high) };
        self.set_axis_limits(axes, name, bound, None, true);
    }
}
