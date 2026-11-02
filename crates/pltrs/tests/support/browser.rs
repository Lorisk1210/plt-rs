//! A minimal browser for the web backend's tests: HTTP requests on fresh
//! connections, and a WebSocket client speaking matplotlib's protocol as
//! `mpl.js` does.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use pltrs_codec::{base64, sha1};

/// How long any step may take before the test fails.
pub(crate) const PATIENCE: Duration = Duration::from_secs(20);

pub(crate) fn connect(addr: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect(addr).expect("the server listens");
    stream
        .set_read_timeout(Some(PATIENCE))
        .expect("a socket timeout");
    stream
}

/// An HTTP response: status, headers (lower-case names) and body.
pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
}

impl Reply {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value.as_str()))
    }

    pub(crate) fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Reads a response head and, unless `head_only`, its body.
pub(crate) fn read_reply(reader: &mut BufReader<TcpStream>, head_only: bool) -> Reply {
    let mut line = String::new();
    reader.read_line(&mut line).expect("a status line");
    let status = line
        .split(' ')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("a status code");
    let mut headers = Vec::new();
    loop {
        line.clear();
        reader.read_line(&mut line).expect("a header line");
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':').expect("a header field");
        headers.push((name.to_ascii_lowercase(), value.trim().to_owned()));
    }
    let mut reply = Reply {
        status,
        headers,
        body: Vec::new(),
    };
    if !head_only && let Some(length) = reply.header("content-length") {
        let mut body = vec![0; length.parse().expect("a content length")];
        reader.read_exact(&mut body).expect("the body");
        reply.body = body;
    }
    reply
}

/// One request on a fresh connection.
pub(crate) fn request(addr: SocketAddr, raw: &str) -> Reply {
    let mut stream = connect(addr);
    stream
        .write_all(raw.as_bytes())
        .expect("the request is sent");
    read_reply(&mut BufReader::new(stream), raw.starts_with("HEAD"))
}

pub(crate) fn get(addr: SocketAddr, path: &str) -> Reply {
    request(
        addr,
        &format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\n\r\n"),
    )
}

/// What a server frame carried.
#[derive(Debug, PartialEq)]
pub(crate) enum Incoming {
    Text(String),
    Binary(Vec<u8>),
    Pong(Vec<u8>),
    Close(Option<u16>),
}

/// A minimal browser-side WebSocket.
pub(crate) struct Socket {
    pub(crate) reader: BufReader<TcpStream>,
    pub(crate) writer: TcpStream,
    /// Text messages seen while waiting for others.
    pub(crate) seen: Vec<String>,
}

impl Socket {
    /// Opens the WebSocket of figure `number`, checking the handshake.
    pub(crate) fn open(addr: SocketAddr, number: u32) -> Self {
        let mut writer = connect(addr);
        let key = base64::encode(b"plt-rs test key!");
        write!(
            writer,
            "GET /{number}/ws HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\
             Origin: http://{addr}\r\n\r\n"
        )
        .expect("the handshake is sent");
        let mut reader = BufReader::new(writer.try_clone().expect("a second handle"));
        let reply = read_reply(&mut reader, true);
        assert_eq!(reply.status, 101);
        let expected = base64::encode(&sha1::sha1(
            format!("{key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_bytes(),
        ));
        assert_eq!(
            reply.header("sec-websocket-accept"),
            Some(expected.as_str())
        );
        Self {
            reader,
            writer,
            seen: Vec::new(),
        }
    }

    /// Sends a masked frame.
    pub(crate) fn send_frame(&mut self, first: u8, payload: &[u8]) {
        let mask = [0x12, 0x34, 0x56, 0x78];
        let mut frame = vec![first];
        match payload.len() {
            len @ 0..126 => frame.push(0x80 | len as u8),
            len @ 126..65536 => {
                frame.push(0x80 | 0x7E);
                frame.extend_from_slice(&(len as u16).to_be_bytes());
            }
            len => {
                frame.push(0x80 | 0x7F);
                frame.extend_from_slice(&(len as u64).to_be_bytes());
            }
        }
        frame.extend_from_slice(&mask);
        frame.extend(payload.iter().zip(mask.iter().cycle()).map(|(b, m)| b ^ m));
        self.writer.write_all(&frame).expect("the frame is sent");
    }

    /// Sends a message of the client, as `mpl.js` does.
    pub(crate) fn send(&mut self, json: &str) {
        self.send_frame(0x81, json.as_bytes());
    }

    /// The next frame from the server.
    pub(crate) fn next(&mut self) -> Incoming {
        let mut head = [0; 2];
        self.reader.read_exact(&mut head).expect("a frame header");
        assert_eq!(head[0] & 0x80, 0x80, "the server sends whole messages");
        assert_eq!(head[1] & 0x80, 0, "server frames are not masked");
        let length = match head[1] & 0x7F {
            126 => {
                let mut bytes = [0; 2];
                self.reader.read_exact(&mut bytes).expect("a frame length");
                usize::from(u16::from_be_bytes(bytes))
            }
            127 => {
                let mut bytes = [0; 8];
                self.reader.read_exact(&mut bytes).expect("a frame length");
                usize::try_from(u64::from_be_bytes(bytes)).expect("a frame length that fits")
            }
            short => usize::from(short),
        };
        let mut payload = vec![0; length];
        self.reader
            .read_exact(&mut payload)
            .expect("the frame payload");
        match head[0] & 0x0F {
            0x1 => Incoming::Text(String::from_utf8(payload).expect("UTF-8 text")),
            0x2 => Incoming::Binary(payload),
            0xA => Incoming::Pong(payload),
            0x8 => Incoming::Close(
                payload
                    .first_chunk::<2>()
                    .map(|code| u16::from_be_bytes(*code)),
            ),
            other => panic!("unexpected opcode {other}"),
        }
    }

    /// Reads until an event of type `kind` arrives and returns it.
    pub(crate) fn event(&mut self, kind: &str) -> String {
        let wanted = format!(r#""type": "{kind}""#);
        loop {
            match self.next() {
                Incoming::Text(text) if text.contains(&wanted) => return text,
                Incoming::Text(text) => self.seen.push(text),
                other => panic!("waiting for {kind}, got {other:?}"),
            }
        }
    }

    /// Reads until a PNG frame arrives and returns it.
    pub(crate) fn frame(&mut self) -> Vec<u8> {
        loop {
            match self.next() {
                Incoming::Binary(png) => return png,
                Incoming::Text(text) => self.seen.push(text),
                other => panic!("waiting for a frame, got {other:?}"),
            }
        }
    }

    /// Whether an event of type `kind` arrived among those skipped.
    pub(crate) fn saw(&self, kind: &str) -> bool {
        let wanted = format!(r#""type": "{kind}""#);
        self.seen.iter().any(|text| text.contains(&wanted))
    }

    pub(crate) fn mouse(&mut self, kind: &str, x: f64, y: f64, buttons: u8) {
        self.send(&format!(
            r#"{{"type": "{kind}", "x": {x}, "y": {y}, "button": 0, "buttons": {buttons},
                "modifiers": [], "guiEvent": {{"isTrusted": true}}, "figure_id": 1}}"#
        ));
    }

    pub(crate) fn key(&mut self, kind: &str, key: &str) {
        self.send(&format!(
            r#"{{"type": "{kind}", "key": "{key}", "guiEvent": {{}}, "figure_id": 1}}"#
        ));
    }
}
