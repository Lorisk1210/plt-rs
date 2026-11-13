//! How much of the thread's stack is left, for the native calls code
//! pltrs (Rust) calls back makes (a formatter saving its figure, an
//! artist's `draw` drawing its own Axes, a `draw_event` callback drawing
//! again).
//!
//! Recursion through such calls runs native frames between CPython's own
//! checks. CPython 3.14 checks the stack itself before each Python call,
//! within a few dozen kilobytes of its end: enough for Python's frames,
//! not for a native call drawing a figure. 3.12 and 3.13 only count
//! nested Python calls, which a round of recursion through native code
//! makes few of. Either way the native stack can end first, and the
//! process dies (on macOS under Rosetta it hangs, unkillable). So a native
//! call made inside another one raises matplotlib's `RecursionError`
//! instead once less than [`RESERVE`] is left of the thread's stack
//! (`function::call_nested`).
//!
//! That is where every round of such a recursion passes: the Python code
//! a native call runs reaches native code again only through a native
//! function. The calls back into Python are not checked: some are queries
//! whose failure native code takes as "no answer" and goes on (whether a
//! class of the program's draws its Axes itself), so a refused one would
//! change what is drawn instead of raising.
//!
//! The lowest address the thread's stack may reach is found once per
//! thread ([`end`]: the operating system's bounds of the thread, for the
//! main thread on Linux the limit it may grow to); the check itself
//! compares the address of a local with it. Where the bounds are not
//! known (no such query, a stack that is not the thread's own), the
//! stack never counts as nearly full.

use std::cell::OnceCell;

use super::error::{ExceptionKind, PyErr, PyResult};

#[cfg(target_os = "linux")]
mod linux;

/// The stack a nested native call needs left. Measured on Linux x86-64
/// for savefigs in every format (tight ones too) of figures with nested
/// subfigures, constrained layout, mathtext, usetex, a legend and a
/// colorbar, recursing through a formatter, a locator or an artist's
/// `draw`: a round takes at most 120 KiB of stack from one nested native
/// call to the next (an unoptimised build, a tight PDF save; 112 KiB
/// optimised), and a round's draw goes on at most 59 KiB below its last
/// nested call. Not much
/// more, with margin: a thread of 512 KiB (macOS' for threads Python did
/// not make) still has room for a formatter's nested calls.
const RESERVE: usize = 256 * 1024;

/// The most of a larger stack native recursion may use, below where the
/// thread was first measured: twice Linux's default stack. A round of
/// recursion through native code may nest no Python frames (an artist's
/// `draw` drawing its own Axes is recorded, then replayed natively), so
/// CPython's recursion limit never ends it, and an unlimited stack
/// (`ulimit -s unlimited`) would grow until memory runs out. matplotlib's
/// limit of 1,000 frames lets programs nest a few MiB of pltrs' frames.
const MOST: usize = 16 << 20;

thread_local! {
    /// The lowest address the thread's stack may reach, once measured:
    /// `None` where it is not known.
    static END: OnceCell<Option<usize>> = const { OnceCell::new() };
}

/// Refuses a native call with `RecursionError`, CPython's for a Python
/// call past its limit (as matplotlib's recursion meets it), when less
/// than [`RESERVE`] is left of the thread's stack below the caller.
pub(super) fn check() -> PyResult<()> {
    if nearly_full() {
        Err(PyErr::new(
            ExceptionKind::RecursionError,
            "maximum recursion depth exceeded",
        ))
    } else {
        Ok(())
    }
}

/// Whether less than [`RESERVE`] is left of the calling thread's stack
/// below the caller's frame.
fn nearly_full() -> bool {
    let here = here();
    // Above the stack's end by less than the reserve; an address below
    // the end (a stack other than the thread's own) wraps around to far
    // above it.
    END.with(|end| *end.get_or_init(|| end_of_stack(here)))
        .is_some_and(|end| here.wrapping_sub(end) < RESERVE)
}

/// The address of a local, where the caller's frame ends (about).
fn here() -> usize {
    let marker = 0_u8;
    std::ptr::from_ref(&marker).addr()
}

/// The lowest address native recursion may reach on the calling thread:
/// the end of its stack, if `here` lies in the stack the operating system
/// reports (a thread may run on another), and at most [`MOST`] below
/// `here`.
fn end_of_stack(here: usize) -> Option<usize> {
    end(here)
        .filter(|&end| end < here)
        .map(|end| end.max(here.saturating_sub(MOST)))
}

/// The lowest address the calling thread's stack may reach.
#[cfg(target_os = "linux")]
fn end(here: usize) -> Option<usize> {
    linux::end(here)
}

/// The lowest address of the calling thread's stack (macOS reports the
/// main thread's at its limit, `RLIMIT_STACK`).
#[cfg(target_os = "macos")]
fn end(here: usize) -> Option<usize> {
    // SAFETY: `pthread_self` has no preconditions, and the two queries
    // take the calling thread's own handle, valid while it runs.
    let (high, size) = unsafe {
        let thread = crate::ffi::pthread_self();
        (
            crate::ffi::pthread_get_stackaddr_np(thread).addr(),
            crate::ffi::pthread_get_stacksize_np(thread),
        )
    };
    (here <= high).then(|| high.saturating_sub(size))
}

/// The lowest address of the calling thread's stack (Windows stacks are
/// 2 MB for Python's threads, the main one included).
#[cfg(windows)]
fn end(here: usize) -> Option<usize> {
    let (mut low, mut high) = (0_usize, 0_usize);
    // SAFETY: the function writes the two bounds to the addresses given,
    // both of a live `usize`.
    unsafe { crate::ffi::GetCurrentThreadStackLimits(&raw mut low, &raw mut high) };
    (here <= high).then_some(low)
}

/// Elsewhere the bounds are not measured.
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn end(_here: usize) -> Option<usize> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether a new thread with a stack of `size` bytes finds its stack
    /// nearly full at once.
    fn full_on_a_thread_of(size: usize) -> bool {
        std::thread::Builder::new()
            .stack_size(size)
            .spawn(nearly_full)
            .unwrap()
            .join()
            .unwrap()
    }

    #[test]
    fn a_fresh_stack_is_far_from_full() {
        assert!(!nearly_full());
        assert!(check().is_ok());
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn the_stacks_of_threads_are_measured() {
        assert!(full_on_a_thread_of(RESERVE / 2));
        assert!(!full_on_a_thread_of(4 * RESERVE));
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn the_stack_fills_once_the_reserve_is_reached() {
        /// Recurses with a kilobyte per frame until the stack is nearly
        /// full: how much of it was left there.
        fn left() -> usize {
            let frame = std::hint::black_box([0_u8; 1024]);
            if nearly_full() {
                let here = here();
                return here - end_of_stack(here).unwrap() + usize::from(frame[0]);
            }
            left()
        }
        let left = std::thread::Builder::new()
            .stack_size(4 * RESERVE)
            .spawn(left)
            .unwrap()
            .join()
            .unwrap();
        assert!((RESERVE - 4096..RESERVE).contains(&left), "{left}");
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn an_address_off_the_threads_stack_has_no_end() {
        assert_eq!(end_of_stack(0), None);
        assert!(end_of_stack(here()).is_some_and(|end| end < here()));
    }
}
