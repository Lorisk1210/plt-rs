//! Tests against a real X server, run when `$DISPLAY` is set (and passing
//! trivially otherwise), e.g. under a private Xvfb:
//!
//! ```text
//! Xvfb :117 -screen 0 1280x800x24 -nolisten tcp &
//! DISPLAY=:117 cargo test -p pltrs-platform
//! ```
//!
//! A second connection (the [`Driver`]) plays the user and the window
//! manager. Tests that move the real pointer or change root-window
//! properties only run on a bare server without a window manager, so they
//! never disturb a desktop session.
//!
//! This module holds the harness; the tests are grouped into [`windows`]
//! (properties, window-manager messages, scale), [`input`], [`clipboard`]
//! and [`backend`] (frames and errors through the backend directly).

mod backend;
mod clipboard;
mod input;
mod windows;

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use super::atoms::predefined;
use super::connection::Connection;
use super::display::DisplayName;
use super::keymap::Keymap;
use super::reply;
use super::request::{self, testing};
use crate::backend::BackendKind;
use crate::error::Error;
use crate::event::{Event, WindowEvent};
use crate::event_loop::EventLoop;
use crate::window::{WindowId, WindowOptions};

/// The tests share the server's pointer, focus and root properties, so
/// they run one at a time.
static SERVER: Mutex<()> = Mutex::new(());

/// A connection kept open for the whole test run: X servers reset when
/// their last client disconnects, dropping connection attempts meanwhile.
static ANCHOR: Mutex<Option<Connection>> = Mutex::new(None);

/// Exclusive access to the server named by `$DISPLAY`, if set.
struct Live {
    name: DisplayName,
    _serial: MutexGuard<'static, ()>,
}

fn live() -> Option<Live> {
    let name = std::env::var("DISPLAY")
        .ok()
        .filter(|name| !name.is_empty())?;
    let name = DisplayName::parse(&name).expect("$DISPLAY is a valid display name");
    let serial = SERVER.lock().unwrap_or_else(PoisonError::into_inner);
    let mut anchor = ANCHOR.lock().unwrap_or_else(PoisonError::into_inner);
    if anchor.is_none() {
        *anchor = Some(connect_patiently(&name));
    }
    Some(Live {
        name,
        _serial: serial,
    })
}

/// Connects, waiting out a server reset left behind by an earlier test
/// process.
fn connect_patiently(name: &DisplayName) -> Connection {
    let mut attempts = 0;
    loop {
        match Connection::connect(name) {
            Ok(conn) => return conn,
            Err(Error::ConnectionClosed | Error::Io { .. }) if attempts < 100 => {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => panic!("cannot connect to the X server: {error}"),
        }
    }
}

/// A second client that inspects windows and fakes user input.
struct Driver {
    conn: Connection,
    root: u32,
    /// Major opcode of the XTEST extension, if present.
    xtest: Option<u8>,
}

impl Driver {
    fn connect(name: &DisplayName) -> Self {
        let mut conn = Connection::connect(name).expect("driver connects");
        let root = conn.setup().screens[usize::from(name.screen)].root;
        let xtest = conn.extension("XTEST").unwrap();
        Self { conn, root, xtest }
    }

    /// Whether no window manager runs (so input can be faked and root
    /// properties changed without disturbing anyone).
    fn is_bare_server(&mut self) -> bool {
        let check = self.atom("_NET_SUPPORTING_WM_CHECK");
        self.property(self.root, check).is_empty()
    }

    fn atom(&mut self, name: &str) -> u32 {
        let reply = self.conn.round_trip(&request::intern_atom(name)).unwrap();
        reply::intern_atom(&reply).unwrap()
    }

    fn property(&mut self, window: u32, property: u32) -> Vec<u8> {
        let reply = self
            .conn
            .round_trip(&request::get_property(window, property, 1 << 16));
        reply::property(&reply.unwrap()).unwrap().value.to_vec()
    }

    fn property_u32(&mut self, window: u32, property: u32) -> Vec<u32> {
        let bytes = self.property(window, property);
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|w| u32::from_le_bytes(*w))
            .collect()
    }

    fn send(&mut self, request: &request::Request) {
        self.conn
            .send_checked(request)
            .expect("driver request succeeds");
    }

    fn fake(&mut self, kind: u8, detail: u8, x: i16, y: i16) {
        let xtest = self.xtest.expect("the server has XTEST");
        self.send(&testing::fake_input(xtest, kind, detail, self.root, x, y));
    }

    fn key(&mut self, keymap: &Keymap, keysym: u32, kind: u8) {
        let keycode = keymap.keycode_of(keysym).expect("the keysym is mapped");
        self.fake(kind, keycode, 0, 0);
    }

    fn client_message(&mut self, window: u32, message_type: u32, data: [u32; 5]) {
        let event = request::client_message(window, message_type, data);
        self.send(&request::send_event(window, 0, &event));
    }

    fn set_resources(&mut self, database: &str) {
        let root = self.root;
        let property = predefined::RESOURCE_MANAGER;
        let request =
            request::set_property_bytes(root, property, predefined::STRING, database.as_bytes());
        self.send(&request);
    }
}

/// Removes the root window's resource database when dropped, even if the
/// test fails.
struct ResetResources<'a>(&'a DisplayName);

impl Drop for ResetResources<'_> {
    fn drop(&mut self) {
        let mut driver = Driver::connect(self.0);
        let root = driver.root;
        driver.send(&testing::delete_property(
            root,
            predefined::RESOURCE_MANAGER,
        ));
    }
}

fn x11_loop() -> EventLoop {
    EventLoop::with_backend(BackendKind::X11).expect("connects to $DISPLAY")
}

fn xid(window: WindowId) -> u32 {
    u32::try_from(window.to_raw()).unwrap()
}

/// Pumps the loop until `done` accepts an event; returns every event seen.
fn pump_until(
    event_loop: &mut EventLoop,
    what: &str,
    mut done: impl FnMut(&Event) -> bool,
) -> Vec<Event> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    while Instant::now() < deadline {
        let mut found = false;
        event_loop
            .pump_events(Some(Duration::from_millis(20)), |_, event| {
                found |= done(&event);
                seen.push(event);
            })
            .expect("pumping events succeeds");
        if found {
            return seen;
        }
    }
    panic!("timed out waiting for {what}; saw {seen:?}");
}

/// Whether `event` is `expected` for `window`.
fn is(event: &Event, window: WindowId, expected: &WindowEvent) -> bool {
    matches!(event, Event::Window { window: w, event } if *w == window && event == expected)
}

/// The events of `window` among `events`.
fn of(events: &[Event], window: WindowId) -> Vec<WindowEvent> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Window { window: w, event } if *w == window => Some(event.clone()),
            _ => None,
        })
        .collect()
}

fn open_window(event_loop: &mut EventLoop, options: &WindowOptions) -> WindowId {
    let window = event_loop
        .context()
        .create_window(options)
        .expect("window is created");
    pump_until(event_loop, "the first redraw", |e| {
        is(e, window, &WindowEvent::RedrawRequested)
    });
    window
}
