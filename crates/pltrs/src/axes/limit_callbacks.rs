//! Callbacks run when the view limits of an axes change (matplotlib's
//! `ax.callbacks.connect("xlim_changed", ...)` and `"ylim_changed"`).
//!
//! Every limit change that matplotlib's `Axis._set_lim(..., emit=True)`
//! makes queues the axes it changed, and the axes sharing the axis, when a
//! callback waits for them; the queue runs as soon as the figure is free
//! again: after `set_xlim` and friends, after autoscaling when the limits
//! are read, and after the layout pass of a draw.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;
use std::rc::Rc;

use super::{Axes, AxisName};
use crate::artist::{ArtistHandle, ArtistId, DrawError, RawHandle};
use crate::figure::FigureCell;

/// Identifies a limit callback, for [`Axes::disconnect_limits_changed`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LimitsCallbackId(u64);

/// A callback, shared so that it can run while the registry is borrowed
/// for something else.
type Callback = Rc<RefCell<dyn FnMut(&Axes)>>;

/// The limit callbacks of one figure, and the changes waiting for them.
#[derive(Default)]
pub(crate) struct LimitCallbacks {
    next: u64,
    callbacks: Vec<(LimitsCallbackId, ArtistId, AxisName, Callback)>,
    pending: VecDeque<(ArtistId, AxisName)>,
}

impl fmt::Debug for LimitCallbacks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LimitCallbacks")
            .field("connected", &self.callbacks.len())
            .field("pending", &self.pending.len())
            .finish_non_exhaustive()
    }
}

impl LimitCallbacks {
    /// Queues a change of axis `name` of `axes` when a callback waits for
    /// it.
    pub(crate) fn notify(&mut self, axes: ArtistId, name: AxisName) {
        if self
            .callbacks
            .iter()
            .any(|(_, owner, axis, _)| *owner == axes && *axis == name)
        {
            self.pending.push_back((axes, name));
        }
    }

    /// Forgets the callbacks of `axes` (matplotlib's `clear` makes a new
    /// registry), returning them for the caller to drop once the figure is
    /// no longer borrowed.
    pub(crate) fn take_axes(&mut self, axes: ArtistId) -> Vec<Callback> {
        let (taken, kept) = std::mem::take(&mut self.callbacks)
            .into_iter()
            .partition(|(_, owner, _, _)| *owner == axes);
        self.callbacks = kept;
        taken
            .into_iter()
            .map(|(_, _, _, callback)| callback)
            .collect()
    }

    /// Whether limit changes wait for their callbacks to run.
    pub(crate) fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// The oldest queued change, with the callbacks waiting for it.
    fn next_change(&mut self) -> Option<(ArtistId, Vec<Callback>)> {
        let (axes, name) = self.pending.pop_front()?;
        let callbacks = self
            .callbacks
            .iter()
            .filter(|(_, owner, axis, _)| *owner == axes && *axis == name)
            .map(|(_, _, _, callback)| Rc::clone(callback))
            .collect();
        Some((axes, callbacks))
    }
}

/// The most limit changes one run of the callbacks handles: callbacks
/// whose work keeps changing the limits again (matplotlib's recurse until
/// Python's recursion limit) stop there, and the next draw reports it.
const MAX_CHANGES: usize = 1000;

/// Runs the callbacks of the limit changes queued in `figure`, oldest
/// first, up to [`MAX_CHANGES`] of them. A callback that is already
/// running (a change it made itself, or one a callback it triggered made)
/// is skipped, so callbacks that keep two axes in step end.
pub(crate) fn run_limit_callbacks(figure: &Rc<FigureCell>) {
    for _ in 0..MAX_CHANGES {
        let Some((axes, callbacks)) = figure.borrow_mut().limit_callbacks.next_change() else {
            return;
        };
        let handle = Axes::from_raw(RawHandle::new(Rc::clone(figure), axes));
        for callback in callbacks {
            if let Ok(mut callback) = callback.try_borrow_mut() {
                callback(&handle);
            }
        }
    }
    let mut state = figure.borrow_mut();
    if let Some((axes, _)) = state.limit_callbacks.pending.pop_front() {
        state.limit_callbacks.pending.clear();
        state.pending_errors.push(DrawError::new(
            axes,
            format!(
                "the limit callbacks kept changing the limits: stopped after \
                 {MAX_CHANGES} changes"
            ),
        ));
    }
}

