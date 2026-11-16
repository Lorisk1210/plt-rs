//! The object protocol: reference counts, attributes, calls, comparison,
//! items and iteration.

use std::ffi::{c_char, c_int};

use super::{PyObject, PyTypeObject};

/// `PY_VECTORCALL_ARGUMENTS_OFFSET`: set in `nargsf` when the callee may
/// write to `args[-1]`.
pub(crate) const PY_VECTORCALL_ARGUMENTS_OFFSET: usize = 1 << (usize::BITS - 1);

#[cfg_attr(
    pltrs_python_link_python3,
    link(name = "python3.lib", modifiers = "+verbatim")
)]
#[cfg_attr(
    all(windows, not(pltrs_python_link_python3)),
    link(name = "python3", kind = "raw-dylib")
)]
unsafe extern "C" {
    /// `Py_IncRef`: a new strong reference (the function form, which the
    /// limited API provides whatever the reference count's layout).
    pub(crate) fn Py_IncRef(o: *mut PyObject);
    /// `Py_DecRef`: drops a strong reference.
    pub(crate) fn Py_DecRef(o: *mut PyObject);

    /// `PyObject_GetAttrString`: `getattr(o, name)`, a new reference.
    pub(crate) fn PyObject_GetAttrString(o: *mut PyObject, name: *const c_char) -> *mut PyObject;

    /// `PyObject_Repr`: `repr(o)`.
    pub(crate) fn PyObject_Repr(o: *mut PyObject) -> *mut PyObject;
    /// `PyObject_Str`: `str(o)`.
    pub(crate) fn PyObject_Str(o: *mut PyObject) -> *mut PyObject;
    /// `PyObject_IsTrue`: `bool(o)`, or -1 on error.
    pub(crate) fn PyObject_IsTrue(o: *mut PyObject) -> c_int;
    /// `PyObject_RichCompareBool`: whether `a op b` holds (`op` one of the
    /// `Py_EQ`... constants), or -1 on error.
    pub(crate) fn PyObject_RichCompareBool(a: *mut PyObject, b: *mut PyObject, op: c_int) -> c_int;
    /// `PyObject_SetItem`: `o[key] = v`.
    pub(crate) fn PyObject_SetItem(o: *mut PyObject, key: *mut PyObject, v: *mut PyObject)
    -> c_int;
    /// `PyObject_GetIter`: `iter(o)`.
    pub(crate) fn PyObject_GetIter(o: *mut PyObject) -> *mut PyObject;
    /// `PyIter_Next`: the next item, or NULL at the end (without an error
    /// set) or on error (with one).
    pub(crate) fn PyIter_Next(o: *mut PyObject) -> *mut PyObject;

    /// `PyObject_Vectorcall`: calls `callable` with the positional
    /// arguments `args[..nargsf]` and the keyword arguments named by the
    /// tuple `kwnames`, whose values follow them in `args`.
    pub(crate) fn PyObject_Vectorcall(
        callable: *mut PyObject,
        args: *const *mut PyObject,
        nargsf: usize,
        kwnames: *mut PyObject,
    ) -> *mut PyObject;
    /// `PyObject_VectorcallMethod`: calls the method `name` of `args[0]`
    /// with `args[1..nargsf]`.
    pub(crate) fn PyObject_VectorcallMethod(
        name: *mut PyObject,
        args: *const *mut PyObject,
        nargsf: usize,
        kwnames: *mut PyObject,
    ) -> *mut PyObject;

    /// `PyType_IsSubtype`: whether `a` is `b` or a subtype of it.
    pub(crate) fn PyType_IsSubtype(a: *mut PyTypeObject, b: *mut PyTypeObject) -> c_int;
    /// `PyType_GetFlags`: the `tp_flags` of a type.
    pub(crate) fn PyType_GetFlags(t: *mut PyTypeObject) -> std::ffi::c_ulong;
    /// `PyType_GetName`: `type.__name__`, a new reference.
    pub(crate) fn PyType_GetName(t: *mut PyTypeObject) -> *mut PyObject;

    /// `PyOS_FSPath`: `os.fspath(path)`.
    pub(crate) fn PyOS_FSPath(path: *mut PyObject) -> *mut PyObject;
}
