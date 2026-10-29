//! The HTTP/1.1 subset the web backend serves (RFC 9112): reading and
//! parsing request heads from untrusted clients, within size limits, and
//! writing responses.

use std::borrow::Cow;
use std::fmt::{self, Write as _};
use std::io::{self, BufRead, Read, Write};

/// The longest request head (request line and headers) accepted.
pub(crate) const MAX_HEAD: usize = 16 * 1024;

/// The most header fields a request may have.
pub(crate) const MAX_HEADERS: usize = 64;

/// A request method the server distinguishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Method {
    /// `GET`.
    Get,
    /// `HEAD`: a `GET` without the body.
    Head,
    /// Any other method, which is refused.
    Other,
}

/// The protocol version of a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Version {
    /// `HTTP/1.0`.
    Http10,
    /// `HTTP/1.1`.
    Http11,
}

/// A parsed request head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Request {
    /// The method.
    pub(crate) method: Method,
    /// The path of the request target, without its query.
    pub(crate) path: String,
    /// The protocol version.
    pub(crate) version: Version,
    /// The header fields in order, names as sent.
    headers: Vec<(String, String)>,
}

impl Request {
    /// The value of header `name` (compared without case); the first, if
    /// repeated.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find_map(|(key, value)| key.eq_ignore_ascii_case(name).then_some(value.as_str()))
    }

    /// Whether the comma-separated header `name` lists `token` (compared
    /// without case), in any of its fields.
    pub(crate) fn has_token(&self, name: &str, token: &str) -> bool {
        self.headers
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case(name))
            .flat_map(|(_, value)| value.split(','))
            .any(|item| item.trim().eq_ignore_ascii_case(token))
    }

    /// Whether the connection stays open after the response: by default in
    /// HTTP/1.1, on request in HTTP/1.0.
    pub(crate) fn keep_alive(&self) -> bool {
        match self.version {
            Version::Http11 => !self.has_token("connection", "close"),
            Version::Http10 => self.has_token("connection", "keep-alive"),
        }
    }
}

/// Why a request could not be read.
#[derive(Debug)]
pub(crate) enum HttpError {
    /// The connection failed or timed out.
    Io(io::Error),
    /// The connection closed in the middle of a request head.
    Truncated,
    /// The request head is not valid HTTP/1.x.
    Malformed(&'static str),
    /// The request head exceeds [`MAX_HEAD`] or has too many fields.
    TooLarge,
    /// The request carries a body, which no route accepts.
    UnexpectedBody,
    /// A protocol version other than 1.0 and 1.1.
    UnsupportedVersion,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "connection error: {err}"),
            Self::Truncated => f.write_str("the connection closed within a request"),
            Self::Malformed(why) => write!(f, "malformed request: {why}"),
            Self::TooLarge => f.write_str("request head too large"),
            Self::UnexpectedBody => f.write_str("requests may not carry a body"),
            Self::UnsupportedVersion => f.write_str("unsupported HTTP version"),
        }
    }
}

impl std::error::Error for HttpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for HttpError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl HttpError {
    /// The status to answer with, if the client can still be answered.
    pub(crate) fn status(&self) -> Option<Status> {
        match self {
            Self::Io(_) | Self::Truncated => None,
            Self::Malformed(_) => Some(Status::BadRequest),
            Self::TooLarge => Some(Status::HeaderFieldsTooLarge),
            Self::UnexpectedBody => Some(Status::PayloadTooLarge),
            Self::UnsupportedVersion => Some(Status::VersionNotSupported),
        }
    }
}

