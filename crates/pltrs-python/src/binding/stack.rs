//! How much of the thread's stack is left, for the native calls code
//! pltrs (Rust) calls back makes (a formatter saving its figure).
//!
//! Recursion through such calls runs native frames between CPython's own
//! stack checks, which stop it within a few dozen kilobytes of the end of
//! the stack: enough for Python's frames, not for a native call drawing a
//! figure. Where stacks are small, Windows' 2 MB (Linux gives 8 MB, where
//! Python's frame limit ends the recursion first), a nested native call
//! raises `RecursionError` while 256 KiB are left instead.

/// The stack a nested native call needs left (the native frames of one
/// round of recursion through a draw take about 50 KiB unoptimised).
#[cfg(windows)]
const RESERVE: usize = 256 * 1024;

/// Whether less than [`RESERVE`] is left of the calling thread's stack
/// below this frame.
#[cfg(windows)]
pub(super) fn nearly_full() -> bool {
    let (mut low, mut high) = (0_usize, 0_usize);
    // SAFETY: the function writes the two bounds to the addresses given,
    // both of a live `usize`.
    unsafe { crate::ffi::GetCurrentThreadStackLimits(&raw mut low, &raw mut high) };
    let here = std::ptr::from_ref(&low).addr();
    here.saturating_sub(low) < RESERVE
}

/// Whether too little of the calling thread's stack is left: never, the
/// stacks being large.
#[cfg(not(windows))]
pub(super) const fn nearly_full() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_stack_is_far_from_full() {
        assert!(!nearly_full());
    }

    #[cfg(windows)]
    #[test]
    fn a_stack_smaller_than_the_reserve_is_nearly_full() {
        let full = std::thread::Builder::new()
            .stack_size(RESERVE / 2)
            .spawn(nearly_full)
            .unwrap()
            .join()
            .unwrap();
        assert!(full);
    }
}
