//! A scripted compositor for tests, on the other end of a socket pair:
//! it announces globals, answers round trips, configures new windows,
//! hands out a keymap, shows frames (frame callbacks done, buffers
//! released at the next commit) and records every request. Tests inject
//! input with [`Fake::send`]; the fuzz tests inject anything.

use std::collections::{HashMap, VecDeque};
use std::os::fd::OwnedFd;
use std::os::unix::fs::FileExt;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use super::WaylandBackend;
use super::connection::Connection;
use super::fixtures::DE_RU_KEYMAP;
use super::sys;
use super::wire::{Args, HEADER_LEN, Header, Request};

/// What the fake compositor offers.
#[derive(Debug, Clone)]
pub(crate) struct Options {
    /// Globals beyond the core ones (compositor, subcompositor, shm,
    /// `xdg_wm_base`, output), with their versions.
    pub(crate) globals: Vec<(&'static str, u32)>,
    /// The output's scale.
    pub(crate) output_scale: i32,
    /// The fractional scale offered to new surfaces, in 120ths.
    pub(crate) fractional: u32,
    /// The decoration mode chosen for toplevels asking for one.
    pub(crate) decoration_mode: u32,
}

impl Default for Options {
    /// A desktop with a seat and a clipboard, without the optional
    /// protocols.
    fn default() -> Self {
        Self {
            globals: vec![("wl_seat", 8), ("wl_data_device_manager", 3)],
            output_scale: 1,
            fractional: 120,
            decoration_mode: 2,
        }
    }
}

/// A request the fake received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Received {
    /// The object's interface name.
    pub(crate) interface: String,
    /// The object.
    pub(crate) object: u32,
    /// The request.
    pub(crate) opcode: u16,
    /// The arguments' bytes.
    pub(crate) body: Vec<u8>,
}

impl Received {
    /// Argument word `index`.
    pub(crate) fn word(&self, index: usize) -> u32 {
        let bytes = &self.body[index * 4..index * 4 + 4];
        u32::from_ne_bytes(bytes.try_into().expect("4 bytes"))
    }
}

/// What the fake knows.
#[derive(Debug, Default)]
pub(crate) struct State {
    /// The interface of every client object.
    pub(crate) objects: HashMap<u32, String>,
    /// Every request, in order.
    pub(crate) requests: Vec<Received>,
    /// Frame callbacks waiting for the next commit of their surface.
    frames: HashMap<u32, Vec<u32>>,
    /// The buffer attached to each surface, released by the next.
    attached: HashMap<u32, u32>,
    /// The `xdg_surface` and toplevel of each surface.
    toplevels: HashMap<u32, (u32, u32)>,
    /// The surfaces configured so far.
    configured: Vec<u32>,
    /// The next id of objects the fake creates.
    next_server_id: u32,
    serial: u32,
}

impl State {
    /// The first object of `interface`.
    pub(crate) fn id_of(&self, interface: &str) -> Option<u32> {
        let mut ids: Vec<u32> = self
            .objects
            .iter()
            .filter(|(_, known)| known.as_str() == interface)
            .map(|(&id, _)| id)
            .collect();
        ids.sort_unstable();
        ids.first().copied()
    }

    /// The requests `opcode` to objects of `interface`.
    pub(crate) fn requests(&self, interface: &str, opcode: u16) -> Vec<&Received> {
        self.requests
            .iter()
            .filter(|r| r.interface == interface && r.opcode == opcode)
            .collect()
    }

    /// A fresh serial.
    pub(crate) fn next_serial(&mut self) -> u32 {
        self.serial += 1;
        self.serial
    }

    /// The `xdg_surface` and toplevel of a window surface.
    pub(crate) fn toplevel(&self, surface: u32) -> (u32, u32) {
        self.toplevels[&surface]
    }
}

/// The fake compositor of one backend.
#[derive(Debug)]
pub(crate) struct Fake {
    writer: Arc<Mutex<UnixStream>>,
    state: Arc<Mutex<State>>,
}

