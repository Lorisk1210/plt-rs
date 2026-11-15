//! References to Python objects: [`Bound`], tied to the GIL, and
//! [`Owned`], which Rust state keeps.
//!
//! Both are strong references: making one increments the object's
//! reference count and dropping it decrements it, so a Python object
//! lives as long as Rust holds it. Arguments CPython lends a native
//! function become `Bound`s too (one increment each), which keeps every
//! reference in Rust owned.

use std::ffi::CString;
use std::fmt;
use std::ptr::NonNull;

use super::convert::{FromPy, IntoPy};
use super::error::{PyErr, PyResult};
use super::gil::{Python, defer_decref, gil_held};
use crate::ffi;

/// A strong reference to a Python object, usable while the GIL is held
/// (the token's lifetime `'py`).
pub(crate) struct Bound<'py> {
    object: NonNull<ffi::PyObject>,
    py: Python<'py>,
}

impl<'py> Bound<'py> {
    /// Takes ownership of a new reference a C API call returned; NULL
    /// means the call raised, and gives its exception.
    ///
    /// # Safety
    ///
    /// `object` is NULL or a strong reference the caller owns.
    pub(crate) unsafe fn from_owned_ptr(
        py: Python<'py>,
        object: *mut ffi::PyObject,
    ) -> PyResult<Self> {
        NonNull::new(object)
            .map(|object| Self { object, py })
            .ok_or_else(|| PyErr::fetch(py))
    }

    /// A new strong reference to an object the caller borrows.
    ///
    /// # Safety
    ///
    /// `object` points to a live Python object.
    pub(crate) unsafe fn from_borrowed_ptr(py: Python<'py>, object: *mut ffi::PyObject) -> Self {
        let object = NonNull::new(object).expect("a borrowed reference is not NULL");
        // SAFETY: the object is live (caller) and the GIL is held (`py`).
        unsafe { ffi::Py_IncRef(object.as_ptr()) };
        Self { object, py }
    }

    /// `None`.
    pub(crate) fn none(py: Python<'py>) -> Self {
        // SAFETY: `_Py_NoneStruct` is the `None` singleton, live for the
        // interpreter's lifetime; only its address is taken.
        unsafe { Self::from_borrowed_ptr(py, &raw mut ffi::_Py_NoneStruct) }
    }

    /// The object pointer, still owned by `self`.
    pub(crate) fn as_ptr(&self) -> *mut ffi::PyObject {
        self.object.as_ptr()
    }

    /// Gives up ownership: the caller now owns the reference.
    pub(crate) fn into_ptr(self) -> *mut ffi::PyObject {
        let object = self.object.as_ptr();
        std::mem::forget(self);
        object
    }