impl Axes {
    /// Calls `callback` with this axes whenever the view limits of `axis`
    /// change (matplotlib's `ax.callbacks.connect("xlim_changed", ...)`):
    /// when they are set, on this axes or on one sharing the axis, when
    /// autoscaling moves them, and when the view is panned or zoomed.
    /// Callbacks run once the change is complete, in the order they were
    /// connected; a change a callback makes runs the other callbacks but
    /// not itself again. [`Axes::clear`] disconnects every callback of the
    /// axes, as matplotlib's does.
    ///
    /// ```
    /// use std::cell::Cell;
    /// use std::rc::Rc;
    ///
    /// use pltrs::axes::AxisName;
    /// use pltrs::prelude::*;
    ///
    /// let ax = Figure::new().subplot();
    /// let seen = Rc::new(Cell::new((0.0, 0.0)));
    /// let record = Rc::clone(&seen);
    /// let id = ax.on_limits_changed(AxisName::X, move |ax| record.set(ax.xlim()));
    /// ax.set_xlim(2.0, 5.0)?;
    /// assert_eq!(seen.get(), (2.0, 5.0));
    /// ax.set_ylim(0.0, 9.0)?;
    /// assert_eq!(seen.get(), (2.0, 5.0), "y limits do not call it");
    /// assert!(ax.disconnect_limits_changed(id));
    /// ax.set_xlim(0.0, 1.0)?;
    /// assert_eq!(seen.get(), (2.0, 5.0));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn on_limits_changed(
        &self,
        axis: AxisName,
        callback: impl FnMut(&Axes) + 'static,
    ) -> LimitsCallbackId {
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let registry = &mut state.limit_callbacks;
        let id = LimitsCallbackId(registry.next);
        registry.next += 1;
        registry
            .callbacks
            .push((id, self.id(), axis, Rc::new(RefCell::new(callback))));
        id
    }

    /// Disconnects a callback connected with
    /// [`on_limits_changed`](Self::on_limits_changed) (matplotlib's
    /// `ax.callbacks.disconnect`); returns whether it was connected.
    pub fn disconnect_limits_changed(&self, id: LimitsCallbackId) -> bool {
        let figure = self.raw().figure_state();
        let removed = {
            let mut state = figure.borrow_mut();
            let callbacks = &mut state.limit_callbacks.callbacks;
            callbacks
                .iter()
                .position(|(cid, ..)| *cid == id)
                .map(|index| callbacks.remove(index))
        };
        removed.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::figure::Figure;

    /// A callback counting its calls on `axis` of `ax`.
    fn counter(ax: &Axes, axis: AxisName) -> Rc<Cell<usize>> {
        let count = Rc::new(Cell::new(0));
        let seen = Rc::clone(&count);
        ax.on_limits_changed(axis, move |_| seen.set(seen.get() + 1));
        count
    }

    #[test]
    fn shared_axes_run_their_callbacks_too() {
        let fig = Figure::new();
        let grid = fig
            .subplots_shared(1, 2, crate::figure::Share::All, crate::figure::Share::None)
            .expect("a 1 × 2 grid");
        let (left, right) = (grid.as_slice()[0].clone(), grid.as_slice()[1].clone());
        let (left_x, right_x, right_y) = (
            counter(&left, AxisName::X),
            counter(&right, AxisName::X),
            counter(&right, AxisName::Y),
        );
        left.set_xlim(0.0, 3.0).expect("finite limits");
        assert_eq!((left_x.get(), right_x.get(), right_y.get()), (1, 1, 0));
        assert_eq!(right.xlim(), (0.0, 3.0));
    }

    #[test]
    fn autoscaling_runs_the_callbacks_when_the_limits_are_read() {
        let ax = Figure::new().subplot();
        let count = counter(&ax, AxisName::Y);
        ax.plot([0.0, 1.0], [0.0, 4.0]).expect("valid data");
        assert_eq!(count.get(), 0, "autoscaling waits");
        ax.ylim();
        assert_eq!(count.get(), 1);
        ax.ylim();
        assert_eq!(count.get(), 1, "nothing changed since");
    }

    #[test]
    fn callbacks_keeping_axes_in_step_end() {
        let fig = Figure::new();
        let (a, b) = (
            fig.add_subplot(1, 2, 1).expect("cell"),
            fig.add_subplot(1, 2, 2).expect("cell"),
        );
        let follow = |from: &Axes, to: &Axes| {
            let to = to.clone();
            from.on_limits_changed(AxisName::X, move |ax| {
                let (low, high) = ax.xlim();
                to.set_xlim(low, high).expect("finite limits");
            });
        };
        follow(&a, &b);
        follow(&b, &a);
        a.set_xlim(1.0, 2.0).expect("finite limits");
        assert_eq!(b.xlim(), (1.0, 2.0));
        b.set_xlim(3.0, 4.0).expect("finite limits");
        assert_eq!(a.xlim(), (3.0, 4.0));
    }

    #[test]
    fn callbacks_changing_the_limits_without_end_stop() {
        // Found by fuzzing: each change a callback made queued another,
        // and the callbacks ran forever.
        let fig = Figure::new();
        let ax = fig.subplot();
        let calls = Rc::new(Cell::new(0_u32));
        let (seen, again) = (Rc::clone(&calls), ax.clone());
        ax.on_limits_changed(AxisName::X, move |_| {
            seen.set(seen.get() + 1);
            // A change left for later, as a layout run from the callback
            // leaves the limits it changes (a pick event's hit test).
            let mut state = again.raw().figure_state().borrow_mut();
            state.limit_callbacks.notify(again.id(), AxisName::X);
        });
        ax.set_xlim(0.0, 1.0).expect("finite limits");
        assert_eq!(calls.get(), u32::try_from(MAX_CHANGES).expect("small"));
        assert!(fig.to_png().is_err(), "the next draw reports it");
        assert!(fig.to_png().is_ok(), "once");
    }

    #[test]
    fn clearing_disconnects_the_callbacks_of_the_axes() {
        let ax = Figure::new().subplot();
        let count = counter(&ax, AxisName::X);
        ax.clear();
        ax.set_xlim(0.0, 2.0).expect("finite limits");
        assert_eq!(count.get(), 0);
    }
}
