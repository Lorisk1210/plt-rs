//! State of an artist that is a Python object: told before a native call
//! reaches the artist.
//!
//! matplotlib's artists keep their state in Python objects (the array of
//! a scatter's offsets, the list of a collection's paths), which programs
//! change in place. pltrs (Rust) keeps that state itself, so the Python
//! object of an artist that handed such an object out, or was given one
//! it keeps, asks to be called before any native call takes its native
//! object as an argument ([`set_asker`]): it then hands pltrs (Rust) what
//! the program changed, and the call reads the artist as matplotlib's
//! code would read the attribute. A native object nobody asked about
//! costs a call nothing: while no object in the process asks, [`run`] is
//! one load, and otherwise one comparison per argument.
//!
//! Only a thread's outermost native call asks: a call made while pltrs
//! (Rust) lays a figure out or draws it could change nothing in it, and
//! the calls the asked object makes to hand its state over ask nobody.
//!
//! State no artist owns is looked at before every such call instead: the
//! lists the current rcParams handed out ([`install_lists`]), which any
//! native call may read. Each is compared with what it held when it was
//! last taken, and the package is told when one changed.

use std::cell::Cell;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::error::{PyErr, PyResult};
use super::gil::{Python, gil_held};
use super::native;
use super::object::Owned;
use crate::ffi;

/// How many native objects ask to be told (see [`Asker`]).
static ASKING: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Whether the thread is telling an object now: the native calls made
    /// meanwhile tell nobody.
    static TELLING: Cell<bool> = const { Cell::new(false) };
}

/// The callable of a native object asking to be told, counted while it
/// lives.
#[derive(Debug)]
pub(super) struct Asker(Owned);

impl Asker {
    /// `callable`, counted as asking.
    pub(super) fn new(callable: Owned) -> Self {
        ASKING.fetch_add(1, Ordering::Relaxed);
        Self(callable)
    }

    /// The callable.
    pub(super) fn callable(&self) -> &Owned {
        &self.0
    }
}

impl Drop for Asker {
    fn drop(&mut self) {
        ASKING.fetch_sub(1, Ordering::Relaxed);
    }
}

/// The lists handed out that are no artist's, and the function told
/// when one changed.
#[derive(Debug)]
struct Lists {
    /// A dict whose values are Python lists starting with a list handed
    /// out and a copy of it as it was last taken.
    records: Owned,
    /// Called without arguments once a list no longer equals its copy.
    changed: Owned,
}

/// What was installed, once.
static LISTS: OnceLock<Lists> = OnceLock::new();

/// Installs `changed`, called before a native call of a thread's own
/// whenever the first item of a value of the dict `records` (a list
/// handed out) no longer equals the second (a copy of it as pltrs (Rust)
/// last took it); returns the dict in force, the first one installed.
pub(crate) fn install_lists(records: Owned, changed: Owned) -> PyResult<Owned> {
    let is_dict = Python::with_gil(|py| {
        records
            .bind(py)
            .has_type_flag(ffi::Py_TPFLAGS_DICT_SUBCLASS)
    });
    if !is_dict {
        return Err(PyErr::type_error("the lists handed out are kept in a dict"));
    }
    Ok(LISTS
        .get_or_init(|| Lists { records, changed })
        .records
        .clone())
}

/// Whether a list handed out changed since it was last taken; `None`
/// (with the exception set) when comparing one raised.
///
/// # Safety
///
/// The GIL is held.
unsafe fn a_list_changed(lists: &Lists) -> Option<bool> {
    let mut position: ffi::Py_ssize_t = 0;
    let (mut key, mut record) = (std::ptr::null_mut(), std::ptr::null_mut());
    // SAFETY: (caller) the GIL is held; the records are a dict (checked
    // when installed), iterated with CPython's cursor, whose values come
    // back borrowed; each is read as a list of at least two items
    // (`PyList_GetItem` gives NULL with an error set for anything else),
    // which are compared with `==`: lists of numbers and strings, whose
    // comparison runs no code of the program's and leaves the dict alone.
    unsafe {
        while ffi::PyDict_Next(
            lists.records.as_ptr(),
            &raw mut position,
            &raw mut key,
            &raw mut record,
        ) != 0
        {
            let (handed, taken) = (
                ffi::PyList_GetItem(record, 0),
                ffi::PyList_GetItem(record, 1),
            );
            if handed.is_null() || taken.is_null() {
                return None;
            }
            match ffi::PyObject_RichCompareBool(handed, taken, ffi::Py_EQ) {
                1 => {}
                0 => return Some(true),
                _ => return None,
            }
        }
    }
    Some(false)
}

