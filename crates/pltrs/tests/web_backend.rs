//! The web backend end to end: a server on a free local port, a browser
//! played by a minimal HTTP and WebSocket client, and the figure changing
//! as matplotlib's client drives it.

use std::cell::RefCell;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use pltrs::events::{Event, EventKind};
use pltrs::figure::Figure;
use pltrs::web::{WebOptions, WebServer};
use pltrs_codec::{base64, png, sha1};

/// How long any step may take before the test fails.
const PATIENCE: Duration = Duration::from_secs(20);

/// A phase the client thread reached; `FAILED` when it panicked.
struct Phase(Arc<AtomicUsize>);

const FAILED: usize = usize::MAX;

impl Drop for Phase {
    fn drop(&mut self) {
        if thread::panicking() {
            self.0.store(FAILED, Ordering::SeqCst);
        }
    }
}

impl Phase {
    /// Records the phase; the server notices it after its next message.
    fn reach(&self, phase: usize) {
        self.0.store(phase, Ordering::SeqCst);
    }
}

/// Runs the server until the client reaches `phase`; a failed client
/// fails the test with its own message.
fn serve_until(
    server: &mut WebServer,
    phase: &Arc<AtomicUsize>,
    target: usize,
    client: &mut Option<JoinHandle<()>>,
) {
    let deadline = Instant::now() + PATIENCE;
    server.run(Some(deadline), || phase.load(Ordering::SeqCst) >= target);
    let reached = phase.load(Ordering::SeqCst);
    if reached == FAILED || reached < target {
        if let Some(client) = client.take()
            && let Err(panic) = client.join()
        {
            std::panic::resume_unwind(panic);
        }
        panic!("the client did not reach phase {target} (at {reached})");
    }
}

/// A 400 × 300 figure with one axes over [0, 10]², served on a free port.
fn served() -> (WebServer, Figure, u32) {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    let ax = figure
        .add_axes([0.0, 0.0, 1.0, 1.0])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 10.0)
        .and_then(|ax| ax.set_ylim(0.0, 10.0))
        .expect("finite limits");
    let mut server = WebServer::bind(WebOptions::new("127.0.0.1", 0)).expect("a free port");
    let number = server.add_figure(&figure);
    (server, figure, number)
}

fn connect(addr: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect(addr).expect("the server listens");
    stream
        .set_read_timeout(Some(PATIENCE))
        .expect("a socket timeout");
    stream
}