/// Reads the next request head, line by line, up to the empty line ending
/// it; `None` when the connection closed cleanly between requests. Empty
/// lines before the request line are skipped (RFC 9112 section 2.2).
pub(crate) fn read_head(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, HttpError> {
    let mut limited = reader.take(MAX_HEAD as u64);
    let mut head = Vec::new();
    loop {
        let start = head.len();
        let read = limited.read_until(b'\n', &mut head)?;
        if read == 0 {
            return if head.iter().all(|&b| b == b'\r' || b == b'\n') {
                Ok(None)
            } else if limited.limit() == 0 {
                Err(HttpError::TooLarge)
            } else {
                Err(HttpError::Truncated)
            };
        }
        if !head.ends_with(b"\n") {
            return Err(if limited.limit() == 0 {
                HttpError::TooLarge
            } else {
                HttpError::Truncated
            });
        }
        let line = &head[start..];
        if line == b"\r\n" || line == b"\n" {
            if head[..start].iter().all(|&b| b == b'\r' || b == b'\n') {
                // An empty line before the request line.
                head.clear();
                continue;
            }
            return Ok(Some(head));
        }
    }
}

/// Whether `byte` may appear in a token (RFC 9110 `tchar`).
fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

/// Parses a request head as [`read_head`] returns it.
pub(crate) fn parse_request(head: &[u8]) -> Result<Request, HttpError> {
    let mut lines = head
        .split(|&b| b == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line));
    let request_line = lines.next().ok_or(HttpError::Malformed("empty request"))?;
    let mut parts = request_line.split(|&b| b == b' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(HttpError::Malformed("bad request line"));
    };
    if method.is_empty() || !method.iter().copied().all(is_token_byte) {
        return Err(HttpError::Malformed("bad method"));
    }
    let method = match method {
        b"GET" => Method::Get,
        b"HEAD" => Method::Head,
        _ => Method::Other,
    };
    let version = match version {
        b"HTTP/1.1" => Version::Http11,
        b"HTTP/1.0" => Version::Http10,
        _ if version.starts_with(b"HTTP/") => return Err(HttpError::UnsupportedVersion),
        _ => return Err(HttpError::Malformed("bad version")),
    };
    if !target.starts_with(b"/") || !target.iter().all(|b| (0x21..0x7f).contains(b)) {
        return Err(HttpError::Malformed("bad request target"));
    }
    let path = target.split(|&b| b == b'?').next().unwrap_or(target);
    let path = String::from_utf8(path.to_vec()).map_err(|_| HttpError::Malformed("bad path"))?;
    let mut headers = Vec::new();
    for line in lines.filter(|line| !line.is_empty()) {
        if headers.len() == MAX_HEADERS {
            return Err(HttpError::TooLarge);
        }
        headers.push(parse_field(line)?);
    }
    let request = Request {
        method,
        path,
        version,
        headers,
    };
    let has_body = request.header("transfer-encoding").is_some()
        || match request.header("content-length") {
            None => false,
            Some(length) => {
                length
                    .parse::<u64>()
                    .map_err(|_| HttpError::Malformed("bad content length"))?
                    > 0
            }
        };
    if has_body {
        return Err(HttpError::UnexpectedBody);
    }
    Ok(request)
}

/// One `name: value` header field line.
fn parse_field(line: &[u8]) -> Result<(String, String), HttpError> {
    let colon = line
        .iter()
        .position(|&b| b == b':')
        .ok_or(HttpError::Malformed("header without a colon"))?;
    let (name, value) = (&line[..colon], &line[colon + 1..]);
    // Folded lines and whitespace before the colon are not allowed.
    if name.is_empty() || !name.iter().copied().all(is_token_byte) {
        return Err(HttpError::Malformed("bad header name"));
    }
    if value.iter().any(|&b| (b < 0x20 && b != b'\t') || b == 0x7f) {
        return Err(HttpError::Malformed("bad header value"));
    }
    let value = value.trim_ascii();
    Ok((
        String::from_utf8_lossy(name).into_owned(),
        String::from_utf8_lossy(value).into_owned(),
    ))
}

/// The response statuses the server uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    /// 101: the connection becomes a WebSocket.
    SwitchingProtocols,
    /// 200.
    Ok,
    /// 400.
    BadRequest,
    /// 403: a cross-origin WebSocket.
    Forbidden,
    /// 404.
    NotFound,
    /// 405: methods other than `GET` and `HEAD`.
    MethodNotAllowed,
    /// 413: a request with a body.
    PayloadTooLarge,
    /// 426: a WebSocket version other than 13.
    UpgradeRequired,
    /// 431.
    HeaderFieldsTooLarge,
    /// 500: a figure could not be rendered.
    InternalServerError,
    /// 503: too many connections, or the server is not running.
    ServiceUnavailable,
    /// 505.
    VersionNotSupported,
}

