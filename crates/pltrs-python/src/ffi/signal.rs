//! The POSIX signal API the binding uses on Linux, beside CPython's: a
//! handler of the fatal faults that reports the address a fault touched
//! (`binding::fault`).

use std::ffi::{c_int, c_void};

/// `SIGBUS`, on x86-64 and 64-bit Arm.
pub(crate) const SIGBUS: c_int = 7;
/// `SIGSEGV`.
pub(crate) const SIGSEGV: c_int = 11;
/// `SA_SIGINFO`: the handler takes the signal's `siginfo_t`.
pub(crate) const SA_SIGINFO: c_int = 4;
/// `SA_ONSTACK`: the handler runs on the thread's alternate signal stack,
/// if it has one (Python's `faulthandler` gives the thread enabling it
/// one), so that it can run once the thread's own stack is full.
pub(crate) const SA_ONSTACK: c_int = 0x0800_0000;

/// A handler taking the signal's number, its `siginfo_t` and the context
/// the signal interrupted.
pub(crate) type sa_sigaction_t = unsafe extern "C" fn(c_int, *mut c_void, *mut c_void);

/// `struct sigaction`, as glibc and musl lay it out on x86-64 and 64-bit
/// Arm: the handler (a union of the two kinds, the size of an address),
/// the mask (a `sigset_t` of 1,024 bits), the flags and the restorer.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct sigaction_t {
    /// The handler, `SIG_DFL` (0) or `SIG_IGN` (1).
    pub(crate) sa_sigaction: usize,
    /// The signals blocked while the handler runs.
    pub(crate) sa_mask: [u64; 16],
    /// `SA_*` flags.
    pub(crate) sa_flags: c_int,
    /// Set by the C library.
    pub(crate) sa_restorer: usize,
}

/// Where `si_addr` is in a `siginfo_t` on 64-bit Linux, in addresses
/// from its start: the third, after `si_signo`, `si_errno` and `si_code`
/// (three `int`s, and padding to an address).
pub(crate) const SI_ADDR_INDEX: usize = 2;

unsafe extern "C" {
    /// `sigaction`: installs `act` for `signum` (if not NULL) and stores
    /// the action it replaces in `oldact` (if not NULL); 0 on success.
    pub(crate) fn sigaction(
        signum: c_int,
        act: *const sigaction_t,
        oldact: *mut sigaction_t,
    ) -> c_int;
    /// `write`: writes up to `count` bytes of `buf` to `fd`
    /// (async-signal-safe).
    pub(crate) fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
}
