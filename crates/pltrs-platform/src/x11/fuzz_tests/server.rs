//! A whole backend against a fake X server on the other end of a socket
//! pair. The server sends the captured setup reply, sometimes mutated,
//! and answers every query in order with a fuzzed reply — atoms that
//! repeat or are zero, extensions, request size limits, the captured
//! keyboard and modifier mappings mutated, resource databases with any
//! `Xft.dpi`, clipboard properties of any type and format (`INCR`
//! included), images of the wrong size — or with an error. Before
//! replies it slips in random events: input for the windows, focus and
//! geometry changes, selection requests and answers, property and
//! mapping changes, client messages. Its last words may be anything;
//! then it hangs up. Meanwhile the backend opens, creates a window,
//! pastes, copies, paints, reads pixels and polls.
//!
//! Replies and events always carry the right sequence numbers and
//! lengths until the server's last message, and it hangs up rather than
//! leave the backend waiting, so that every run is deterministic.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::Instant;

use super::{HEADER_LEN, reply, word};
use crate::backend::Backend;
use crate::fuzz::{Corruption, Gen, fuzz};
use crate::window::{CursorIcon, WindowId, WindowOptions};
use crate::x11::X11Backend;
use crate::x11::atoms::predefined;
use crate::x11::connection::Connection;
use crate::x11::fixtures::{KEYBOARD_MAPPING, MODIFIER_MAPPING, SETUP};
use crate::x11::request;
use crate::x11::setup;
use crate::x11::transport::Stream;
use crate::{Frame, LogicalSize, Size};

/// Request opcodes the fake server answers.
mod opcode {
    pub(super) const CREATE_WINDOW: u8 = 1;
    pub(super) const INTERN_ATOM: u8 = 16;
    pub(super) const GET_PROPERTY: u8 = 20;
    pub(super) const GET_INPUT_FOCUS: u8 = 43;
    pub(super) const GET_IMAGE: u8 = 73;
    pub(super) const QUERY_EXTENSION: u8 = 98;
    pub(super) const GET_KEYBOARD_MAPPING: u8 = 101;
    pub(super) const GET_MODIFIER_MAPPING: u8 = 119;
}

/// The event codes the server sends: those the backend reads, a few
/// others, and a generic event.
const EVENT_CODES: [u8; 20] = [
    2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 22, 28, 29, 30, 31, 33, 34, 35, 0x7F,
];

/// The forms of the clipboard other clients ask for.
const SELECTION_TARGETS: [&str; 5] = [
    "TARGETS",
    "UTF8_STRING",
    "TEXT",
    "text/plain;charset=utf-8",
    "image/png",
];

/// The values of `Xft.dpi` the server publishes.
const DPI: [&str; 11] = [
    "96", "192", "0", "-1", "1e-300", "1e300", "NaN", "inf", "lots", "4.9e-324", "",
];

/// The fake server's state: what it was asked, and what it answered.
struct FakeServer {
    stream: UnixStream,
    g: Gen,
    /// Requests read so far (the low 16 bits are their sequence number).
    sequence: u16,
    /// Requests left before the server hangs up.
    budget: usize,
    /// The atoms it gave out, by name.
    atoms: HashMap<Vec<u8>, u32>,
    next_atom: u32,
    /// The major opcode of BIG-REQUESTS, once given out.
    big_requests: Option<u8>,
    /// The root window of the first screen.
    root: u32,
    /// The windows created.
    windows: Vec<u32>,
}

impl FakeServer {
    /// Sends the setup reply (mutated by `setup`), then answers requests
    /// until the budget is spent or the client hangs up.
    fn run(stream: UnixStream, setup: Option<&Corruption>, seed: u64) {
        let reply = setup.map_or_else(|| SETUP.to_vec(), |c| c.apply(SETUP));
        let root = setup::parse_reply(&reply)
            .ok()
            .and_then(|setup| Some(setup.screens.first()?.root))
            .unwrap_or_default();
        let mut g = Gen::new(seed);
        let mut server = Self {
            stream,
            budget: g.pick(&[30, 200, 400, 2_000, 2_000]),
            g,
            sequence: 0,
            atoms: HashMap::new(),
            next_atom: 0x200,
            big_requests: None,
            root,
            windows: Vec::new(),
        };
        let declared = reply
            .get(6..8)
            .map(|len| 8 + 4 * usize::from(u16::from_le_bytes([len[0], len[1]])));
        if server.read_setup_request() && server.write(&reply) && declared == Some(reply.len()) {
            server.serve();
        }
    }

