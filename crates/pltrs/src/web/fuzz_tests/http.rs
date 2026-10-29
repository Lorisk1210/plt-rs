//! HTTP request heads: the requests of browsers and other clients mutated
//! as text, corrupted as bytes (invalid UTF-8, control bytes, cut short),
//! grown around the size and field-count limits and pipelined on one
//! connection, read in pieces of any size and answered as a connection
//! answers them. Routes may only name the server's own files and URLs.

use std::fmt::Write as _;
use std::io::BufReader;

use super::Trickle;
use crate::fuzz::{Corruption, Gen, fuzz};
use crate::web::http::{self, HttpError, MAX_HEAD, MAX_HEADERS, Method, Request, Response, Status};
use crate::web::routes::{self, Route};
use crate::web::websocket;

/// Request heads browsers and other clients send.
pub(super) const REQUESTS: [&str; 10] = [
    "GET / HTTP/1.1\r\nHost: 127.0.0.1:8988\r\n\r\n",
    "GET /1 HTTP/1.1\r\nHost: localhost:8988\r\nConnection: keep-alive\r\nAccept: */*\r\n\r\n",
    "GET /1/ws HTTP/1.1\r\nHost: 127.0.0.1:8988\r\nUpgrade: websocket\r\n\
     Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
     Sec-WebSocket-Version: 13\r\nOrigin: http://127.0.0.1:8988\r\n\r\n",
    "HEAD /js/mpl.js HTTP/1.0\r\nConnection: Keep-Alive\r\n\r\n",
    "GET /1/download.svg?t=1 HTTP/1.1\r\nHost: h\r\nConnection: close\r\n\r\n",
    "GET /_images/home.png HTTP/1.1\r\nHost: [::1]:8988\r\n\r\n",
    "GET /css/page.css HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n",
    "POST /1 HTTP/1.1\r\nHost: h\r\nContent-Length: 3\r\n\r\nabc",
    "\r\n\nGET /favicon.ico HTTP/1.1\nHost: h\n\n",
    "GET /2/ws HTTP/1.1\r\nHost: h\r\nUpgrade: WebSocket\r\nConnection: keep-alive, Upgrade\r\n\
     Sec-WebSocket-Key: AAAAAAAAAAAAAAAAAAAAAA==\r\nSec-WebSocket-Version: 8\r\n\r\n",
];

/// Pieces of request heads: line ends, separators, path traversal,
/// methods, versions and the header fields the server reads.
const PIECES: [&str; 36] = [
    "\r\n",
    "\n",
    "\r",
    " ",
    "  ",
    "\t",
    ":",
    "/",
    "//",
    "..",
    "/../",
    "%2e%2e",
    "%00",
    "\\",
    "?",
    "#",
    "\0",
    "GET",
    "HEAD",
    "OPTIONS",
    "HTTP/1.1",
    "HTTP/1.0",
    "HTTP/2.0",
    "HTTP/",
    "Host: ",
    "Connection: close",
    "Connection: Upgrade",
    "Upgrade: websocket",
    "Content-Length: ",
    "Transfer-Encoding: chunked",
    "Sec-WebSocket-Key: ",
    "Sec-WebSocket-Version: 13",
    "Origin: ",
    "/1/ws",
    "/_images/../../etc/passwd",
    "/1/download.",
];

/// What a head is grown at, to reach a limit.
#[derive(Clone, Copy, Debug)]
pub(super) enum Growth {
    /// The request target, made longer.
    Target,
    /// A header value, made longer.
    Value,
    /// Header fields, added.
    Fields,
}

/// One request head, or what passes for one.
#[derive(Clone, Debug)]
pub(super) enum Head {
    /// Text built from the examples and pieces.
    Text(String),
    /// The bytes of example `.0`, corrupted.
    Corrupted(usize, Corruption),
    /// Example `.0` grown to `.2` bytes (or fields).
    Grown(usize, Growth, usize),
}

impl Head {
    /// A random head.
    pub(super) fn generate(g: &mut Gen) -> Self {
        match g.below(6) {
            0 | 1 => Self::Text(g.text(&REQUESTS, &PIECES)),
            2 | 3 => {
                let example = g.below(REQUESTS.len());
                Self::Corrupted(example, g.corrupt(REQUESTS[example].as_bytes()))
            }
            _ => {
                let growth = g.pick(&[Growth::Target, Growth::Value, Growth::Fields]);
                let size = match growth {
                    Growth::Fields => {
                        g.pick(&[MAX_HEADERS - 2, MAX_HEADERS - 1, MAX_HEADERS]) + g.below(3)
                    }
                    Growth::Target | Growth::Value => {
                        let near = g.pick(&[MAX_HEAD - 64, MAX_HEAD, 2 * MAX_HEAD]);
                        near + g.below(80) - 40
                    }
                };
                Self::Grown(g.below(REQUESTS.len()), growth, size)
            }
        }
    }

