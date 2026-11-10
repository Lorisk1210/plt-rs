//! View limits across axes that share an axis: lazy autoscaling, limit
//! propagation and joining share groups (matplotlib's `_unstale_viewLim`,
//! `autoscale_view`, `Axis._set_lim` and `sharex`/`sharey`).

use std::rc::Rc;

use super::PlotError;
use super::axis::{AxisRole, AxisState};
use super::limits::oriented;
use super::state::{AxesState, AxisSpace, SharedData};
use super::tick_compute::axis_length_points;
use crate::artist::{ArtistId, ArtistLookup, DrawError, DrawErrorKind};
use crate::figure::FigureState;
use crate::layout::failures;
use crate::scale::{Scale, ScaleName};
use crate::ticker::{AxisName, TickSpace};
use crate::transforms::TransformContext;

/// Both axis names, in matplotlib's order.
const AXES: [AxisName; 2] = [AxisName::X, AxisName::Y];

impl FigureState {
    /// Axes `id` while it is in the figure, which autoscaling reads.
    fn axes_state(&self, id: ArtistId) -> Option<&AxesState> {
        self.store.get_as::<AxesState>(id)
    }

    /// Axes `id`, in the figure or removed from it, to change: limits,
    /// scales and tick helpers set on a removed axes read back, though
    /// the figure no longer draws it.
    fn axes_state_mut(&mut self, id: ArtistId) -> Option<&mut AxesState> {
        self.artist_as_mut::<AxesState>(id)
    }

    /// The axes sharing axis `name` with `axes`, `axes` included.
    pub(crate) fn siblings(&self, name: AxisName, axes: ArtistId) -> Vec<ArtistId> {
        self.shared.siblings(name, axes)
    }

    /// For the x and the y axis of `axes`, how many ticks fit along the
    /// axis of another axes its locators are bound to, if they are
    /// (matplotlib's `locator.axis.get_tick_space()` for locators shared
    /// with a sibling of another size).
    pub(crate) fn shared_tick_spaces(&self, axes: ArtistId) -> [Option<TickSpace>; 2] {
        AXES.map(|name| {
            let owner = self
                .axes_state(axes)?
                .axis(name)
                .ticker_owner
                .filter(|&owner| owner != axes)?;
            let points = self.axis_length_points(owner, name)?;
            Some(self.axes_state(owner)?.axis(name).tick_space(points))
        })
    }

    /// The length of axis `name` of `axes` in points, once the axes has a
    /// box (see [`axis_length_points`]).
    pub(crate) fn axis_length_points(&self, axes: ArtistId, name: AxisName) -> Option<f64> {
        Some(axis_length_points(&self.axes_bbox(axes)?, self.dpi(), name))
    }