    /// Reads the setup request with its credentials.
    fn read_setup_request(&mut self) -> bool {
        let mut head = [0; 12];
        if self.stream.read_exact(&mut head).is_err() {
            return false;
        }
        let padded = |at: usize| {
            usize::from(u16::from_le_bytes([head[at], head[at + 1]])).next_multiple_of(4)
        };
        let mut credentials = vec![0; padded(6) + padded(8)];
        self.stream.read_exact(&mut credentials).is_ok()
    }

    fn write(&mut self, bytes: &[u8]) -> bool {
        self.stream.write_all(bytes).is_ok()
    }

    /// Answers requests until the budget is spent, then says its last
    /// words and hangs up.
    fn serve(&mut self) {
        while self.budget > 0 {
            self.budget -= 1;
            let Some((op, body)) = self.read_request() else {
                return;
            };
            self.sequence = self.sequence.wrapping_add(1);
            if !self.answer(op, &body) {
                return;
            }
        }
        let last = self.last_words();
        self.write(&last);
    }

    /// Reads a request: its opcode and the bytes after its header.
    fn read_request(&mut self) -> Option<(u8, Vec<u8>)> {
        let mut head = [0; 4];
        self.stream.read_exact(&mut head).ok()?;
        let words = usize::from(u16::from_le_bytes([head[2], head[3]]));
        let (words, header) = if words == 0 {
            let mut long = [0; 4];
            self.stream.read_exact(&mut long).ok()?;
            (usize::try_from(u32::from_le_bytes(long)).ok()?, 8)
        } else {
            (words, 4)
        };
        let mut body = vec![0; (words * 4).checked_sub(header)?];
        self.stream.read_exact(&mut body).ok()?;
        Some((head[0], body))
    }

    /// Answers one request; `false` once the client is gone.
    fn answer(&mut self, op: u8, body: &[u8]) -> bool {
        if op == opcode::CREATE_WINDOW
            && let Some(id) = body.first_chunk::<4>()
        {
            self.windows.push(u32::from_le_bytes(*id));
        }
        let reply = match op {
            opcode::INTERN_ATOM => Some(self.intern_atom(body)),
            opcode::QUERY_EXTENSION => Some(self.query_extension(body)),
            opcode::GET_KEYBOARD_MAPPING => Some(self.mapping(KEYBOARD_MAPPING)),
            opcode::GET_MODIFIER_MAPPING => Some(self.mapping(MODIFIER_MAPPING)),
            opcode::GET_PROPERTY => Some(self.property(body)),
            opcode::GET_IMAGE => Some(self.image(body)),
            opcode::GET_INPUT_FOCUS => Some(reply(1, self.sequence, &[1], &[])),
            op if Some(op) == self.big_requests => {
                let words = self.g.pick(&[0, 1, 4096, 65_535, 0x40_0000, u32::MAX]);
                Some(reply(0, self.sequence, &[words], &[]))
            }
            _ => None,
        };
        let mut out = Vec::new();
        if reply.is_some() && self.g.chance(0.4) {
            for _ in 0..=self.g.below(6) {
                out.extend(self.event());
            }
        }
        match reply {
            Some(_) if self.g.chance(0.01) => out.extend(self.error(op)),
            Some(reply) => out.extend(reply),
            None if self.g.chance(0.02) => out.extend(self.error(op)),
            None => {}
        }
        out.is_empty() || self.write(&out)
    }

