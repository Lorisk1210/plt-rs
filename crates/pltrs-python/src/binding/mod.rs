//! The binding core: safe Rust over the CPython API of [`crate::ffi`].
//!
//! * [`Python`] — the token proving the GIL is held;
//! * [`Bound`] and [`Owned`] — strong references to Python objects, tied
//!   to the GIL or kept by Rust state;
//! * [`PyErr`] and [`PyResult`] — exceptions, raised when a native
//!   function returns `Err`;
//! * [`FromPy`] and [`IntoPy`] — conversions of arguments and results;
//!   [`NdArray`] for numbers in bulk through the buffer protocol;
//! * [`Native`] and the [`native_value!`] / [`artist_handle!`] macros —
//!   the native object that holds a Rust value for Python;
//! * [`Module`] — the extension's modules and their native functions;
//! * [`Callback`] — Python callables pltrs (Rust) calls;
//! * [`registered_error`] — exceptions of the classes the package registers
//!   for native code ([`registry::lookup`] finds any registered object).
//!
//! This is the only module besides [`crate::ffi`] with `unsafe` code;
//! every block says why it is sound. The matplotlib modules
//! ([`crate::modules`]) use it through [`prelude`].
#![allow(
    unsafe_code,
    reason = "the binding core is the safe wrapper around the CPython API"
)]

mod buffer;
mod callback;
mod containers;
mod convert;
mod error;
mod function;
mod gil;
mod init;
mod module;
mod native;
mod object;
mod registry;
#[cfg(test)]
pub(crate) mod testing;

pub(crate) use buffer::{Element, NdArray};
pub(crate) use callback::Callback;
pub(crate) use containers::{dict_items, dict_value};
pub(crate) use convert::{Bytes, FromPy, IntoPy};
pub(crate) use error::{ExceptionKind, PyErr, PyResult};
pub(crate) use gil::Python;
pub(crate) use init::init_module;
pub(crate) use module::Module;
pub(crate) use native::{
    Native, artist_handle, native_value, retyped, wrap as wrap_native, wrap_artist,
};
pub(crate) use object::{Bound, Owned};
pub(crate) use registry::registered_error;

/// Everything a matplotlib module's file needs.
pub(crate) mod prelude {
    pub(crate) use super::{
        Bound, Bytes, Callback, ExceptionKind, FromPy, IntoPy, Module, Native, NdArray, Owned,
        PyErr, PyResult, Python, artist_handle, native_value,
    };
}
