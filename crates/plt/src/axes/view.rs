//! View limits across axes that share an axis: lazy autoscaling, limit
//! propagation and joining share groups (matplotlib's `_unstale_viewLim`,
//! `autoscale_view`, `Axis._set_lim` and `sharex`/`sharey`).

use std::rc::Rc;

use super::PlotError;
use super::axis::{AxisRole, AxisState};
use super::limits::oriented;
use super::state::{AxesState, SharedData};
use crate::artist::{ArtistId, DrawError};
use crate::figure::FigureState;
use crate::scale::Scale;
use crate::ticker::AxisName;
use crate::transforms::TransformContext;

/// Both axis names, in matplotlib's order.
const AXES: [AxisName; 2] = [AxisName::X, AxisName::Y];

impl FigureState {
    fn axes_state(&self, id: ArtistId) -> Option<&AxesState> {
        self.store.get_as::<AxesState>(id)
    }

    fn axes_state_mut(&mut self, id: ArtistId) -> Option<&mut AxesState> {
        self.store.get_as_mut::<AxesState>(id)
    }

    /// The axes sharing axis `name` with `axes`, `axes` included.
    pub(crate) fn siblings(&self, name: AxisName, axes: ArtistId) -> Vec<ArtistId> {
        self.shared.siblings(name, axes)
    }

    /// For the x and the y axis of `axes`, how many ticks fit along the
    /// axis of another axes its locators are bound to, if they are
    /// (matplotlib's `locator.axis.get_tick_space()` for locators shared
    /// with a sibling of another size).
    pub(crate) fn shared_tick_spaces(&self, axes: ArtistId) -> [Option<usize>; 2] {
        AXES.map(|name| {
            let owner = self
                .axes_state(axes)?
                .axis(name)
                .ticker_owner
                .filter(|&owner| owner != axes)?;
            let extent = self.axes_bbox(owner)?;
            let pixels = match name {
                AxisName::X => extent.width(),
                AxisName::Y => extent.height(),
            };
            let points = pixels.abs() / self.dpi * 72.0;
            Some(self.axes_state(owner)?.axis(name).tick_space(points))
        })
    }

    /// Applies `update` to axis `name` of `axes` and of every axes sharing
    /// it (for state matplotlib shares between them, such as tickers).
    pub(crate) fn update_shared_axis(
        &mut self,
        axes: ArtistId,
        name: AxisName,
        mut update: impl FnMut(&mut AxisState),
    ) {
        for id in self.siblings(name, axes) {
            if let Some(state) = self.axes_state_mut(id) {
                update(state.axis_mut(name));
            }
        }
        self.stale = true;
    }

    /// Installs `scale` on axis `name` of `axes` and its siblings, each
    /// with fresh default tick helpers (matplotlib's `_set_axes_scale`).
    pub(crate) fn set_shared_scale(
        &mut self,
        axes: ArtistId,
        name: AxisName,
        scale: &Rc<dyn Scale>,
    ) {
        let linear_only = self.axes_state(axes).and_then(|state| {
            if state.geo().is_some() {
                Some("the axes of geographic projections")
            } else {
                (state.axis(name).role == AxisRole::Theta)
                    .then_some("the angular axis of polar axes")
            }
        });
        if let Some(axis) = linear_only.filter(|_| scale.name() != "linear") {
            self.pending_errors.push(DrawError {
                artist: axes,
                message: format!("{axis} must stay linear, not {:?}", scale.name()),
            });
            return;
        }
        self.update_shared_axis(axes, name, |axis| {
            axis.set_scale(Rc::clone(scale));
            axis.autoscale.stale = true;
        });
    }

    /// Recomputes the stale autoscaled view limits of `axes` (matplotlib's
    /// `_unstale_viewLim`): an axis is rescaled when it or any axes
    /// sharing it changed, from the data of all of them.
    pub(crate) fn unstale_axes(&mut self, axes: ArtistId) {
        let need = AXES.map(|name| {
            self.siblings(name, axes).into_iter().any(|id| {
                self.axes_state(id)
                    .is_some_and(|state| state.axis(name).autoscale.stale)
            })
        });
        for (name, _) in AXES.into_iter().zip(need).filter(|(_, need)| *need) {
            self.update_shared_axis(axes, name, |axis| axis.autoscale.stale = false);
        }
        for (name, _) in AXES.into_iter().zip(need).filter(|(_, need)| *need) {
            self.autoscale_axis(axes, name);
        }
    }

    /// matplotlib's `autoscale_view` for one axis of `axes`.
    fn autoscale_axis(&mut self, axes: ArtistId, name: AxisName) {
        let Some(state) = self.axes_state(axes) else {
            return;
        };
        if !state.axis(name).autoscale.enabled {
            return;
        }
        let shared = self.shared_data(axes, name, state.uses_stickies(name));
        let inverted = {
            let (v0, v1) = state.view_interval(name);
            v0 > v1
        };
        match state.autoscale_bound(name, &shared) {
            Ok(Some(bound)) => {
                let (v0, v1) = oriented(bound, inverted);
                self.set_axis_limits(axes, name, (v0, v1), None, true);
            }
            Ok(None) => {}
            Err(err) => self.pending_errors.push(DrawError {
                artist: axes,
                message: err.to_string(),
            }),
        }
    }