    /// An error for request `op`.
    fn error(&mut self, op: u8) -> Vec<u8> {
        let mut error = vec![0, 1 + self.g.below(17) as u8];
        error.extend(self.sequence.to_le_bytes());
        error.extend(word(&mut self.g).to_le_bytes());
        error.extend([0, 0, op]);
        error.resize(HEADER_LEN, 0);
        error
    }

    /// The atom of the name in an `InternAtom` request: a new one mostly,
    /// sometimes zero, one given out before or anything.
    fn intern_atom(&mut self, body: &[u8]) -> Vec<u8> {
        let name = string(body);
        let atom = match self.g.below(20) {
            0 => 0,
            1 => self.atoms.values().next().copied().unwrap_or(1),
            2 => word(&mut self.g),
            _ => {
                self.next_atom += 1;
                self.next_atom
            }
        };
        self.atoms.insert(name, atom);
        reply(0, self.sequence, &[atom], &[])
    }

    /// Whether an extension is present: BIG-REQUESTS mostly is.
    fn query_extension(&mut self, body: &[u8]) -> Vec<u8> {
        let present = self.g.chance(0.7);
        let major = 128 + self.g.below(100) as u8;
        if present && string(body) == b"BIG-REQUESTS" {
            self.big_requests = Some(major);
        }
        let fields = u32::from(present) | u32::from(major) << 8;
        reply(0, self.sequence, &[fields], &[])
    }

    /// A captured mapping reply, mutated sometimes, with this sequence
    /// number and a length that matches.
    fn mapping(&mut self, captured: &[u8]) -> Vec<u8> {
        let mut bytes = if self.g.chance(0.1) {
            self.g.corrupt(captured).apply(captured)
        } else {
            captured.to_vec()
        };
        bytes.resize(bytes.len().max(HEADER_LEN).next_multiple_of(4), 0);
        bytes[0] = 1;
        bytes[2..4].copy_from_slice(&self.sequence.to_le_bytes());
        let words = u32::try_from((bytes.len() - HEADER_LEN) / 4).unwrap_or(u32::MAX);
        bytes[4..8].copy_from_slice(&words.to_le_bytes());
        bytes
    }

    /// The atom `name` was given, if it was.
    fn atom(&self, name: &str) -> u32 {
        self.atoms.get(name.as_bytes()).copied().unwrap_or(0)
    }

    /// A property: the resource database with some `Xft.dpi`, or what a
    /// clipboard owner stored, in any type and format.
    fn property(&mut self, body: &[u8]) -> Vec<u8> {
        let property = body
            .get(4..8)
            .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        let (kind, format, value) = if property == predefined::RESOURCE_MANAGER {
            let dpi = self.g.pick(&DPI);
            let database = format!("Xcursor.size:\t24\nXft.dpi:\t{dpi}\n");
            (predefined::STRING, 8, database.into_bytes())
        } else {
            let kinds = [
                self.atom("UTF8_STRING"),
                predefined::STRING,
                self.atom("INCR"),
                predefined::ATOM,
                word(&mut self.g),
            ];
            let value = match self.g.below(3) {
                0 => "pasted é 😀\n".as_bytes().to_vec(),
                1 => (0..self.g.len(64))
                    .map(|_| self.g.below(256) as u8)
                    .collect(),
                _ => word(&mut self.g).to_le_bytes().to_vec(),
            };
            (
                self.g.pick(&kinds),
                self.g.pick(&[8, 8, 0, 16, 32, 7]),
                value,
            )
        };
        let unit = usize::from(format / 8).max(1);
        let units = if self.g.chance(0.1) {
            word(&mut self.g)
        } else {
            u32::try_from(value.len() / unit).unwrap_or(u32::MAX)
        };
        reply(format, self.sequence, &[kind, 0, units], &value)
    }

    /// Image data for a `GetImage` request: of the size asked for (at 32
    /// bits a pixel), or any size.
    fn image(&mut self, body: &[u8]) -> Vec<u8> {
        let side = |at: usize| {
            body.get(at..at + 2)
                .map_or(0, |b| usize::from(u16::from_le_bytes([b[0], b[1]])))
        };
        let asked = side(8) * side(10) * 4;
        let len = match self.g.below(3) {
            0 if asked <= 1 << 22 => asked,
            1 => asked.min(1 << 22).saturating_sub(1 + self.g.below(8)),
            _ => self.g.below(5_000),
        };
        let data: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        reply(24, self.sequence, &[0x21], &data)
    }

