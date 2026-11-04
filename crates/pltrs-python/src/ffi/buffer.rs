//! The buffer protocol (in the limited API since 3.11): reading numpy
//! arrays and other buffers without numpy's C API.

use std::ffi::{c_char, c_int, c_void};

use super::{Py_ssize_t, PyObject};

/// `PyBUF_FORMAT`: the exporter fills in `format`.
pub(crate) const PyBUF_FORMAT: c_int = 0x0004;
/// `PyBUF_ND`: the exporter fills in `shape`.
pub(crate) const PyBUF_ND: c_int = 0x0008;
/// `PyBUF_STRIDES`: the exporter fills in `strides` (implies `ND`).
pub(crate) const PyBUF_STRIDES: c_int = 0x0010 | PyBUF_ND;
/// `PyBUF_RECORDS_RO`: shape, strides and format, read-only access.
pub(crate) const PyBUF_RECORDS_RO: c_int = PyBUF_STRIDES | PyBUF_FORMAT;
/// `PyBUF_WRITABLE`: the memory may be written.
pub(crate) const PyBUF_WRITABLE: c_int = 0x0001;
/// `PyBUF_C_CONTIGUOUS`: the memory is one row-major block.
pub(crate) const PyBUF_C_CONTIGUOUS: c_int = 0x0020 | PyBUF_STRIDES;

/// `Py_buffer`: a view of an exporter's memory.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct Py_buffer {
    /// The start of the memory.
    pub(crate) buf: *mut c_void,
    /// The exporter, a new reference released by `PyBuffer_Release`.
    pub(crate) obj: *mut PyObject,
    /// The total size in bytes.
    pub(crate) len: Py_ssize_t,
    /// The size of one item.
    pub(crate) itemsize: Py_ssize_t,
    /// Whether the memory is read-only.
    pub(crate) readonly: c_int,
    /// The number of dimensions.
    pub(crate) ndim: c_int,
    /// The `struct`-module format of an item, NUL-terminated.
    pub(crate) format: *mut c_char,
    /// The extent of each dimension (`ndim` of them).
    pub(crate) shape: *mut Py_ssize_t,
    /// The byte step of each dimension (`ndim` of them).
    pub(crate) strides: *mut Py_ssize_t,
    /// PIL-style indirections (NULL for the arrays read here).
    pub(crate) suboffsets: *mut Py_ssize_t,
    /// The exporter's own.
    pub(crate) internal: *mut c_void,
}

#[cfg_attr(windows, link(name = "python3", kind = "raw-dylib"))]
unsafe extern "C" {
    /// `PyObject_GetBuffer`: fills `view` from an exporter.
    pub(crate) fn PyObject_GetBuffer(
        exporter: *mut PyObject,
        view: *mut Py_buffer,
        flags: c_int,
    ) -> c_int;
    /// `PyBuffer_Release`: releases a view filled by `PyObject_GetBuffer`.
    pub(crate) fn PyBuffer_Release(view: *mut Py_buffer);
}
