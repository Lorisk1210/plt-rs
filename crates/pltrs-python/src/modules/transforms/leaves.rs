//! The leaves of matplotlib's transform trees (`pltrs::transforms::TreeLeaves`):
//! read once when Python builds a figure's or an axes' trees, then
//! followed. Python registers the figures whose trees it built
//! ([`watch`]) and the subfigures and axes among them ([`watch_frame`],
//! [`watch_axes`]); at the end of every native call ([`sync_watched`], an
//! after-call hook) the watched figures a handle borrowed for change
//! meanwhile (`pltrs::figure::take_changed_figures`) are read again, and
//! the leaves that changed go to the figure's Python `sync`, which sets
//! them as matplotlib sets its own (invalidating what is built on them).
//! A call that changed no watched figure costs the hook next to nothing.
//!
//! Leaves cross as tuples: a figure's as `(dpi, (width, height), frames,
//! axes)`, a subfigure's as `(id, (x0, y0, x1, y1))` (`bbox_relative`), an
//! axes' as `(id, (position, original), (view, view_sets), (data, minpos),
//! (xscale, yscale), projection)`, boxes as `(x0, y0, x1, y1)`,
//! `view_sets` counting how often the view limits were set, the
//! projection as
//! `None`, `("polar", theta_offset, direction, rorigin, rlabel_position)`
//! or `("geo", longitude_cap)`.

use std::cell::{Cell, RefCell};

use pltrs::artist::{ArtistHandle, ArtistId, RawHandle};
use pltrs::axes::Axes;
use pltrs::figure::{Figure, WeakFigure, take_changed_figures};
use pltrs::transforms::{AxesLeaves, Bbox, FrameLeaves, ProjectionLeaves, TreeLeaves};

use crate::binding::prelude::*;

/// A figure whose trees Python built.
struct Watched {
    figure: WeakFigure,
    /// The subfigures and axes with trees.
    frames: Vec<ArtistId>,
    axes: Vec<ArtistId>,
    /// Whether the leaves are to be read again (they may have changed).
    pending: bool,
    /// The leaves read last.
    last: Option<TreeLeaves>,
    /// The Python function setting changed leaves.
    sync: Owned,
}

pltrs_core::thread_state! {
    /// The figures watched (by every thread: see `binding::threads`).
    static WATCHED: RefCell<Vec<Watched>> = const { RefCell::new(Vec::new()) };
    /// Whether an entry of [`WATCHED`] is `pending`.
    static PENDING: Cell<bool> = const { Cell::new(false) };
}

thread_local! {
    /// Whether this thread is handing leaves to Python, whose native calls
    /// must not hand them again meanwhile.
    static SYNCING: Cell<bool> = const { Cell::new(false) };
}

/// A box as `(x0, y0, x1, y1)`.
fn extents(bbox: Bbox) -> (f64, f64, f64, f64) {
    (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())
}

/// A subfigure's leaf as Python takes it.
struct PyFrame(FrameLeaves);

impl IntoPy for PyFrame {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        (self.0.id.index(), extents(self.0.relative)).into_py(py)
    }
}

/// An axes' leaves as Python takes them.
struct PyAxes(AxesLeaves);

impl IntoPy for PyAxes {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let leaves = self.0;
        let projection = match leaves.projection {
            ProjectionLeaves::Plain => None::<Bound<'_>>.into_py(py)?,
            ProjectionLeaves::Polar {
                theta_offset,
                direction,
                rorigin,
                rlabel_position,
            } => ("polar", theta_offset, direction, rorigin, rlabel_position).into_py(py)?,
            ProjectionLeaves::Geo { longitude_cap } => ("geo", longitude_cap).into_py(py)?,
        };
        let [xscale, yscale] = leaves.scales.map(|name| name.to_string());
        (
            leaves.id.index(),
            (extents(leaves.position), extents(leaves.original)),
            (extents(leaves.view), leaves.view_sets),
            (
                extents(leaves.data.bbox),
                (leaves.data.minpos.x, leaves.data.minpos.y),
            ),
            (xscale, yscale),
            projection,
        )
            .into_py(py)
    }
}

/// A figure's leaves as Python takes them.
struct PyLeaves(TreeLeaves);

impl IntoPy for PyLeaves {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let leaves = self.0;
        (
            leaves.dpi,
            leaves.size_inches,
            leaves.frames.into_iter().map(PyFrame).collect::<Vec<_>>(),
            leaves.axes.into_iter().map(PyAxes).collect::<Vec<_>>(),
        )
            .into_py(py)
    }
}

/// The leaves of `figure`, of its subfigures `frames` and of its axes
/// `axes` as they are now (a `RuntimeError` while the figure is being
/// laid out or drawn, when it cannot be read).
fn tree_leaves(figure: Figure, frames: Vec<RawHandle>, axes: Vec<Axes>) -> PyResult<PyLeaves> {
    let frames: Vec<ArtistId> = frames.iter().map(RawHandle::id).collect();
    let axes: Vec<ArtistId> = axes.iter().map(ArtistHandle::id).collect();
    figure
        .tree_leaves(&frames, &axes)
        .map(PyLeaves)
        .ok_or_else(|| {
            PyErr::runtime_error(
                "the figure's transforms cannot be built while it is laid out or drawn",
            )
        })
}