/// An HTTP response: status, headers (lower-case names) and body.
struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value.as_str()))
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Reads a response head and, unless `head_only`, its body.
fn read_reply(reader: &mut BufReader<TcpStream>, head_only: bool) -> Reply {
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
fn request(addr: SocketAddr, raw: &str) -> Reply {
    let mut stream = connect(addr);
    stream
        .write_all(raw.as_bytes())
        .expect("the request is sent");
    read_reply(&mut BufReader::new(stream), raw.starts_with("HEAD"))
}

fn get(addr: SocketAddr, path: &str) -> Reply {
    request(
        addr,
        &format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\n\r\n"),
    )
}

/// What a server frame carried.
#[derive(Debug, PartialEq)]
enum Incoming {
    Text(String),
    Binary(Vec<u8>),
    Pong(Vec<u8>),
    Close(Option<u16>),
}

/// A minimal browser-side WebSocket.
struct Socket {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    /// Text messages seen while waiting for others.
    seen: Vec<String>,
}

impl Socket {
    /// Opens the WebSocket of figure `number`, checking the handshake.
    fn open(addr: SocketAddr, number: u32) -> Self {
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
    fn send_frame(&mut self, first: u8, payload: &[u8]) {
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
    fn send(&mut self, json: &str) {
        self.send_frame(0x81, json.as_bytes());
    }

    /// The next frame from the server.
    fn next(&mut self) -> Incoming {
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
    fn event(&mut self, kind: &str) -> String {
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
    fn frame(&mut self) -> Vec<u8> {
        loop {
            match self.next() {
                Incoming::Binary(png) => return png,
                Incoming::Text(text) => self.seen.push(text),
                other => panic!("waiting for a frame, got {other:?}"),
            }
        }
    }

    /// Whether an event of type `kind` arrived among those skipped.
    fn saw(&self, kind: &str) -> bool {
        let wanted = format!(r#""type": "{kind}""#);
        self.seen.iter().any(|text| text.contains(&wanted))
    }

    fn mouse(&mut self, kind: &str, x: f64, y: f64, buttons: u8) {
        self.send(&format!(
            r#"{{"type": "{kind}", "x": {x}, "y": {y}, "button": 0, "buttons": {buttons},
                "modifiers": [], "guiEvent": {{"isTrusted": true}}, "figure_id": 1}}"#
        ));
    }

    fn key(&mut self, kind: &str, key: &str) {
        self.send(&format!(
            r#"{{"type": "{kind}", "key": "{key}", "guiEvent": {{}}, "figure_id": 1}}"#
        ));
    }
}

/// The browser's side of the session, in phases the server's thread
/// checks the figure between.
fn browse(addr: SocketAddr, number: u32, phase: &Phase, go: &Receiver<()>) {
    let mut socket = Socket::open(addr, number);
    // On connecting, the server sizes the client's canvas.
    let resize = socket.event("resize");
    assert!(
        resize.contains(r#""size": [400, 300], "forward": true"#),
        "{resize}"
    );
    // What mpl.js sends once the socket is open.
    socket.send(r#"{"type": "supports_binary", "value": true, "figure_id": 1}"#);
    socket.send(r#"{"type": "send_image_mode", "figure_id": 1}"#);
    socket.send(r#"{"type": "set_device_pixel_ratio", "device_pixel_ratio": 1, "figure_id": 1}"#);
    socket.send(r#"{"type": "refresh", "figure_id": 1}"#);
    assert!(socket.event("image_mode").contains(r#""mode": "full""#));
    assert_eq!(
        socket.event("figure_label"),
        r#"{"type": "figure_label", "label": "Figure 1"}"#
    );
    let history = socket.event("history_buttons");
    assert!(
        history.contains(r#""Back": false, "Forward": false"#),
        "{history}"
    );
    socket.event("draw");
    // The canvas element reports its size; the server confirms it.
    socket.send(r#"{"type": "resize", "width": 400, "height": 300, "figure_id": 1}"#);
    let resize = socket.event("resize");
    assert!(
        resize.contains(r#""size": [400, 300], "forward": false"#),
        "{resize}"
    );
    socket.event("draw");
    socket.send(r#"{"type": "draw", "figure_id": 1}"#);
    let image = png::decode(&socket.frame()).expect("a PNG frame");
    assert_eq!((image.width(), image.height()), (400, 300));
    socket.send(r#"{"type": "ack", "figure_id": 1}"#);

    // Pan: drag the view 40 pixels (one unit) to the right.
    socket.send(r#"{"type": "toolbar_button", "name": "pan", "figure_id": 1}"#);
    assert!(socket.event("navigate_mode").contains(r#""mode": "PAN""#));
    socket.mouse("button_press", 200.0, 150.0, 1);
    socket.mouse("motion_notify", 240.0, 150.0, 1);
    let readout = socket.event("message");
    assert!(readout.contains("(x, y) = "), "{readout}");
    socket.mouse("button_release", 240.0, 150.0, 0);
    let history = socket.event("history_buttons");
    assert!(
        history.contains(r#""Back": true, "Forward": false"#),
        "{history}"
    );
    assert!(socket.saw("cursor"), "the pan cursor");
    phase.reach(1);
    socket.send(r#"{"type": "ack", "figure_id": 1}"#);
    go.recv().expect("the server goes on");

    // The view changed: a draw brings a difference frame.
    socket.send(r#"{"type": "draw", "figure_id": 1}"#);
    assert!(socket.event("image_mode").contains(r#""mode": "diff""#));
    png::decode(&socket.frame()).expect("a PNG frame");
    socket.send(r#"{"type": "ack", "figure_id": 1}"#);
    // Pings are answered.
    socket.send_frame(0x89, b"ping?");
    loop {
        match socket.next() {
            Incoming::Pong(payload) => {
                assert_eq!(payload, b"ping?");
                break;
            }
            Incoming::Text(text) => socket.seen.push(text),
            other => panic!("waiting for the pong, got {other:?}"),
        }
    }
    // A key for the callbacks, then `h` (home); the image mode answers
    // once the server has handled them.
    socket.key("key_press", "ka");
    socket.key("key_release", "ka");
    socket.key("key_press", "kh");
    socket.key("key_release", "kh");
    socket.send(r#"{"type": "send_image_mode", "figure_id": 1}"#);
    socket.event("image_mode");
    phase.reach(2);
    socket.send(r#"{"type": "ack", "figure_id": 1}"#);
    go.recv().expect("the server goes on");

    // `q` closes the figure, and its WebSocket.
    socket.key("key_press", "kq");
    loop {
        match socket.next() {
            Incoming::Close(code) => {
                assert_eq!(code, Some(1001));
                break;
            }
            Incoming::Text(text) => socket.seen.push(text),
            other => panic!("waiting for the close, got {other:?}"),
        }
    }
    phase.reach(3);
}

#[test]
fn a_browser_session_drives_the_figure() {
    let (mut server, figure, number) = served();
    let keys = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&keys);
    figure.connect(EventKind::KeyPress, move |event| {
        if let Event::KeyPress(key) = event {
            log.borrow_mut().push(key.key.clone());
        }
    });
    let addr = server.local_addr();
    let phase = Arc::new(AtomicUsize::new(0));
    let (go, ready) = mpsc::channel();
    let reached = Phase(Arc::clone(&phase));
    let mut client = Some(thread::spawn(move || {
        browse(addr, number, &reached, &ready);
    }));

    serve_until(&mut server, &phase, 1, &mut client);
    let ax = &figure.axes()[0];
    let (low, high) = ax.xlim();
    assert!(
        (low + 1.0).abs() < 1e-9 && (high - 9.0).abs() < 1e-9,
        "panned to {low}..{high}"
    );
    go.send(()).unwrap();

    serve_until(&mut server, &phase, 2, &mut client);
    assert_eq!(ax.xlim(), (0.0, 10.0), "home");
    assert_eq!(keys.borrow().as_slice(), ["a", "h"]);
    go.send(()).unwrap();

    // The last figure closes, so serving ends by itself.
    server.run(Some(Instant::now() + PATIENCE), || false);
    assert!(server.is_empty());
    assert_eq!(server.take_closed(), [figure]);
    client.take().unwrap().join().unwrap();
}

/// Plain HTTP: the pages, the script, static files and downloads.
fn fetch(addr: SocketAddr, number: u32, phase: &Phase) {
    let page = get(addr, "/");
    assert_eq!(page.status, 200);
    assert_eq!(
        page.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert!(
        page.text()
            .contains(&format!("var uri = \"ws://{addr}/\" + fig_id + \"/ws\";"))
    );
    assert!(
        page.text()
            .contains(&format!("ready(figure_ready({number}));"))
    );
    let page = get(addr, &format!("/{number}"));
    assert!(
        page.text()
            .contains(&format!("\"ws://{addr}/\" + {number} + \"/ws\""))
    );
    assert_eq!(get(addr, "/42").status, 404);
    let script = get(addr, "/js/mpl.js");
    assert!(script.text().contains("mpl.toolbar_items = [[\"Home\""));
    assert!(script.text().ends_with("mpl.default_extension = \"png\";"));
    let icon = get(addr, "/_images/home.png");
    assert_eq!(icon.header("content-type"), Some("image/png"));
    assert!(icon.body.starts_with(b"\x89PNG"));
    let head = request(
        addr,
        &format!("HEAD /favicon.ico HTTP/1.1\r\nHost: {addr}\r\n\r\n"),
    );
    assert_eq!(head.status, 200);
    assert_eq!(head.body, b"");
    // Downloads are rendered by the server's thread.
    let svg = get(addr, &format!("/{number}/download.svg"));
    assert_eq!(svg.header("content-type"), Some("image/svg+xml"));
    assert!(svg.text().contains("<svg"));
    let png = get(addr, &format!("/{number}/download.png"));
    let image = png::decode(&png.body).expect("a PNG download");
    assert_eq!((image.width(), image.height()), (400, 300));
    assert_eq!(get(addr, &format!("/{number}/download.avif")).status, 404);
    // Two requests on one connection.
    let mut stream = connect(addr);
    write!(
        stream,
        "GET /_static/css/mpl.css HTTP/1.1\r\nHost: {addr}\r\n\r\n\
         GET /nothing HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
    )
    .expect("the requests are sent");
    let mut reader = BufReader::new(stream);
    assert_eq!(read_reply(&mut reader, false).status, 200);
    let last = read_reply(&mut reader, false);
    assert_eq!(last.status, 404);
    assert_eq!(last.header("connection"), Some("close"));
    phase.reach(1);
    // A last download lets the server see the phase.
    assert_eq!(get(addr, &format!("/{number}/download.pdf")).status, 200);
}

#[test]
fn pages_scripts_and_downloads() {
    let (mut server, _figure, number) = served();
    let addr = server.local_addr();
    let phase = Arc::new(AtomicUsize::new(0));
    let reached = Phase(Arc::clone(&phase));
    let mut client = Some(thread::spawn(move || fetch(addr, number, &reached)));
    serve_until(&mut server, &phase, 1, &mut client);
    client.take().unwrap().join().unwrap();
}

/// Reads until the server closes the connection; the close code, if a
/// close frame came first.
fn closed_with(socket: &mut Socket) -> Option<u16> {
    loop {
        match socket.reader.fill_buf() {
            Ok([]) | Err(_) => return None,
            Ok(_) => {}
        }
        if let Incoming::Close(code) = socket.next() {
            // Nothing may follow the close frame.
            let mut byte = [0];
            assert!(matches!(socket.reader.read(&mut byte), Ok(0) | Err(_)));
            return code;
        }
    }
}

/// Hostile clients: each is answered with an error and disconnected.
fn attack(addr: SocketAddr, number: u32, phase: &Phase) {
    // Unmasked client frames break the protocol (1002).
    let mut socket = Socket::open(addr, number);
    socket
        .writer
        .write_all(&[0x81, 0x02])
        .expect("the frame is sent");
    assert_eq!(closed_with(&mut socket), Some(1002));
    // A frame claiming a terabyte is refused from its header (1009).
    let mut socket = Socket::open(addr, number);
    socket
        .writer
        .write_all(&[0x81, 0xFF, 0, 0, 0x01, 0, 0, 0, 0, 0])
        .expect("the frame is sent");
    assert_eq!(closed_with(&mut socket), Some(1009));
    // Text that is not a client message (1008), or not UTF-8 (1007).
    let mut socket = Socket::open(addr, number);
    socket.send(r#"{"type": "motion_notify", "x": "left"}"#);
    assert_eq!(closed_with(&mut socket), Some(1008));
    let mut socket = Socket::open(addr, number);
    socket.send_frame(0x81, &[0xC3, 0x28]);
    assert_eq!(closed_with(&mut socket), Some(1007));
    // A client may close politely: the server answers in kind.
    let mut socket = Socket::open(addr, number);
    socket.send_frame(0x88, &1000_u16.to_be_bytes());
    assert_eq!(closed_with(&mut socket), Some(1000));
    // Broken HTTP.
    assert_eq!(request(addr, "GARBAGE\r\n\r\n").status, 400);
    assert_eq!(request(addr, "GET / HTTP/3.0\r\n\r\n").status, 505);
    let long = format!("GET /{} HTTP/1.1\r\n\r\n", "a".repeat(20_000));
    assert_eq!(request(addr, &long).status, 431);
    assert_eq!(
        request(addr, "POST / HTTP/1.1\r\nContent-Length: 0\r\n\r\n").status,
        405
    );
    let foreign = format!(
        "GET /{number}/ws HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: {}\r\nSec-WebSocket-Version: 13\r\nOrigin: http://evil.example\r\n\r\n",
        base64::encode(b"0123456789abcdef")
    );
    assert_eq!(request(addr, &foreign).status, 403);
    let unknown = foreign.replace(&format!("/{number}/ws"), "/77/ws");
    assert_eq!(request(addr, &unknown).status, 404);
    // The server still serves well-behaved clients.
    assert_eq!(get(addr, "/").status, 200);
    let mut socket = Socket::open(addr, number);
    socket.send(r#"{"type": "refresh", "figure_id": 1}"#);
    socket.event("figure_label");
    phase.reach(1);
    socket.send(r#"{"type": "ack", "figure_id": 1}"#);
}

#[test]
fn hostile_clients_are_disconnected() {
    let (mut server, _figure, number) = served();
    let addr = server.local_addr();
    let phase = Arc::new(AtomicUsize::new(0));
    let reached = Phase(Arc::clone(&phase));
    let mut client = Some(thread::spawn(move || attack(addr, number, &reached)));
    serve_until(&mut server, &phase, 1, &mut client);
    client.take().unwrap().join().unwrap();
}
