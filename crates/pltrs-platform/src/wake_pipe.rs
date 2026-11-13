//! A pipe whose writes wake an event loop ([`WakePipe`]): what lets a
//! loop that blocks until input, a timer or a proxy wakes it also wake for
//! what another part of the program learns by a write to a descriptor,
//! such as Python's `signal.set_wakeup_fd`, whose signal handler writes
//! the signal's number to it.
//!
//! It is a connected pair of sockets: a `socketpair` on Unix, and on
//! Windows a loopback TCP connection (what Python's `socket.socketpair`
//! makes there, and the kind of handle its `set_wakeup_fd` takes). The
//! writing end is non-blocking, as `set_wakeup_fd` requires. A thread of
//! its own blocks reading the other end and calls the wake function for
//! each read, so the loop needs no descriptor of its own to watch: each
//! backend's [`Waker`](crate::EventLoopProxy) already interrupts its wait
//! (a message through the X server or the compositor, a posted event or
//! message on macOS and Windows), and the thread sleeps until a write
//! comes. It ends once the writing end is dropped.

use std::io::{self, Read};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use crate::error::Error;

/// How many of the bytes written are kept for [`WakePipe::take_received`]
/// (signal numbers, one byte each): the oldest are kept.
const KEPT: usize = 256;

#[cfg(unix)]
type Socket = std::os::unix::net::UnixStream;
#[cfg(windows)]
type Socket = std::net::TcpStream;

/// The writing end of a pipe waking an event loop, and what was written
/// to it.
#[derive(Debug)]
pub struct WakePipe {
    writer: Socket,
    received: Arc<Mutex<Vec<u8>>>,
}

impl WakePipe {
    /// A pipe calling `wake` (for instance `EventLoopProxy::wake_up`) each
    /// time something is written to it, from a thread of its own.
    ///
    /// # Errors
    ///
    /// When the sockets or the thread cannot be made.
    pub fn new(wake: impl Fn() + Send + 'static) -> Result<Self, Error> {
        let (writer, reader) = pair().map_err(|error| Error::io("making a wake pipe", &error))?;
        writer
            .set_nonblocking(true)
            .map_err(|error| Error::io("making a wake pipe", &error))?;
        let received = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&received);
        thread::Builder::new()
            .name("pltrs wake pipe".into())
            .spawn(move || watch(reader, &kept, &wake))
            .map_err(|error| Error::io("starting a wake pipe's thread", &error))?;
        Ok(Self { writer, received })
    }

    /// The bytes written so far (the first 256), taken.
    pub fn take_received(&self) -> Vec<u8> {
        std::mem::take(&mut *self.received.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// The writing end's file descriptor.
    #[cfg(unix)]
    pub fn writer_fd(&self) -> std::os::fd::RawFd {
        std::os::fd::AsRawFd::as_raw_fd(&self.writer)
    }

    /// The writing end's socket handle.
    #[cfg(windows)]
    pub fn writer_socket(&self) -> std::os::windows::io::RawSocket {
        std::os::windows::io::AsRawSocket::as_raw_socket(&self.writer)
    }

    /// Writes `bytes` to the pipe, as code given its descriptor would.
    ///
    /// # Errors
    ///
    /// When the write fails, such as when the pipe is full: it does not
    /// block.
    pub fn write(&self, bytes: &[u8]) -> Result<(), Error> {
        io::Write::write_all(&mut &self.writer, bytes)
            .map_err(|error| Error::io("writing to a wake pipe", &error))
    }
}

/// A connected pair of sockets: `(writer, reader)`.
#[cfg(unix)]
fn pair() -> io::Result<(Socket, Socket)> {
    Socket::pair()
}

/// A connected pair of sockets: `(writer, reader)`, a loopback TCP
/// connection whose ends are checked to be each other's.
#[cfg(windows)]
fn pair() -> io::Result<(Socket, Socket)> {
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    let writer = Socket::connect(listener.local_addr()?)?;
    loop {
        let (reader, from) = listener.accept()?;
        // Another program may connect to the port first: take ours only.
        if from == writer.local_addr()? {
            writer.set_nodelay(true)?;
            return Ok((writer, reader));
        }
    }
}

/// Reads `reader` until its writer is gone, keeping what it reads in
/// `kept` and calling `wake` for each read.
fn watch(mut reader: Socket, kept: &Mutex<Vec<u8>>, wake: &dyn Fn()) {
    let mut buf = [0; 64];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => {
                {
                    let mut kept = kept.lock().unwrap_or_else(PoisonError::into_inner);
                    let room = KEPT.saturating_sub(kept.len());
                    kept.extend_from_slice(&buf[..n.min(room)]);
                }
                wake();
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    #[test]
    fn writes_wake_and_are_kept() {
        let (woken, wakes) = mpsc::channel();
        let pipe = WakePipe::new(move || {
            let _ = woken.send(());
        })
        .unwrap();
        pipe.write(&[2]).unwrap();
        wakes.recv_timeout(Duration::from_secs(10)).unwrap();
        pipe.write(&[15, 2]).unwrap();
        let mut received = pipe.take_received();
        while received.len() < 3 {
            wakes.recv_timeout(Duration::from_secs(10)).unwrap();
            received.extend(pipe.take_received());
        }
        assert_eq!(received, [2, 15, 2]);
        assert_eq!(pipe.take_received(), []);
    }

    #[test]
    fn the_oldest_bytes_are_kept() {
        let (woken, wakes) = mpsc::channel();
        let pipe = WakePipe::new(move || {
            let _ = woken.send(());
        })
        .unwrap();
        let bytes: Vec<u8> = (0..=255).cycle().take(KEPT + 10).collect();
        pipe.write(&bytes).unwrap();
        // Every byte read is followed by a wake-up; wait until the reader
        // has had them all.
        while wakes.recv_timeout(Duration::from_millis(500)).is_ok() {}
        assert_eq!(pipe.take_received(), bytes[..KEPT]);
    }
}
