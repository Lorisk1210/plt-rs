//! Concrete objects: the singletons, numbers, strings, bytes, tuples,
//! lists, dicts and capsules.

use std::ffi::{c_char, c_double, c_int, c_long, c_longlong, c_void};

use super::{Py_ssize_t, PyObject, PyTypeObject};

/// `Py_TPFLAGS_BYTES_SUBCLASS`.
pub(crate) const Py_TPFLAGS_BYTES_SUBCLASS: std::ffi::c_ulong = 1 << 27;
/// `Py_TPFLAGS_UNICODE_SUBCLASS`.
pub(crate) const Py_TPFLAGS_UNICODE_SUBCLASS: std::ffi::c_ulong = 1 << 28;
/// `Py_TPFLAGS_DICT_SUBCLASS`.
pub(crate) const Py_TPFLAGS_DICT_SUBCLASS: std::ffi::c_ulong = 1 << 29;

/// `PyCapsule_Destructor`.
pub(crate) type PyCapsule_Destructor = unsafe extern "C" fn(capsule: *mut PyObject);

#[cfg_attr(windows, link(name = "python3", kind = "raw-dylib"))]
unsafe extern "C" {
    /// `_Py_NoneStruct`: `None` (`Py_None` is its address).
    pub(crate) static mut _Py_NoneStruct: PyObject;
    /// `_Py_TrueStruct`: `True`.
    pub(crate) static mut _Py_TrueStruct: PyObject;
    /// `_Py_FalseStruct`: `False`.
    pub(crate) static mut _Py_FalseStruct: PyObject;
    /// `PyFloat_Type`: `float`.
    pub(crate) static mut PyFloat_Type: PyTypeObject;
    /// `PyLong_Type`: `int`.
    pub(crate) static mut PyLong_Type: PyTypeObject;
    /// `PyTuple_Type`: `tuple`.
    pub(crate) static mut PyTuple_Type: PyTypeObject;
    /// `PyList_Type`: `list`.
    pub(crate) static mut PyList_Type: PyTypeObject;

    /// `PyBool_FromLong`: `True` or `False`.
    pub(crate) fn PyBool_FromLong(v: c_long) -> *mut PyObject;
    /// `PyLong_FromLongLong`.
    pub(crate) fn PyLong_FromLongLong(v: c_longlong) -> *mut PyObject;
    /// `PyLong_FromUnsignedLongLong`.
    pub(crate) fn PyLong_FromUnsignedLongLong(v: std::ffi::c_ulonglong) -> *mut PyObject;
    /// `PyLong_AsLongLong`: -1 with `OverflowError` set when out of range.
    pub(crate) fn PyLong_AsLongLong(o: *mut PyObject) -> c_longlong;
    /// `PyNumber_Index`: `operator.index(o)`, an `int`.
    pub(crate) fn PyNumber_Index(o: *mut PyObject) -> *mut PyObject;
    /// `PyLong_AsDouble`: the nearest `float`, -1.0 with `OverflowError`
    /// set beyond its range.
    pub(crate) fn PyLong_AsDouble(o: *mut PyObject) -> c_double;
    /// `PyFloat_FromDouble`.
    pub(crate) fn PyFloat_FromDouble(v: c_double) -> *mut PyObject;
    /// `PyFloat_AsDouble`: `float(o)` for numbers, -1.0 with an error set
    /// otherwise.
    pub(crate) fn PyFloat_AsDouble(o: *mut PyObject) -> c_double;

    /// `PyUnicode_FromStringAndSize`: a `str` from UTF-8.
    pub(crate) fn PyUnicode_FromStringAndSize(s: *const c_char, size: Py_ssize_t) -> *mut PyObject;
    /// `PyUnicode_AsUTF8AndSize`: the UTF-8 of a `str`, owned by it.
    pub(crate) fn PyUnicode_AsUTF8AndSize(o: *mut PyObject, size: *mut Py_ssize_t)
    -> *const c_char;
    /// `PyBytes_FromStringAndSize`: `bytes` from a copy of the data.
    pub(crate) fn PyBytes_FromStringAndSize(s: *const c_char, size: Py_ssize_t) -> *mut PyObject;
    /// `PyBytes_AsString`: the contents of a `bytes`, owned by it.
    pub(crate) fn PyBytes_AsString(o: *mut PyObject) -> *mut c_char;
    /// `PyBytes_Size`: the length of a `bytes`.
    pub(crate) fn PyBytes_Size(o: *mut PyObject) -> Py_ssize_t;

    /// `PyTuple_New`: a tuple whose items must all be set before use.
    pub(crate) fn PyTuple_New(size: Py_ssize_t) -> *mut PyObject;
    /// `PyTuple_SetItem`: sets an item of a new tuple, stealing `o`.
    pub(crate) fn PyTuple_SetItem(t: *mut PyObject, pos: Py_ssize_t, o: *mut PyObject) -> c_int;
    /// `PyTuple_Size`: the length of a tuple.
    pub(crate) fn PyTuple_Size(t: *mut PyObject) -> Py_ssize_t;
    /// `PyTuple_GetItem`: an item of a tuple, borrowed from it.
    pub(crate) fn PyTuple_GetItem(t: *mut PyObject, pos: Py_ssize_t) -> *mut PyObject;
    /// `PyList_Size`: the length of a list.
    pub(crate) fn PyList_Size(l: *mut PyObject) -> Py_ssize_t;
    /// `PyList_GetItem`: an item of a list, borrowed from it.
    pub(crate) fn PyList_GetItem(l: *mut PyObject, pos: Py_ssize_t) -> *mut PyObject;
    /// `PyList_New`: a list whose items must all be set before use.
    pub(crate) fn PyList_New(size: Py_ssize_t) -> *mut PyObject;
    /// `PyList_SetItem`: sets an item of a list, stealing `o`.
    pub(crate) fn PyList_SetItem(l: *mut PyObject, pos: Py_ssize_t, o: *mut PyObject) -> c_int;
    /// `PyDict_New`.
    pub(crate) fn PyDict_New() -> *mut PyObject;
    /// `PyDict_Size`: `len(d)`, -1 with an error set for a non-dict.
    pub(crate) fn PyDict_Size(d: *mut PyObject) -> Py_ssize_t;
    /// `PyDict_Copy`: a new dict of the same items.
    pub(crate) fn PyDict_Copy(d: *mut PyObject) -> *mut PyObject;
    /// `PyDict_Clear`: removes every item.
    pub(crate) fn PyDict_Clear(d: *mut PyObject);
    /// `PyDict_Next`: iterates a dict's items as borrowed references.
    pub(crate) fn PyDict_Next(
        d: *mut PyObject,
        pos: *mut Py_ssize_t,
        key: *mut *mut PyObject,
        value: *mut *mut PyObject,
    ) -> c_int;

    /// `PyCapsule_New`: wraps a pointer, with a destructor run when the
    /// capsule goes.
    pub(crate) fn PyCapsule_New(
        pointer: *mut c_void,
        name: *const c_char,
        destructor: Option<PyCapsule_Destructor>,
    ) -> *mut PyObject;
    /// `PyCapsule_GetPointer`: the pointer a capsule of that name wraps.
    pub(crate) fn PyCapsule_GetPointer(capsule: *mut PyObject, name: *const c_char) -> *mut c_void;
}
