//! Exceptions and warnings.

use std::ffi::{c_char, c_int};

use super::{Py_ssize_t, PyObject};

#[cfg_attr(windows, link(name = "python3", kind = "raw-dylib"))]
unsafe extern "C" {
    /// `PyExc_ValueError`.
    pub(crate) static mut PyExc_ValueError: *mut PyObject;
    /// `PyExc_TypeError`.
    pub(crate) static mut PyExc_TypeError: *mut PyObject;
    /// `PyExc_KeyError`.
    pub(crate) static mut PyExc_KeyError: *mut PyObject;
    /// `PyExc_LookupError`.
    pub(crate) static mut PyExc_LookupError: *mut PyObject;
    /// `PyExc_IndexError`.
    pub(crate) static mut PyExc_IndexError: *mut PyObject;
    /// `PyExc_AttributeError`.
    pub(crate) static mut PyExc_AttributeError: *mut PyObject;
    /// `PyExc_RuntimeError`.
    pub(crate) static mut PyExc_RuntimeError: *mut PyObject;
    /// `PyExc_RecursionError`.
    pub(crate) static mut PyExc_RecursionError: *mut PyObject;
    /// `PyExc_OSError`.
    pub(crate) static mut PyExc_OSError: *mut PyObject;
    /// `PyExc_FileNotFoundError`.
    pub(crate) static mut PyExc_FileNotFoundError: *mut PyObject;
    /// `PyExc_OverflowError`.
    pub(crate) static mut PyExc_OverflowError: *mut PyObject;
    /// `PyExc_SystemError`.
    pub(crate) static mut PyExc_SystemError: *mut PyObject;
    /// `PyExc_NotImplementedError`.
    pub(crate) static mut PyExc_NotImplementedError: *mut PyObject;
    /// `PyExc_RuntimeWarning`.
    pub(crate) static mut PyExc_RuntimeWarning: *mut PyObject;

    /// `PyErr_SetString`: raises `exception(message)`.
    pub(crate) fn PyErr_SetString(exception: *mut PyObject, message: *const c_char);
    /// `PyErr_GetRaisedException`: takes the raised exception, a new
    /// reference, or NULL (3.12).
    pub(crate) fn PyErr_GetRaisedException() -> *mut PyObject;
    /// `PyErr_SetRaisedException`: raises an exception instance, stealing
    /// it (3.12).
    pub(crate) fn PyErr_SetRaisedException(exc: *mut PyObject);
    /// `PyErr_WarnEx`: issues a warning; -1 when it was turned into an
    /// exception.
    pub(crate) fn PyErr_WarnEx(
        category: *mut PyObject,
        message: *const c_char,
        stack_level: Py_ssize_t,
    ) -> c_int;
}