/// The globals every fake offers.
const CORE_GLOBALS: [(&str, u32); 5] = [
    ("wl_compositor", 6),
    ("wl_subcompositor", 1),
    ("wl_shm", 1),
    ("xdg_wm_base", 6),
    ("wl_output", 4),
];

/// Requests destroying their object, answered with `delete_id`.
const DESTRUCTORS: [(&str, u16); 13] = [
    ("wl_surface", 0),
    ("wl_buffer", 0),
    ("wl_shm_pool", 1),
    ("wl_subsurface", 0),
    ("xdg_surface", 0),
    ("xdg_toplevel", 0),
    ("zxdg_toplevel_decoration_v1", 0),
    ("wp_viewport", 0),
    ("wp_fractional_scale_v1", 0),
    ("wl_data_source", 1),
    ("wp_cursor_shape_device_v1", 0),
    ("xdg_activation_token_v1", 4),
    ("wl_pointer", 1),
];

impl Fake {
    /// Starts a fake compositor and connects a backend to it.
    pub(crate) fn start(options: Options) -> (WaylandBackend, Self) {
        let (client, server) = UnixStream::pair().expect("a socket pair");
        let writer = Arc::new(Mutex::new(server.try_clone().expect("a second handle")));
        let state = Arc::new(Mutex::new(State::default()));
        let fake = Self {
            writer: Arc::clone(&writer),
            state: Arc::clone(&state),
        };
        thread::spawn(move || serve(&server, &writer, &state, &options));
        let conn = Connection::new(client).expect("the first request queues");
        let backend = WaylandBackend::with_connection(conn).expect("the backend connects");
        (backend, fake)
    }

    /// Sends an event to the client.
    pub(crate) fn send(&self, event: Request) {
        send(&self.writer, event);
    }

    /// Sends bytes that need not be a message.
    pub(crate) fn send_bytes(&self, bytes: &[u8]) {
        let writer = self.writer.lock().unwrap_or_else(PoisonError::into_inner);
        // A client that hung up gets nothing more.
        let _ = sys::send(&writer, bytes, &[]);
    }

    /// Closes the connection, as a compositor that quits.
    pub(crate) fn hang_up(&self) {
        let writer = self.writer.lock().unwrap_or_else(PoisonError::into_inner);
        // A connection already closed needs no shutdown.
        let _ = writer.shutdown(std::net::Shutdown::Both);
    }

    /// What the fake knows.
    pub(crate) fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Puts an offer of `mimes` from another client on the clipboard;
    /// returns its id.
    pub(crate) fn offer_clipboard(&self, mimes: &[&str]) -> u32 {
        let (device, offer) = {
            let mut state = self.state();
            let device = state
                .id_of("wl_data_device")
                .expect("the data device exists");
            let offer = 0xff00_0000 + state.next_server_id;
            state.next_server_id += 1;
            state.objects.insert(offer, String::from("wl_data_offer"));
            (device, offer)
        };
        self.send(Request::new(device, 0).uint(offer));
        for mime in mimes {
            self.send(Request::new(offer, 0).string(mime));
        }
        self.send(Request::new(device, 5).uint(offer));
        offer
    }

    /// Waits until `ready` holds for the fake's state.
    pub(crate) fn wait_for(&self, what: &str, ready: impl Fn(&State) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !ready(&self.state()) {
            assert!(Instant::now() < deadline, "the fake never saw {what}");
            thread::sleep(Duration::from_millis(1));
        }
    }
}

fn send(writer: &Mutex<UnixStream>, event: Request) {
    let (bytes, fds) = event.finish();
    let fds: Vec<_> = fds.iter().map(std::os::fd::AsFd::as_fd).collect();
    let writer = writer.lock().unwrap_or_else(PoisonError::into_inner);
    // A client that hung up gets nothing more.
    let _ = sys::send(&writer, &bytes, &fds);
}

