//! Callbacks run when the view limits of an axes change (matplotlib's
//! `ax.callbacks.connect("xlim_changed", ...)`, `"ylim_changed"`, and a
//! 3-D axes' `"zlim_changed"`).
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
use crate::mplot3d::{Axes3D, Axis3};

/// Identifies a limit callback, for [`Axes::disconnect_limits_changed`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LimitsCallbackId(u64);

/// A callback, given the axes whose limits changed, shared so that it can
/// run while the registry is borrowed for something else.
type Callback = Rc<RefCell<dyn FnMut(&RawHandle)>>;

/// The axis whose limits a callback waits for: one of a 2-D axes, or one
/// of the three of a 3-D axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LimitAxis {
    X,
    Y,
    Z,
}

impl From<AxisName> for LimitAxis {
    fn from(name: AxisName) -> Self {
        match name {
            AxisName::X => Self::X,
            AxisName::Y => Self::Y,
        }
    }
}

impl From<Axis3> for LimitAxis {
    fn from(axis: Axis3) -> Self {
        match axis {
            Axis3::X => Self::X,
            Axis3::Y => Self::Y,
            Axis3::Z => Self::Z,
        }
    }
}

/// The limit callbacks of one figure, and the changes waiting for them.
#[derive(Default)]
pub(crate) struct LimitCallbacks {
    next: u64,
    callbacks: Vec<(LimitsCallbackId, ArtistId, LimitAxis, Callback)>,
    pending: VecDeque<(ArtistId, LimitAxis)>,
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
    /// Takes out the callbacks of the axes `gone` (let go for good), for
    /// the caller to drop once the figure is free (what they hold may run
    /// the program's code as it goes).
    pub(crate) fn forget(&mut self, gone: &[ArtistId]) -> Vec<impl Sized + use<>> {
        self.pending.retain(|(axes, _)| !gone.contains(axes));
        let (forgotten, kept) = std::mem::take(&mut self.callbacks)
            .into_iter()
            .partition(|(_, axes, _, _)| gone.contains(axes));
        self.callbacks = kept;
        forgotten
    }

    /// Queues a change of axis `name` of `axes` when a callback waits for
    /// it.
    pub(crate) fn notify(&mut self, axes: ArtistId, name: impl Into<LimitAxis>) {
        let name = name.into();
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

    /// The callbacks waiting for a change of axis `name` of `axes`.
    fn callbacks_of(&self, axes: ArtistId, name: LimitAxis) -> Vec<Callback> {
        self.callbacks
            .iter()
            .filter(|(_, owner, axis, _)| *owner == axes && *axis == name)
            .map(|(_, _, _, callback)| Rc::clone(callback))
            .collect()
    }
}

/// The most limit changes one run of the callbacks handles: callbacks
/// whose work keeps changing the limits again (matplotlib's recurse until
/// Python's recursion limit) stop there, and the next draw reports it.
const MAX_CHANGES: usize = 1000;

/// Runs the callbacks of the limit changes queued in `figure`, oldest
/// first, up to [`MAX_CHANGES`] of them. The changes queued so far run as
/// one batch, and a run started by a callback (it changed or read the
/// limits) runs only the changes queued since: as matplotlib's callbacks
/// run as each change is made, a callback of the x limits an autoscale
/// changed finishes before the callbacks of the y limits it changed next,
/// and the changes a callback makes run inside it. A callback that is
/// already running (a change it made itself, or one a callback it
/// triggered made) is skipped, so callbacks that keep two axes in step
/// end.
pub(crate) fn run_limit_callbacks(figure: &Rc<FigureCell>) {
    let mut ran = 0;
    loop {
        let batch = std::mem::take(&mut figure.borrow_mut().limit_callbacks.pending);
        if batch.is_empty() {
            // Then those of the figures whose axes share the changed axis.
            super::share_across::run_peers_callbacks();
            return;
        }
        for (axes, name) in batch {
            if ran == MAX_CHANGES {
                report_endless_changes(figure, axes);
                return;
            }
            ran += 1;
            let callbacks = figure.borrow().limit_callbacks.callbacks_of(axes, name);
            let handle = RawHandle::new(Rc::clone(figure), axes);
            for callback in callbacks {
                if let Ok(mut callback) = callback.try_borrow_mut() {
                    callback(&handle);
                }
            }
        }
    }
}

/// Stops the limit changes of `figure` the callbacks keep making, the
/// change of `axes` being the first left, and reports it at the next draw.
fn report_endless_changes(figure: &Rc<FigureCell>, axes: ArtistId) {
    let mut state = figure.borrow_mut();
    state.limit_callbacks.pending.clear();
    state.pending_errors.push(DrawError::new(
        axes,
        format!(
            "the limit callbacks kept changing the limits: stopped after \
             {MAX_CHANGES} changes"
        ),
    ));
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
        mut callback: impl FnMut(&Axes) + 'static,
    ) -> LimitsCallbackId {
        connect(self.raw(), axis.into(), move |raw| {
            callback(&Axes::from_raw(raw.clone()));
        })
    }

