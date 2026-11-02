//! The bookkeeping of the native backends' wakers (macOS, Windows): they
//! interrupt the loop's wait by posting a message to its thread's queue,
//! and these flags coalesce wake-ups so that at most one such message is
//! in flight, and fail them once the loop is gone.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::Error;

/// Whether a loop is alive and whether a wake-up is pending.
#[derive(Debug)]
pub(crate) struct WakeFlags {
    alive: AtomicBool,
    pending: AtomicBool,
}

impl WakeFlags {
    /// Flags of a live loop with no wake-up pending.
    pub(crate) const fn new() -> Self {
        Self {
            alive: AtomicBool::new(true),
            pending: AtomicBool::new(false),
        }
    }

    /// Asks for a wake-up: posts it with `post` unless one is pending
    /// already. Fails with [`Error::EventLoopClosed`] once the loop is
    /// gone, and with `post`'s error, after which the next call tries
    /// again.
    pub(crate) fn wake(&self, post: impl FnOnce() -> Result<(), Error>) -> Result<(), Error> {
        if !self.alive.load(Ordering::SeqCst) {
            return Err(Error::EventLoopClosed);
        }
        if self.pending.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        post().inspect_err(|_| self.pending.store(false, Ordering::SeqCst))
    }

    /// Takes the pending wake-up: whether the loop should deliver
    /// [`Event::WakeUp`](crate::Event::WakeUp).
    pub(crate) fn take(&self) -> bool {
        self.pending.swap(false, Ordering::SeqCst)
    }

    /// Marks the loop as gone: later wake-ups fail.
    pub(crate) fn close(&self) {
        self.alive.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn wake_ups_coalesce_until_taken() {
        let flags = WakeFlags::new();
        let posts = Cell::new(0);
        let post = || {
            posts.set(posts.get() + 1);
            Ok(())
        };
        assert!(!flags.take());
        flags.wake(post).unwrap();
        flags.wake(post).unwrap();
        assert_eq!(posts.get(), 1);
        assert!(flags.take());
        assert!(!flags.take());
        flags.wake(post).unwrap();
        assert_eq!(posts.get(), 2);
    }

    #[test]
    fn failed_posts_are_retried_and_closed_loops_refuse() {
        let flags = WakeFlags::new();
        assert_eq!(
            flags.wake(|| Err(Error::ConnectionClosed)),
            Err(Error::ConnectionClosed)
        );
        assert!(!flags.take(), "nothing was posted");
        flags.wake(|| Ok(())).unwrap();
        flags.close();
        assert_eq!(flags.wake(|| Ok(())), Err(Error::EventLoopClosed));
    }
}
