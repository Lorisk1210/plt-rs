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
//!   for native code ([`registry::lookup`] finds any registered object);
//! * [`threads`] — why every thread holding the GIL may use every pltrs
//!   object, and the pltrs (Rust) state all of them share;
//! * [`reentrant`] — changes Python code makes to a figure pltrs (Rust) is
//!   laying out or drawing, applied once it is done;
//! * [`deferred`] — work Python leaves to the next native call;
//! * [`own_state`] — the Python objects of artists told before a native
//!   call reaches their artist, to hand over the state they keep;
//! * [`Watched`] — attributes of the package a program may replace, read
//!   where pltrs (Rust) needs to know whether they are;
//! * the extension's allocator (Linux): the system's, with transparent
//!   huge pages asked for large blocks.
//!
//! This is the only module besides [`crate::ffi`] with `unsafe` code;
//! every block says why it is sound. The matplotlib modules
//! ([`crate::modules`]) use it through [`prelude`].
#![allow(
    unsafe_code,
    reason = "the binding core is the safe wrapper around the CPython API"
)]

mod after_call;
#[cfg(target_os = "linux")]
mod allocator;
mod blocking;
mod buffer;
mod callback;
mod containers;
mod convert;
mod deferred;
mod error;
#[cfg(target_os = "linux")]
mod fault;
mod function;
mod gil;
mod home;
mod init;
mod module;
mod native;
mod object;
mod own_state;
mod reentrant;
mod registry;
mod stack;
#[cfg(test)]
pub(crate) mod testing;
mod threads;
mod watched;

pub(crate) use after_call::install as install_after_call;
pub(crate) use blocking::releasing_gil;
pub(crate) use buffer::{Element, FloatArray, NdArray, with_item_bytes};
pub(crate) use callback::{
    Callback, current_call, failed as callback_failed, forgive, report, setting_failure_aside,
};
pub(crate) use containers::{dict_items, dict_value, new_dict, tuple};
pub(crate) use convert::{Bytes, FromPy, IntoPy};
pub(crate) use deferred::install as install_deferred;
pub(crate) use error::{ExceptionKind, PyErr, PyResult, runtime_warning};
pub(crate) use gil::Python;
pub(crate) use home::{set_loop_waker, wake_loop};
pub(crate) use init::init_module;
pub(crate) use module::Module;
pub(crate) use native::{
    Native, artist_handle, live_objects, native_value, retyped, set_asker, wrap as wrap_native,
    wrap_artist, wrap_thread_bound,
};
pub(crate) use object::{Bound, Owned};
pub(crate) use own_state::{PASSING_BY, install_lists as install_own_lists};
#[cfg(test)]
pub(crate) use registry::register;
pub(crate) use registry::{lookup, registered_error};
pub(crate) use watched::Watched;

/// Everything a matplotlib module's file needs.
pub(crate) mod prelude {
    pub(crate) use super::{
        Bound, Bytes, Callback, ExceptionKind, FromPy, IntoPy, Module, Native, NdArray, Owned,
        PyErr, PyResult, Python, artist_handle, native_value,
    };
}
