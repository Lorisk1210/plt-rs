//! The byte stream to an X server (a Unix-domain or TCP socket) and the
//! credentials that go with its address.

use std::io::{self, Read, Write};
use std::net::{IpAddr, Shutdown, TcpStream};
use std::os::unix::net::UnixStream;
use std::time::Instant;

use super::auth::{self, Credentials};
use super::display::{DisplayName, Target};
use crate::backend::Wait;
use crate::error::Error;

/// First TCP port of X servers; display `n` listens on `6000 + n`.
const X_TCP_PORT_BASE: u32 = 6000;

/// A connected socket to the X server.
#[derive(Debug)]
pub(crate) enum Stream {
    /// A local server.
    Unix(UnixStream),
    /// A remote (or `localhost:n`) server.
    Tcp(TcpStream),
}

impl Stream {
    /// Connects to the server named by `name` and looks up the user's
    /// credentials for it.
    pub(crate) fn connect(name: &DisplayName) -> Result<(Self, Option<Credentials>), Error> {
        let failed = |error: io::Error| Error::io("connecting to the X server", &error);
        match &name.target {
            Target::Local => {
                let stream = connect_local(&name.local_socket_path()).map_err(failed)?;
                Ok((Self::Unix(stream), local_credentials(name.display)))
            }
            Target::Path(path) => {
                let stream = UnixStream::connect(path).map_err(failed)?;
                Ok((Self::Unix(stream), local_credentials(name.display)))
            }
            Target::Tcp(host) => {
                let port = u16::try_from(X_TCP_PORT_BASE + u32::from(name.display))
                    .map_err(|_| Error::InvalidDisplay(format!("{host}:{}", name.display)))?;
                let stream = TcpStream::connect((host.as_str(), port)).map_err(failed)?;
                stream.set_nodelay(true).map_err(failed)?;
                let peer = stream.peer_addr().map_err(failed)?;
                Ok((Self::Tcp(stream), tcp_credentials(peer.ip(), name.display)))
            }
        }
    }

    /// Reads whatever is available into `buf`, waiting as `wait` allows.
    /// Returns `None` when nothing arrived in time and `Some(0)` at end of
    /// stream.
    ///
    /// [`Wait::Poll`] briefly switches the socket to non-blocking mode,
    /// which also affects writes through other handles of the same socket:
    /// callers must keep other threads from writing meanwhile.
    pub(crate) fn read(&self, buf: &mut [u8], wait: Wait) -> io::Result<Option<usize>> {
        match wait {
            Wait::Poll => {
                self.set_nonblocking(true)?;
                let result = self.read_once(buf);
                self.set_nonblocking(false)?;
                result
            }
            Wait::Until(deadline) => match deadline.checked_duration_since(Instant::now()) {
                Some(left) if !left.is_zero() => {
                    self.set_read_timeout(Some(left))?;
                    self.read_once(buf)
                }
                _ => self.read(buf, Wait::Poll),
            },
            Wait::Forever => {
                self.set_read_timeout(None)?;
                self.read_once(buf)
            }
        }
    }

    /// Fills `buf` completely, blocking as long as needed.
    pub(crate) fn read_exact(&self, buf: &mut [u8]) -> Result<(), Error> {
        let mut filled = 0;
        while filled < buf.len() {
            match self.read(&mut buf[filled..], Wait::Forever) {
                Ok(Some(0)) => return Err(Error::ConnectionClosed),
                Ok(Some(n)) => filled += n,
                Ok(None) => {}
                Err(error) => return Err(Error::io("reading from the X server", &error)),
            }
        }
        Ok(())
    }

    /// Writes all of `bytes`.
    pub(crate) fn write_all(&self, bytes: &[u8]) -> Result<(), Error> {
        let result = match self {
            Self::Unix(stream) => (&*stream).write_all(bytes),
            Self::Tcp(stream) => (&*stream).write_all(bytes),
        };
        result.map_err(|error| match error.kind() {
            io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset => Error::ConnectionClosed,
            _ => Error::io("writing to the X server", &error),
        })
    }

    /// Closes the connection for every handle of the socket, so the server
    /// releases this client's resources.
    pub(crate) fn shutdown(&self) {
        // The connection is being abandoned; a socket that is already
        // closed needs no further shutdown.
        let _ = match self {
            Self::Unix(stream) => stream.shutdown(Shutdown::Both),
            Self::Tcp(stream) => stream.shutdown(Shutdown::Both),
        };
    }

