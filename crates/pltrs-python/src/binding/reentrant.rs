//! Changes made while their figure is in use, applied once it is free.
//!
//! pltrs (Rust) runs Python code while it lays a figure out or draws it:
//! a tick formatter or locator, a transform, a norm, a path effect, the
//! text measurer. That code may change the figure, as matplotlib's may
//! (a formatter setting its axes' title), and finds it in use
//! ([`pltrs::figure::reenter`]). A native function returning nothing (a
//! change: `set_color`, `set_text`, ...) called then is queued here, with
//! its arguments, and made when the thread's outermost native call ends,
//! once nothing of the thread uses the figure any more: matplotlib's
//! change, applied when the draw is done. Its exception, if any, is
//! raised from that call, as a callback's exception is. Any other native
//! function raises [`in_use_error`].
//!
//! A call made again with the same function and the same arguments (the
//! very same objects) before the queue is drained replaces the one queued
//! (a path effect called for each of ten thousand markers, setting the
//! same colour): the queue holds each change once, at its last place, and
//! work queuing itself again cannot grow it without end.
//!
//! The queue is drained whenever the thread's outermost native call ends,
//! so the thread-local storage holding it is empty whenever the thread
//! is outside native calls (see [`threads`](super::threads)).

use std::cell::RefCell;
use std::collections::HashMap;

use pltrs::figure::{Access, FigureInUse};

use super::callback::ErrorFrame;
use super::error::{ExceptionKind, PyErr};
use super::function::try_nested;
use super::gil::Python;
use super::object::{Bound, Owned};

thread_local! {
    /// The changes waiting for the thread's outermost native call to end.
    static QUEUE: RefCell<Queue> = RefCell::new(Queue::default());
}

/// A native call waiting for the figure it changes to be free: the
/// capsule of its function and its arguments.
#[derive(Clone)]
struct Deferred {
    capsule: Owned,
    args: Vec<Owned>,
}

impl Deferred {
    /// What tells the call apart: the addresses of its function's capsule
    /// and of its arguments, which the call keeps alive (so unique).
    fn key(&self) -> Vec<usize> {
        std::iter::once(&self.capsule)
            .chain(&self.args)
            .map(|object| object.as_ptr().addr())
            .collect()
    }
}

/// The queued calls, oldest first; a call queued again replaces its
/// earlier place (`None`).
#[derive(Default)]
struct Queue {
    calls: Vec<Option<Deferred>>,
    places: HashMap<Vec<usize>, usize>,
}

impl Queue {
    /// Queues `call`, returning the same call queued before, for the
    /// caller to drop once the queue is no longer borrowed (dropping
    /// Python objects may run Python code).
    fn push(&mut self, call: Deferred) -> Option<Deferred> {
        let earlier = self.places.insert(call.key(), self.calls.len());
        self.calls.push(Some(call));
        earlier.and_then(|place| self.calls[place].take())
    }

    /// The keys of the calls queued, in order.
    fn keys(&self) -> Vec<Vec<usize>> {
        self.calls.iter().flatten().map(Deferred::key).collect()
    }

    /// The calls queued, in order, leaving the queue empty.
    fn take(&mut self) -> Vec<Deferred> {
        self.places.clear();
        std::mem::take(&mut self.calls)
            .into_iter()
            .flatten()
            .collect()
    }
}

/// Queues the call of the native function in `capsule` with `args`, which
/// reached a figure in use.
pub(super) fn push(capsule: &Bound<'_>, args: &[Bound<'_>]) {
    let call = Deferred {
        capsule: capsule.clone().unbind(),
        args: args.iter().map(|arg| arg.clone().unbind()).collect(),
    };
    drop(QUEUE.with_borrow_mut(|queue| queue.push(call)));
}

/// How many times the queued changes are made, as long as making them
/// queues other changes, before the outermost call raises
/// `RecursionError`: matplotlib's draw recurses until Python's recursion
/// limit.
const MAX_ROUNDS: usize = 64;