    /// Disconnects a callback connected with
    /// [`on_limits_changed`](Self::on_limits_changed) (matplotlib's
    /// `ax.callbacks.disconnect`); returns whether it was connected.
    pub fn disconnect_limits_changed(&self, id: LimitsCallbackId) -> bool {
        disconnect(self.raw(), id)
    }
}

impl Axes3D {
    /// Calls `callback` with this axes whenever the view limits of `axis`
    /// change (matplotlib's `ax.callbacks.connect("zlim_changed", ...)`
    /// and those of the x and y limits): when they are set, on this axes
    /// or on one sharing the axis, and when autoscaling moves them, as
    /// [`Axes::on_limits_changed`] calls those of a 2-D axes.
    /// [`Axes3D::clear`] disconnects every callback of the axes.
    ///
    /// ```
    /// use std::cell::Cell;
    /// use std::rc::Rc;
    ///
    /// use pltrs::mplot3d::Axis3;
    /// use pltrs::prelude::*;
    ///
    /// let ax = Figure::new().subplot_3d();
    /// let seen = Rc::new(Cell::new(0));
    /// let record = Rc::clone(&seen);
    /// ax.on_limits_changed(Axis3::Z, move |_| record.set(record.get() + 1));
    /// ax.set_zlim(0.0, 4.0)?;
    /// ax.set_xlim(0.0, 1.0)?;
    /// assert_eq!(seen.get(), 1, "x limits do not call it");
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    pub fn on_limits_changed(
        &self,
        axis: Axis3,
        mut callback: impl FnMut(&Axes3D) + 'static,
    ) -> LimitsCallbackId {
        connect(self.raw(), axis.into(), move |raw| {
            callback(&Axes3D::from_raw(raw.clone()));
        })
    }

    /// Disconnects a callback connected with
    /// [`on_limits_changed`](Self::on_limits_changed); returns whether it
    /// was connected.
    pub fn disconnect_limits_changed(&self, id: LimitsCallbackId) -> bool {
        disconnect(self.raw(), id)
    }
}

/// Connects `callback` to the changes of axis `axis` of the axes `raw`.
fn connect(
    raw: &RawHandle,
    axis: LimitAxis,
    callback: impl FnMut(&RawHandle) + 'static,
) -> LimitsCallbackId {
    let figure = raw.figure_state();
    let mut state = figure.borrow_mut();
    let registry = &mut state.limit_callbacks;
    let id = LimitsCallbackId(registry.next);
    registry.next += 1;
    registry
        .callbacks
        .push((id, raw.id(), axis, Rc::new(RefCell::new(callback))));
    id
}

/// Disconnects the callback `id` of the figure of `raw`; whether it was
/// connected.
fn disconnect(raw: &RawHandle, id: LimitsCallbackId) -> bool {
    let figure = raw.figure_state();
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
    fn a_callback_reading_the_limits_ends_before_the_next_change_runs() {
        // matplotlib 3.11 runs the x callback of an autoscale before it
        // autoscales y, even when the x callback reads the limits.
        let ax = Figure::new().subplot();
        let order = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&order);
        ax.on_limits_changed(AxisName::X, move |ax| {
            ax.xlim();
            seen.borrow_mut().push("x");
        });
        let seen = Rc::clone(&order);
        ax.on_limits_changed(AxisName::Y, move |_| seen.borrow_mut().push("y"));
        ax.plot([0.0, 1.0], [0.0, 4.0]).expect("valid data");
        ax.ylim();
        assert_eq!(*order.borrow(), ["x", "y"]);
    }

    #[test]
    fn an_explicit_autoscale_runs_the_waiting_one_first() {
        // matplotlib 3.11's autoscale_view brings a stale view up to date
        // (calling back) before it sets the limits (calling back again).
        let ax = Figure::new().subplot();
        let (x, y) = (counter(&ax, AxisName::X), counter(&ax, AxisName::Y));
        ax.plot([0.0, 1.0], [0.0, 4.0]).expect("valid data");
        ax.autoscale_view(None);
        assert_eq!((x.get(), y.get()), (2, 2));
        ax.autoscale_view(None);
        assert_eq!((x.get(), y.get()), (3, 3), "nothing waits any more");
        ax.set_autoscale_on(false);
        ax.plot([0.0, 2.0], [0.0, 8.0]).expect("valid data");
        ax.autoscale_view(None);
        assert_eq!((x.get(), y.get()), (3, 3), "no axis follows the data");
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
