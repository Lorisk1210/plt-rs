//! The native extension of pltrs (Python): pltrs with matplotlib's syntax.
//!
//! The Python package `pltrs` (in `python/pltrs`) mirrors matplotlib's
//! modules and classes; this crate is the native side its classes call,
//! installed as `pltrs/_native.abi3.so` (`cargo xtask python`). The
//! architecture, for those extending it, is in `python/DESIGN.md`.
//!
//! * `ffi` declares the part of CPython's stable ABI (abi3, limited API
//!   3.12) the crate uses, written by hand: no PyO3, no crates at all.
//! * `binding` wraps it in safe Rust: the GIL token, owned references,
//!   exceptions, conversions, numpy arrays through the buffer protocol,
//!   the native object holding Rust values for Python, native functions
//!   and callbacks into Python. These two modules hold all the `unsafe`
//!   code of the crate, each block with the reason it is sound.
//! * `modules` has one file per matplotlib module (`rcsetup`,
//!   `figure`, `axes`, `lines`, ...), each registering the native
//!   functions of the submodule of `_native` its Python module calls.
//!
//! The crate lifts the workspace's ban on `unsafe` code, as only it and
//! `pltrs-platform` may: the CPython API is C.
// Without an interpreter to embed (see build.rs), the unit tests could not
// link, so the test build of the crate is empty.
#![cfg(any(not(test), pltrs_python_embed))]
#![deny(unsafe_code)]
#![warn(clippy::undocumented_unsafe_blocks)]

mod binding;
mod ffi;
mod modules;

/// The extension's initialisation function, which CPython looks up by
/// name when `pltrs._native` is imported.
#[allow(
    unsafe_code,
    reason = "CPython finds the module's initialisation function by its exported name"
)]
#[unsafe(no_mangle)]
pub extern "C" fn PyInit__native() -> *mut std::ffi::c_void {
    binding::init_module(modules::register).cast()
}
