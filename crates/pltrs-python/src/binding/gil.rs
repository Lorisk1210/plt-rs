//! The GIL: the [`Python`] token proving a thread holds it, and the
//! bookkeeping that makes dropping Python references from Rust sound.
//!
//! The extension is abi3, which exists only for CPython builds with a
//! GIL; only the thread holding it may touch Python objects. Native
//! functions run with it held: their entry point ([`entry`]) opens a
//! [`GilScope`], which counts the thread as holding the GIL and hands out
//! a token. Rust code that runs on its own (pltrs (Rust) calling a Python
//! formatter from a draw, a destructor) takes it with
//! [`Python::with_gil`].
//!
//! A strong reference dropped where the thread is not known to hold the
//! GIL is not released there: it waits in a pool released at the next
//! entry from Python.

use std::any::Any;
use std::cell::Cell;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::NonNull;
use std::sync::Mutex;

use super::callback;
use super::error::{PyErr, PyResult};
use super::object::Bound;
use crate::ffi;

thread_local! {
    /// How many [`GilScope`]s are open on this thread: it holds the GIL
    /// while the count is positive.
    static GIL_COUNT: Cell<usize> = const { Cell::new(0) };
}

/// Strong references dropped without the GIL, released at the next entry.
static PENDING_DECREFS: Mutex<Vec<PendingDecref>> = Mutex::new(Vec::new());

/// A strong reference waiting for the GIL to be released.
struct PendingDecref(NonNull<ffi::PyObject>);

// SAFETY: the pointer is only handed to `Py_DecRef`, by
// `release_pending`, on a thread holding the GIL; it is never
// dereferenced by Rust.
unsafe impl Send for PendingDecref {}

/// Proof that the current thread holds the GIL for the lifetime `'py`.
///
/// It is `Copy`, zero-sized and neither `Send` nor `Sync`: a token cannot
/// leave the thread or the scope it was made for.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Python<'py>(PhantomData<(&'py (), *mut ())>);

impl Python<'_> {
    /// Runs `f` with the GIL, taking it first if this thread does not
    /// hold it (from pltrs (Rust) code running outside a native call, such as a
    /// destructor). Re-entrant: cheap when the GIL is already held.
    pub(crate) fn with_gil<R>(f: impl for<'py> FnOnce(Python<'py>) -> R) -> R {
        /// Releases a GIL taken with `PyGILState_Ensure`, on panics too.
        struct Release(ffi::PyGILState_STATE);
        impl Drop for Release {
            fn drop(&mut self) {
                // SAFETY: undoes the `PyGILState_Ensure` below, on the
                // same thread, after the scope that used the GIL closed.
                unsafe { ffi::PyGILState_Release(self.0) };
            }
        }
        // SAFETY: the interpreter is running (this extension is loaded in
        // it), so `PyGILState_Ensure` may take the GIL for this thread.
        let _release = (!gil_held()).then(|| Release(unsafe { ffi::PyGILState_Ensure() }));
        // SAFETY: the GIL is held: it already was, or was just taken.
        let scope = unsafe { GilScope::enter() };
        f(scope.python())
    }
}

/// Whether this thread is known to hold the GIL.
pub(crate) fn gil_held() -> bool {
    GIL_COUNT.with(|count| count.get() > 0)
}

/// Counts the thread as holding the GIL while it lives.
#[derive(Debug)]
pub(crate) struct GilScope {
    _not_send: PhantomData<*mut ()>,
}

impl GilScope {
    /// Opens a scope, releasing the references dropped without the GIL.
    ///
    /// # Safety
    ///
    /// The current thread holds the GIL, and keeps it until the scope is
    /// dropped.
    pub(crate) unsafe fn enter() -> Self {
        GIL_COUNT.with(|count| count.set(count.get() + 1));
        release_pending();
        Self {
            _not_send: PhantomData,
        }
    }

    /// The token of the scope.
    #[expect(
        clippy::unused_self,
        reason = "the token borrows the scope: it lives as long"
    )]
    pub(crate) fn python(&self) -> Python<'_> {
        Python(PhantomData)
    }
}

impl Drop for GilScope {
    fn drop(&mut self) {
        GIL_COUNT.with(|count| count.set(count.get() - 1));
    }
}

/// Keeps a strong reference for release at the next entry from Python.
pub(crate) fn defer_decref(object: NonNull<ffi::PyObject>) {
    PENDING_DECREFS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(PendingDecref(object));
}

/// Releases the references dropped without the GIL (called with it held).
fn release_pending() {
    let pending = std::mem::take(
        &mut *PENDING_DECREFS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    for PendingDecref(object) in pending {
        // SAFETY: the GIL is held (a scope was just entered) and each
        // pointer is a strong reference its owner gave up.
        unsafe { ffi::Py_DecRef(object.as_ptr()) };
    }
}

/// Runs the Rust side of an entry point from Python (a native function, a
/// slot of the native type) and turns its outcome into what CPython
/// expects: a new reference, or NULL with the exception raised.
///
/// A panic becomes a `RuntimeError` instead of unwinding into C, and an
/// exception a Python callback raised meanwhile ([`callback`]) replaces
/// the result.
///
/// # Safety
///
/// CPython called the entry point, so the current thread holds the GIL.
pub(crate) unsafe fn entry(
    body: impl for<'py> FnOnce(Python<'py>) -> PyResult<Bound<'py>>,
) -> *mut ffi::PyObject {
    // SAFETY: the caller guarantees the GIL is held for the whole call.
    let scope = unsafe { GilScope::enter() };
    let py = scope.python();
    let errors = callback::ErrorFrame::push();
    let outcome = catch_unwind(AssertUnwindSafe(|| body(py)))
        .unwrap_or_else(|panic| Err(PyErr::from_panic(&*panic)));
    let outcome = match errors.pop() {
        Some(raised) => Err(raised),
        None => outcome,
    };
    match outcome {
        Ok(object) => object.into_ptr(),
        Err(err) => {
            err.restore(py);
            std::ptr::null_mut()
        }
    }
}

/// The message of a panic payload.
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic")
}