    /// The data, smallest positive datum and (if `stickies`) the sticky
    /// edges of every axes sharing axis `name` with `axes`.
    fn shared_data(&self, axes: ArtistId, name: AxisName, stickies: bool) -> SharedData {
        let siblings: Vec<&AxesState> = self
            .siblings(name, axes)
            .into_iter()
            .filter_map(|id| self.axes_state(id))
            .collect();
        let finite: Vec<f64> = siblings
            .iter()
            .flat_map(|state| {
                let ((low, high), _) = state.data_interval(name);
                [low, high]
            })
            .filter(|v| v.is_finite())
            .collect();
        let interval = (!finite.is_empty()).then(|| {
            (
                finite.iter().copied().fold(f64::INFINITY, f64::min),
                finite.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            )
        });
        let minpos = siblings
            .iter()
            .map(|state| state.data_interval(name).1)
            .fold(f64::INFINITY, f64::min);
        let mut edges: Vec<f64> = if stickies {
            siblings
                .iter()
                .flat_map(|state| &state.children)
                .filter_map(|&id| self.store.get(id))
                .flat_map(|child| {
                    let (x, y) = &child.base().sticky_edges;
                    match name {
                        AxisName::X => x.clone(),
                        AxisName::Y => y.clone(),
                    }
                })
                .chain(
                    siblings
                        .iter()
                        .flat_map(|state| state.axis(name).sticky_edges().iter().copied()),
                )
                .collect()
        } else {
            Vec::new()
        };
        edges.sort_by(f64::total_cmp);
        SharedData {
            interval,
            minpos,
            stickies: edges,
        }
    }

    /// Sets the view limits of axis `name` of `axes` (matplotlib's
    /// `Axis._set_lim`): validates them, marks the axis and its siblings
    /// as autoscaled, switches autoscaling to `auto` when given and, with
    /// `emit`, applies the same limits to every sibling. The fixed limits
    /// of geographic axes refuse to change (matplotlib raises).
    pub(crate) fn set_axis_limits(
        &mut self,
        axes: ArtistId,
        name: AxisName,
        (v0, v1): (f64, f64),
        auto: Option<bool>,
        emit: bool,
    ) {
        if let Some(state) = self.axes_state(axes).filter(|state| state.geo().is_some()) {
            if state.view_interval(name) != (v0, v1) {
                self.pending_errors.push(DrawError {
                    artist: axes,
                    message: "changing the limits of geographic axes is not supported".into(),
                });
            }
            return;
        }
        let Some(state) = self.axes_state_mut(axes) else {
            return;
        };
        let limits = state.validated_limits(name, v0, v1);
        state.set_view_interval(name, limits);
        if let Some(auto) = auto {
            state.axis_mut(name).autoscale.enabled = auto;
        }
        self.update_shared_axis(axes, name, |axis| axis.autoscale.stale = false);
        if emit {
            for sibling in self.siblings(name, axes) {
                if sibling != axes {
                    self.set_axis_limits(sibling, name, limits, auto, false);
                }
            }
        }
    }