impl Status {
    /// The status code.
    pub(crate) fn code(self) -> u16 {
        match self {
            Self::SwitchingProtocols => 101,
            Self::Ok => 200,
            Self::BadRequest => 400,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::MethodNotAllowed => 405,
            Self::PayloadTooLarge => 413,
            Self::UpgradeRequired => 426,
            Self::HeaderFieldsTooLarge => 431,
            Self::InternalServerError => 500,
            Self::ServiceUnavailable => 503,
            Self::VersionNotSupported => 505,
        }
    }

    /// The reason phrase.
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::SwitchingProtocols => "Switching Protocols",
            Self::Ok => "OK",
            Self::BadRequest => "Bad Request",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not Found",
            Self::MethodNotAllowed => "Method Not Allowed",
            Self::PayloadTooLarge => "Content Too Large",
            Self::UpgradeRequired => "Upgrade Required",
            Self::HeaderFieldsTooLarge => "Request Header Fields Too Large",
            Self::InternalServerError => "Internal Server Error",
            Self::ServiceUnavailable => "Service Unavailable",
            Self::VersionNotSupported => "HTTP Version Not Supported",
        }
    }
}

/// A response to send.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Response {
    /// The status.
    pub(crate) status: Status,
    /// Header fields besides the framing ones the writer adds.
    pub(crate) headers: Vec<(&'static str, String)>,
    /// The body.
    pub(crate) body: Cow<'static, [u8]>,
}

impl Response {
    /// A 200 response with `body` of type `content_type`.
    pub(crate) fn ok(content_type: &str, body: impl Into<Cow<'static, [u8]>>) -> Self {
        Self {
            status: Status::Ok,
            headers: vec![("Content-Type", content_type.to_owned())],
            body: body.into(),
        }
    }

    /// An error response whose body names the status.
    pub(crate) fn error(status: Status) -> Self {
        let text = format!("{} {}\n", status.code(), status.reason());
        Self {
            status,
            headers: vec![("Content-Type", "text/plain; charset=utf-8".to_owned())],
            body: text.into_bytes().into(),
        }
    }

