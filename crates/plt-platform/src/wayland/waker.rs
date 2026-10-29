//! Waking a blocked event loop from other threads.
//!
//! The loop blocks reading its socket, so the waker makes the compositor
//! send it something: it asks for a `wl_display.sync` callback, always
//! with the id reserved for it. Wake-ups are coalesced: while one
//! callback is pending, further calls send nothing, so at most one such
//! object exists at a time. The connection's first request is such a
//! callback, so the waker starts with one pending.

use std::sync::atomic::{AtomicBool, Ordering};

use super::connection::WAKE_CALLBACK;
use super::protocol::core;
use super::transport::SharedOutput;
use crate::backend::Waker;
use crate::error::Error;

/// Wakes a Wayland event loop.
#[derive(Debug)]
pub(crate) struct WaylandWaker {
    output: SharedOutput,
    /// Whether a wake-up callback is pending.
    pending: AtomicBool,
    /// Whether the loop was asked to wake up since the last callback.
    requested: AtomicBool,
}

impl WaylandWaker {
    /// A waker writing through `output`, whose connection sent the first
    /// callback already.
    pub(crate) fn new(output: SharedOutput) -> Self {
        Self {
            output,
            pending: AtomicBool::new(true),
            requested: AtomicBool::new(false),
        }
    }

    /// Notes that the wake-up callback is done, so that later wake calls
    /// send a new one; returns whether a wake-up was asked for.
    pub(crate) fn accept(&self) -> bool {
        self.pending.store(false, Ordering::SeqCst);
        self.requested.swap(false, Ordering::SeqCst)
    }
}

impl Waker for WaylandWaker {
    fn wake(&self) -> Result<(), Error> {
        self.requested.store(true, Ordering::SeqCst);
        if self.pending.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let (bytes, fds) = core::sync(WAKE_CALLBACK).finish();
        let mut output = self.output.lock();
        let result = output.send(&bytes, &fds).and_then(|()| output.flush());
        if result.is_err() {
            self.pending.store(false, Ordering::SeqCst);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixStream;
    use std::sync::Arc;

    use super::*;
    use crate::wayland::sys;

    #[test]
    fn coalesces_until_accepted_and_fails_when_closed() {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let output = SharedOutput::new(Arc::new(ours));
        let waker = WaylandWaker::new(output.clone());
        assert!(!waker.accept(), "the connection's own callback");
        waker.wake().unwrap();
        waker.wake().unwrap();
        let mut buf = [0; 64];
        let mut fds = Vec::new();
        let n = sys::receive(&theirs, &mut buf, &mut fds, true).unwrap();
        assert_eq!(n, 12, "one sync request");
        assert_eq!(&buf[8..12], WAKE_CALLBACK.to_ne_bytes());
        assert!(waker.accept());
        assert!(!waker.accept(), "each wake-up is reported once");
        waker.wake().unwrap();
        assert_eq!(sys::receive(&theirs, &mut buf, &mut fds, true).unwrap(), 12);
        output.lock().close();
        waker.accept();
        assert_eq!(waker.wake(), Err(Error::EventLoopClosed));
    }
}
