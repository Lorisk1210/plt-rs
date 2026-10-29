//! The socket to the compositor: finding it, writing whole messages with
//! their file descriptors (from the loop's thread and the waker's), and
//! reading with a deadline.

use std::io;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::sys;
use crate::backend::Wait;
use crate::error::Error;

/// Bytes buffered before a write goes out on its own.
const FLUSH_THRESHOLD: usize = 4096;

/// The socket named by `$WAYLAND_DISPLAY`: an absolute path, or a name in
/// `$XDG_RUNTIME_DIR` (`wayland-0` when unset).
pub(crate) fn socket_path(
    display: Option<&str>,
    runtime_dir: Option<&str>,
) -> Result<PathBuf, Error> {
    let display = display
        .filter(|name| !name.is_empty())
        .unwrap_or("wayland-0");
    if display.starts_with('/') {
        return Ok(PathBuf::from(display));
    }
    let runtime_dir = runtime_dir
        .filter(|dir| !dir.is_empty())
        .ok_or_else(|| Error::BackendUnavailable(String::from("$XDG_RUNTIME_DIR is not set")))?;
    Ok(PathBuf::from(runtime_dir).join(display))
}

/// Connects to the compositor named by the environment.
pub(crate) fn connect() -> Result<UnixStream, Error> {
    let display = std::env::var("WAYLAND_DISPLAY").ok();
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();
    let path = socket_path(display.as_deref(), runtime_dir.as_deref())?;
    UnixStream::connect(&path).map_err(|error| Error::io("connecting to the compositor", &error))
}

/// Requests waiting to be written, shared by the event loop and its
/// waker.
#[derive(Debug)]
pub(crate) struct Output {
    stream: Arc<UnixStream>,
    buffer: Vec<u8>,
    closed: bool,
}

/// An [`Output`] shared between threads.
#[derive(Debug, Clone)]
pub(crate) struct SharedOutput(Arc<Mutex<Output>>);

impl SharedOutput {
    /// Output to `stream`.
    pub(crate) fn new(stream: Arc<UnixStream>) -> Self {
        Self(Arc::new(Mutex::new(Output {
            stream,
            buffer: Vec::new(),
            closed: false,
        })))
    }

    /// Exclusive access to the output.
    pub(crate) fn lock(&self) -> MutexGuard<'_, Output> {
        // Every mutation leaves the output consistent, even if a holder
        // panicked midway.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Output {
    /// Queues one message with the descriptors it carries. Messages with
    /// descriptors go out at once, so that each descriptor travels with
    /// the bytes of its message.
    pub(crate) fn send(&mut self, bytes: &[u8], fds: &[OwnedFd]) -> Result<(), Error> {
        if self.closed {
            return Err(Error::EventLoopClosed);
        }
        if fds.is_empty() {
            self.buffer.extend_from_slice(bytes);
            if self.buffer.len() >= FLUSH_THRESHOLD {
                self.flush()?;
            }
            return Ok(());
        }
        self.flush()?;
        let borrowed: Vec<_> = fds.iter().map(AsFd::as_fd).collect();
        let sent = retry(|| sys::send(&self.stream, bytes, &borrowed))?;
        self.write_all(&bytes[sent..])
    }

    /// Writes every queued message.
    pub(crate) fn flush(&mut self) -> Result<(), Error> {
        let buffer = std::mem::take(&mut self.buffer);
        self.write_all(&buffer)
    }

    fn write_all(&mut self, mut bytes: &[u8]) -> Result<(), Error> {
        while !bytes.is_empty() {
            let sent = retry(|| sys::send(&self.stream, bytes, &[]))?;
            if sent == 0 {
                return Err(Error::ConnectionClosed);
            }
            bytes = &bytes[sent..];
        }
        Ok(())
    }

    /// Stops all writing, e.g. once the connection failed or the loop is
    /// gone; later sends fail with [`Error::EventLoopClosed`].
    pub(crate) fn close(&mut self) {
        self.closed = true;
        self.buffer.clear();
        // The connection is being abandoned; a socket that is already
        // closed needs no further shutdown.
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }
}

