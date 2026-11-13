//! The POSIX threads API the binding uses, beside CPython's: the bounds
//! of the calling thread's stack (glibc's and musl's `pthread_getattr_np`
//! on Linux, the `_np` queries of macOS).

#[cfg(target_os = "linux")]
use std::ffi::c_int;
use std::ffi::c_void;

/// `pthread_t`: an `unsigned long` in glibc, a pointer in musl and on
/// macOS, the size of an address on every target pltrs supports.
pub(crate) type pthread_t = usize;

/// `pthread_attr_t`, opaque: 56 bytes in glibc on x86-64 and in musl, 64
/// in glibc on 64-bit Arm, aligned as a `long`; this is as large as the
/// largest.
#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug)]
pub(crate) struct pthread_attr_t {
    _opaque: [u64; 8],
}

#[cfg(target_os = "linux")]
impl pthread_attr_t {
    /// Storage for `pthread_getattr_np` to initialise.
    pub(crate) const fn uninit() -> Self {
        Self { _opaque: [0; 8] }
    }
}

unsafe extern "C" {
    /// `pthread_self`: the calling thread.
    pub(crate) fn pthread_self() -> pthread_t;
}

#[cfg(target_os = "linux")]
unsafe extern "C" {
    /// `pthread_getattr_np`: initialises `attr` with the attributes of the
    /// running thread `thread`, its stack among them; 0 on success.
    pub(crate) fn pthread_getattr_np(thread: pthread_t, attr: *mut pthread_attr_t) -> c_int;
    /// `pthread_attr_getstack`: the lowest address and the size of the
    /// stack `attr` describes; 0 on success.
    pub(crate) fn pthread_attr_getstack(
        attr: *const pthread_attr_t,
        addr: *mut *mut c_void,
        size: *mut usize,
    ) -> c_int;
    /// `pthread_attr_destroy`: releases what `pthread_getattr_np` gave
    /// `attr`.
    pub(crate) fn pthread_attr_destroy(attr: *mut pthread_attr_t) -> c_int;
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    /// `pthread_get_stackaddr_np`: the highest address of the stack of
    /// `thread` (where it starts, growing down).
    pub(crate) fn pthread_get_stackaddr_np(thread: pthread_t) -> *mut c_void;
    /// `pthread_get_stacksize_np`: the size of the stack of `thread`.
    pub(crate) fn pthread_get_stacksize_np(thread: pthread_t) -> usize;
}