/// Answers requests until the client hangs up.
fn serve(server: &UnixStream, writer: &Mutex<UnixStream>, state: &Mutex<State>, options: &Options) {
    let mut input = Vec::new();
    let mut fds = VecDeque::new();
    let mut chunk = [0; 4096];
    loop {
        let mut received = Vec::new();
        match sys::receive(server, &mut chunk, &mut received, true) {
            Ok(0) | Err(_) => return,
            Ok(n) => input.extend_from_slice(&chunk[..n]),
        }
        fds.extend(received);
        while let Ok(Some(header)) = Header::parse(&input) {
            if input.len() < header.len {
                break;
            }
            let body: Vec<u8> = input[HEADER_LEN..header.len].to_vec();
            input.drain(..header.len);
            let mut state = state.lock().unwrap_or_else(PoisonError::into_inner);
            let interface = state
                .objects
                .get(&header.object)
                .cloned()
                .unwrap_or_else(|| {
                    String::from(if header.object == 1 {
                        "wl_display"
                    } else {
                        "unknown"
                    })
                });
            let mut args = Args::new(&body, &mut fds);
            let replies = answer(&mut state, options, &interface, header, &mut args);
            state.requests.push(Received {
                interface,
                object: header.object,
                opcode: header.opcode,
                body,
            });
            drop(state);
            for reply in replies {
                send(writer, reply);
            }
        }
    }
}

/// Requests creating an object (their first argument), and its
/// interface.
const CREATORS: [(&str, u16, &str); 17] = [
    ("wl_display", 1, "wl_registry"),
    ("wl_compositor", 0, "wl_surface"),
    ("wl_subcompositor", 1, "wl_subsurface"),
    ("wl_shm", 0, "wl_shm_pool"),
    ("wl_shm_pool", 0, "wl_buffer"),
    ("wl_surface", 3, "wl_callback"),
    ("xdg_wm_base", 2, "xdg_surface"),
    ("xdg_surface", 1, "xdg_toplevel"),
    (
        "zxdg_decoration_manager_v1",
        1,
        "zxdg_toplevel_decoration_v1",
    ),
    ("wp_viewporter", 1, "wp_viewport"),
    (
        "wp_fractional_scale_manager_v1",
        1,
        "wp_fractional_scale_v1",
    ),
    ("wl_seat", 0, "wl_pointer"),
    ("wl_seat", 1, "wl_keyboard"),
    ("wp_cursor_shape_manager_v1", 1, "wp_cursor_shape_device_v1"),
    ("wl_data_device_manager", 0, "wl_data_source"),
    ("wl_data_device_manager", 1, "wl_data_device"),
    ("xdg_activation_v1", 1, "xdg_activation_token_v1"),
];

/// The events answering one request.
fn answer(
    state: &mut State,
    options: &Options,
    interface: &str,
    header: Header,
    args: &mut Args<'_>,
) -> Vec<Request> {
    let created = CREATORS
        .iter()
        .find(|&&(known, opcode, _)| known == interface && opcode == header.opcode)
        .map(|&(_, _, kind)| {
            let id = args.uint().unwrap_or(0);
            state.objects.insert(id, kind.to_owned());
            id
        });
    let mut replies = reply(
        state,
        options,
        (interface, header),
        created.unwrap_or(0),
        args,
    );
    if DESTRUCTORS.contains(&(interface, header.opcode)) {
        state.objects.remove(&header.object);
        replies.push(Request::new(1, 1).uint(header.object));
    }
    replies
}

