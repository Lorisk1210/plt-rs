//! Randomised robustness tests of what the network sends the web server:
//! HTTP request heads ([`http`]), WebSocket frames ([`frames`]) and the
//! JSON messages of the browser client ([`messages`]) fed to their
//! parsers directly, the messages that parse played on a figure's
//! canvas, and all of it sent to a server over a loopback socket
//! ([`loopback`]). Nothing may panic, hang or allocate without bound;
//! every malformed input must end in a well-defined error or close.
//!
//! The targets run on the harness of `tests/fuzz` ([`crate::fuzz`]), so
//! `PLTRS_FUZZ_SCALE`, `PLTRS_FUZZ_SEED` and `PLTRS_FUZZ_REPLAY` apply.

mod frames;
mod http;
mod loopback;
mod messages;

use std::io::{self, Read};

/// A reader handing out at most `chunk` bytes per read, as a socket may,
/// so that parsers see their input split anywhere.
#[derive(Debug)]
struct Trickle<'a> {
    bytes: &'a [u8],
    chunk: usize,
}

impl<'a> Trickle<'a> {
    /// Reads `bytes`, at most `chunk` (at least one) at a time.
    fn new(bytes: &'a [u8], chunk: usize) -> Self {
        Self {
            bytes,
            chunk: chunk.max(1),
        }
    }
}

impl Read for Trickle<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = buf.len().min(self.chunk).min(self.bytes.len());
        let (now, rest) = self.bytes.split_at(len);
        buf[..len].copy_from_slice(now);
        self.bytes = rest;
        Ok(len)
    }
}
