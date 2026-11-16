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
//! function. The calls native code makes into Python are checked too, with
//! less left ([`TO_PYTHON`]), only so that CPython 3.14's own check of the
//! stack never refuses a round first: its `RecursionError` says "Stack
//! overflow (used 478 kB)", not matplotlib's "maximum recursion depth
//! exceeded", and which of the two a recursion ends with would depend on
//! where in a round the stack happens to run out (an admitted native call
//! goes some 40 KiB deeper before it calls the program's formatter).
//!
//! The bounds of the thread's stack are found once per thread
//! ([`bounds`]: the operating system's, for the main thread on Linux the
//! limit it may grow to); the check itself compares the address of a
//! local with the end. Where the bounds are not known (no such query, a
//! stack that is not the thread's own), the stack never counts as nearly
//! full. On a thread whose whole stack is small (musl's default of 128
//! KiB, a size a program set), the error says so and what to do
//! ([`message`]).

use std::cell::OnceCell;

use super::error::{ExceptionKind, PyErr, PyResult};
use super::module::Module;

#[cfg(target_os = "linux")]
mod linux;

/// The stack a nested native call needs left: what the deepest native
/// stretch below a nested call takes, with margin. Measured on Linux
/// (x86-64, glibc and musl) for saves in every format, tight ones too, of
/// figures with nested subfigures, constrained layout, mathtext, a legend
/// and a colorbar, recursing through a formatter, a locator or an
/// artist's `draw`: a round takes at most 44 KiB from one nested call to
/// the next (an optimised build, a tight PDF save), and a call goes on at
/// most 47 KiB below the check that let it in, the nested calls it makes
/// refused (an SVG save under constrained layout drawing its tick labels;
/// 46 KiB optimised; on arm64 macOS 49 and 46), which `test_stack1.py`
/// keeps 12 KiB under the reserve of the system it runs on. Shaping a new
/// line of text and typesetting a formula, which go deeper with their
/// input, run where the stack has room for them (`pltrs_text`,
/// `pltrs_core::stack`). So a thread of 128 KiB (musl's default) draws
/// with callbacks of the program's, as matplotlib does there.
const RESERVE: usize = 64 * 1024;

/// The stack a call from native code into Python needs left. CPython 3.14
/// refuses a Python call once less than twice `_PyOS_STACK_MARGIN_BYTES`
/// (2,048 pointers, 16 KiB on 64-bit systems) is left above the end of
/// the thread's stack, as it reads the bounds: 32 KiB (`Stack overflow`;
/// measured: a thread of 256 KiB, 512 KiB or 1 MiB refuses with 36 KiB of
/// what was asked for left, its guard page included). Refusing here with
/// 16 KiB more leaves the Python code a callback runs (a formatter calling
/// `savefig`: 4.4 KiB of CPython's frames) room to reach pltrs' own check,
/// so that every round ends with matplotlib's message. CPython 3.12 and
/// 3.13 do not check the stack (they count nested C calls, 10,000).
const TO_PYTHON: usize = 48 * 1024;

/// The most of a larger stack native recursion may use, below where the
/// thread was first measured: twice Linux's default stack. A round of
/// recursion through native code may nest no Python frames (an artist's
/// `draw` drawing its own Axes is recorded, then replayed natively), so
/// CPython's recursion limit never ends it, and an unlimited stack
/// (`ulimit -s unlimited`) would grow until memory runs out. matplotlib's
/// limit of 1,000 frames lets programs nest a few MiB of pltrs' frames.
const MOST: usize = 16 << 20;

/// The size below which a thread's stack counts as small, and its
/// `RecursionError` says so: 1 MiB, what the Pythons of Alpine Linux and
/// Docker's Alpine images give threads.
const SMALL: usize = 1 << 20;

/// The first line of the error: CPython's for a Python call past its
/// limit, as matplotlib's recursion meets it (and as code matching on it
/// expects).
const FIRST_LINE: &str = "maximum recursion depth exceeded";

/// The lowest and the highest address of a thread's stack, as the
/// operating system bounds it (for the main thread on Linux, the limit it
/// may grow to).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Bounds {
    end: usize,
    top: usize,
}

