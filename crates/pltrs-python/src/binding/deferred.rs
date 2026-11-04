//! Work Python leaves to the next native call.
//!
//! Setting a line's data hands pltrs (Rust) its numbers only when the
//! next native call is made, much as matplotlib recomputes a line's path
//! only when the line is next drawn or read: any native call might read
//! them, so every one first runs what is waiting. The work waits in a
//! dict Python fills; before a native call runs, the work is taken out of
//! it at once (no Python code runs meanwhile, so another thread adds
//! nothing that would be lost) and handed to the function installed with
//! it.

use std::sync::OnceLock;

use super::error::{PyErr, PyResult};
use super::gil::Python;
use super::object::Owned;
use crate::ffi;

/// The dict of waiting work and the function that does it.
#[derive(Debug)]
struct Deferred {
    queue: Owned,
    flush: Owned,
}

/// What was installed, once.
static DEFERRED: OnceLock<Deferred> = OnceLock::new();

/// Installs `flush`, called with the items of the dict `queue` (taken out
/// of it) before a native call runs whenever it is not empty, and returns
/// the dict in force: the first one installed (a module imported again
/// keeps using it, and the first `flush`).
pub(crate) fn install(queue: Owned, flush: Owned) -> PyResult<Owned> {
    let is_dict =
        Python::with_gil(|py| queue.bind(py).has_type_flag(ffi::Py_TPFLAGS_DICT_SUBCLASS));
    if !is_dict {
        return Err(PyErr::type_error("the queue of deferred work is a dict"));
    }
    Ok(DEFERRED
        .get_or_init(|| Deferred { queue, flush })
        .queue
        .clone())
}

/// Does the waiting work, if any; `false` (with the exception set) when
/// the function doing it raised.
///
/// # Safety
///
/// The GIL is held, and the thread may run Python code, as at the start of
/// a native call: no Rust borrow is held across this call, and a native
/// call the flush makes while a figure of the thread is in use stops at it
/// (see [`reentrant`](super::reentrant)).
pub(super) unsafe fn run() -> bool {
    let Some(deferred) = DEFERRED.get() else {
        return true;
    };
    // SAFETY: (caller) the GIL is held; the queue is a dict (checked when
    // installed), which `PyDict_Size` reads without raising.
    if unsafe { ffi::PyDict_Size(deferred.queue.as_ptr()) } <= 0 {
        return true;
    }
    // SAFETY: (caller) the GIL is held; copying a dict runs no Python code
    // and gives a new reference (or NULL with an error set), which keeps
    // the items alive, so that clearing the queue frees none of them and
    // runs no Python code either; the flush is called with the copy as its
    // one argument (the slot before it free for the callee,
    // `PY_VECTORCALL_ARGUMENTS_OFFSET`), returning a new reference or NULL
    // with an error set.
    unsafe {
        let waiting = ffi::PyDict_Copy(deferred.queue.as_ptr());
        if waiting.is_null() {
            return false;
        }
        ffi::PyDict_Clear(deferred.queue.as_ptr());
        let mut args = [std::ptr::null_mut(), waiting];
        let result = ffi::PyObject_Vectorcall(
            deferred.flush.as_ptr(),
            args.as_mut_ptr().add(1).cast_const(),
            1 | ffi::PY_VECTORCALL_ARGUMENTS_OFFSET,
            std::ptr::null_mut(),
        );
        ffi::Py_DecRef(waiting);
        if result.is_null() {
            return false;
        }
        ffi::Py_DecRef(result);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::convert::FromPy;
    use crate::binding::testing::{eval, with_python};

    #[test]
    fn waiting_work_is_done_once_before_the_next_call() {
        with_python(|py| {
            let setup = "import types\n\
                         ns = types.SimpleNamespace(queue={}, calls=[])\n\
                         def flush(waiting):\n    ns.calls.append(sorted(waiting) + list(ns.queue))\n\
                         ns.flush = flush";
            let ns = eval(py, setup, "ns");
            let attr = |name: &str| ns.getattr(name).expect("set up");
            let queue = attr("queue");
            let calls = || Vec::<Vec<i64>>::from_py(&attr("calls")).expect("lists");
            let installed = install(queue.clone().unbind(), attr("flush").unbind()).unwrap();
            assert_eq!(installed.as_ptr(), queue.as_ptr());
            let other = eval(py, "", "{}");
            let again = install(other.unbind(), attr("flush").unbind()).unwrap();
            assert_eq!(
                again.as_ptr(),
                queue.as_ptr(),
                "the first installation counts"
            );
            // SAFETY: the GIL is held and no pltrs (Rust) state is borrowed.
            assert!(unsafe { run() });
            assert_eq!(
                calls(),
                Vec::<Vec<i64>>::new(),
                "an empty queue calls nothing"
            );
            queue.set_item(2_i64, 0_i64).unwrap();
            queue.set_item(1_i64, 0_i64).unwrap();
            // SAFETY: as above, twice.
            assert!(unsafe { run() } && unsafe { run() });
            assert_eq!(
                calls(),
                vec![vec![1, 2]],
                "the work was taken out of the queue"
            );
            let not_a_dict = install(eval(py, "", "[]").unbind(), attr("flush").unbind());
            assert!(not_a_dict.is_err());
        });
    }
}