    /// The GIL token.
    pub(crate) fn py(&self) -> Python<'py> {
        self.py
    }

    /// A reference that can outlive the GIL scope.
    pub(crate) fn unbind(self) -> Owned {
        Owned {
            object: NonNull::new(self.into_ptr()).expect("a bound reference is not NULL"),
        }
    }

    /// Whether this is `None`.
    pub(crate) fn is_none(&self) -> bool {
        std::ptr::eq(self.as_ptr(), &raw mut ffi::_Py_NoneStruct)
    }

    /// Python's `id(self)`: the object's address, unique among the objects
    /// alive.
    pub(crate) fn id(&self) -> usize {
        self.as_ptr().addr()
    }

    /// Whether this is the very object `other` (Python's `is`).
    pub(crate) fn is(&self, other: &Bound<'_>) -> bool {
        std::ptr::eq(self.as_ptr(), other.as_ptr())
    }

    /// The object's type, borrowed from it.
    fn type_ptr(&self) -> *mut ffi::PyTypeObject {
        // SAFETY: `ob_type` of the object header is in the limited API,
        // and the object is live while `self` holds it.
        unsafe { (*self.as_ptr()).ob_type }
    }

    /// Whether the object's type has the `Py_TPFLAGS_*` bit `flag` (how
    /// the limited API tests for `int`, `str`, `tuple`, ...).
    pub(crate) fn has_type_flag(&self, flag: std::ffi::c_ulong) -> bool {
        // SAFETY: the type pointer comes from a live object.
        unsafe { ffi::PyType_GetFlags(self.type_ptr()) & flag != 0 }
    }

    /// Whether the object is an instance of the static type `of`.
    pub(crate) fn is_type(&self, of: *mut ffi::PyTypeObject) -> bool {
        // SAFETY: both pointers are live type objects.
        unsafe { ffi::PyType_IsSubtype(self.type_ptr(), of) != 0 }
    }

    /// The name of the object's type, for messages (`float`).
    pub(crate) fn type_name(&self) -> String {
        // SAFETY: the type pointer comes from a live object; the call
        // returns a new reference or raises.
        let name = unsafe { Self::from_owned_ptr(self.py, ffi::PyType_GetName(self.type_ptr())) };
        name.and_then(|name| name.extract::<String>())
            .unwrap_or_else(|_| "object".to_owned())
    }

    /// `getattr(self, name)`.
    pub(crate) fn getattr(&self, name: &str) -> PyResult<Bound<'py>> {
        let name = c_string(name)?;
        // SAFETY: valid object and NUL-terminated name; new reference.
        unsafe {
            Self::from_owned_ptr(
                self.py,
                ffi::PyObject_GetAttrString(self.as_ptr(), name.as_ptr()),
            )
        }
    }

    /// Calls the object with positional arguments and keyword arguments.
    pub(crate) fn call(
        &self,
        args: &[Bound<'py>],
        kwargs: &[(&str, Bound<'py>)],
    ) -> PyResult<Bound<'py>> {
        vectorcall(self.py, None, self, args, kwargs)
    }

    /// Calls the method `name` with positional arguments.
    pub(crate) fn call_method(&self, name: &str, args: &[Bound<'py>]) -> PyResult<Bound<'py>> {
        let name = name.into_py(self.py)?;
        vectorcall(self.py, Some(&name), self, args, &[])
    }

    /// `repr(self)`.
    pub(crate) fn repr(&self) -> PyResult<String> {
        // SAFETY: valid object; new reference or NULL.
        unsafe { Self::from_owned_ptr(self.py, ffi::PyObject_Repr(self.as_ptr())) }?.extract()
    }

    /// `str(self)`.
    pub(crate) fn str(&self) -> PyResult<String> {
        // SAFETY: valid object; new reference or NULL.
        unsafe { Self::from_owned_ptr(self.py, ffi::PyObject_Str(self.as_ptr())) }?.extract()
    }

    /// `bool(self)`.
    pub(crate) fn is_truthy(&self) -> PyResult<bool> {
        // SAFETY: valid object.
        let truth = unsafe { ffi::PyObject_IsTrue(self.as_ptr()) };
        check_status(self.py, truth).map(|()| truth == 1)
    }

    /// `self[key] = value`.
    pub(crate) fn set_item(&self, key: impl IntoPy, value: impl IntoPy) -> PyResult<()> {
        let (key, value) = (key.into_py(self.py)?, value.into_py(self.py)?);
        // SAFETY: valid objects, none stolen.
        let status = unsafe { ffi::PyObject_SetItem(self.as_ptr(), key.as_ptr(), value.as_ptr()) };
        check_status(self.py, status)
    }

    /// `iter(self)`.
    pub(crate) fn try_iter(&self) -> PyResult<Iter<'py>> {
        // SAFETY: valid object; new reference or NULL.
        let iterator =
            unsafe { Self::from_owned_ptr(self.py, ffi::PyObject_GetIter(self.as_ptr())) }?;
        Ok(Iter { iterator })
    }

    /// Converts the object to a Rust value.
    pub(crate) fn extract<T: FromPy<'py>>(&self) -> PyResult<T> {
        T::from_py(self)
    }
}

impl Clone for Bound<'_> {
    fn clone(&self) -> Self {
        // SAFETY: the object is live and the GIL held (`'py`).
        unsafe { Self::from_borrowed_ptr(self.py, self.as_ptr()) }
    }
}

