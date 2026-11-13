//! Threads: any thread holding the GIL may use any pltrs object, as any
//! may use matplotlib's.
//!
//! pltrs (Rust) is single-threaded code. Figures share their state
//! through `Rc`s and `RefCell`s, and what matplotlib keeps in
//! process-wide variables (rcParams, the registries of colormaps and
//! projections, the handlers of warnings, the program's text measurer)
//! pltrs (Rust) keeps per thread (`pltrs_core::state`). Python runs one
//! thread at a time, the one holding the GIL, and that is what lets its
//! threads share pltrs (Rust) values:
//!
//! * While a thread holds a GIL scope ([`GilScope`]: a native call, a
//!   slot of the native type, [`Python::with_gil`]), it works with the
//!   interpreter's [`State`], made once, instead of its own: rcParams,
//!   registries and handlers are the same for every Python thread, as
//!   matplotlib's globals are (`rc_context` in one thread is seen by the
//!   others).
//! * Native objects hold their values for every thread
//!   ([`native`](super::native)), but for the few tied to the thread that
//!   made them, pltrs (Rust)'s windows, whose calls from other threads are
//!   run by that thread ([`home`](super::home)).
//! * What pltrs (Rust) keeps per thread across native calls, a renderer's
//!   metrics cache that `text._measuring_with` installs around several
//!   calls, is parked in the shared state while the thread is outside
//!   native calls ([`leave`], [`enter`]), so that a thread's own
//!   thread-local storage, which is dropped without the GIL when the
//!   thread ends, never holds a value another thread can reach.
//!
//! What every thread works with is only touched with the GIL held, which
//! orders all those touches: the GIL is the lock of pltrs (Rust)'s state.
//! The binding never releases it while pltrs (Rust) works (only waits
//! that touch nothing release it: [`home`](super::home)'s, and the
//! windows' event loop's, [`blocking`](super::blocking)), so a native
//! call that does not call back into Python runs as if alone. One that
//! does (a draw calling the program's formatter, or any Python code a
//! garbage collection runs) may see other threads run meanwhile, as
//! Python switches threads in Python code; what it borrows stays borrowed
//! until it returns. So shared state is never borrowed across a call into
//! Python, and a thread's outermost native call on an artist whose figure
//! another thread's call is using waits for it ([`wait_for_figures`]),
//! where matplotlib, which documents it is not made for concurrent use,
//! would change the figure in the middle of the other thread's draw.
//!
//! [`GilScope`]: super::gil::GilScope
//! [`Python::with_gil`]: super::gil::Python::with_gil

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use pltrs::artist::RawHandle;
use pltrs::text::{MetricsCache, set_metrics_cache};
use pltrs_core::state::{self, State};

use super::gil::without_gil;
use super::native::artist_of;
use crate::ffi;

/// The longest a native call waits for another thread's call to be done
/// with a figure.
const WAIT_FOR_FIGURES: Duration = Duration::from_secs(5);

/// How often a waiting call looks at the figures again.
const LOOK_AGAIN: Duration = Duration::from_millis(1);

/// The state every thread works with while it holds a GIL scope.
struct Shared(&'static State);

// SAFETY: the state is only ever made current by `enter`, on a thread
// opening a GIL scope, which holds the GIL until the scope closes and
// `leave` makes its previous state current again; so its values are only
// reached by pltrs (Rust) code running inside a GIL scope. Rust code runs
// inside a GIL scope only while its thread holds the GIL: the binding
// never releases it while Rust code reaches pltrs (Rust) values, and
// Python code releasing it in the middle of a native call (a callback)
// runs no Rust code of that thread until it takes the GIL back. The GIL
// is a lock, whose release and acquisition order every touch of the
// state's values, so their non-atomic reference counts and borrow flags
// are never touched by two threads at once; they are of no other kind of
// thread-bound type (the pltrs crates forbid unsafe code, the windows of
// `pltrs-platform` are not part of the state). The state is never
// dropped (it lives as long as the program), so no thread drops its
// values either.
unsafe impl Send for Shared {}
// SAFETY: as for `Send`: `&Shared` only hands the state to `enter`.
unsafe impl Sync for Shared {}

/// The shared state, made when first needed.
static SHARED: OnceLock<Shared> = OnceLock::new();

pltrs_core::thread_state! {
    /// The metrics caches threads had installed when they left their
    /// last GIL scope, with the threads.
    static PARKED: RefCell<Vec<(ThreadId, Rc<MetricsCache>)>> =
        const { RefCell::new(Vec::new()) };
}

/// How many caches [`PARKED`] holds, so that opening a scope looks there
/// only when a thread parked one (an atomic is cheaper to read than the
/// shared state).
static PARKED_COUNT: AtomicUsize = AtomicUsize::new(0);

/// How many threads have called into pltrs so far: until a second one
/// does, no figure can be in use by another thread's call.
static THREADS_SEEN: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Whether this thread counts in [`THREADS_SEEN`].
    static SEEN: Cell<bool> = const { Cell::new(false) };
}