    /// The bytes of the head.
    pub(super) fn bytes(&self) -> Vec<u8> {
        match self {
            Self::Text(text) => text.clone().into_bytes(),
            Self::Corrupted(example, corruption) => corruption.apply(REQUESTS[*example].as_bytes()),
            Self::Grown(example, growth, size) => grown(REQUESTS[*example], *growth, *size),
        }
    }
}

/// `example` grown at `growth` to about `size` bytes, or `size` fields.
fn grown(example: &str, growth: Growth, size: usize) -> Vec<u8> {
    let (line, rest) = example.split_once('\n').unwrap_or((example, ""));
    match growth {
        Growth::Target => {
            let padding = "a".repeat(size.saturating_sub(example.len()));
            example
                .replacen(' ', &format!(" /{padding}"), 1)
                .into_bytes()
        }
        Growth::Value => {
            let padding = "v".repeat(size.saturating_sub(example.len()));
            format!("{line}\nX-Padding: {padding}\n{rest}").into_bytes()
        }
        Growth::Fields => {
            let mut fields = String::new();
            for i in 0..size {
                let _ = write!(fields, "X-{i}: y\r\n");
            }
            format!("{line}\n{fields}{rest}").into_bytes()
        }
    }
}

/// What one connection receives: heads sent one after another.
pub(super) fn connection(g: &mut Gen) -> Vec<Head> {
    (0..=g.below(3)).map(|_| Head::generate(g)).collect()
}

/// Reads and answers the requests in `bytes` as a connection does, the
/// bytes arriving `chunk` at a time, until the client is done or the
/// connection is refused.
pub(super) fn serve(bytes: &[u8], chunk: usize) {
    let mut reader = BufReader::new(Trickle::new(bytes, chunk));
    // Every head read takes at least a byte.
    for _ in 0..=bytes.len() {
        let head = match http::read_head(&mut reader) {
            Ok(Some(head)) => head,
            Ok(None) => return,
            Err(err) => return refuse(&err),
        };
        assert!(head.len() <= MAX_HEAD, "a head of {} bytes", head.len());
        match http::parse_request(&head) {
            Ok(request) if answer(&request) => {}
            Ok(_) => return,
            Err(err) => return refuse(&err),
        }
    }
    panic!("requests without end");
}

/// Writes the refusal of a request that could not be read.
fn refuse(err: &HttpError) {
    assert_ne!(err.to_string(), "");
    if matches!(err, HttpError::TooLarge) {
        assert_eq!(err.status(), Some(Status::HeaderFieldsTooLarge));
    }
    if let Some(status) = err.status() {
        write(&Response::error(status), false);
    }
}

/// Answers `request` as a connection does, short of a server; returns
/// whether the connection stays open.
fn answer(request: &Request) -> bool {
    let route = routes::route(&request.path);
    check_route(&request.path, route);
    if request.method == Method::Other {
        write(&Response::error(Status::MethodNotAllowed), false);
        return false;
    }
    if matches!(route, Route::WebSocket(_)) && websocket::is_upgrade(request) {
        match websocket::handshake(request) {
            Ok(accepted) => {
                assert_eq!(accepted.status, Status::SwitchingProtocols);
                let key = request.header("sec-websocket-key").expect("a key");
                assert!(
                    accepted
                        .headers
                        .iter()
                        .any(|(name, value)| *name == "Sec-WebSocket-Accept"
                            && *value == websocket::accept_key(key)),
                    "the accept key answers the key"
                );
                write(&accepted, false);
            }
            Err(refusal) => write(&refusal, false),
        }
        return false;
    }
    let keep_alive = request.keep_alive();
    write(&Response::error(Status::NotFound), keep_alive);
    keep_alive
}

/// Checks that a route names one of the server's files or URLs exactly:
/// no path leaves them.
fn check_route(path: &str, route: Route) {
    match route {
        Route::Static(asset) => assert_eq!(asset.path, path),
        Route::Modern(file) => assert_eq!(file.path, path),
        Route::NotFound => {}
        _ => assert!(
            path.split('/')
                .all(|segment| segment != ".." && segment != ".")
                && !path.contains(['\\', '%', '\0']),
            "{path:?} routes to {route:?}"
        ),
    }
}

/// Writes a response, as the connection would.
fn write(response: &Response, keep_alive: bool) {
    let mut out = Vec::new();
    response
        .write_to(&mut out, false, keep_alive)
        .expect("writing to memory succeeds");
    assert!(out.starts_with(b"HTTP/1.1 "));
}

#[test]
fn http_requests() {
    fuzz(
        "web http",
        3_000,
        |g| (connection(g), 1 + g.below(4) * g.below(300)),
        |(heads, chunk)| {
            let bytes: Vec<u8> = heads.iter().flat_map(Head::bytes).collect();
            serve(&bytes, *chunk);
        },
    );
}