    /// A random event, mostly about the windows and atoms the client
    /// knows.
    fn event(&mut self) -> [u8; 32] {
        let mut raw = [0; 32];
        for byte in &mut raw[4..] {
            *byte = self.g.below(256) as u8;
        }
        let code = self.g.pick(&EVENT_CODES);
        raw[0] = if self.g.chance(0.1) {
            code | 0x80
        } else {
            code
        };
        raw[1] = self.g.pick(&[0, 1, 3, 4, 5, 6, 7, 8, 32, 38, 255]);
        raw[2..4].copy_from_slice(&self.sequence.to_le_bytes());
        let window = if self.windows.is_empty() || self.g.chance(0.1) {
            word(&mut self.g)
        } else {
            self.g.pick(&self.windows)
        };
        let set = |raw: &mut [u8; 32], at: usize, value: u32| {
            raw[at..at + 4].copy_from_slice(&value.to_le_bytes());
        };
        match code {
            // Key, button, motion and crossing events: the window at 12.
            2..=8 => set(&mut raw, 12, window),
            // Focus, expose, property and client messages: the window at 4.
            9 | 10 | 12 | 28 | 33 => set(&mut raw, 4, window),
            // ConfigureNotify: the window at 8.
            22 => set(&mut raw, 8, window),
            // A generic event without extra data.
            35 => set(&mut raw, 4, 0),
            _ => {}
        }
        let clipboard = self.atom("CLIPBOARD");
        match code {
            28 if self.g.chance(0.7) => {
                set(&mut raw, 4, self.root);
                set(&mut raw, 8, predefined::RESOURCE_MANAGER);
            }
            29 => set(&mut raw, 12, clipboard),
            30 => {
                set(&mut raw, 12, window);
                set(&mut raw, 16, clipboard);
                let target = if self.g.chance(0.8) {
                    let name = self.g.pick(&SELECTION_TARGETS);
                    self.atom(name)
                } else {
                    word(&mut self.g)
                };
                set(&mut raw, 20, target);
            }
            31 => {
                set(&mut raw, 8, window);
                set(&mut raw, 12, clipboard);
                let property = if self.g.chance(0.8) {
                    self.atom("_PLTRS_PLATFORM_PASTE")
                } else {
                    0
                };
                set(&mut raw, 20, property);
            }
            33 if self.g.chance(0.8) => {
                raw[1] = 32;
                let (kind, protocol) = match self.g.below(3) {
                    0 => (self.atom("WM_PROTOCOLS"), self.atom("WM_DELETE_WINDOW")),
                    1 => (self.atom("WM_PROTOCOLS"), self.atom("_NET_WM_PING")),
                    _ => (self.atom("_PLTRS_PLATFORM_WAKE"), 0),
                };
                set(&mut raw, 8, kind);
                set(&mut raw, 12, protocol);
            }
            34 => raw[4] = self.g.pick(&[0, 1, 2]),
            _ => {}
        }
        raw
    }

    /// What the server says before it hangs up: nothing, junk, or a reply
    /// that promises more than ever comes.
    fn last_words(&mut self) -> Vec<u8> {
        match self.g.below(3) {
            0 => Vec::new(),
            1 => (0..=self.g.below(64))
                .map(|_| self.g.below(256) as u8)
                .collect(),
            _ => {
                let mut promise = reply(0, self.sequence, &[], &[]);
                promise[4..8].copy_from_slice(&word(&mut self.g).to_le_bytes());
                promise
            }
        }
    }
}

/// The string of an `InternAtom` or `QueryExtension` request body.
fn string(body: &[u8]) -> Vec<u8> {
    let len = body
        .first_chunk::<2>()
        .map_or(0, |b| usize::from(u16::from_le_bytes(*b)));
    body.get(4..4 + len).unwrap_or_default().to_vec()
}