/// Tells the package when a list handed out changed; `false` (with the
/// exception set) when that raised.
///
/// # Safety
///
/// As for [`run`].
unsafe fn tell_of_lists() -> bool {
    let Some(lists) = LISTS.get() else {
        return true;
    };
    // SAFETY: (caller) the GIL is held; the dict's size is read without
    // raising, and the function is called without arguments (as in
    // `tell`), returning a new reference or NULL with an error set.
    unsafe {
        if ffi::PyDict_Size(lists.records.as_ptr()) <= 0 {
            return true;
        }
        match a_list_changed(lists) {
            Some(false) => return true,
            Some(true) => {}
            None => return false,
        }
        let mut slots: [*mut ffi::PyObject; 1] = [std::ptr::null_mut()];
        let result = ffi::PyObject_Vectorcall(
            lists.changed.as_ptr(),
            slots.as_mut_ptr().add(1).cast_const(),
            ffi::PY_VECTORCALL_ARGUMENTS_OFFSET,
            std::ptr::null_mut(),
        );
        if result.is_null() {
            return false;
        }
        ffi::Py_DecRef(result);
    }
    true
}

/// Puts `telling` back when dropped, also when the callable panics.
struct Telling(bool);

impl Drop for Telling {
    fn drop(&mut self) {
        TELLING.set(self.0);
    }
}

/// Calls the callable of each native object among `args` that asked to be
/// told, unless the call is made inside another native call of the thread
/// or by an object being told; `false` (with the exception set) when one
/// of them raised.
///
/// # Safety
///
/// The GIL is held, `args` are live objects, and the thread may run
/// Python code, as at the start of a native call (see
/// [`deferred::run`](super::deferred::run)).
pub(super) unsafe fn run(args: &[*mut ffi::PyObject]) -> bool {
    let lists = LISTS.get().is_some();
    if (ASKING.load(Ordering::Relaxed) == 0 && !lists) || gil_held() || TELLING.get() {
        return true;
    }
    let _telling = Telling(TELLING.replace(true));
    // SAFETY: (caller) the same contract.
    if lists && !unsafe { tell_of_lists() } {
        return false;
    }
    if ASKING.load(Ordering::Relaxed) == 0 {
        return true;
    }
    // What an object hands over is state its program counts as the
    // artist's already: it makes nothing stale.
    // SAFETY: (caller) the same contract.
    pltrs::figure::unseen(|| unsafe { tell(args) })
}

/// Calls the callable of each native object among `args` that asked to be
/// told; `false` (with the exception set) when one of them raised.
///
/// # Safety
///
/// As for [`run`].
unsafe fn tell(args: &[*mut ffi::PyObject]) -> bool {
    for &arg in args {
        // SAFETY: (caller) the GIL is held and `arg` is live; the callable
        // is cloned out of the cell before it is called, so that it may
        // replace itself; it is called without arguments (`nargs` 0, the
        // slot before the arguments free for the callee), returning a new
        // reference or NULL with an error set.
        unsafe {
            let Some(callable) = native::asker_of(arg) else {
                continue;
            };
            let mut slots: [*mut ffi::PyObject; 1] = [std::ptr::null_mut()];
            let result = ffi::PyObject_Vectorcall(
                callable.as_ptr(),
                slots.as_mut_ptr().add(1).cast_const(),
                ffi::PY_VECTORCALL_ARGUMENTS_OFFSET,
                std::ptr::null_mut(),
            );
            if result.is_null() {
                return false;
            }
            ffi::Py_DecRef(result);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::convert::FromPy;
    use crate::binding::testing::{eval, extension, with_python};

    #[test]
    fn asking_objects_are_counted_and_told_only_by_outermost_calls() {
        with_python(|py| {
            extension(py);
            let ns = eval(
                py,
                "import types\nns = types.SimpleNamespace(told=[])\n\
                 ns.tell = lambda: ns.told.append(len(ns.told))",
                "ns",
            );
            let told = || Vec::<i64>::from_py(&ns.getattr("told").expect("set up")).expect("ints");
            let asking = native::wrap(py, 1_u8).unwrap();
            let before = ASKING.load(Ordering::Relaxed);
            native::set_asker(&asking, Some(ns.getattr("tell").unwrap().unbind())).unwrap();
            assert_eq!(ASKING.load(Ordering::Relaxed), before + 1);
            // The test holds the GIL as a native call does, so this call
            // counts as made inside one: nobody is told (the smoke tests
            // check the outermost calls, `python/tests/test_mut1.py`).
            // SAFETY: the GIL is held and the object is live.
            assert!(unsafe { run(&[asking.as_ptr()]) });
            assert_eq!(told(), Vec::<i64>::new());
            native::set_asker(&asking, None).unwrap();
            assert_eq!(ASKING.load(Ordering::Relaxed), before);
            native::set_asker(&asking, Some(ns.getattr("tell").unwrap().unbind())).unwrap();
            drop(asking);
            assert_eq!(
                ASKING.load(Ordering::Relaxed),
                before,
                "an object that goes asks no more"
            );
        });
    }
}
