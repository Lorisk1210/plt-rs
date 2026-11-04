//! The native object: the one Python type (`_native.Native`) that holds a
//! Rust value for Python, the *native handle* every pltrs class wraps.
//!
//! A native object holds any `'static` Rust value: a pltrs (Rust) artist handle
//! (`Figure`, `Axes`, `Line2D`, ...), or a value such as a colormap or a
//! transform. An artist handle's object also keeps the artist's
//! [`RawHandle`], so that functions common to all artists take any of
//! them. Python code cannot make or subclass the type; native functions
//! return it, and take it back as an argument through [`Native<T>`] or
//! the conversions the [`native_value!`] and [`artist_handle!`] macros
//! give a type.
//!
//! pltrs (Rust) values are not `Send` (handles share their figure through an
//! `Rc`), and Python objects may be reached from any thread holding the
//! GIL. The GIL is what makes that sound ([`threads`](super::threads)): a
//! native object's value is only reached in native calls and in the type's
//! slots, with the GIL held, so any thread may use it and drop it. The
//! values tied to the thread that made them, pltrs (Rust)'s windows (on
//! macOS only the main thread may use them), are made with
//! [`wrap_thread_bound`]: their objects refuse other threads
//! (`RuntimeError`), whose calls the main thread runs while it runs an
//! event loop ([`home`](super::home)), and one deallocated on another
//! thread leaks its value rather than dropping it there.
//!
//! [`RawHandle`]: pltrs::artist::RawHandle

use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::ffi::{c_int, c_void};
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::NonNull;
use std::sync::OnceLock;
use std::thread::ThreadId;

use super::convert::{FromPy, IntoPy};
use super::error::{PyErr, PyResult};
use super::gil::{GilScope, Python, entry};
use super::module::Module;
use super::object::{Bound, Owned};
use crate::ffi;
use pltrs::artist::{ArtistHandle, RawHandle};

/// What a native object holds.
struct NativeCell {
    /// The thread that made the value, if it is tied to it: the only one
    /// that may use it then.
    owner: Option<ThreadId>,
    /// The value's type, for messages (`pltrs::lines::Line2D`).
    type_name: &'static str,
    /// The untyped handle of the artist the value is a handle of, if it
    /// is one: what functions common to all artists take.
    artist: Option<RawHandle>,
    /// The value.
    value: RefCell<Box<dyn Any>>,
}

/// The native type, made once when the module is initialised.
struct NativeType(NonNull<ffi::PyTypeObject>);

// SAFETY: the type object is only used with the GIL held (to allocate
// instances and compare types); the strong reference the module keeps
// makes it live for the interpreter's lifetime.
unsafe impl Send for NativeType {}
// SAFETY: as for `Send`.
unsafe impl Sync for NativeType {}

static NATIVE_TYPE: OnceLock<NativeType> = OnceLock::new();

/// The type of native objects.
fn native_type() -> *mut ffi::PyTypeObject {
    NATIVE_TYPE
        .get()
        .expect("the native type is made when the module is initialised")
        .0
        .as_ptr()
}

