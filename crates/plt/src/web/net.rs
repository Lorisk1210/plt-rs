//! The network side of the server: binding the listening socket (with
//! matplotlib's port retries), accepting connections on a thread of their
//! own, and what connection threads and the figures' thread tell each
//! other.
//!
//! Figures are single-threaded, so one thread — the one running
//! [`WebServer::run`](super::WebServer::run) — owns them. Every connection
//! has a thread reading and parsing what the client sends; plain HTTP
//! requests are answered there, while WebSocket messages and downloads go
//! to the figures' thread through a bounded channel. Each WebSocket also
//! has a writer thread fed by a bounded queue, so that a slow client never
//! blocks the figures.

use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SendError, Sender, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::http::{Response, Status};
use super::protocol::ClientMessage;
use super::{WebError, WebOptions, connection};
use crate::figure::SaveFormat;

/// The most connections served at once; more are refused.
const MAX_CONNECTIONS: usize = 64;

/// How many messages from clients may wait for the figures' thread.
const INBOUND_CAPACITY: usize = 1024;

/// Identifies a connection.
pub(crate) type ConnectionId = u64;

/// Why a download failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DownloadError {
    /// No figure has the number.
    NoFigure,
    /// The figure could not be saved in the format.
    Failed,
}

/// What connection threads tell the figures' thread.
#[derive(Debug)]
pub(crate) enum Inbound {
    /// A WebSocket for figure `figure` opened; `outbox` feeds its writer
    /// and `stream` shuts it down.
    Opened {
        /// The connection.
        id: ConnectionId,
        /// The figure number from the URL.
        figure: u32,
        /// The queue of the connection's writer thread.
        outbox: SyncSender<Outbound>,
        /// The socket, to shut down a client that falls behind.
        stream: TcpStream,
    },
    /// A message from the client of a WebSocket.
    Message {
        /// The connection.
        id: ConnectionId,
        /// The message.
        message: ClientMessage,
    },
    /// A WebSocket closed.
    Closed {
        /// The connection.
        id: ConnectionId,
    },
    /// Save figure `figure` in `format` for an HTTP download.
    Download {
        /// The figure number.
        figure: u32,
        /// The format.
        format: SaveFormat,
        /// Where the file goes.
        reply: Sender<Result<Vec<u8>, DownloadError>>,
    },
}

/// What a WebSocket's writer thread sends its client.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Outbound {
    /// A text message.
    Text(Arc<str>),
    /// A binary message.
    Binary(Arc<[u8]>),
    /// The answer to a ping.
    Pong(Vec<u8>),
    /// A close frame with this status code; the connection ends after it.
    Close(u16),
}

/// The connections being served, to shut down with the server.
#[derive(Debug, Default)]
struct Connections {
    next: ConnectionId,
    streams: HashMap<ConnectionId, TcpStream>,
}

/// What the server's threads share.
pub(crate) struct Shared {
    /// The channel to the figures' thread.
    inbound: SyncSender<Inbound>,
    /// The numbers of the served figures.
    figures: Mutex<Vec<u32>>,
    /// The client script.
    pub(crate) script: String,
    /// The address the server listens on.
    pub(crate) local_addr: SocketAddr,
    connections: Mutex<Connections>,
    stopping: AtomicBool,
}

impl fmt::Debug for Shared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shared")
            .field("local_addr", &self.local_addr)
            .field("figures", &self.figures)
            .field("stopping", &self.stopping)
            .finish_non_exhaustive()
    }
}

/// Locks `mutex`, also after a thread panicked while holding it: the data
/// guarded here stays consistent between statements.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Shared {
    /// Sends `message` to the figures' thread, waiting while its queue is
    /// full.
    pub(crate) fn send(&self, message: Inbound) -> Result<(), SendError<Inbound>> {
        self.inbound.send(message)
    }

    /// Whether figure `number` is served.
    pub(crate) fn serves(&self, number: u32) -> bool {
        lock(&self.figures).contains(&number)
    }

    /// The numbers of the served figures, in order.
    pub(crate) fn figure_numbers(&self) -> Vec<u32> {
        lock(&self.figures).clone()
    }

    /// Publishes the numbers of the served figures.
    pub(crate) fn set_figure_numbers(&self, mut numbers: Vec<u32>) {
        numbers.sort_unstable();
        *lock(&self.figures) = numbers;
    }

    /// Registers a new connection, unless the server is stopping or full.
    fn register(&self, stream: &TcpStream) -> Option<ConnectionId> {
        let mut connections = lock(&self.connections);
        if self.stopping.load(Ordering::Acquire) || connections.streams.len() >= MAX_CONNECTIONS {
            return None;
        }
        let closer = stream.try_clone().ok()?;
        let id = connections.next;
        connections.next += 1;
        connections.streams.insert(id, closer);
        Some(id)
    }

    /// Forgets a connection that ended.
    fn unregister(&self, id: ConnectionId) {
        lock(&self.connections).streams.remove(&id);
    }

    /// Stops accepting and shuts every connection down.
    fn stop(&self) {
        self.stopping.store(true, Ordering::Release);
        for stream in lock(&self.connections).streams.values() {
            // The connection may be gone already.
            let _ = stream.shutdown(Shutdown::Both);
        }
    }
}

/// A `SplitMix64` step, for picking ports to retry.
fn split_mix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The ports to try (matplotlib's `random_ports`, from IPython): `port`
/// and the next four, then random ones within `2 × tries` of `port`;
/// `tries` ports in all, at least one.
fn candidate_ports(port: u16, tries: u32) -> Vec<u16> {
    let tries = tries.max(1);
    let mut state = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos() as u64)
        ^ u64::from(std::process::id());
    let spread = i64::from(tries) * 2;
    (0..tries)
        .map(|i| {
            let offset = if i < 5 {
                i64::from(i)
            } else {
                (split_mix(&mut state) % (2 * spread as u64 + 1)) as i64 - spread
            };
            (i64::from(port) + offset).clamp(1, i64::from(u16::MAX)) as u16
        })
        .collect()
}