    /// Adds a header field.
    #[must_use]
    pub(crate) fn with_header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }

    /// Writes the response; the body is left out for `HEAD` requests.
    /// `keep_alive` says whether the connection stays open.
    pub(crate) fn write_to(
        &self,
        out: &mut impl Write,
        head_only: bool,
        keep_alive: bool,
    ) -> io::Result<()> {
        let mut head = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status.code(),
            self.status.reason()
        );
        // Writing to a String cannot fail.
        for (name, value) in &self.headers {
            let _ = write!(head, "{name}: {value}\r\n");
        }
        if self.status != Status::SwitchingProtocols {
            let _ = write!(head, "Content-Length: {}\r\n", self.body.len());
            head.push_str("Cache-Control: no-store\r\n");
            if !keep_alive {
                head.push_str("Connection: close\r\n");
            }
        }
        head.push_str("\r\n");
        out.write_all(head.as_bytes())?;
        if !head_only {
            out.write_all(&self.body)?;
        }
        out.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(raw: &[u8]) -> Result<Option<Request>, HttpError> {
        let mut reader = io::BufReader::new(raw);
        read_head(&mut reader)?
            .map(|head| parse_request(&head))
            .transpose()
    }

    #[test]
    fn parses_a_browser_request() {
        let raw = b"GET /1/ws?x=1 HTTP/1.1\r\nHost: localhost:8988\r\nUpgrade: websocket\r\n\
                    Connection: keep-alive, Upgrade\r\nSec-WebSocket-Version:13\r\n\r\n";
        let request = read(raw).unwrap().unwrap();
        assert_eq!(request.method, Method::Get);
        assert_eq!(request.path, "/1/ws");
        assert_eq!(request.version, Version::Http11);
        assert_eq!(request.header("HOST"), Some("localhost:8988"));
        assert_eq!(request.header("sec-websocket-version"), Some("13"));
        assert!(request.has_token("connection", "upgrade"));
        assert!(!request.has_token("connection", "close"));
        assert!(request.keep_alive());
    }

    #[test]
    fn keep_alive_follows_the_version() {
        let old = read(b"GET / HTTP/1.0\r\n\r\n").unwrap().unwrap();
        assert!(!old.keep_alive());
        let old = read(b"GET / HTTP/1.0\r\nConnection: Keep-Alive\r\n\r\n")
            .unwrap()
            .unwrap();
        assert!(old.keep_alive());
        let new = read(b"HEAD / HTTP/1.1\nConnection: close\n\n")
            .unwrap()
            .unwrap();
        assert_eq!(new.method, Method::Head);
        assert!(!new.keep_alive());
    }

    #[test]
    fn pipelined_requests_and_leading_blank_lines() {
        let raw = b"\r\nGET /a HTTP/1.1\r\n\r\nGET /b HTTP/1.1\r\n\r\n";
        let mut reader = io::BufReader::new(&raw[..]);
        let first = read_head(&mut reader).unwrap().unwrap();
        assert_eq!(parse_request(&first).unwrap().path, "/a");
        let second = read_head(&mut reader).unwrap().unwrap();
        assert_eq!(parse_request(&second).unwrap().path, "/b");
        assert!(read_head(&mut reader).unwrap().is_none(), "a clean close");
    }

    #[test]
    fn malformed_requests_are_rejected() {
        let malformed: [&[u8]; 11] = [
            b"GET\r\n\r\n",
            b"GET /  HTTP/1.1\r\n\r\n",
            b"GET / HTTP/1.1 extra\r\n\r\n",
            b"GET index HTTP/1.1\r\n\r\n",
            b"G(T / HTTP/1.1\r\n\r\n",
            b"GET / HTTX/1.1\r\n\r\n",
            b"GET / HTTP/1.1\r\nNo colon\r\n\r\n",
            b"GET / HTTP/1.1\r\nName : value\r\n\r\n",
            b"GET / HTTP/1.1\r\nName: a\r\n folded\r\n\r\n",
            b"GET / HTTP/1.1\r\nName: a\x01b\r\n\r\n",
            b"GET /\xff HTTP/1.1\r\n\r\n",
        ];
        for raw in malformed {
            let err = read(raw).expect_err(&String::from_utf8_lossy(raw));
            assert_eq!(err.status(), Some(Status::BadRequest), "{raw:?}: {err}");
        }
        let err = read(b"GET / HTTP/2.0\r\n\r\n").unwrap_err();
        assert_eq!(err.status(), Some(Status::VersionNotSupported));
        let err = read(b"GET / HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello").unwrap_err();
        assert_eq!(err.status(), Some(Status::PayloadTooLarge));
        let err = read(b"GET / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap_err();
        assert_eq!(err.status(), Some(Status::PayloadTooLarge));
        let err = read(b"GET / HTTP/1.1\r\nContent-Length: x\r\n\r\n").unwrap_err();
        assert_eq!(err.status(), Some(Status::BadRequest));
        assert!(matches!(
            read(b"GET / HTTP/1.1\r\nHost: a"),
            Err(HttpError::Truncated)
        ));
        let other = read(b"POST / HTTP/1.1\r\n\r\n").unwrap().unwrap();
        assert_eq!(other.method, Method::Other);
    }

    #[test]
    fn sizes_are_limited() {
        let mut huge = b"GET /".to_vec();
        huge.extend(std::iter::repeat_n(b'a', MAX_HEAD));
        huge.extend_from_slice(b" HTTP/1.1\r\n\r\n");
        assert!(matches!(read(&huge), Err(HttpError::TooLarge)));
        let endless = vec![b'a'; MAX_HEAD * 2];
        assert!(matches!(read(&endless), Err(HttpError::TooLarge)));
        let mut many = b"GET / HTTP/1.1\r\n".to_vec();
        for i in 0..=MAX_HEADERS {
            many.extend_from_slice(format!("X-{i}: y\r\n").as_bytes());
        }
        many.extend_from_slice(b"\r\n");
        assert!(matches!(read(&many), Err(HttpError::TooLarge)));
    }

    #[test]
    fn responses_carry_their_length() {
        let mut out = Vec::new();
        Response::ok("text/plain", &b"hi"[..])
            .write_to(&mut out, false, true)
            .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("Content-Type: text/plain\r\n"));
        assert!(text.contains("Content-Length: 2\r\n"));
        assert!(!text.contains("Connection: close"));
        assert!(text.ends_with("\r\n\r\nhi"));
        let mut out = Vec::new();
        Response::error(Status::NotFound)
            .write_to(&mut out, true, false)
            .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with("HTTP/1.1 404 Not Found\r\n"));
        assert!(text.contains("Connection: close\r\n"));
        assert!(text.ends_with("\r\n\r\n"), "no body for HEAD");
    }
}
