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
//! A call asks whatever its function is, with one kind of exception: the
//! functions listed in [`PASSING_BY`], which read nothing of what such
//! objects hold and whose effect does not depend on it (the setter of a
//! line width, the getter of a z-order). The hand-over waits for the
//! next call that reads the artist, so a program that sets the styles of
//! an artist whose array it holds pays for no comparison of the array.
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

/// The native functions that pass by what the Python objects of artists
/// keep, by module and name, each with what it reads or writes: no
/// artist is told before a call of one of them ([`run`]).
///
/// A function belongs here only if its Rust code was read and found to
/// read none of the state an artist may keep as a Python object (offsets,
/// paths, sizes, widths, dash patterns, flags, URLs, colours, item
/// transforms, mesh coordinates, the numbers and the path of a line, the
/// path and the points of a patch, the array of a mappable, a clip box),
/// or to read it only for what the next hand-over of that state computes
/// again. One that writes such state is called by setters that forget the
/// object the artist kept for it, so that nothing old is handed over
/// afterwards. A function not listed tells: slower where an artist keeps
/// a large array, never wrong. `python/tests/mutability` has rows for
/// every function listed here (`setters.py`), and `test_mut1.py` fails on
/// one without.
pub(crate) const PASSING_BY: &[(&str, &str, &str)] = &[
    ("artist", "axes", "reads which Axes the artist is in"),
    ("artist", "id", "reads the artist's id in its figure"),
    (
        "artist",
        "draw_count",
        "reads how many times the artist was drawn",
    ),
    ("artist", "set_stale", "marks the artist changed, or drawn"),
    ("artist", "get_visible", "reads whether the artist is drawn"),
    (
        "artist",
        "set_visible",
        "writes whether the artist is drawn",
    ),
    ("artist", "get_zorder", "reads the drawing order"),
    ("artist", "set_zorder", "writes the drawing order"),
    (
        "artist",
        "get_alpha",
        "reads the opacity of the artist as a whole",
    ),
    (
        "artist",
        "set_alpha",
        "writes the opacity of the artist as a whole (the colours of a collection take it in \
         `collections.set_alpha`, which tells)",
    ),
    ("artist", "get_label", "reads the legend label"),
    ("artist", "set_label", "writes the legend label"),
    (
        "artist",
        "get_animated",
        "reads whether normal draws leave the artist out",
    ),
    (
        "artist",
        "set_animated",
        "writes whether normal draws leave the artist out",
    ),
    (
        "artist",
        "set_clip_on",
        "writes whether the artist is clipped (not what it is clipped to)",
    ),
    (
        "artist",
        "set_rasterized",
        "writes whether vector output rasterizes the artist",
    ),
    (
        "artist",
        "set_in_layout",
        "writes whether layouts account for the artist",
    ),
    ("artist", "set_snap", "writes the pixel snapping preference"),
    ("artist", "set_gid", "writes the group id of vector output"),
    ("artist", "set_url", "writes the hyperlink of vector output"),
    ("artist", "set_picker", "writes the picker"),
    (
        "collections",
        "set_linewidths",
        "writes the edge widths (`set_linewidth` forgets the widths and dash patterns kept)",
    ),
    (
        "collections",
        "set_linestyles",
        "writes the dash patterns, unscaled (`set_linestyle` forgets the widths and patterns kept)",
    ),
    (
        "collections",
        "set_antialiaseds",
        "writes the antialiasing flags (`set_antialiased` forgets the flags kept)",
    ),
    ("collections", "set_capstyle", "writes the cap style"),
    ("collections", "set_joinstyle", "writes the join style"),
    ("collections", "set_pickradius", "writes the pick radius"),
    (
        "collections",
        "set_urls",
        "writes the URLs (`set_urls` forgets the list kept)",
    ),
    ("collections", "set_hatch", "writes the hatch pattern"),
    (
        "collections",
        "set_hatch_linewidth",
        "writes the hatch line width",
    ),
    (
        "collections",
        "set_sizes",
        "writes the marker areas (`set_sizes` forgets the sizes and item transforms kept)",
    ),
    (
        "collections",
        "set_offsets",
        "writes the offsets, and a display offset transform where there is none (`set_offsets` \
         and an assigned `_offsets` forget the offsets kept)",
    ),
    ("lines", "get_drawstyle", "reads the draw style"),
    (
        "lines",
        "update",
        "writes the style properties named; a draw style or interpolation steps make the path \
         anew from the numbers pltrs (Rust) holds, as the next hand-over of the numbers does \
         again (`set_markevery` forgets or keeps the markers given)",
    ),
    (
        "patches",
        "update",
        "writes the edge, face and hatch properties named and the label, z-order, visibility, \
         clip flag and opacity",
    ),
];

/// Whether the function `name` of the module whose name ends in `module`
/// passes by what the objects of artists keep ([`PASSING_BY`]).
pub(super) fn passes_by(module: &str, name: &str) -> bool {
    PASSING_BY
        .iter()
        .any(|&(of, function, _)| of == module && function == name)
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
/// or by an object being told, or `tells` (asked only where an object
/// would be told) says that the function called passes by what the
/// objects keep ([`PASSING_BY`]); `false` (with the exception set) when
/// one of them raised. The lists that are no artist's are looked at
/// whatever the function.
///
/// # Safety
///
/// The GIL is held, `args` are live objects, and the thread may run
/// Python code, as at the start of a native call (see
/// [`deferred::run`](super::deferred::run)).
pub(super) unsafe fn run(args: &[*mut ffi::PyObject], tells: impl FnOnce() -> bool) -> bool {
    let lists = LISTS.get().is_some();
    if (ASKING.load(Ordering::Relaxed) == 0 && !lists) || gil_held() || TELLING.get() {
        return true;
    }
    let _telling = Telling(TELLING.replace(true));
    // SAFETY: (caller) the same contract.
    if lists && !unsafe { tell_of_lists() } {
        return false;
    }
    if ASKING.load(Ordering::Relaxed) == 0 || !tells() {
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
    fn functions_passing_by_are_listed_once_with_what_they_do() {
        for (index, &(module, name, what)) in PASSING_BY.iter().enumerate() {
            assert!(passes_by(module, name));
            assert!(!what.is_empty(), "{module}.{name} says what it does");
            assert!(
                !PASSING_BY[..index]
                    .iter()
                    .any(|&(of, function, _)| (of, function) == (module, name)),
                "{module}.{name} is listed twice"
            );
        }
        assert!(!passes_by("artist", "touch"), "a touch tells");
        assert!(!passes_by("collections", "get"));
        assert!(!passes_by("lines", "set_visible"), "by module and name");
    }

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
            assert!(unsafe { run(&[asking.as_ptr()], || true) });
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