/// Binds the listening socket: on `options.port`, retrying other ports
/// while they are in use, or on a free port chosen by the system for
/// port 0.
fn bind(options: &WebOptions) -> Result<TcpListener, WebError> {
    let address = options.address.as_str();
    let bind_error = |source| WebError::Bind {
        address: options.address.clone(),
        source,
    };
    if options.port == 0 {
        return TcpListener::bind((address, 0)).map_err(bind_error);
    }
    for port in candidate_ports(options.port, options.port_retries) {
        match TcpListener::bind((address, port)) {
            Ok(listener) => return Ok(listener),
            Err(err) if err.kind() == io::ErrorKind::AddrInUse => {}
            Err(err) => return Err(bind_error(err)),
        }
    }
    Err(WebError::NoFreePort {
        address: options.address.clone(),
        port: options.port,
    })
}

/// The address to connect to for reaching a listener on `addr`.
fn reachable(addr: SocketAddr) -> SocketAddr {
    let ip = match addr.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        ip => ip,
    };
    SocketAddr::new(ip, addr.port())
}

/// The listening socket and its accepting thread; dropping it stops the
/// server and shuts its connections down.
#[derive(Debug)]
pub(crate) struct Listener {
    shared: Arc<Shared>,
    acceptor: Option<JoinHandle<()>>,
}

impl Listener {
    /// Binds as `options` say and starts accepting connections; the
    /// returned receiver gets what connection threads send.
    pub(crate) fn start(
        options: &WebOptions,
        script: String,
    ) -> Result<(Self, Receiver<Inbound>), WebError> {
        let listener = bind(options)?;
        let local_addr = listener.local_addr().map_err(|source| WebError::Bind {
            address: options.address.clone(),
            source,
        })?;
        let (inbound, receiver) = mpsc::sync_channel(INBOUND_CAPACITY);
        let shared = Arc::new(Shared {
            inbound,
            figures: Mutex::new(Vec::new()),
            script,
            local_addr,
            connections: Mutex::new(Connections::default()),
            stopping: AtomicBool::new(false),
        });
        let accepting = Arc::clone(&shared);
        let acceptor = thread::Builder::new()
            .name("plt-web-accept".into())
            .spawn(move || accept_loop(&listener, &accepting))
            .map_err(WebError::Thread)?;
        let listener = Self {
            shared,
            acceptor: Some(acceptor),
        };
        Ok((listener, receiver))
    }

    /// What the threads share.
    pub(crate) fn shared(&self) -> &Shared {
        &self.shared
    }

    /// The address listened on, reachable from this machine.
    pub(crate) fn local_addr(&self) -> SocketAddr {
        reachable(self.shared.local_addr)
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.shared.stop();
        // A connection wakes the accepting thread to see the server stop.
        let woken = TcpStream::connect_timeout(&self.local_addr(), Duration::from_secs(1)).is_ok();
        if let Some(acceptor) = self.acceptor.take()
            && woken
        {
            // The thread only ends by returning.
            let _ = acceptor.join();
        }
    }
}

/// Accepts connections until the server stops, each served on a thread.
fn accept_loop(listener: &TcpListener, shared: &Arc<Shared>) {
    for stream in listener.incoming() {
        if shared.stopping.load(Ordering::Acquire) {
            return;
        }
        let Ok(stream) = stream else {
            // Out of descriptors or an aborted handshake: try again soon.
            thread::sleep(Duration::from_millis(10));
            continue;
        };
        let Some(id) = shared.register(&stream) else {
            refuse(stream);
            continue;
        };
        let serving = Arc::clone(shared);
        let spawned = thread::Builder::new()
            .name("plt-web-connection".into())
            .spawn(move || {
                connection::serve(stream, &serving, id);
                serving.unregister(id);
            });
        if spawned.is_err() {
            shared.unregister(id);
        }
    }
}

/// Answers a connection the server cannot take with a 503.
fn refuse(mut stream: TcpStream) {
    // The client may not read: give up quickly; it is closed either way.
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let _ = Response::error(Status::ServiceUnavailable).write_to(&mut stream, false, false);
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ports_follow_matplotlib() {
        let ports = candidate_ports(8988, 50);
        assert_eq!(ports.len(), 50);
        assert_eq!(ports[..5], [8988, 8989, 8990, 8991, 8992]);
        assert!(ports.iter().all(|&p| (8888..=9088).contains(&p)));
        assert_eq!(candidate_ports(8988, 0), [8988]);
        assert!(candidate_ports(65535, 10).iter().all(|&p| p >= 65515));
    }

    #[test]
    fn busy_ports_are_skipped() {
        let busy = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = busy.local_addr().unwrap().port();
        let options = WebOptions {
            port_retries: 0,
            ..WebOptions::new("127.0.0.1", port)
        };
        assert!(matches!(bind(&options), Err(WebError::NoFreePort { .. })));
        let options = WebOptions {
            port_retries: 20,
            ..options
        };
        let listener = bind(&options).expect("a free port nearby");
        assert_ne!(listener.local_addr().unwrap().port(), port);
    }

    #[test]
    fn unspecified_addresses_are_reached_through_loopback() {
        let any: SocketAddr = "0.0.0.0:80".parse().unwrap();
        assert_eq!(reachable(any), "127.0.0.1:80".parse().unwrap());
        let any6: SocketAddr = "[::]:80".parse().unwrap();
        assert_eq!(reachable(any6), "[::1]:80".parse().unwrap());
    }
}