/// The entry of `figure` in `watched`, if it is watched.
fn entry<'a>(watched: &'a mut [Watched], figure: &Figure) -> Option<&'a mut Watched> {
    watched.iter_mut().find(|entry| entry.figure.is_of(figure))
}

/// Has `entry` read again at the end of the next native call.
fn mark_pending(entry: &mut Watched) {
    entry.pending = true;
    PENDING.set(true);
}

/// Takes the entries of the figures dropped out of `watched`, to be
/// dropped once [`WATCHED`] is no longer borrowed: dropping their Python
/// functions may run Python code, and other threads' native calls
/// meanwhile.
fn take_dropped(watched: &mut Vec<Watched>) -> Vec<Watched> {
    watched
        .extract_if(.., |entry| !entry.figure.is_alive())
        .collect()
}

/// Follows the leaves of the root `figure`: `sync((dpi, size, frames,
/// axes))` gets those that changed at the end of every native call that
/// changed them.
fn watch(figure: Figure, sync: Owned) -> PyResult<()> {
    figure.watch_changes();
    let replaced = WATCHED.with_borrow_mut(|watched| {
        let dropped = take_dropped(watched);
        let replaced = if let Some(entry) = entry(watched, &figure) {
            mark_pending(entry);
            Some(std::mem::replace(&mut entry.sync, sync))
        } else {
            watched.push(Watched {
                figure: figure.downgrade(),
                frames: Vec::new(),
                axes: Vec::new(),
                pending: true,
                last: None,
                sync,
            });
            PENDING.set(true);
            None
        };
        (dropped, replaced)
    });
    drop(replaced);
    Ok(())
}