/// What the guard knows of the calling thread's stack.
#[derive(Clone, Copy, Debug)]
struct Stack {
    /// The lowest address native recursion may reach (see [`MOST`]).
    end: usize,
    /// The size of the whole stack.
    size: usize,
}

thread_local! {
    /// The thread's stack, once measured: `None` where it is not known.
    static STACK: OnceCell<Option<Stack>> = const { OnceCell::new() };
}

/// Refuses a native call with `RecursionError` ([`message`]) when less
/// than [`RESERVE`] is left of the thread's stack below the caller.
pub(super) fn check() -> PyResult<()> {
    refused_below(RESERVE)
}

/// Refuses a call from native code into Python with `RecursionError`
/// ([`message`]) when less than [`TO_PYTHON`] is left of the thread's
/// stack below the caller.
pub(super) fn check_call_into_python() -> PyResult<()> {
    refused_below(TO_PYTHON)
}

/// `RecursionError` ([`message`]) when less than `reserve` is left of the
/// thread's stack below the caller.
fn refused_below(reserve: usize) -> PyResult<()> {
    match nearly_full(reserve) {
        Some(size) => Err(PyErr::new(ExceptionKind::RecursionError, message(size))),
        None => Ok(()),
    }
}

/// Measures the calling thread's stack, once: pltrs (Rust) then knows
/// where it ends, and runs work that goes deep where it has room for it
/// (`pltrs_core::stack`). Called by the thread's outermost native call.
pub(super) fn measure() {
    measured(here());
}

/// The calling thread's stack, measured the first time (`here` being an
/// address on it), when pltrs (Rust) is told where it ends.
fn measured(here: usize) -> Option<Stack> {
    STACK.with(|stack| {
        *stack.get_or_init(|| {
            let stack = stack_of(here);
            pltrs_core::stack::set_end(stack.map(|stack| stack.end));
            stack
        })
    })
}

/// The size of the calling thread's stack when less than `reserve` is
/// left of it below the caller's frame.
fn nearly_full(reserve: usize) -> Option<usize> {
    let here = here();
    // Above the stack's end by less than the reserve; an address below
    // the end (a stack other than the thread's own) wraps around to far
    // above it.
    measured(here)
        .filter(|stack| here.wrapping_sub(stack.end) < reserve)
        .map(|stack| stack.size)
}

/// The error's message for a thread whose stack is `size` bytes: Python's
/// own first line, then, for a small stack, why and what to do. A large
/// stack ends this way only when a program recurses through drawing,
/// which matplotlib ends with the first line alone.
fn message(size: usize) -> String {
    if size >= SMALL {
        return FIRST_LINE.to_owned();
    }
    format!(
        "{FIRST_LINE}\npltrs: this thread's stack is {} KiB, and a call into pltrs from \
         code pltrs runs (a formatter, a callback) needs {} KiB of it left; start the \
         thread after threading.stack_size({SMALL}) to give it more",
        size / 1024,
        RESERVE / 1024,
    )
}

/// Adds `_stack_room()` to the extension: where the stack of the calling
/// thread is, for the smoke tests that measure how much of it a draw
/// takes (`test_stack1.py`) on any system.
pub(crate) fn add_to(module: &Module<'_>) -> PyResult<()> {
    module.add("_stack_room", stack_room)
}

/// The address of a local of this native call, the lowest address native
/// recursion may reach on the calling thread (0 where not known) and the
/// stack a nested native call needs left ([`RESERVE`]).
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function returns a result"
)]
fn stack_room() -> PyResult<(usize, usize, usize)> {
    let here = here();
    Ok((here, measured(here).map_or(0, |stack| stack.end), RESERVE))
}

/// The address of a local, where the caller's frame ends (about).
fn here() -> usize {
    let marker = 0_u8;
    std::ptr::from_ref(&marker).addr()
}

/// The calling thread's stack, if `here` lies in the stack the operating
/// system reports (a thread may run on another): its size, and the lowest
/// address native recursion may reach, its end but at most [`MOST`] below
/// `here`.
fn stack_of(here: usize) -> Option<Stack> {
    bounds(here)
        .filter(|bounds| bounds.end < here)
        .map(|bounds| Stack {
            end: bounds.end.max(here.saturating_sub(MOST)),
            size: bounds.top.saturating_sub(bounds.end),
        })
}