/// Counts this thread among those that used pltrs, once; returns how many
/// did so far.
fn count_thread() -> usize {
    if SEEN.replace(true) {
        THREADS_SEEN.load(Ordering::Relaxed)
    } else {
        THREADS_SEEN.fetch_add(1, Ordering::Relaxed) + 1
    }
}

/// What a thread worked with outside GIL scopes, given back when its
/// outermost scope closes ([`leave`]).
#[derive(Debug)]
pub(super) struct Outside {
    /// The state it worked with.
    state: Option<&'static State>,
    /// The metrics cache it had installed.
    metrics_cache: Option<Rc<MetricsCache>>,
}

/// Opens the outermost GIL scope of this thread: from now on it works
/// with the shared state, and with the metrics cache it parked when it
/// left its last scope; returns what it worked with until now.
///
/// # Safety
///
/// The thread holds the GIL until it calls [`leave`].
pub(super) unsafe fn enter() -> Outside {
    count_thread();
    let shared = SHARED.get_or_init(|| Shared(State::new())).0;
    let state = state::set_current(Some(shared));
    let parked = if PARKED_COUNT.load(Ordering::Relaxed) == 0 {
        None
    } else {
        let thread = std::thread::current().id();
        PARKED.with_borrow_mut(|parked| {
            let index = parked.iter().position(|(owner, _)| *owner == thread)?;
            PARKED_COUNT.fetch_sub(1, Ordering::Relaxed);
            Some(parked.swap_remove(index).1)
        })
    };
    Outside {
        state,
        metrics_cache: set_metrics_cache(parked),
    }
}

/// Closes the outermost GIL scope [`enter`] opened: the thread parks the
/// metrics cache it has installed for its next scope, and works with what
/// it worked with outside again.
pub(super) fn leave(outside: Outside) {
    if let Some(cache) = set_metrics_cache(outside.metrics_cache) {
        let thread = std::thread::current().id();
        PARKED.with_borrow_mut(|parked| parked.push((thread, cache)));
        PARKED_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    state::set_current(outside.state);
}

/// Waits, without the GIL, while the figure of an artist among `args` is
/// in use by another thread's native call (one paused in Python code that
/// the figure's draw called), so that the call does not find the figure
/// borrowed: up to [`WAIT_FOR_FIGURES`], past which it runs anyway (the
/// other thread may be waiting for this one).
///
/// # Safety
///
/// CPython called a native function on this thread with `args`, which are
/// live, and it is the thread's outermost native call (it holds no GIL
/// scope): a figure in use is then another thread's.
pub(super) unsafe fn wait_for_figures(args: &[*mut ffi::PyObject]) {
    if count_thread() < 2 {
        return;
    }
    let in_use = || {
        args.iter()
            // SAFETY: (caller) live objects, with the GIL held whenever
            // this runs.
            .any(|&arg| unsafe { artist_of(arg) }.is_some_and(RawHandle::figure_in_use))
    };
    if !in_use() {
        return;
    }
    let deadline = Instant::now() + WAIT_FOR_FIGURES;
    while Instant::now() < deadline {
        // SAFETY: (caller) a native call holds the GIL; sleeping touches
        // nothing.
        unsafe { without_gil(|| std::thread::sleep(LOOK_AGAIN)) };
        if !in_use() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use pltrs::colors::{Colormap, colormaps};

    use super::*;
    use crate::binding::testing::with_python;

    /// Runs `f` on a new thread, which has ended when this returns.
    fn on_a_thread<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
        std::thread::spawn(f).join().expect("the thread runs")
    }

    #[test]
    fn python_threads_share_what_threads_keep_apart_on_their_own() {
        let name = "binding_threads_shared_colormap";
        on_a_thread(move || {
            with_python(|_| {
                let cmap = Colormap::listed(name, vec!["red", "blue"]).expect("valid colours");
                colormaps::register(cmap).expect("a new name");
            });
            assert!(!colormaps::contains(name), "not in the thread's own state");
        });
        let (shared, own) = on_a_thread(move || {
            let shared = with_python(|_| colormaps::contains(name));
            (shared, colormaps::contains(name))
        });
        with_python(|_| colormaps::unregister(name)).expect("not built in");
        assert_eq!((shared, own), (true, false));
    }

    #[test]
    fn a_metrics_cache_installed_across_calls_is_parked_between_them() {
        on_a_thread(|| {
            let cache = Rc::new(MetricsCache::new());
            let before = with_python(|_| set_metrics_cache(Some(Rc::clone(&cache))));
            assert!(before.is_none());
            assert!(
                set_metrics_cache(None).is_none(),
                "the thread's own storage holds nothing outside native calls"
            );
            let back = with_python(|_| set_metrics_cache(None));
            assert!(back.is_some_and(|back| Rc::ptr_eq(&back, &cache)));
            assert!(
                with_python(|_| set_metrics_cache(None)).is_none(),
                "taken back"
            );
        });
    }
}