/// Makes the native type and adds it to `module` as `Native`.
pub(crate) fn add_type(module: &Module<'_>) -> PyResult<()> {
    let py = module.py();
    if let Some(existing) = NATIVE_TYPE.get() {
        // SAFETY: the type made earlier is live (the first module keeps
        // it); a new strong reference for this module.
        let class = unsafe { Bound::from_borrowed_ptr(py, existing.0.as_ptr().cast()) };
        return module.add_object("Native", class);
    }
    let name = format!("{}.Native\0", module.name());
    // The spec is read by `PyType_FromSpec`, which copies what it needs,
    // except the name, which must outlive the type: it is leaked.
    let name: &'static str = Box::leak(name.into_boxed_str());
    let mut slots = [
        slot(
            ffi::Py_tp_dealloc,
            dealloc as ffi::destructor as *mut c_void,
        ),
        slot(ffi::Py_tp_repr, repr as ffi::reprfunc as *mut c_void),
        slot(
            ffi::Py_tp_doc,
            c"A pltrs (Rust) value held for Python.".as_ptr() as *mut c_void,
        ),
        slot(0, std::ptr::null_mut()),
    ];
    let mut spec = ffi::PyType_Spec {
        name: name.as_ptr().cast(),
        basicsize: -c_int::try_from(std::mem::size_of::<NativeCell>()).expect("a small cell"),
        itemsize: 0,
        flags: ffi::Py_TPFLAGS_DEFAULT
            | ffi::Py_TPFLAGS_DISALLOW_INSTANTIATION
            | ffi::Py_TPFLAGS_IMMUTABLETYPE,
        slots: slots.as_mut_ptr(),
    };
    // SAFETY: a complete spec ending with a zero slot; the GIL is held.
    let class = unsafe { Bound::from_owned_ptr(py, ffi::PyType_FromSpec(&raw mut spec)) }?;
    let pointer = NonNull::new(class.as_ptr().cast()).expect("a made type is not NULL");
    let _ = NATIVE_TYPE.set(NativeType(pointer));
    // The module keeps the type; this extra reference keeps it for the
    // interpreter's lifetime, as `NATIVE_TYPE` refers to it.
    std::mem::forget(class.clone());
    module.add_object("Native", class)
}

/// A type slot.
fn slot(id: c_int, function: *mut c_void) -> ffi::PyType_Slot {
    ffi::PyType_Slot {
        slot: id,
        pfunc: function,
    }
}

/// The cell of a native object.
///
/// # Safety
///
/// `object` is a live instance of the native type.
unsafe fn cell<'a>(object: *mut ffi::PyObject) -> &'a NativeCell {
    // SAFETY: (caller) a native instance, whose type data holds a
    // `NativeCell` written when it was made, live as long as the object.
    unsafe { &*ffi::PyObject_GetTypeData(object, native_type()).cast::<NativeCell>() }
}

/// `tp_dealloc`: drops the value (a value tied to its thread only there)
/// and frees the object.
unsafe extern "C" fn dealloc(object: *mut ffi::PyObject) {
    // SAFETY: CPython deallocates instances of the native type here, with
    // the GIL held; the cell is dropped once (a panic in a destructor is
    // contained, the value then leaks), then the memory freed with the
    // type's `tp_free`, and the instance's reference to its heap type
    // released, as heap types require.
    unsafe {
        let scope = GilScope::enter();
        let data = ffi::PyObject_GetTypeData(object, native_type()).cast::<NativeCell>();
        if (*data)
            .owner
            .is_none_or(|owner| owner == std::thread::current().id())
        {
            let _ = catch_unwind(AssertUnwindSafe(|| std::ptr::drop_in_place(data)));
        }
        drop(scope);
        let class = (*object).ob_type;
        let free: Option<ffi::freefunc> =
            std::mem::transmute(ffi::PyType_GetSlot(class, ffi::Py_tp_free));
        if let Some(free) = free {
            free(object.cast());
        }
        ffi::Py_DecRef(class.cast());
    }
}

/// `tp_repr`: `<pltrs native pltrs::lines::Line2D>`.
unsafe extern "C" fn repr(object: *mut ffi::PyObject) -> *mut ffi::PyObject {
    // SAFETY: CPython calls `tp_repr` with the GIL held, on a live
    // instance of the native type.
    unsafe {
        entry(|py| {
            let name = cell(object).type_name;
            format!("<pltrs native {name}>").into_py(py)
        })
    }
}