    /// Applies `update` to axis `name` of `axes` and of every axes sharing
    /// it (for state matplotlib shares between them, such as tickers).
    /// The 3-D axes sharing it take its tick helpers and scale as they are
    /// then (see [`lend_flat_tickers`](Self::lend_flat_tickers)).
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
        self.lend_flat_tickers(axes, name);
        self.stale = true;
    }

    /// Calls off the autoscaling axis `name` of `axes` and of every axes
    /// sharing it, of either kind, waits for (matplotlib's
    /// `ax._stale_viewlims[name] = False` for every sibling); the figure
    /// is to be drawn again.
    fn settle_shared_axis(&mut self, axes: ArtistId, name: AxisName) {
        for id in self.siblings(name, axes) {
            if let Some(state) = self.axes_state_mut(id) {
                state.axis_mut(name).autoscale.stale = false;
            }
            self.settle_deep_axis(id, name);
        }
        self.stale = true;
    }

    /// Installs `scale` on axis `name` of `axes` and its siblings, each
    /// with fresh default tick helpers (matplotlib's `_set_axes_scale`).
    /// When the new locator's default limits differ, `axes` autoscales
    /// that axis now, while the scale's locator is still in place.
    pub(crate) fn set_shared_scale(
        &mut self,
        axes: ArtistId,
        name: AxisName,
        scale: &Rc<dyn Scale>,
    ) {
        let linear_only = self.axes_state(axes).and_then(|state| {
            if state.projection.linear_only() {
                Some(format!(
                    "the axes of {} projections",
                    projection_noun(state)
                ))
            } else {
                (state.axis(name).role == AxisRole::Theta)
                    .then(|| "the angular axis of polar axes".to_owned())
            }
        });
        if let Some(axis) = linear_only.filter(|_| scale.name() != ScaleName::Linear) {
            self.pending_errors.push(DrawError::new(
                axes,
                format!("{axis} must stay linear, not {:?}", scale.name().as_str()),
            ));
            return;
        }
        let default_limits = |state: &Self| {
            state
                .axes_state(axes)
                .map(|a| a.nonsingular(name, f64::NEG_INFINITY, f64::INFINITY))
        };
        let before = default_limits(self);
        self.update_shared_axis(axes, name, |axis| axis.set_scale(Rc::clone(scale)));
        if default_limits(self) != before {
            // matplotlib's `autoscale_view` along this axis, whose setting
            // of the limits brings any other axis waiting to be rescaled
            // up to date too.
            if let Some(state) = self.axes_state_mut(axes) {
                state.request_autoscale(name == AxisName::X, name == AxisName::Y);
            }
            self.unstale_axes(axes);
        }
    }

    /// Recomputes the stale autoscaled view limits of `axes` (matplotlib's
    /// `_unstale_viewLim`): an axis is rescaled when it or any axes
    /// sharing it changed, from the data of all of them.
    pub(crate) fn unstale_axes(&mut self, axes: ArtistId) {
        let need = AXES.map(|name| self.autoscale_is_stale(axes, name));
        for (name, _) in AXES.into_iter().zip(need).filter(|(_, need)| *need) {
            self.settle_shared_axis(axes, name);
        }
        // matplotlib's `autoscale_view` stops at what the program's code
        // (a locator's `nonsingular`) raises, leaving the views as they
        // are.
        let failed_before = failures::failed();
        for (name, _) in AXES.into_iter().zip(need).filter(|(_, need)| *need) {
            if !failed_before && failures::failed() {
                break;
            }
            self.autoscale_axis(axes, name);
        }
    }

    /// Which axes of `axes`, `[x, y]`, wait to be rescaled (or one sharing
    /// them does), which then no longer wait: what matplotlib's
    /// `_unstale_viewLim` takes before it calls `autoscale_view`.
    pub(crate) fn take_stale_axes(&mut self, axes: ArtistId) -> [bool; 2] {
        let stale = self.stale_axes(axes);
        for (name, _) in AXES.into_iter().zip(stale).filter(|(_, stale)| *stale) {
            self.settle_shared_axis(axes, name);
        }
        stale
    }

    /// Which axes of `axes`, `[x, y]`, wait to be rescaled (see
    /// [`FigureState::take_stale_axes`]), left waiting.
    pub(crate) fn stale_axes(&self, axes: ArtistId) -> [bool; 2] {
        AXES.map(|name| self.autoscale_is_stale(axes, name))
    }

    /// Whether [`FigureState::unstale_axes`] has anything to do for
    /// `axes`: an axis to rescale, or limit changes whose callbacks have
    /// yet to run. Reading the view limits of a figure being drawn then
    /// needs no change to it.
    pub(crate) fn view_is_stale(&self, axes: ArtistId) -> bool {
        AXES.into_iter()
            .any(|name| self.autoscale_is_stale(axes, name))
            || self.limit_callbacks.has_pending()
    }

    /// Whether axis `name` of `axes`, or of an axes of either kind
    /// sharing it, changed since it was last autoscaled.
    fn autoscale_is_stale(&self, axes: ArtistId, name: AxisName) -> bool {
        self.siblings(name, axes).into_iter().any(|id| {
            self.deep_axis_pending(id, name).unwrap_or_else(|| {
                self.axes_state(id)
                    .is_some_and(|state| state.axis(name).autoscale.stale)
            })
        })
    }

    /// matplotlib's `autoscale_view` for one axis of `axes`.
    fn autoscale_axis(&mut self, axes: ArtistId, name: AxisName) {
        let Some(state) = self.axes_state(axes) else {
            return;
        };
        if !state.axis(name).autoscale.enabled {
            return;
        }
        // matplotlib's `set_xbound` asks the axis whether it is inverted.
        let asked = self.program_inverted(axes, name);
        let Some(state) = self.axes_state(axes) else {
            return;
        };
        let shared = self.shared_data(axes, name, state.uses_stickies(name));
        let inverted = asked.unwrap_or_else(|| {
            let (v0, v1) = state.view_interval(name);
            v0 > v1
        });
        // Round-number limits depend on how many ticks fit along the axis.
        let [x_space, y_space] = self.shared_tick_spaces(axes);
        let space = AxisSpace {
            length_points: self.axis_length_points(axes, name).unwrap_or(0.0),
            tick_space: match name {
                AxisName::X => x_space,
                AxisName::Y => y_space,
            },
        };
        let failed_before = failures::failed();
        let bound = if state.axis(name).calls_program() {
            // The program's locator and scale run with the figure lent, as
            // matplotlib's `autoscale_view` lets them read and change it;
            // the axes is held apart from the figure meanwhile.
            let held = self.store.shared_as::<AxesState>(axes);
            held.and_then(|state| self.lend(|_| state.autoscale_bound(name, &shared, space)))
                .unwrap_or_else(|| {
                    self.axes_state(axes).map_or(Ok(None), |state| {
                        state.autoscale_bound(name, &shared, space)
                    })
                })
        } else {
            state.autoscale_bound(name, &shared, space)
        };
        match bound {
            Ok(Some(_)) if !failed_before && failures::failed() => {}
            Ok(Some(bound)) => {
                let (v0, v1) = oriented(bound, inverted);
                self.set_axis_limits(axes, name, (v0, v1), None, true);
            }
            Ok(None) => {}
            Err(err) => self
                .pending_errors
                .push(DrawError::new(axes, err.to_string())),
        }
    }

    /// The data, smallest positive datum and (if `stickies`) the sticky
    /// edges of every axes sharing axis `name` with `axes` (those of a 3-D
    /// one its projection plane's, see [`flat_part`](Self::flat_part)).
    fn shared_data(&self, axes: ArtistId, name: AxisName, stickies: bool) -> SharedData {
        let siblings: Vec<&AxesState> = self
            .siblings(name, axes)
            .into_iter()
            .filter_map(|id| self.flat_part(id))
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

    /// Sets the view limits of axis `name` of `axes` through the axes
    /// (matplotlib's `Axes.set_xlim` and `set_xbound`, which autoscaling,
    /// the aspect and images call), as [`set_axis_view`](Self::set_axis_view)
    /// does. The fixed limits of geographic axes, and of custom projections
    /// fixing theirs, refuse to change: matplotlib's `set_xlim` raises
    /// there, which is reported when the figure is drawn.
    pub(crate) fn set_axis_limits(
        &mut self,
        axes: ArtistId,
        name: AxisName,
        (v0, v1): (f64, f64),
        auto: Option<bool>,
        emit: bool,
    ) {
        if let Some(state) = self
            .axes_state(axes)
            .filter(|state| state.projection.fixed_limits())
        {
            if state.view_interval(name) != (v0, v1) {
                let noun = projection_noun(state);
                let mut message =
                    format!("Changing axes limits of a {noun} projection is not supported.");
                if state.geo().is_some() {
                    message.push_str("  Please consider using Cartopy.");
                }
                self.pending_errors.push(DrawError {
                    kind: DrawErrorKind::InvalidType,
                    ..DrawError::new(axes, message)
                });
            }
            return;
        }
        self.set_axis_view(axes, name, (v0, v1), auto, emit);
    }

    /// Sets the view limits of axis `name` of `axes` (matplotlib's
    /// `Axis._set_lim`, which fixed limits do not refuse): validates them,
    /// marks the axis and its siblings as autoscaled, switches autoscaling
    /// to `auto` when given and, with `emit`, applies the same limits to
    /// every sibling and queues the limit callbacks of all of them
    /// (`xlim_changed`).
    pub(crate) fn set_axis_view(
        &mut self,
        axes: ArtistId,
        name: AxisName,
        (v0, v1): (f64, f64),
        auto: Option<bool>,
        emit: bool,
    ) {
        let Some(state) = self.axes_state_mut(axes) else {
            return;
        };
        let limits = state.validated_limits(name, v0, v1);
        state.set_view_interval(name, limits);
        if let Some(auto) = auto {
            state.axis_mut(name).autoscale.enabled = auto;
        }
        self.settle_shared_axis(axes, name);
        if emit {
            // The limit callbacks, matplotlib's parasites' `_sync_lims`
            // among them.
            self.limit_callbacks.notify(axes, name);
            self.sync_parasites(axes);
            for sibling in self.siblings(name, axes) {
                if sibling == axes || self.set_deep_view(sibling, name, limits, auto) {
                    continue;
                }
                self.set_axis_view(sibling, name, limits, auto, false);
                self.limit_callbacks.notify(sibling, name);
                self.sync_parasites(sibling);
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
        // Either may be an axes made but not added yet (`Axes(fig, rect,
        // sharex=other)`), which shares as one in the figure does.
        let current = self
            .artist_as::<AxesState>(axes)
            .and_then(|state| match name {
                AxisName::X => state.shares.0,
                AxisName::Y => state.shares.1,
            });
        if current.is_some_and(|current| current != other) {
            return Err(PlotError::AlreadyShared(name));
        }
        // A 3-D axes gives its limits as they are (`Axes3D.get_xlim`).
        if self.deep_axis_pending(other, name).is_none() {
            self.unstale_axes(other);
        }
        let Some((tickers, owner, scale, view, auto)) = self.shared_axis_of(other, name) else {
            return Ok(());
        };
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

/// How errors name the axes of `state`: geographic, or by the name of
/// their projection.
fn projection_noun(state: &AxesState) -> &'static str {
    if state.geo().is_some() {
        "geographic"
    } else {
        state.projection.kind().name()
    }
}

#[cfg(test)]
mod tests {
    use crate::axes::{AxisName, PlotError};
    use crate::figure::{Figure, Share};
    use crate::prelude::*;
    use crate::scale::ScaleName;
    use crate::ticker::MultipleLocator;

    fn pair() -> (Figure, Axes, Axes) {
        let fig = Figure::new();
        let a = fig.add_subplot(2, 1, 1).unwrap();
        let b = fig.add_subplot(2, 1, 2).unwrap();
        b.share_x(&a).unwrap();
        (fig, a, b)
    }

    /// A locator of the program's whose `nonsingular` fails, as the
    /// program's code tells pltrs through [`failures`](crate::layout::failures).
    #[derive(Debug)]
    struct FailingLocator;

    thread_local! {
        static FAILED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    struct Program;

    impl crate::layout::failures::Failures for Program {
        fn failed(&self) -> bool {
            FAILED.get()
        }

        fn pass_over(&self, _class: crate::layout::failures::ErrorClass) -> bool {
            false
        }
    }

    impl crate::ticker::Locator for FailingLocator {
        fn tick_values(
            &self,
            _vmin: f64,
            _vmax: f64,
            _ctx: &crate::ticker::TickContext<'_>,
        ) -> Result<Vec<f64>, crate::ticker::TickerError> {
            Ok(Vec::new())
        }

        fn nonsingular(
            &self,
            v0: f64,
            v1: f64,
            _ctx: &crate::ticker::TickContext<'_>,
        ) -> (f64, f64) {
            FAILED.set(true);
            (v0, v1)
        }
    }

    #[test]
    fn a_failing_locator_leaves_the_views_as_they_are() {
        crate::layout::failures::set_failures(Program);
        let ax = Figure::new().subplot();
        ax.plot([0.0, 10.0], [0.0, 5.0]).unwrap();
        ax.xaxis().set_major_locator(FailingLocator);
        assert_eq!(ax.xlim(), (0.0, 1.0), "autoscaling stopped at x");
        assert_eq!(ax.ylim(), (0.0, 1.0), "and never reached y");
        crate::layout::failures::clear_failures();
        FAILED.set(false);
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
        b.set_xlim(2.0, 3.0).expect("finite limits");
        assert_eq!(a.xlim(), (2.0, 3.0));
        a.plot([0.0, 10.0], [0.0, 1.0]).unwrap();
        assert_eq!(
            b.xlim(),
            (2.0, 3.0),
            "explicit limits switch autoscaling off"
        );
        a.set_xlim(5.0, 4.0).expect("finite limits");
        assert!(b.xaxis().inverted());
    }

    #[test]
    fn singular_and_invalid_log_limits_are_fixed_up() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(1.0, 1.0).expect("finite limits");
        let (low, high) = ax.xlim();
        assert!(low < 1.0 && high > 1.0);
        ax.set_yscale("log").unwrap();
        ax.set_ylim(10.0, 100.0).expect("finite limits");
        ax.set_ylim(-1.0, 1000.0).expect("finite limits");
        assert_eq!(ax.ylim(), (10.0, 1000.0));
    }

    #[test]
    fn scales_and_tickers_are_shared() {
        let (_fig, a, b) = pair();
        b.set_xscale("log").unwrap();
        assert_eq!(a.xscale(), ScaleName::Log);
        a.set_xscale("linear").unwrap();
        a.set_xlim(0.0, 10.0).expect("finite limits");
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
        let c = fig
            .add_axes([0.1, 0.1, 0.2, 0.2])
            .expect("a valid rectangle");
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
    fn axes_made_apart_share_before_they_are_added() {
        // matplotlib's `Axes(fig, rect, sharex=a)` then `fig.add_axes(...)`,
        // as axes_grid1's grids make their axes.
        let fig = Figure::new();
        let a = fig.removed_axes([0.1, 0.1, 0.3, 0.3]).unwrap();
        let b = fig.removed_axes([0.5, 0.1, 0.3, 0.3]).unwrap();
        b.share_x(&a).unwrap();
        fig.attach(&a).unwrap();
        fig.attach(&b).unwrap();
        b.xaxis()
            .set_major_locator(MultipleLocator::new(0.5).unwrap());
        let locs: Vec<f64> = a
            .ticks(AxisName::X)
            .unwrap()
            .major
            .iter()
            .map(|t| t.loc)
            .collect();
        assert_eq!(locs, [0.0, 0.5, 1.0]);
        assert_eq!(a.shared_axes(AxisName::X).len(), 2);
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
        let wide = fig
            .add_axes([0.1, 0.55, 0.8, 0.4])
            .expect("a valid rectangle");
        let narrow = fig
            .add_axes([0.1, 0.1, 0.1, 0.4])
            .expect("a valid rectangle");
        narrow.share_x(&wide).unwrap();
        wide.set_xlim(0.0, 24.0).expect("finite limits");
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
