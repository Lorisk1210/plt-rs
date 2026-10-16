//! The sending half of a connection, shared by the event loop and the
//! wakers on other threads.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::request::{self, Encoded};
use super::transport::Stream;
use crate::error::Error;

/// Buffered requests are written once this many bytes accumulate; larger
/// requests bypass the buffer.
const FLUSH_THRESHOLD: usize = 64 * 1024;

/// After this many requests without a reply, a reply-bearing request is
/// slipped in, so the server answers at least once every 2¹⁶ requests and
/// 16-bit sequence numbers stay unambiguous.
const SYNC_INTERVAL: u32 = 65_000;

/// Buffers requests and numbers them.
#[derive(Debug)]
pub(crate) struct Output {
    stream: Arc<Stream>,
    buffer: Vec<u8>,
    /// Sequence number of the last request sent.
    sequence: u64,
    /// Requests sent since the last one with a reply.
    unanswered: u32,
    max_request_bytes: usize,
    closed: bool,
}

impl Output {
    /// Queues a request and returns its sequence number. Requests are
    /// written when the buffer fills up or on [`Output::flush`].
    pub(crate) fn send(&mut self, request: &Encoded, has_reply: bool) -> Result<u64, Error> {
        if self.closed {
            return Err(Error::EventLoopClosed);
        }
        if request.wire_len() > self.max_request_bytes {
            return Err(Error::InvalidArgument(
                "request exceeds the X server's maximum request length",
            ));
        }
        if !has_reply && self.unanswered >= SYNC_INTERVAL {
            self.push(request::get_input_focus().encoded())?;
            self.unanswered = 0;
        }
        self.push(request)?;
        self.unanswered = if has_reply { 0 } else { self.unanswered + 1 };
        Ok(self.sequence)
    }

    /// Writes all buffered requests.
    pub(crate) fn flush(&mut self) -> Result<(), Error> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let result = self.stream.write_all(&self.buffer);
        self.buffer.clear();
        result
    }

    /// Sets the largest request the server accepts, in bytes.
    pub(crate) fn set_max_request_bytes(&mut self, bytes: usize) {
        self.max_request_bytes = bytes;
    }

    /// Flushes what it can and closes the connection; later sends fail
    /// with [`Error::EventLoopClosed`].
    pub(crate) fn close(&mut self) {
        if self.closed {
            return;
        }
        // The connection is going away: requests that cannot be delivered
        // any more are moot.
        let _ = self.flush();
        self.closed = true;
        self.stream.shutdown();
    }

    fn push(&mut self, request: &Encoded) -> Result<(), Error> {
        self.sequence += 1;
        if request.wire_len() < FLUSH_THRESHOLD {
            request.write_to(&mut self.buffer);
            if self.buffer.len() >= FLUSH_THRESHOLD {
                self.flush()?;
            }
            return Ok(());
        }
        self.flush()?;
        let mut header = Vec::with_capacity(8);
        request.write_header(&mut header);
        self.stream.write_all(&header)?;
        self.stream.write_all(request.body())?;
        self.stream.write_all(&[0; 3][..request.padding()])
    }
}

/// An [`Output`] shared between threads.
#[derive(Debug, Clone)]
pub(crate) struct SharedOutput(Arc<Mutex<Output>>);

impl SharedOutput {
    /// Wraps the sending half of `stream`; sequence numbering starts after
    /// the connection setup.
    pub(crate) fn new(stream: Arc<Stream>, max_request_bytes: usize) -> Self {
        Self(Arc::new(Mutex::new(Output {
            stream,
            buffer: Vec::new(),
            sequence: 0,
            unanswered: 0,
            max_request_bytes,
            closed: false,
        })))
    }

    /// Locks the output for exclusive use.
    pub(crate) fn lock(&self) -> MutexGuard<'_, Output> {
        // Every `Output` method leaves it consistent before it can fail,
        // so a panic elsewhere while holding the lock does no harm.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::os::unix::net::UnixStream;

    use super::*;
    use crate::x11::request::{ImageTarget, map_window, put_image};

    fn output() -> (SharedOutput, UnixStream) {
        let (ours, theirs) = UnixStream::pair().unwrap();
        (
            SharedOutput::new(Arc::new(Stream::Unix(ours)), 1 << 20),
            theirs,
        )
    }

    fn received(peer: &mut UnixStream, len: usize) -> Vec<u8> {
        let mut buf = vec![0; len];
        peer.read_exact(&mut buf).unwrap();
        buf
    }

    #[test]
    fn buffers_until_flush_and_numbers_requests() {
        let (output, mut peer) = output();
        let mut out = output.lock();
        assert_eq!(out.send(map_window(1).encoded(), false), Ok(1));
        assert_eq!(out.send(request::get_input_focus().encoded(), true), Ok(2));
        out.flush().unwrap();
        assert_eq!(
            received(&mut peer, 12),
            [8, 0, 2, 0, 1, 0, 0, 0, 43, 0, 1, 0]
        );
    }

    #[test]
    fn large_requests_bypass_the_buffer() {
        let (output, mut peer) = output();
        let target = ImageTarget {
            drawable: 1,
            gc: 2,
            depth: 24,
        };
        let request = put_image(target, (0, 0, 128, 128), 65_536, |b| {
            b.resize(b.len() + 65_536, 7);
        });
        let mut out = output.lock();
        out.send(map_window(1).encoded(), false).unwrap();
        out.send(request.encoded(), false).unwrap();
        let bytes = received(&mut peer, 8 + request.encoded().wire_len());
        assert_eq!(
            &bytes[..4],
            [8, 0, 2, 0],
            "earlier requests are flushed first"
        );
        assert_eq!(bytes[8], 72);
        assert_eq!(bytes[bytes.len() - 1], 7);
    }

    #[test]
    fn inserts_a_sync_request_after_many_void_requests() {
        let (output, _peer) = output();
        let mut out = output.lock();
        out.unanswered = SYNC_INTERVAL;
        assert_eq!(out.send(map_window(1).encoded(), false), Ok(2));
        assert_eq!(out.unanswered, 1);
    }

    #[test]
    fn rejects_oversized_requests_and_sends_after_close() {
        let (output, _peer) = output();
        let mut out = output.lock();
        out.set_max_request_bytes(16);
        let long = request::intern_atom("A_RATHER_LONG_ATOM_NAME");
        assert!(matches!(
            out.send(long.encoded(), true),
            Err(Error::InvalidArgument(_))
        ));
        out.close();
        assert_eq!(
            out.send(map_window(1).encoded(), false),
            Err(Error::EventLoopClosed)
        );
    }
}