impl Drop for Bound<'_> {
    fn drop(&mut self) {
        // SAFETY: `self` owns one strong reference, and the GIL is held
        // for `'py`.
        unsafe { ffi::Py_DecRef(self.as_ptr()) };
    }
}

impl fmt::Debug for Bound<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.repr() {
            Ok(repr) => f.write_str(&repr),
            Err(_) => write!(f, "<{} object>", self.type_name()),
        }
    }
}

/// The items of a Python iterator, each a result: iterating can raise.
#[derive(Debug)]
pub(crate) struct Iter<'py> {
    iterator: Bound<'py>,
}

impl<'py> Iterator for Iter<'py> {
    type Item = PyResult<Bound<'py>>;

    fn next(&mut self) -> Option<Self::Item> {
        let py = self.iterator.py();
        // SAFETY: valid iterator; new reference, or NULL at the end
        // (without an exception) or on error (with one).
        let item = unsafe { ffi::PyIter_Next(self.iterator.as_ptr()) };
        if item.is_null() {
            PyErr::take(py).map(Err)
        } else {
            // SAFETY: a new reference from `PyIter_Next`.
            Some(unsafe { Bound::from_owned_ptr(py, item) })
        }
    }
}

/// A strong reference that Rust state keeps beyond a GIL scope: a Python
/// callable held by a pltrs (Rust) closure, a class looked up later.
///
/// It is `Send` and `Sync`, so it can live in the `Send` closures pltrs (Rust)
/// takes; using the object takes a [`Python`] token ([`Owned::bind`]).
/// Dropped where the thread does not hold the GIL, it is released at the
/// next entry from Python.
pub(crate) struct Owned {
    object: NonNull<ffi::PyObject>,
}

// SAFETY: the pointer is only used with the GIL held: through `bind` and
// `clone_ref`, which take a token, and in `Drop`, which defers the
// release to a thread holding the GIL when this one does not.
unsafe impl Send for Owned {}
// SAFETY: as for `Send`: `&Owned` gives no access to the object without
// a token.
unsafe impl Sync for Owned {}

impl Owned {
    /// A new strong reference, usable while the GIL is held.
    pub(crate) fn bind<'py>(&self, py: Python<'py>) -> Bound<'py> {
        // SAFETY: `self` keeps the object alive, and `py` proves the GIL.
        unsafe { Bound::from_borrowed_ptr(py, self.object.as_ptr()) }
    }

    /// The object pointer, still owned by `self`.
    pub(crate) fn as_ptr(&self) -> *mut ffi::PyObject {
        self.object.as_ptr()
    }

    /// A second strong reference to the object.
    pub(crate) fn clone_ref(&self, py: Python<'_>) -> Self {
        self.bind(py).unbind()
    }
}

impl Clone for Owned {
    /// Takes the GIL if needed: pltrs (Rust) clones the closures holding Python
    /// objects outside native calls too.
    fn clone(&self) -> Self {
        Python::with_gil(|py| self.clone_ref(py))
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        if gil_held() {
            // SAFETY: `self` owns one strong reference, and this thread
            // holds the GIL.
            unsafe { ffi::Py_DecRef(self.object.as_ptr()) };
        } else {
            defer_decref(self.object);
        }
    }
}

impl fmt::Debug for Owned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Owned({:p})", self.object)
    }
}

/// `text` as a C string, refusing embedded NULs.
pub(crate) fn c_string(text: &str) -> PyResult<CString> {
    CString::new(text)
        .map_err(|_| PyErr::value_error(format!("embedded null character in {text:?}")))
}

/// Turns a C API status (-1 on error) into a result.
pub(crate) fn check_status(py: Python<'_>, status: std::ffi::c_int) -> PyResult<()> {
    if status == -1 {
        Err(PyErr::fetch(py))
    } else {
        Ok(())
    }
}