/// What the backend does between polls.
#[derive(Clone, Copy, Debug)]
enum Action {
    Poll,
    Paste,
    CopyText,
    CopyImage,
    Screenshot,
    Paint,
    Title,
    Cursor(CursorIcon),
    Raise,
    Fullscreen(bool),
    Resize(f64, f64),
}

impl Action {
    fn generate(g: &mut Gen) -> Self {
        match g.below(12) {
            0 | 1 => Self::Poll,
            2 => Self::Paste,
            3 => Self::CopyText,
            4 => Self::CopyImage,
            5 => Self::Screenshot,
            6 => Self::Paint,
            7 => Self::Title,
            8 => Self::Cursor(g.pick(&[CursorIcon::Pointer, CursorIcon::Text, CursorIcon::Wait])),
            9 => Self::Raise,
            10 => Self::Fullscreen(g.chance(0.5)),
            _ => Self::Resize(g.pick(&[0.0, 1.0, 300.0, 1e6]), g.pick(&[0.0, 1.0, 200.0])),
        }
    }
}

/// Does `action` on `window`, then waits for the server to catch up and
/// takes the events it sent; errors are fine.
fn perform(backend: &mut X11Backend, window: WindowId, action: Action) {
    let _ = match action {
        Action::Poll => Ok(()),
        Action::Paste => backend.request_clipboard_text(window),
        Action::CopyText => backend.set_clipboard_text(window, "copied é".to_owned()),
        Action::CopyImage => backend.set_clipboard_image(window, vec![0x89, b'P', b'N', b'G']),
        Action::Screenshot => backend.read_pixels(window).map(drop),
        Action::Paint => paint(backend, window),
        Action::Title => backend.set_title(window, "Figure é 1"),
        Action::Cursor(icon) => backend.set_cursor(window, icon),
        Action::Raise => backend.raise_window(window),
        Action::Fullscreen(on) => backend.set_fullscreen(window, on),
        Action::Resize(width, height) => {
            backend.set_inner_size(window, Size::Logical(LogicalSize::new(width, height)))
        }
    };
    let _ = backend.conn.round_trip(&request::get_input_focus());
    let mut events = Vec::new();
    let _ = backend.poll(Some(Instant::now()), &mut events);
}

/// Presents a grey frame over the whole window, if it is not huge.
fn paint(backend: &mut X11Backend, window: WindowId) -> Result<(), crate::Error> {
    let size = backend.inner_size(window)?;
    if size.area() > 1 << 20 {
        return Ok(());
    }
    let pixels = vec![128; size.area() as usize * 4];
    let frame = Frame::new(size.width, size.height, &pixels)?;
    backend.present(window, &frame, crate::PhysicalRect::from_size(size))
}

/// What the server does and what the backend does meanwhile.
#[derive(Debug)]
struct Session {
    setup: Option<Corruption>,
    server_seed: u64,
    screen: u16,
    actions: Vec<Action>,
}

#[test]
fn fake_servers() {
    fuzz(
        "x11 fake server",
        500,
        |g| Session {
            setup: g.chance(0.3).then(|| g.corrupt(SETUP)),
            server_seed: word(g).into(),
            screen: u16::from(g.chance(0.05)),
            actions: (0..g.len(24)).map(|_| Action::generate(g)).collect(),
        },
        |session| {
            let (ours, theirs) = UnixStream::pair().expect("a socket pair");
            let setup = session.setup.clone();
            let seed = session.server_seed;
            let server = thread::spawn(move || FakeServer::run(theirs, setup.as_ref(), seed));
            if let Ok(conn) = Connection::handshake(Stream::Unix(ours), None)
                && let Ok(mut backend) = X11Backend::with_connection(conn, session.screen)
                && let Ok(window) = backend.create_window(&WindowOptions::new("fuzz"))
            {
                for &action in &session.actions {
                    perform(&mut backend, window, action);
                }
                let _ = backend.destroy_window(window);
            }
            server.join().expect("the fake server does not panic");
        },
    );
}
