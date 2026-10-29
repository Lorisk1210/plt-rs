//! The raw CPython C API this crate uses: declarations only.
//!
//! Every function, constant and struct here belongs to CPython's limited
//! API as of 3.12 (`Py_LIMITED_API = 0x030c0000`), whose binary interface
//! (the stable ABI, "abi3") holds for every later CPython: the extension
//! built from this crate loads in any of them. Nothing here depends on
//! CPython's struct layouts beyond what that API exposes: the object
//! header (`ob_refcnt`, `ob_type`), `Py_buffer`, and the structs that
//! describe types, methods and modules. The extension links against no
//! libpython; the interpreter that imports it provides the symbols.
//!
//! The rest of the crate never calls these functions directly: the
//! [`binding`](crate::binding) module wraps them in safe types, each
//! `unsafe` block there with the reason it is sound.
//!
//! Names keep CPython's spelling, so that its documentation applies.
#![allow(
    unsafe_code,
    reason = "declaring the CPython API is the crate's FFI layer"
)]
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    clippy::struct_field_names,
    reason = "the declarations keep CPython's names"
)]

mod buffer;
mod errors;
mod object;
mod runtime;
mod types;
mod values;

pub(crate) use buffer::*;
pub(crate) use errors::*;
pub(crate) use object::*;
pub(crate) use runtime::*;
pub(crate) use types::*;
pub(crate) use values::*;

/// `Py_ssize_t`.
pub(crate) type Py_ssize_t = isize;

/// The header every Python object starts with (`PyObject`): its
/// reference count and its type. The limited API exposes both fields in
/// the default (GIL) build, the only one abi3 supports.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct PyObject {
    /// `ob_refcnt`; never touched here (`Py_IncRef` and `Py_DecRef` are).
    pub(crate) ob_refcnt: Py_ssize_t,
    /// `ob_type`: the object's type, borrowed.
    pub(crate) ob_type: *mut PyTypeObject,
}

/// A type object (`PyTypeObject`), opaque in the limited API.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct PyTypeObject {
    _opaque: [u8; 0],
}

// The unit tests embed an interpreter: they link the libpython that
// `build.rs` found.
#[cfg(all(test, pltrs_python_embed))]
#[link(name = "pltrs_python_embed")]
unsafe extern "C" {}