/// A new native object holding `value`, for every thread.
///
/// The value must not be tied to its thread (it holds no pltrs (Rust)
/// window): what is not `Send` in pltrs (Rust) values is their `Rc`s and
/// `RefCell`s, which the GIL serialises ([`threads`](super::threads)).
pub(crate) fn wrap<T: Any>(py: Python<'_>, value: T) -> PyResult<Bound<'_>> {
    wrap_cell(py, value, None, None)
}

/// A new native object holding `value`, which only this thread may use:
/// a value tied to the thread that made it (a pltrs (Rust) window host).
pub(crate) fn wrap_thread_bound<T: Any>(py: Python<'_>, value: T) -> PyResult<Bound<'_>> {
    wrap_cell(py, value, None, Some(std::thread::current().id()))
}

/// A new native object holding the artist handle `handle`.
pub(crate) fn wrap_artist<T: ArtistHandle + Any>(py: Python<'_>, handle: T) -> PyResult<Bound<'_>> {
    let raw = handle.raw().clone();
    wrap_cell(py, handle, Some(raw), None)
}

/// A new native object holding `value`, and `artist` if it is a handle,
/// tied to the thread `owner` if given.
fn wrap_cell<T: Any>(
    py: Python<'_>,
    value: T,
    artist: Option<RawHandle>,
    owner: Option<ThreadId>,
) -> PyResult<Bound<'_>> {
    let class = native_type();
    // SAFETY: the GIL is held; the native type allocates zeroed memory
    // with room for its cell, or returns NULL with an exception.
    let object = unsafe { Bound::from_owned_ptr(py, ffi::PyType_GenericAlloc(class, 0)) }?;
    let cell = NativeCell {
        owner,
        type_name: std::any::type_name::<T>(),
        artist,
        value: RefCell::new(Box::new(value)),
    };
    // SAFETY: the type data of a fresh instance, aligned for any type and
    // sized for a cell, written once before anything reads it.
    unsafe {
        ffi::PyObject_GetTypeData(object.as_ptr(), class)
            .cast::<NativeCell>()
            .write(cell);
    }
    Ok(object)
}

/// The native object `object` holding a `T`, for typed access: what a
/// native function takes to reach the value inside a native handle.
///
/// Converting checks the object's type, the value's type and, for a value
/// tied to its thread, the thread, so the accessors cannot fail but by
/// re-entrant borrowing (a mutable borrow while another borrow is alive
/// panics, as `RefCell` does; a call another thread makes while a
/// callback of this one runs is re-entrant too).
pub(crate) struct Native<T> {
    object: Owned,
    _value: PhantomData<(T, *mut ())>,
}

impl<T: Any> Native<T> {
    /// The cell, valid while `self` keeps the object.
    fn cell(&self) -> &NativeCell {
        // SAFETY: the type was checked on conversion, and `self` keeps
        // the object alive; the cell is Rust data, read by the native call
        // that converted `self` (`Native` is not `Send`), so with the GIL
        // held (see `threads`), and on its thread for a value tied to one.
        unsafe { cell(self.object.as_ptr()) }
    }

    /// Borrows the value.
    pub(crate) fn borrow(&self) -> Ref<'_, T> {
        Ref::map(self.cell().value.borrow(), |value| {
            value
                .downcast_ref()
                .expect("the value type was checked on conversion")
        })
    }

    /// Borrows the value mutably.
    pub(crate) fn borrow_mut(&self) -> RefMut<'_, T> {
        RefMut::map(self.cell().value.borrow_mut(), |value| {
            value
                .downcast_mut()
                .expect("the value type was checked on conversion")
        })
    }
}

/// The thread `object` is tied to, if it is the native object of a value
/// tied to its thread.
///
/// # Safety
///
/// The GIL is held and `object` is a live Python object.
pub(super) unsafe fn owner_of(object: *mut ffi::PyObject) -> Option<ThreadId> {
    let native = NATIVE_TYPE.get()?.0.as_ptr();
    // SAFETY: (caller) a live object, whose header is readable; the native
    // type cannot be subclassed, so its instances are those of that exact
    // type, whose cell is live while the object is.
    unsafe { ((*object).ob_type == native).then(|| cell(object).owner) }.flatten()
}

/// The untyped handle of the artist `object` holds a handle of, if it is
/// the native object of one.
///
/// # Safety
///
/// The GIL is held and `object` is a live Python object, which outlives
/// the handle borrowed from it.
pub(super) unsafe fn artist_of<'a>(object: *mut ffi::PyObject) -> Option<&'a RawHandle> {
    let native = NATIVE_TYPE.get()?.0.as_ptr();
    // SAFETY: as in `owner_of`; the cell lives as long as the object.
    unsafe { ((*object).ob_type == native).then(|| cell(object).artist.as_ref()) }.flatten()
}

