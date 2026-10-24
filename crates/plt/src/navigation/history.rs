//! The view history of a figure (matplotlib's `_nav_stack`, a
//! `cbook._Stack` of `{axes: (view, positions)}`).

use plt_core::Bbox;

use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxesState, AxisName};
use crate::colorbar::Colorbar;
use crate::figure::Figure;
use crate::mplot3d::{Axes3D, Axis3, ViewAngles};

/// What restores the view of one axes (matplotlib's `_get_view` plus its
/// original position).
#[derive(Clone, Debug, PartialEq)]
struct AxesView {
    xlim: (f64, f64),
    ylim: (f64, f64),
    autoscale: (bool, bool),
    position: Bbox,
    /// The norm limits of the colorbar drawn in the axes, which make up
    /// its view (matplotlib's `Colorbar._get_view`).
    colorbar: Option<(f64, f64)>,
}

impl AxesView {
    /// The current view of `ax`.
    fn of(ax: &Axes) -> Self {
        let (xlim, ylim) = (ax.xlim(), ax.ylim());
        let (autoscale, position) = ax.raw().with::<AxesState, _>(|state| {
            (
                (state.xaxis.autoscale.enabled, state.yaxis.autoscale.enabled),
                state.position,
            )
        });
        Self {
            xlim,
            ylim,
            autoscale,
            position,
            colorbar: Colorbar::in_axes(ax).and_then(|colorbar| colorbar.view()),
        }
    }

    /// Applies the view to `ax` (matplotlib's `_set_view`, which a
    /// colorbar replaces by setting its norm's limits, and
    /// `_set_position(..., 'original')`).
    fn apply(&self, ax: &Axes) {
        if let (Some(view), Some(colorbar)) = (self.colorbar, Colorbar::in_axes(ax)) {
            colorbar.set_view(view);
        } else {
            ax.set_xlim(self.xlim.0, self.xlim.1)
                .set_ylim(self.ylim.0, self.ylim.1);
            ax.raw().with_mut::<AxesState, _>(|state| {
                state.axis_mut(AxisName::X).autoscale.enabled = self.autoscale.0;
                state.axis_mut(AxisName::Y).autoscale.enabled = self.autoscale.1;
            });
        }
        ax.raw()
            .with_mut::<AxesState, _>(|state| state.position = self.position);
    }
}

/// What restores the view of one 3-D axes (matplotlib's `Axes3D._get_view`).
#[derive(Clone, Debug, PartialEq)]
struct Axes3DView {
    limits: [(f64, f64); 3],
    autoscale: [bool; 3],
    angles: ViewAngles,
}

impl Axes3DView {
    /// The current view of `ax`.
    fn of(ax: &Axes3D) -> Self {
        Self {
            limits: Axis3::ALL.map(|axis| ax.limits(axis)),
            autoscale: Axis3::ALL.map(|axis| ax.autoscale(axis)),
            angles: ax.view(),
        }
    }

    /// Applies the view to `ax` (matplotlib's `Axes3D._set_view`).
    fn apply(&self, ax: &Axes3D) {
        for ((axis, (lo, hi)), on) in Axis3::ALL.into_iter().zip(self.limits).zip(self.autoscale) {
            // Recorded limits are finite, so setting them cannot fail.
            let _ = ax.set_limits(axis, lo, hi);
            ax.set_autoscale(axis, on);
        }
        ax.view_init(self.angles.elev, self.angles.azim, self.angles.roll);
    }
}

/// One entry: the view of every axes of the figure.
#[derive(Clone, Debug, Default)]
struct Snapshot {
    axes: Vec<(Axes, AxesView)>,
    axes3d: Vec<(Axes3D, Axes3DView)>,
}

/// Snapshots with a movable cursor, like a browser's history.
#[derive(Debug, Default)]
pub(super) struct ViewStack {
    snapshots: Vec<Snapshot>,
    position: usize,
}

impl ViewStack {
    /// Whether nothing was recorded yet.
    pub(super) fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Forgets every snapshot.
    pub(super) fn clear(&mut self) {
        self.snapshots.clear();
        self.position = 0;
    }

    /// Records the current views after the cursor, discarding later ones.
    pub(super) fn push(&mut self, figure: &Figure) {
        let axes = figure
            .axes()
            .into_iter()
            .map(|ax| {
                let view = AxesView::of(&ax);
                (ax, view)
            })
            .collect();
        let axes3d = figure
            .axes3d()
            .into_iter()
            .map(|ax| {
                let view = Axes3DView::of(&ax);
                (ax, view)
            })
            .collect();
        self.insert(Snapshot { axes, axes3d });
    }

    fn insert(&mut self, snapshot: Snapshot) {
        self.snapshots.truncate(if self.is_empty() {
            0
        } else {
            self.position + 1
        });
        self.snapshots.push(snapshot);
        self.position = self.snapshots.len() - 1;
    }

    /// Records the first snapshot again (matplotlib's `_Stack.home`).
    pub(super) fn home(&mut self) {
        if let Some(first) = self.snapshots.first().cloned() {
            self.insert(first);
        }
    }

    /// Moves the cursor one snapshot back.
    pub(super) fn back(&mut self) {
        self.position = self.position.saturating_sub(1);
    }

    /// Moves the cursor one snapshot forward.
    pub(super) fn forward(&mut self) {
        self.position = (self.position + 1).min(self.snapshots.len().saturating_sub(1));
    }

    /// Whether the cursor can move back.
    pub(super) fn can_go_back(&self) -> bool {
        self.position > 0
    }

    /// Whether the cursor can move forward.
    pub(super) fn can_go_forward(&self) -> bool {
        self.position + 1 < self.snapshots.len()
    }

    /// Applies the snapshot at the cursor to the axes still alive
    /// (matplotlib's `_update_view`).
    pub(super) fn restore(&self, figure: &Figure) {
        let Some(snapshot) = self.snapshots.get(self.position) else {
            return;
        };
        for (ax, view) in &snapshot.axes {
            if ax.raw().is_alive() {
                view.apply(ax);
            }
        }
        for (ax, view) in &snapshot.axes3d {
            if ax.raw().is_alive() {
                view.apply(ax);
            }
        }
        figure.layout();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_behaves_like_a_browser() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let mut stack = ViewStack::default();
        assert!(stack.is_empty());
        stack.push(&fig);
        ax.set_xlim(0.0, 2.0);
        stack.push(&fig);
        ax.set_xlim(0.0, 3.0);
        stack.push(&fig);
        stack.back();
        stack.restore(&fig);
        assert_eq!(ax.xlim(), (0.0, 2.0));
        assert!(stack.can_go_forward());
        ax.set_xlim(5.0, 6.0);
        stack.push(&fig);
        assert!(!stack.can_go_forward(), "a push discards later views");
        stack.home();
        stack.restore(&fig);
        assert_eq!(ax.xlim(), (0.0, 1.0));
        stack.back();
        stack.restore(&fig);
        assert_eq!(ax.xlim(), (5.0, 6.0));
    }

    #[test]
    fn three_d_views_are_recorded() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let mut stack = ViewStack::default();
        stack.push(&fig);
        ax.view_init(10.0, 20.0, 30.0).set_zlim(0.0, 5.0).unwrap();
        stack.push(&fig);
        stack.back();
        stack.restore(&fig);
        assert_eq!(ax.view(), ViewAngles::default());
        assert!(ax.autoscale(Axis3::Z), "autoscaling comes back too");
        stack.forward();
        stack.restore(&fig);
        assert_eq!((ax.view().roll, ax.zlim()), (30.0, (0.0, 5.0)));
    }
}