/// The bounds of the calling thread's stack.
#[cfg(target_os = "linux")]
fn bounds(here: usize) -> Option<Bounds> {
    linux::bounds(here)
}

/// The bounds of the calling thread's stack (macOS reports the main
/// thread's at its limit, `RLIMIT_STACK`).
#[cfg(target_os = "macos")]
fn bounds(here: usize) -> Option<Bounds> {
    // SAFETY: `pthread_self` has no preconditions, and the two queries
    // take the calling thread's own handle, valid while it runs.
    let (high, size) = unsafe {
        let thread = crate::ffi::pthread_self();
        (
            crate::ffi::pthread_get_stackaddr_np(thread).addr(),
            crate::ffi::pthread_get_stacksize_np(thread),
        )
    };
    (here <= high).then(|| Bounds {
        end: high.saturating_sub(size),
        top: high,
    })
}

/// The bounds of the calling thread's stack (Windows stacks are 2 MB for
/// Python's threads, the main one included).
#[cfg(windows)]
fn bounds(here: usize) -> Option<Bounds> {
    let (mut low, mut high) = (0_usize, 0_usize);
    // SAFETY: the function writes the two bounds to the addresses given,
    // both of a live `usize`.
    unsafe { crate::ffi::GetCurrentThreadStackLimits(&raw mut low, &raw mut high) };
    (here <= high).then_some(Bounds {
        end: low,
        top: high,
    })
}

/// Elsewhere the bounds are not measured.
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn bounds(_here: usize) -> Option<Bounds> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On a new thread asking for a stack of `size` bytes: the size of the
    /// stack it got (glibc may hand it a cached one up to four times as
    /// large) and whether that is nearly full at once.
    fn on_a_thread_of(size: usize) -> (usize, bool) {
        std::thread::Builder::new()
            .stack_size(size)
            .spawn(|| {
                let size = stack_of(here()).map_or(0, |stack| stack.size);
                (size, nearly_full(RESERVE).is_some())
            })
            .unwrap()
            .join()
            .unwrap()
    }

    #[test]
    fn a_fresh_stack_is_far_from_full() {
        assert_eq!(nearly_full(RESERVE), None);
        assert!(check().is_ok());
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn the_stacks_of_threads_are_measured() {
        let (size, full) = on_a_thread_of(RESERVE / 2);
        assert_eq!(full, size <= RESERVE, "{size}");
        assert!(!on_a_thread_of(4 * RESERVE).1);
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn the_stack_fills_once_the_reserve_is_reached() {
        /// Recurses with a kilobyte per frame until the stack is nearly
        /// full: how much of it was left there.
        fn left() -> usize {
            let frame = std::hint::black_box([0_u8; 1024]);
            if nearly_full(RESERVE).is_some() {
                let here = here();
                return here - stack_of(here).unwrap().end + usize::from(frame[0]);
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
        assert!(stack_of(0).is_none());
        assert!(stack_of(here()).is_some_and(|stack| stack.end < here()));
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn a_small_stack_is_named_with_its_size() {
        // At least what was asked for, at most a cached stack four times
        // as large (and its guard page).
        let (size, _) = on_a_thread_of(2 * RESERVE);
        assert!((2 * RESERVE..=8 * RESERVE + 4096).contains(&size), "{size}");
    }

    #[test]
    fn the_message_explains_a_small_stack_only() {
        assert_eq!(message(8 << 20), "maximum recursion depth exceeded");
        assert_eq!(message(SMALL), "maximum recursion depth exceeded");
        let small = message(130 * 1024);
        let (first, rest) = small.split_once('\n').unwrap();
        assert_eq!(first, "maximum recursion depth exceeded");
        assert!(
            rest.contains("130 KiB") && rest.contains("64 KiB"),
            "{rest}"
        );
        assert!(rest.contains("threading.stack_size(1048576)"), "{rest}");
    }
}