/// Calls `callable` (or the method `method` of it) through vectorcall.
fn vectorcall<'py>(
    py: Python<'py>,
    method: Option<&Bound<'py>>,
    callable: &Bound<'py>,
    args: &[Bound<'py>],
    kwargs: &[(&str, Bound<'py>)],
) -> PyResult<Bound<'py>> {
    // Slot 0 is free for the callee (`PY_VECTORCALL_ARGUMENTS_OFFSET`);
    // a method call puts the object first among the arguments.
    let mut slots: Vec<*mut ffi::PyObject> = Vec::with_capacity(args.len() + kwargs.len() + 2);
    slots.push(std::ptr::null_mut());
    if method.is_some() {
        slots.push(callable.as_ptr());
    }
    slots.extend(args.iter().map(Bound::as_ptr));
    let positional = slots.len() - 1;
    slots.extend(kwargs.iter().map(|(_, value)| value.as_ptr()));
    let names = if kwargs.is_empty() {
        None
    } else {
        let names: Vec<Bound<'py>> = kwargs
            .iter()
            .map(|(name, _)| name.into_py(py))
            .collect::<PyResult<_>>()?;
        Some(super::containers::tuple(py, names)?)
    };
    let kwnames = names.as_ref().map_or(std::ptr::null_mut(), Bound::as_ptr);
    let nargsf = positional | ffi::PY_VECTORCALL_ARGUMENTS_OFFSET;
    // SAFETY: `slots[1..]` holds `positional` positional arguments then
    // the keyword values `kwnames` names, all live for the call (borrowed
    // from `callable`, `args` and `kwargs`), and slot 0 may be written by
    // the callee as `PY_VECTORCALL_ARGUMENTS_OFFSET` allows.
    let result = unsafe {
        let arguments = slots.as_ptr().add(1);
        match method {
            Some(name) => ffi::PyObject_VectorcallMethod(name.as_ptr(), arguments, nargsf, kwnames),
            None => ffi::PyObject_Vectorcall(callable.as_ptr(), arguments, nargsf, kwnames),
        }
    };
    // SAFETY: a new reference or NULL with an exception set.
    unsafe { Bound::from_owned_ptr(py, result) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, import, with_python};

    /// `sys.getrefcount(object)`, to compare with itself.
    fn refcount(object: &Bound<'_>) -> i64 {
        let sys = import(object.py(), "sys");
        let count = sys
            .getattr("getrefcount")
            .unwrap()
            .call(std::slice::from_ref(object), &[])
            .unwrap();
        count.extract::<i64>().unwrap()
    }

    #[test]
    fn owned_references_dropped_without_the_gil_wait_for_it() {
        let (kept, owned, base) = with_python(|py| {
            let object = eval(py, "", "[1, 2]");
            let kept = object.clone().unbind();
            let base = refcount(&object);
            let owned = object.clone().unbind();
            assert_eq!(refcount(&object), base + 1);
            (kept, owned, base)
        });
        drop(owned);
        with_python(|py| {
            let object = kept.bind(py);
            assert_eq!(refcount(&object), base, "the drop was released on entry");
        });
    }

    #[test]
    fn calls_take_keywords_and_methods() {
        with_python(|py| {
            let dict = eval(py, "", "dict");
            let made = dict
                .call(&[], &[("a", 1_i64.into_py(py).unwrap())])
                .unwrap();
            assert_eq!(made.repr().unwrap(), "{'a': 1}");
            let upper = "ab".into_py(py).unwrap().call_method("upper", &[]).unwrap();
            assert_eq!(upper.extract::<String>().unwrap(), "AB");
            assert!(made.getattr("no_such_attribute").is_err());
        });
    }

    #[test]
    fn iteration_yields_items_and_errors() {
        with_python(|py| {
            let items: Vec<i64> = eval(py, "", "range(3)")
                .try_iter()
                .unwrap()
                .map(|item| item.unwrap().extract().unwrap())
                .collect();
            assert_eq!(items, [0, 1, 2]);
            let failing = eval(
                py,
                "def gen():\n    yield 1\n    raise KeyError('k')",
                "gen()",
            );
            let outcomes: Vec<bool> = failing
                .try_iter()
                .unwrap()
                .map(|item| item.is_ok())
                .collect();
            assert_eq!(outcomes, [true, false]);
            assert!(eval(py, "", "5").try_iter().is_err(), "not iterable");
        });
    }
}
