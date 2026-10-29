//! Making tuples, lists and dicts.

use super::error::{PyErr, PyResult};
use super::gil::Python;
use super::object::Bound;
use crate::ffi;

/// The length of a container as `Py_ssize_t`.
fn ssize(length: usize) -> PyResult<ffi::Py_ssize_t> {
    ffi::Py_ssize_t::try_from(length).map_err(|_| PyErr::value_error("too many items"))
}

/// A tuple of `items`.
pub(crate) fn tuple<'py>(py: Python<'py>, items: Vec<Bound<'py>>) -> PyResult<Bound<'py>> {
    // SAFETY: the GIL is held; a new reference or NULL.
    let tuple = unsafe { Bound::from_owned_ptr(py, ffi::PyTuple_New(ssize(items.len())?)) }?;
    for (position, item) in (0..).zip(items) {
        // SAFETY: a new tuple not yet shared, with room for every item;
        // the call steals `item`'s reference.
        unsafe { ffi::PyTuple_SetItem(tuple.as_ptr(), position, item.into_ptr()) };
    }
    Ok(tuple)
}

/// A list of `items`.
pub(crate) fn list<'py>(py: Python<'py>, items: Vec<Bound<'py>>) -> PyResult<Bound<'py>> {
    // SAFETY: the GIL is held; a new reference or NULL.
    let list = unsafe { Bound::from_owned_ptr(py, ffi::PyList_New(ssize(items.len())?)) }?;
    for (position, item) in (0..).zip(items) {
        // SAFETY: a new list not yet shared, with room for every item;
        // the call steals `item`'s reference.
        unsafe { ffi::PyList_SetItem(list.as_ptr(), position, item.into_ptr()) };
    }
    Ok(list)
}

/// An empty dict.
pub(crate) fn new_dict(py: Python<'_>) -> PyResult<Bound<'_>> {
    // SAFETY: the GIL is held; a new reference or NULL.
    unsafe { Bound::from_owned_ptr(py, ffi::PyDict_New()) }
}

/// The items of a dict, in insertion order.
pub(crate) fn dict_items<'py>(dict: &Bound<'py>) -> PyResult<Vec<(Bound<'py>, Bound<'py>)>> {
    if !dict.has_type_flag(ffi::Py_TPFLAGS_DICT_SUBCLASS) {
        return Err(PyErr::type_error(format!(
            "expected dict, got {}",
            dict.type_name()
        )));
    }
    let py = dict.py();
    let mut items = Vec::new();
    let mut position: ffi::Py_ssize_t = 0;
    let (mut key, mut value) = (std::ptr::null_mut(), std::ptr::null_mut());
    // SAFETY: `dict` is a dict, iterated with CPython's cursor; the key
    // and value come back borrowed and are taken as new references
    // before anything else runs.
    while unsafe {
        ffi::PyDict_Next(
            dict.as_ptr(),
            &raw mut position,
            &raw mut key,
            &raw mut value,
        )
    } != 0
    {
        // SAFETY: both are live borrowed references from the dict.
        items.push(unsafe {
            (
                Bound::from_borrowed_ptr(py, key),
                Bound::from_borrowed_ptr(py, value),
            )
        });
    }
    Ok(items)
}