/// The events a request calls for, given the object it created.
fn reply(
    state: &mut State,
    options: &Options,
    (interface, header): (&str, Header),
    created: u32,
    args: &mut Args<'_>,
) -> Vec<Request> {
    let object = header.object;
    match (interface, header.opcode) {
        ("wl_display", 0) => {
            let callback = args.uint().unwrap_or(0);
            vec![
                Request::new(callback, 0).uint(0),
                Request::new(1, 1).uint(callback),
            ]
        }
        ("wl_display", 1) => CORE_GLOBALS
            .iter()
            .chain(&options.globals)
            .zip(1..)
            .map(|((name, version), id)| {
                Request::new(created, 0)
                    .uint(id)
                    .string(name)
                    .uint(*version)
            })
            .collect(),
        ("wl_registry", 0) => bind(state, options, args),
        ("wl_shm", 0) => {
            // The pool's memory is not looked at; its descriptor goes.
            let _pool = args.fd();
            Vec::new()
        }
        ("wl_surface", 1) => {
            let buffer = args.uint().unwrap_or(0);
            match state.attached.insert(object, buffer) {
                Some(old) if old != 0 && old != buffer => vec![Request::new(old, 0)],
                _ => Vec::new(),
            }
        }
        ("wl_surface", 3) => {
            state.frames.entry(object).or_default().push(created);
            Vec::new()
        }
        ("wl_surface", 6) => commit(state, object),
        ("xdg_wm_base", 2) => {
            let surface = args.uint().unwrap_or(0);
            state.toplevels.insert(surface, (created, 0));
            Vec::new()
        }
        ("xdg_surface", 1) => {
            for entry in state
                .toplevels
                .values_mut()
                .filter(|entry| entry.0 == object)
            {
                entry.1 = created;
            }
            Vec::new()
        }
        ("zxdg_toplevel_decoration_v1", 1) => {
            vec![Request::new(object, 0).uint(options.decoration_mode)]
        }
        ("wp_fractional_scale_manager_v1", 1) => {
            vec![Request::new(created, 0).uint(options.fractional)]
        }
        ("wl_seat", 1) => keymap(created),
        ("wl_data_offer", 1) => {
            let mime = args.string().unwrap_or_default();
            if let Ok(fd) = args.fd() {
                let text = format!("pasted as {mime}");
                // The client may have hung up its end.
                let _ = std::io::Write::write_all(&mut std::fs::File::from(fd), text.as_bytes());
            }
            Vec::new()
        }
        ("xdg_activation_token_v1", 3) => vec![Request::new(object, 0).string("token-1")],
        _ => Vec::new(),
    }
}

/// Binds a global: seats announce a pointer and a keyboard, outputs
/// their scale.
fn bind(state: &mut State, options: &Options, args: &mut Args<'_>) -> Vec<Request> {
    let _name = args.uint();
    let kind = args.string().unwrap_or_default();
    let _version = args.uint();
    let id = args.uint().unwrap_or(0);
    state.objects.insert(id, kind.clone());
    match kind.as_str() {
        "wl_seat" => vec![Request::new(id, 0).uint(3)],
        "wl_output" => vec![
            Request::new(id, 3).int(options.output_scale),
            Request::new(id, 2),
        ],
        _ => Vec::new(),
    }
}

/// Shows a surface's new state: its frame callbacks are done, and a new
/// toplevel gets its first configure.
fn commit(state: &mut State, surface: u32) -> Vec<Request> {
    let mut replies = Vec::new();
    for callback in state.frames.remove(&surface).unwrap_or_default() {
        replies.push(Request::new(callback, 0).uint(0));
        replies.push(Request::new(1, 1).uint(callback));
    }
    if let Some(&(xdg_surface, toplevel)) = state.toplevels.get(&surface)
        && toplevel != 0
        && !state.configured.contains(&surface)
    {
        state.configured.push(surface);
        let serial = state.next_serial();
        replies.push(Request::new(toplevel, 0).int(0).int(0).array(&[]));
        replies.push(Request::new(xdg_surface, 0).uint(serial));
    }
    replies
}

/// The keymap and repeat rate of a new keyboard.
fn keymap(keyboard: u32) -> Vec<Request> {
    let file = sys::memfd(c"fake-keymap").expect("a memfd");
    file.write_all_at(DE_RU_KEYMAP.as_bytes(), 0)
        .expect("the keymap is written");
    let size = DE_RU_KEYMAP.len() as u32;
    vec![
        Request::new(keyboard, 0)
            .uint(1)
            .fd(OwnedFd::from(file))
            .uint(size),
        Request::new(keyboard, 5).int(25).int(400),
    ]
}