/// Makes the queued changes, in the order they were made, as the
/// thread's outermost native call ends; returns the first exception one
/// raised. Changes queued meanwhile are made in turn, up to
/// [`MAX_ROUNDS`] times, unless making them queued exactly them again (a
/// `relim` running the program's transform, which calls `relim`): they
/// are done, as matplotlib's recursion ends where its caches answer. A
/// change still finding its figure in use (one another thread is
/// drawing) waits for the next outermost call to end.
pub(super) fn apply(py: Python<'_>) -> Option<PyErr> {
    let mut first = None;
    for _ in 0..MAX_ROUNDS {
        let queued = QUEUE.with_borrow_mut(Queue::take);
        if queued.is_empty() {
            return first;
        }
        // The calls stay alive for the whole round, so that no object
        // of a later call takes the address of one of theirs.
        let mut made = false;
        for call in &queued {
            let capsule = call.capsule.bind(py);
            let args: Vec<Bound<'_>> = call.args.iter().map(|arg| arg.bind(py)).collect();
            let frame = ErrorFrame::push();
            let outcome = match try_nested(&capsule, &args) {
                Ok(Err(_in_use)) => {
                    drop(QUEUE.with_borrow_mut(|queue| queue.push(call.clone())));
                    Ok(())
                }
                Ok(Ok(result)) => {
                    made = true;
                    result.map(drop)
                }
                Err(err) => Err(err),
            };
            if let Some(err) = frame.pop().or_else(|| outcome.err()) {
                first.get_or_insert(err);
            }
        }
        if !made {
            return first;
        }
        let made_keys: Vec<Vec<usize>> = queued.iter().map(Deferred::key).collect();
        if QUEUE.with_borrow(|queue| queue.keys() == made_keys) {
            drop(QUEUE.with_borrow_mut(Queue::take));
            return first;
        }
    }
    drop(QUEUE.with_borrow_mut(Queue::take));
    Some(first.unwrap_or_else(|| {
        PyErr::new(
            ExceptionKind::RecursionError,
            "maximum recursion depth exceeded while making the changes queued while a \
             figure was drawn",
        )
    }))
}

/// The exception of a native call that reached a figure in use and can
/// only run once it is free (it reads the figure, or returns what it
/// changed): `RuntimeError`.
pub(super) fn in_use_error(in_use: FigureInUse) -> PyErr {
    PyErr::runtime_error(match in_use.access() {
        Access::Read => {
            "this figure is being laid out or changed by pltrs, and the code it runs \
             meanwhile (a formatter, a locator, a transform...) cannot read it"
        }
        Access::Change => {
            "this figure is being drawn or laid out by pltrs, and the code it runs \
             meanwhile (a formatter, a path effect, a norm...) can only change it \
             through calls returning nothing, which are applied once it is done"
        }
    })
}

#[cfg(test)]
mod tests {
    use pltrs::axes::{Axes, TitleLocation};
    use pltrs::figure::Figure;
    use pltrs::ticker::{FormattedTicks, Formatter, TickContext, TickerError};

    use crate::binding::function::{IntoNative, function};
    use crate::binding::testing::{eval, extension, with_python};
    use crate::binding::{Bound, Callback, IntoPy, Owned, PyResult};

    use super::{Deferred, Queue};

    /// A formatter labelling ticks with numbers that runs a Python
    /// callable each time, while the figure is laid out.
    #[derive(Debug)]
    struct Running(Callback);

    impl Formatter for Running {
        fn format_ticks(
            &self,
            locs: &[f64],
            _ctx: &TickContext<'_>,
        ) -> Result<FormattedTicks, TickerError> {
            let _ = self.0.call::<Owned>(());
            Ok(FormattedTicks {
                labels: locs.iter().map(f64::to_string).collect(),
                offset: String::new(),
            })
        }