    fn read_once(&self, buf: &mut [u8]) -> io::Result<Option<usize>> {
        loop {
            let result = match self {
                Self::Unix(stream) => (&*stream).read(buf),
                Self::Tcp(stream) => (&*stream).read(buf),
            };
            match result {
                Ok(n) => return Ok(Some(n)),
                Err(error) => match error.kind() {
                    io::ErrorKind::Interrupted => {}
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => return Ok(None),
                    _ => return Err(error),
                },
            }
        }
    }

    fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        match self {
            Self::Unix(stream) => stream.set_nonblocking(nonblocking),
            Self::Tcp(stream) => stream.set_nonblocking(nonblocking),
        }
    }

    fn set_read_timeout(&self, timeout: Option<std::time::Duration>) -> io::Result<()> {
        match self {
            Self::Unix(stream) => stream.set_read_timeout(timeout),
            Self::Tcp(stream) => stream.set_read_timeout(timeout),
        }
    }
}

/// Connects to a local server's socket file, falling back to the abstract
/// socket of the same name that Linux servers also listen on (it survives
/// a wiped `/tmp`).
fn connect_local(path: &str) -> io::Result<UnixStream> {
    UnixStream::connect(path).or_else(|error| connect_abstract(path).map_err(|_| error))
}

#[cfg(target_os = "linux")]
fn connect_abstract(name: &str) -> io::Result<UnixStream> {
    use std::os::linux::net::SocketAddrExt;
    let address = std::os::unix::net::SocketAddr::from_abstract_name(name.as_bytes())?;
    UnixStream::connect_addr(&address)
}

#[cfg(not(target_os = "linux"))]
fn connect_abstract(_name: &str) -> io::Result<UnixStream> {
    Err(io::ErrorKind::NotFound.into())
}

/// Credentials of a server on this machine, recorded under its host name.
fn local_credentials(display: u16) -> Option<Credentials> {
    let host = auth::hostname().unwrap_or_default();
    auth::load(auth::FAMILY_LOCAL, host.as_bytes(), display)
}

/// Credentials of a TCP server: loopback servers use the local entry, as
/// Xlib does.
fn tcp_credentials(ip: IpAddr, display: u16) -> Option<Credentials> {
    if ip.is_loopback() {
        return local_credentials(display);
    }
    match ip {
        IpAddr::V4(v4) => auth::load(auth::FAMILY_INTERNET, &v4.octets(), display),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => auth::load(auth::FAMILY_INTERNET, &v4.octets(), display),
            None => auth::load(auth::FAMILY_INTERNET6, &v6.octets(), display),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn pair() -> (Stream, UnixStream) {
        let (ours, theirs) = UnixStream::pair().unwrap();
        (Stream::Unix(ours), theirs)
    }

    #[test]
    fn poll_and_timeouts_return_none_without_data() {
        let (stream, _peer) = pair();
        let mut buf = [0; 8];
        assert_eq!(stream.read(&mut buf, Wait::Poll).unwrap(), None);
        let soon = Instant::now() + Duration::from_millis(5);
        assert_eq!(stream.read(&mut buf, Wait::Until(soon)).unwrap(), None);
        let past = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
        assert_eq!(stream.read(&mut buf, Wait::Until(past)).unwrap(), None);
    }

    #[test]
    fn reads_available_data_and_end_of_stream() {
        let (stream, mut peer) = pair();
        peer.write_all(b"abc").unwrap();
        let mut buf = [0; 8];
        assert_eq!(stream.read(&mut buf, Wait::Poll).unwrap(), Some(3));
        assert_eq!(&buf[..3], b"abc");
        peer.write_all(b"defg").unwrap();
        let mut exact = [0; 4];
        stream.read_exact(&mut exact).unwrap();
        assert_eq!(&exact, b"defg");
        drop(peer);
        assert_eq!(stream.read(&mut buf, Wait::Forever).unwrap(), Some(0));
        assert_eq!(stream.read_exact(&mut exact), Err(Error::ConnectionClosed));
    }

    #[test]
    fn writes_reach_the_peer_and_fail_after_shutdown() {
        let (stream, mut peer) = pair();
        stream.write_all(b"xyz").unwrap();
        let mut buf = [0; 3];
        peer.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"xyz");
        stream.shutdown();
        assert!(stream.write_all(b"more").is_err());
    }
}