/// Runs a socket call until it is not interrupted, mapping failures.
fn retry(mut call: impl FnMut() -> io::Result<usize>) -> Result<usize, Error> {
    loop {
        match call() {
            Ok(n) => return Ok(n),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                return Err(match error.kind() {
                    io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset => {
                        Error::ConnectionClosed
                    }
                    _ => Error::io("writing to the compositor", &error),
                });
            }
        }
    }
}

/// Reads whatever is available into `buf`, with the descriptors that came
/// along, waiting as `wait` allows. Returns `None` when nothing arrived in
/// time and `Some(0)` at end of stream.
pub(crate) fn read(
    stream: &UnixStream,
    buf: &mut [u8],
    fds: &mut Vec<OwnedFd>,
    wait: Wait,
) -> Result<Option<usize>, Error> {
    let failed = |error: io::Error| Error::io("reading from the compositor", &error);
    let block = match wait {
        Wait::Poll => false,
        Wait::Until(deadline) => match deadline.checked_duration_since(std::time::Instant::now()) {
            Some(left) if !left.is_zero() => {
                stream.set_read_timeout(Some(left)).map_err(failed)?;
                true
            }
            _ => false,
        },
        Wait::Forever => {
            stream.set_read_timeout(None).map_err(failed)?;
            true
        }
    };
    loop {
        match sys::receive(stream, buf, fds, block) {
            Ok(n) => return Ok(Some(n)),
            Err(error) => match error.kind() {
                io::ErrorKind::Interrupted => {}
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => return Ok(None),
                _ => return Err(failed(error)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn socket_paths_follow_the_environment() {
        assert_eq!(
            socket_path(Some("wayland-1"), Some("/run/user/7")),
            Ok(PathBuf::from("/run/user/7/wayland-1"))
        );
        assert_eq!(
            socket_path(None, Some("/run/user/7")),
            Ok(PathBuf::from("/run/user/7/wayland-0"))
        );
        assert_eq!(
            socket_path(Some("/tmp/wl"), None),
            Ok(PathBuf::from("/tmp/wl"))
        );
        assert!(socket_path(Some("wayland-1"), None).is_err());
        assert!(socket_path(Some("wayland-1"), Some("")).is_err());
    }

    #[test]
    fn output_batches_and_sends_descriptors_at_once() {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let output = SharedOutput::new(Arc::new(ours));
        output.lock().send(b"abcd", &[]).unwrap();
        let mut buf = [0; 16];
        let mut fds = Vec::new();
        assert_eq!(read(&theirs, &mut buf, &mut fds, Wait::Poll).unwrap(), None);
        let (reader, _writer) = std::io::pipe().unwrap();
        let fd = OwnedFd::from(reader);
        output.lock().send(b"efgh", &[fd]).unwrap();
        let mut received = Vec::new();
        while received.len() < 8 {
            let n = read(&theirs, &mut buf, &mut fds, Wait::Forever).unwrap();
            received.extend_from_slice(&buf[..n.unwrap()]);
        }
        assert_eq!(received, b"abcdefgh");
        assert_eq!(fds.len(), 1);
        output.lock().close();
        assert_eq!(output.lock().send(b"x", &[]), Err(Error::EventLoopClosed));
    }

    #[test]
    fn reads_time_out_and_see_the_end() {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let mut buf = [0; 4];
        let mut fds = Vec::new();
        let soon = Instant::now() + Duration::from_millis(5);
        assert_eq!(
            read(&theirs, &mut buf, &mut fds, Wait::Until(soon)).unwrap(),
            None
        );
        let past = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
        assert_eq!(
            read(&theirs, &mut buf, &mut fds, Wait::Until(past)).unwrap(),
            None
        );
        drop(ours);
        assert_eq!(
            read(&theirs, &mut buf, &mut fds, Wait::Forever).unwrap(),
            Some(0)
        );
    }

    #[test]
    fn writing_to_a_closed_compositor_fails() {
        let (ours, theirs) = UnixStream::pair().unwrap();
        drop(theirs);
        let output = SharedOutput::new(Arc::new(ours));
        let mut output = output.lock();
        output.send(b"abcd", &[]).unwrap();
        assert_eq!(output.flush(), Err(Error::ConnectionClosed));
    }
}
