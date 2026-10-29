//! Waking a blocked event loop from other threads.
//!
//! The loop blocks reading its socket, so the waker makes the server send
//! it something: a `ClientMessage` to a hidden window of the loop's own
//! connection. Wake-ups are coalesced: while one message is in flight,
//! further calls send nothing.

use std::sync::atomic::{AtomicBool, Ordering};

use super::output::SharedOutput;
use super::request;
use crate::backend::Waker;
use crate::error::Error;

/// Wakes an X11 event loop.
#[derive(Debug)]
pub(crate) struct X11Waker {
    output: SharedOutput,
    window: u32,
    message_type: u32,
    /// Whether a wake-up message is on its way.
    pending: AtomicBool,
}

impl X11Waker {
    /// A waker sending `message_type` messages to `window` through
    /// `output`.
    pub(crate) fn new(output: SharedOutput, window: u32, message_type: u32) -> Self {
        Self {
            output,
            window,
            message_type,
            pending: AtomicBool::new(false),
        }
    }

    /// Whether a received `ClientMessage` is a wake-up; if so, later wake
    /// calls send a new message.
    pub(crate) fn accept(&self, window: u32, message_type: u32) -> bool {
        let wake_up = window == self.window && message_type == self.message_type;
        if wake_up {
            self.pending.store(false, Ordering::SeqCst);
        }
        wake_up
    }
}

impl Waker for X11Waker {
    fn wake(&self) -> Result<(), Error> {
        if self.pending.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let message = request::client_message(self.window, self.message_type, [0; 5]);
        let mut output = self.output.lock();
        let result = output
            .send(
                request::send_event(self.window, 0, &message).encoded(),
                false,
            )
            .and_then(|_| output.flush());
        if result.is_err() {
            self.pending.store(false, Ordering::SeqCst);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::os::unix::net::UnixStream;
    use std::sync::Arc;

    use super::*;
    use crate::x11::transport::Stream;

    #[test]
    fn coalesces_until_accepted_and_fails_when_closed() {
        let (ours, mut theirs) = UnixStream::pair().unwrap();
        let output = SharedOutput::new(Arc::new(Stream::Unix(ours)), 1 << 16);
        let waker = X11Waker::new(output.clone(), 0x77, 0x1ab);
        waker.wake().unwrap();
        waker.wake().unwrap();
        let mut request = [0; 44];
        theirs.read_exact(&mut request).unwrap();
        assert_eq!(
            &request[..8],
            [25, 0, 11, 0, 0x77, 0, 0, 0],
            "SendEvent to the window"
        );
        assert_eq!(request[12], 33, "a ClientMessage");
        assert!(!waker.accept(0x77, 0x1ac));
        assert!(waker.accept(0x77, 0x1ab));
        waker.wake().unwrap();
        theirs.read_exact(&mut request).unwrap();
        output.lock().close();
        assert!(waker.accept(0x77, 0x1ab));
        assert_eq!(waker.wake(), Err(Error::EventLoopClosed));
    }
}