/// Adds `id` to the ids of `figure`'s entry chosen by `ids`, once the
/// figure is watched; its leaves are compared at the end of the next
/// native call (Python read them before it called this).
fn add_watched(figure: &Figure, id: ArtistId, ids: fn(&mut Watched) -> &mut Vec<ArtistId>) {
    WATCHED.with(|watched| {
        if let Some(entry) = entry(&mut watched.borrow_mut(), figure) {
            mark_pending(entry);
            let ids = ids(entry);
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    });
}

/// Follows the leaf of the subfigure `frame` too.
fn watch_frame(frame: Figure) -> PyResult<()> {
    add_watched(&frame.figure(), frame.id(), |entry| &mut entry.frames);
    Ok(())
}

/// Follows the leaves of `axes` too (of its projection plane, for 3-D
/// axes); returns the id its leaves come with.
fn watch_axes(axes: Axes) -> PyResult<u32> {
    add_watched(&axes.figure(), axes.id(), |entry| &mut entry.axes);
    Ok(axes.id().index())
}

/// The leaves of `now` that differ from `before` (all of them without
/// `before`); `None` when none differ.
fn changes(now: &TreeLeaves, before: Option<&TreeLeaves>) -> Option<TreeLeaves> {
    let Some(before) = before else {
        return Some(now.clone());
    };
    if now == before {
        return None;
    }
    Some(TreeLeaves {
        dpi: now.dpi,
        size_inches: now.size_inches,
        frames: (now.frames.iter())
            .filter(|frame| !before.frames.contains(frame))
            .copied()
            .collect(),
        axes: (now.axes.iter())
            .filter(|axes| !before.axes.contains(axes))
            .cloned()
            .collect(),
    })
}

/// Marks the leaves as being handed to Python until dropped.
struct Syncing;

impl Syncing {
    fn start() -> Option<Self> {
        (!SYNCING.replace(true)).then_some(Self)
    }
}

impl Drop for Syncing {
    fn drop(&mut self) {
        SYNCING.set(false);
    }
}

/// The work of every native call's end: hands the `sync` of each watched
/// figure that may have changed the leaves that changed since it was last
/// called (see the module documentation). A figure being drawn is looked
/// at again at the end of a later call.
pub(crate) fn sync_watched(py: Python<'_>) -> PyResult<()> {
    let Some(_syncing) = Syncing::start() else {
        return Ok(());
    };
    let changed = take_changed_figures();
    if changed.is_empty() && !PENDING.get() {
        return Ok(());
    }
    let (due, dropped) = WATCHED.with_borrow_mut(|watched| {
        for figure in &changed {
            if let Some(entry) = entry(watched, figure) {
                entry.pending = true;
            }
        }
        let dropped = take_dropped(watched);
        let mut due = Vec::new();
        let mut still_pending = false;
        for entry in watched.iter_mut().filter(|entry| entry.pending) {
            let read = (entry.figure.upgrade())
                .and_then(|figure| figure.tree_leaves(&entry.frames, &entry.axes));
            let Some(now) = read else {
                still_pending = true;
                continue;
            };
            entry.pending = false;
            if let Some(changed) = changes(&now, entry.last.as_ref()) {
                due.push((entry.sync.clone_ref(py), changed));
            }
            entry.last = Some(now);
        }
        PENDING.set(still_pending);
        (due, dropped)
    });
    drop(dropped);
    for (sync, changed) in due {
        let leaves = PyLeaves(changed);
        let args = leaves.into_py(py)?;
        sync.bind(py).call(&[args], &[])?;
    }
    Ok(())
}

/// Hands the watched leaves of `figure` that changed to its `sync` now
/// (before Python code runs in the middle of a draw, which reads the
/// trees of the laid-out figure).
fn sync(figure: Figure) -> PyResult<()> {
    WATCHED.with(|watched| {
        if let Some(entry) = entry(&mut watched.borrow_mut(), &figure) {
            mark_pending(entry);
        }
    });
    Python::with_gil(sync_watched)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("tree_leaves", tree_leaves)?;
    module.add("watch", watch)?;
    module.add("watch_frame", watch_frame)?;
    module.add("watch_axes", watch_axes)?;
    module.add("sync", sync)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, with_python};

    #[test]
    fn only_the_leaves_that_changed_are_handed_on() {
        let figure = Figure::with_size(4.0, 3.0, 50.0);
        let first = figure.add_axes([0.1, 0.1, 0.5, 0.5]).unwrap();
        let second = figure.add_axes([0.6, 0.6, 0.3, 0.3]).unwrap();
        let read = || figure.tree_leaves(&[], &[first.id(), second.id()]).unwrap();
        let before = read();
        assert_eq!(changes(&before, None), Some(before.clone()), "all at first");
        assert_eq!(changes(&before, Some(&before)), None);
        second.set_xlim(0.0, 10.0).unwrap();
        let changed = changes(&read(), Some(&before)).expect("changed");
        assert_eq!(changed.axes.len(), 1);
        assert_eq!(changed.axes[0].id, second.id());
        assert_eq!(changed.axes[0].view.interval_x(), (0.0, 10.0));
    }

    /// Records every call in `calls`.
    const RECORDER: &str = "calls = []\ndef record(leaves):\n    calls.append(leaves)\n";

    /// A figure's leaves as Python gets them.
    type Leaves<'py> = (f64, (f64, f64), Vec<Bound<'py>>, Vec<Bound<'py>>);

    /// An axes' leaves as Python gets them.
    type AxesTuple<'py> = (
        u32,
        Bound<'py>,
        ((f64, f64, f64, f64), u32),
        Bound<'py>,
        (String, String),
        Option<Bound<'py>>,
    );

    #[test]
    fn watched_figures_hand_their_changes_to_python() {
        with_python(|py| {
            let figure = Figure::with_size(4.0, 3.0, 50.0);
            let axes = figure.add_axes([0.1, 0.1, 0.5, 0.5]).unwrap();
            let record = eval(py, RECORDER, "record");
            let record_calls = record.getattr("__globals__").unwrap();
            let calls_of = || {
                record_calls
                    .call_method("__getitem__", &["calls".into_py(py).unwrap()])
                    .unwrap()
            };
            let count = || -> usize {
                calls_of()
                    .call_method("__len__", &[])
                    .unwrap()
                    .extract()
                    .unwrap()
            };
            let call = |index: usize| {
                calls_of()
                    .call_method("__getitem__", &[index.into_py(py).unwrap()])
                    .unwrap()
            };
            watch(figure.clone(), record.clone().unbind()).unwrap();
            watch_axes(axes.clone()).unwrap();
            sync_watched(py).unwrap();
            assert_eq!(count(), 1, "the leaves read first");
            sync_watched(py).unwrap();
            assert_eq!(count(), 1, "nothing changed since");
            figure.set_dpi(100.0);
            sync_watched(py).unwrap();
            assert_eq!(count(), 2);
            let (dpi, size, frames, changed): Leaves<'_> = call(1).extract().unwrap();
            assert_eq!((dpi, size), (100.0, (4.0, 3.0)));
            assert!(
                frames.is_empty() && changed.is_empty(),
                "the axes did not change"
            );
            axes.set_ylim(-1.0, 1.0).unwrap();
            sync_watched(py).unwrap();
            let (_, _, _, changed): Leaves<'_> = call(2).extract().unwrap();
            let (id, _, (view, view_sets), _, scales, projection): AxesTuple<'_> =
                changed[0].extract().unwrap();
            assert_eq!(id, axes.id().index());
            assert_eq!((view.1, view.3), (-1.0, 1.0));
            assert!(view_sets > 0, "the limits were set");
            assert_eq!(scales, ("linear".to_owned(), "linear".to_owned()));
            assert!(projection.is_none());
            drop(figure);
            drop(axes);
            let other = Figure::new();
            watch(other.clone(), record.clone().unbind()).unwrap();
            assert_eq!(
                WATCHED.with(|watched| watched.borrow().len()),
                1,
                "the figure dropped is forgotten"
            );
        });
    }
}