        fn format_data(&self, value: f64, _ctx: &TickContext<'_>) -> Result<String, TickerError> {
            Ok(value.to_string())
        }
    }

    /// Draws a figure whose formatter calls, through native functions, a
    /// change returning nothing (`retitle`), one returning a value
    /// (`retitled`) and a read (`title`) of its own axes; returns the
    /// outcomes the formatter saw and the title the axes has once the
    /// outermost scope ended.
    fn draw_reentering() -> (String, String) {
        let fig = Figure::with_size(2.0, 1.0, 50.0);
        let ax = fig.subplot();
        let seen = with_python(|py| -> PyResult<String> {
            // The native type of the axes' object.
            extension(py);
            let module = eval(py, "import types", "types.ModuleType('scratch')");
            let retitle = |ax: Axes, text: String| -> PyResult<()> {
                ax.set_title(text);
                Ok(())
            };
            let retitled = |ax: Axes, text: String| -> PyResult<String> {
                Ok(ax.set_title(text).text())
            };
            let title = |ax: Axes| -> PyResult<String> { Ok(titles(&ax)) };
            let natives = [
                function(py, &module, "retitle", retitle.into_native("retitle"))?,
                function(py, &module, "retitled", retitled.into_native("retitled"))?,
                function(py, &module, "title", title.into_native("title"))?,
            ];
            let (make, seen): (Bound<'_>, Bound<'_>) = eval(
                py,
                "def outcome(f, *args):\n    try:\n        return repr(f(*args))\n    except RuntimeError as e:\n        return 'RuntimeError: ' + str(e).split(',')[0]\n\nseen = []\ndef make(retitle, retitled, title, ax):\n    return lambda: seen.append((outcome(retitle, ax, 'deferred'), outcome(retitled, ax, 'refused'), outcome(title, ax)))",
                "(make, seen)",
            )
            .extract()?;
            let [retitle, retitled, title] = natives;
            let callable = make.call(&[retitle, retitled, title, ax.clone().into_py(py)?], &[])?;
            ax.xaxis().set_major_formatter(Running(callable.extract()?));
            fig.to_png().expect("draws");
            seen.call_method("__getitem__", &[0.into_py(py)?])?.repr()
        })
        .expect("the probe runs");
        (seen, titles(&ax))
    }

    /// The titles of `ax`, wherever the look places them.
    fn titles(ax: &Axes) -> String {
        [
            TitleLocation::Left,
            TitleLocation::Center,
            TitleLocation::Right,
        ]
        .map(|location| ax.title(location))
        .concat()
    }

    #[test]
    fn a_call_queued_again_takes_its_last_place() {
        with_python(|py| {
            let capsule = eval(py, "", "object()");
            let (a, b) = (eval(py, "", "[1]"), eval(py, "", "[2]"));
            let call = |arg: &Bound<'_>| Deferred {
                capsule: capsule.clone().unbind(),
                args: vec![arg.clone().unbind()],
            };
            let mut queue = Queue::default();
            assert!(queue.push(call(&a)).is_none());
            assert!(queue.push(call(&b)).is_none());
            assert!(
                queue.push(call(&a)).is_some(),
                "the earlier place is given back"
            );
            let order: Vec<_> = queue
                .take()
                .iter()
                .map(|call| call.args[0].as_ptr())
                .collect();
            assert_eq!(order, [b.as_ptr(), a.as_ptr()]);
            assert!(queue.take().is_empty() && queue.keys().is_empty());
        });
    }

    #[test]
    fn changes_returning_nothing_are_made_once_the_figure_is_free() {
        let (seen, title) = draw_reentering();
        // The layout borrows the figure for change: setting the title,
        // which reads the axes first, and reading it are both refused.
        let refused = "RuntimeError: this figure is being laid out or changed by pltrs";
        assert_eq!(seen, format!("('None', '{refused}', '{refused}')"));
        assert_eq!(title, "deferred");
    }
}