/// The cell of `object`, if it is a native object this thread may use,
/// and, for an artist's handle, one whose figure holds the artist (not a
/// figure being unpickled, whose handles are made before it is read
/// back); `wanted` names what was expected, for messages.
fn checked_cell<'a>(object: &'a Bound<'_>, wanted: &str) -> PyResult<&'a NativeCell> {
    if !object.is_type(native_type()) {
        return Err(PyErr::type_error(format!(
            "expected a native {wanted} handle, got {}",
            object.type_name()
        )));
    }
    // SAFETY: an instance of the native type, kept alive by `object`.
    let cell = unsafe { cell(object.as_ptr()) };
    if cell.artist.as_ref().is_some_and(|artist| !artist.exists()) {
        Err(PyErr::runtime_error(
            "this artist's figure is being unpickled: the artist is not read back yet",
        ))
    } else if cell
        .owner
        .is_none_or(|owner| owner == std::thread::current().id())
    {
        Ok(cell)
    } else {
        Err(PyErr::runtime_error(format!(
            "this pltrs {} belongs to another thread: pltrs (Rust)'s windows are used on the \
             thread that made them (the main thread runs the calls of other threads while it \
             runs an event loop)",
            short_name(cell.type_name)
        )))
    }
}

impl FromPy<'_> for RawHandle {
    /// The untyped handle of any artist's native object.
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let cell = checked_cell(object, "artist")?;
        cell.artist.clone().ok_or_else(|| {
            PyErr::type_error(format!(
                "expected a native artist handle, got one of {}",
                short_name(cell.type_name)
            ))
        })
    }
}

impl IntoPy for RawHandle {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        wrap_artist(py, self)
    }
}

impl<T> std::fmt::Debug for Native<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Native<{}>", short_name(std::any::type_name::<T>()))
    }
}

impl<T: Any> FromPy<'_> for Native<T> {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let cell = checked_cell(object, short_name(std::any::type_name::<T>()))?;
        if !cell.value.borrow().is::<T>() {
            return Err(PyErr::type_error(format!(
                "expected a native {} handle, got one of {}",
                short_name(std::any::type_name::<T>()),
                short_name(cell.type_name)
            )));
        }
        Ok(Self {
            object: object.clone().unbind(),
            _value: PhantomData,
        })
    }
}

/// The last component of a type path (`Line2D` of
/// `pltrs::lines::Line2D`), for messages.
pub(crate) fn short_name(path: &str) -> &str {
    let base = path.split('<').next().unwrap_or(path);
    base.rsplit("::").next().unwrap_or(base)
}

/// Gives types held by value in native objects their conversions: a
/// native function takes and returns them directly (a clone comes out).
macro_rules! native_value {
    ($($ty:ty),* $(,)?) => {$(
        impl $crate::binding::FromPy<'_> for $ty {
            fn from_py(object: &$crate::binding::Bound<'_>) -> $crate::binding::PyResult<Self> {
                let native: $crate::binding::Native<$ty> = object.extract()?;
                let value = native.borrow().clone();
                Ok(value)
            }
        }

        impl $crate::binding::IntoPy for $ty {
            fn into_py(self, py: $crate::binding::Python<'_>) -> $crate::binding::PyResult<$crate::binding::Bound<'_>> {
                $crate::binding::wrap_native(py, self)
            }
        }
    )*};
}

/// The handle of type `H` of the artist behind `object`, a native object
/// of another handle type (an untyped [`RawHandle`] such as an adoption
/// returns, or another typed handle of the same artist); the type error
/// `mismatch` when the artist is no `H`.
pub(crate) fn retyped<H: pltrs::artist::TypedHandle>(
    object: &Bound<'_>,
    mismatch: PyErr,
) -> PyResult<H> {
    match object.extract::<RawHandle>() {
        Ok(raw) => H::from_raw_handle(&raw).ok_or(mismatch),
        Err(_) => Err(mismatch),
    }
}

