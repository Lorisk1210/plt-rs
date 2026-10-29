//! The server itself over a loopback socket: connections carrying random
//! request heads, and WebSocket handshakes followed by random frames and
//! client messages, reach a server showing a figure, sometimes while
//! another connection stalls halfway through what it sends. Each
//! connection must be answered or closed within moments, and the server
//! must still answer a plain request after every one.

use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use super::frames::Frames;
use super::http::{self, Head, REQUESTS};
use crate::figure::Figure;
use crate::fuzz::{Gen, fuzz};
use crate::web::{WebOptions, WebServer};

/// How long a connection may take to be answered and closed.
const PATIENCE: Duration = Duration::from_secs(20);

/// How long the server runs between looks at the client.
const SLICE: Duration = Duration::from_millis(2);

/// What one connection sends.
#[derive(Debug)]
enum Exchange {
    /// Request heads.
    Http(Vec<Head>),
    /// The handshake of a browser (or another head), then frames.
    WebSocket(Option<Head>, Frames),
}

impl Exchange {
    fn generate(g: &mut Gen) -> Self {
        if g.chance(0.4) {
            Self::Http(http::connection(g))
        } else {
            let handshake = g.chance(0.15).then(|| Head::generate(g));
            Self::WebSocket(handshake, Frames::generate(g))
        }
    }

    fn bytes(&self) -> Vec<u8> {
        match self {
            Self::Http(heads) => heads.iter().flat_map(Head::bytes).collect(),
            Self::WebSocket(handshake, frames) => {
                let mut bytes = handshake
                    .as_ref()
                    .map_or_else(|| REQUESTS[2].as_bytes().to_vec(), Head::bytes);
                bytes.extend(frames.bytes());
                bytes
            }
        }
    }
}

/// Sends `bytes` on a new connection, then reads until the server closes
/// it; a connection the server resets counts as closed.
fn exchange(addr: SocketAddr, bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(PATIENCE))?;
    stream.set_write_timeout(Some(PATIENCE))?;
    // The server may close the connection before it read everything.
    let _ = stream.write_all(bytes);
    let _ = stream.shutdown(Shutdown::Write);
    let mut answer = Vec::new();
    match stream.read_to_end(&mut answer) {
        Err(err) if err.kind() != io::ErrorKind::ConnectionReset => Err(err),
        _ => Ok(answer),
    }
}

/// The server and the figure it shows.
struct Served {
    server: WebServer,
    figure: Figure,
}

impl Served {
    fn new() -> Self {
        let server = WebServer::bind(WebOptions::new("127.0.0.1", 0)).expect("a free port");
        let mut served = Self {
            server,
            figure: Figure::new(),
        };
        served.replace_figure();
        served
    }

    /// Shows a fresh small figure as figure 1.
    fn replace_figure(&mut self) {
        self.server.close(&self.figure);
        self.server.take_closed();
        let figure = Figure::with_size(2.0, 1.5, 50.0);
        let ax = figure.subplot();
        ax.plot(vec![0.0, 1.0, 2.0], vec![1.0, 3.0, 2.0])
            .expect("a valid curve");
        assert_eq!(self.server.add_figure(&figure), 1);
        self.figure = figure;
    }

    /// Opens a connection that sends the first `len` bytes of `bytes`
    /// and then waits, holding the connection open until it is dropped.
    fn stall(&self, bytes: &[u8], len: usize) -> Option<TcpStream> {
        let mut stream = TcpStream::connect(self.server.local_addr()).ok()?;
        stream.write_all(&bytes[..len.min(bytes.len())]).ok()?;
        Some(stream)
    }

    /// Serves while a client sends `bytes`, until it is done.
    fn serve(&mut self, bytes: Vec<u8>) {
        let addr = self.server.local_addr();
        let client = thread::spawn(move || exchange(addr, &bytes));
        let started = Instant::now();
        while !client.is_finished() {
            assert!(started.elapsed() < PATIENCE * 2, "the exchange hangs");
            self.server.pause(SLICE);
        }
        let answer = client.join().expect("the client does not panic");
        if let Err(err) = answer {
            panic!("no answer or close: {err}");
        }
    }

    /// Checks that the server still answers a plain request.
    fn check_serving(&mut self) {
        let request = b"GET /1 HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n".to_vec();
        let addr = self.server.local_addr();
        let client = thread::spawn(move || exchange(addr, &request));
        while !client.is_finished() {
            self.server.pause(SLICE);
        }
        let answer = client
            .join()
            .expect("the client does not panic")
            .expect("the server answers");
        assert!(
            answer.starts_with(b"HTTP/1.1 200 OK\r\n"),
            "the server stopped serving: {}",
            String::from_utf8_lossy(&answer)
        );
    }
}

/// A connection, and maybe another that stalls after so many bytes.
#[derive(Debug)]
struct Connections {
    exchange: Exchange,
    stalled: Option<(Exchange, usize)>,
}

#[test]
fn loopback_connections() {
    let served = RefCell::new(Served::new());
    fuzz(
        "web loopback",
        200,
        |g| Connections {
            exchange: Exchange::generate(g),
            stalled: g.chance(0.2).then(|| (Exchange::generate(g), g.below(400))),
        },
        |connections| {
            let mut served = served.borrow_mut();
            let staller = connections
                .stalled
                .as_ref()
                .and_then(|(exchange, len)| served.stall(&exchange.bytes(), *len));
            served.serve(connections.exchange.bytes());
            served.replace_figure();
            served.check_serving();
            drop(staller);
        },
    );
}