    /// Makes `axes` share axis `name` with `other` (matplotlib's `sharex`
    /// / `sharey`): it joins `other`'s group and takes over its tick
    /// helpers, view limits, autoscaling state and scale.
    pub(crate) fn share_axis(
        &mut self,
        name: AxisName,
        axes: ArtistId,
        other: ArtistId,
    ) -> Result<(), PlotError> {
        let current = self.axes_state(axes).and_then(|state| match name {
            AxisName::X => state.shares.0,
            AxisName::Y => state.shares.1,
        });
        if current.is_some_and(|current| current != other) {
            return Err(PlotError::AlreadyShared(name));
        }
        self.unstale_axes(other);
        let Some(source) = self.axes_state(other) else {
            return Ok(());
        };
        let tickers = source.axis(name).tickers();
        let owner = source.axis(name).ticker_owner.unwrap_or(other);
        let scale = source.axis(name).scale_rc();
        let view = source.view_interval(name);
        let auto = source.axis(name).autoscale.enabled;
        self.shared.join(name, axes, other);
        if let Some(state) = self.axes_state_mut(axes) {
            match name {
                AxisName::X => state.shares.0 = Some(other),
                AxisName::Y => state.shares.1 = Some(other),
            }
            state.axis_mut(name).use_tickers(tickers);
            state.axis_mut(name).ticker_owner = Some(owner);
        }
        self.set_axis_limits(axes, name, view, Some(auto), false);
        if let Some(state) = self.axes_state_mut(axes) {
            state.axis_mut(name).adopt_scale(scale);
        }
        self.stale = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::axes::{AxisName, PlotError};
    use crate::figure::{Figure, Share};
    use crate::prelude::*;
    use crate::ticker::MultipleLocator;

    fn pair() -> (Figure, Axes, Axes) {
        let fig = Figure::new();
        let a = fig.add_subplot(2, 1, 1).unwrap();
        let b = fig.add_subplot(2, 1, 2).unwrap();
        b.share_x(&a).unwrap();
        (fig, a, b)
    }

    #[test]
    fn autoscaling_uses_the_data_of_all_siblings() {
        let (_fig, a, b) = pair();
        a.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        b.plot([3.0, 4.0], [5.0, 6.0]).unwrap();
        assert_eq!(a.xlim(), (-0.2, 4.2));
        assert_eq!(b.xlim(), (-0.2, 4.2));
        // y is not shared.
        assert_eq!(a.ylim(), (-0.05, 1.05));
    }

    #[test]
    fn limits_and_autoscale_state_propagate() {
        let (_fig, a, b) = pair();
        b.set_xlim(2.0, 3.0);
        assert_eq!(a.xlim(), (2.0, 3.0));
        a.plot([0.0, 10.0], [0.0, 1.0]).unwrap();
        assert_eq!(
            b.xlim(),
            (2.0, 3.0),
            "explicit limits switch autoscaling off"
        );
        a.set_xlim(5.0, 4.0);
        assert!(b.xaxis().inverted());
    }

    #[test]
    fn singular_and_invalid_log_limits_are_fixed_up() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(1.0, 1.0);
        let (low, high) = ax.xlim();
        assert!(low < 1.0 && high > 1.0);
        ax.set_yscale("log").unwrap();
        ax.set_ylim(10.0, 100.0);
        ax.set_ylim(-1.0, 1000.0);
        assert_eq!(ax.ylim(), (10.0, 1000.0));
    }

    #[test]
    fn scales_and_tickers_are_shared() {
        let (_fig, a, b) = pair();
        b.set_xscale("log").unwrap();
        assert_eq!(a.xscale(), "log");
        a.set_xscale("linear").unwrap();
        a.set_xlim(0.0, 10.0);
        b.xaxis()
            .set_major_locator(MultipleLocator::new(5.0).unwrap());
        let locs = |ax: &Axes| -> Vec<f64> {
            ax.ticks(AxisName::X)
                .unwrap()
                .major
                .iter()
                .map(|t| t.loc)
                .collect()
        };
        assert_eq!(locs(&a), [0.0, 5.0, 10.0]);
        assert_eq!(locs(&b), [0.0, 5.0, 10.0]);
    }

    #[test]
    fn sharing_errors() {
        let (fig, a, b) = pair();
        let c = fig.add_axes([0.1, 0.1, 0.2, 0.2]);
        c.share_x(&a).unwrap();
        assert!(
            c.share_x(&a).is_ok(),
            "re-sharing with the same axes is fine"
        );
        assert_eq!(
            c.share_x(&b).err(),
            Some(PlotError::AlreadyShared(AxisName::X))
        );
        let other = Figure::new().subplot();
        assert_eq!(a.share_y(&other).err(), Some(PlotError::ForeignAxes));
    }

    #[test]
    fn removed_axes_leave_their_group() {
        let (_fig, a, b) = pair();
        b.remove();
        a.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        assert_eq!(a.xlim(), (-0.05, 1.05));
    }

    #[test]
    fn subplot_grids_share_by_row_and_column() {
        let fig = Figure::new();
        let axs = fig.subplots_shared(2, 2, Share::Col, Share::Row).unwrap();
        axs[(1, 1)].plot([0.0, 8.0], [0.0, 4.0]).unwrap();
        assert_eq!(axs[(0, 1)].xlim(), (-0.4, 8.4));
        assert_eq!(axs[(0, 0)].xlim(), (0.0, 1.0), "column 0 has no data");
        assert_eq!(axs[(1, 0)].ylim(), (-0.2, 4.2));
    }

    #[test]
    fn shared_locators_measure_the_axis_they_are_bound_to() {
        let fig = Figure::with_size(6.0, 4.0, 100.0);
        let wide = fig.add_axes([0.1, 0.55, 0.8, 0.4]);
        let narrow = fig.add_axes([0.1, 0.1, 0.1, 0.4]);
        narrow.share_x(&wide).unwrap();
        wide.set_xlim(0.0, 24.0);
        let locs = |ax: &Axes| -> Vec<f64> {
            ax.ticks(AxisName::X)
                .unwrap()
                .major
                .iter()
                .map(|t| t.loc)
                .collect()
        };
        fig.layout();
        assert_eq!(locs(&narrow), locs(&wide), "ticks for the wide axis");
        narrow
            .xaxis()
            .set_major_locator(MultipleLocator::new(12.0).unwrap());
        fig.layout();
        assert_eq!(locs(&wide), [0.0, 12.0, 24.0]);
    }
}