/// Gives pltrs (Rust) artist handle types (`Axes`, `Line2D`, ...) their
/// conversions: a native function takes and returns them directly, and
/// their native objects also serve functions taking any artist. A native
/// object of any handle of the artist converts to the type, which must
/// therefore implement [`TypedHandle`](pltrs::artist::TypedHandle): an
/// artist adopted by an axes is reached through the untyped handle the
/// adoption returns.
macro_rules! artist_handle {
    ($($ty:ty),* $(,)?) => {$(
        impl $crate::binding::FromPy<'_> for $ty {
            fn from_py(object: &$crate::binding::Bound<'_>) -> $crate::binding::PyResult<Self> {
                match object.extract::<$crate::binding::Native<$ty>>() {
                    Ok(native) => Ok(native.borrow().clone()),
                    Err(mismatch) => $crate::binding::retyped(object, mismatch),
                }
            }
        }

        impl $crate::binding::IntoPy for $ty {
            fn into_py(self, py: $crate::binding::Python<'_>) -> $crate::binding::PyResult<$crate::binding::Bound<'_>> {
                $crate::binding::wrap_artist(py, self)
            }
        }
    )*};
}

pub(crate) use {artist_handle, native_value};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{extension, with_python};
    use pltrs::figure::Figure;

    #[test]
    fn values_come_back_typed() {
        with_python(|py| {
            extension(py);
            let object = wrap(py, vec![1_u8, 2]).unwrap();
            let native: Native<Vec<u8>> = object.extract().unwrap();
            native.borrow_mut().push(3);
            assert_eq!(*native.borrow(), [1, 2, 3]);
            assert!(
                object
                    .repr()
                    .unwrap()
                    .starts_with("<pltrs native alloc::vec::Vec")
            );
            let err = object.extract::<Native<String>>().unwrap_err();
            assert_eq!(err.class_name(py), "TypeError");
            assert!(
                err.message(py)
                    .contains("expected a native String handle, got one of Vec")
            );
            let err = 1.0
                .into_py(py)
                .unwrap()
                .extract::<Native<String>>()
                .unwrap_err();
            assert!(err.message(py).contains("got float"));
        });
    }

    #[test]
    fn artist_handles_reach_their_raw_handle() {
        with_python(|py| {
            extension(py);
            let figure = Figure::new();
            let object = wrap_artist(py, figure.clone()).unwrap();
            let raw: RawHandle = object.extract().unwrap();
            assert_eq!(raw, *figure.raw());
            let plain = wrap(py, 5_i32).unwrap();
            assert!(plain.extract::<RawHandle>().is_err(), "not an artist");
        });
    }

    #[test]
    fn values_serve_every_thread_and_outlive_the_one_that_made_them() {
        let (figure, value) = std::thread::spawn(|| {
            with_python(|py| {
                extension(py);
                let figure = wrap_artist(py, Figure::new()).unwrap().unbind();
                (figure, wrap(py, vec![1_i32]).unwrap().unbind())
            })
        })
        .join()
        .unwrap();
        let (dpi, values) = std::thread::spawn(move || {
            with_python(|py| {
                let native: Native<Vec<i32>> = value.bind(py).extract().unwrap();
                native.borrow_mut().push(2);
                let values = native.borrow().clone();
                let figure: Figure = figure.bind(py).extract().unwrap();
                figure.set_dpi(50.0);
                (figure.dpi(), values)
            })
        })
        .join()
        .unwrap();
        assert_eq!((dpi, values), (50.0, vec![1, 2]));
    }

    #[test]
    fn values_tied_to_their_thread_refuse_others() {
        let object = with_python(|py| {
            extension(py);
            wrap_thread_bound(py, 1_i32).unwrap().unbind()
        });
        let message = std::thread::spawn(move || {
            with_python(|py| {
                let err = object.bind(py).extract::<Native<i32>>().unwrap_err();
                err.message(py)
            })
        })
        .join()
        .unwrap();
        assert!(
            message.contains("pltrs i32 belongs to another thread"),
            "{message}"
        );
    }

    #[test]
    fn short_names_drop_paths_and_parameters() {
        assert_eq!(short_name("pltrs::lines::Line2D"), "Line2D");
        assert_eq!(
            short_name("pltrs::patches::Patch<pltrs::patches::Rectangle>"),
            "Patch"
        );
    }
}
