//! Tests against a real X server, run when `$DISPLAY` is set (and passing
//! trivially otherwise), e.g. under a private Xvfb:
//!
//! ```text
//! Xvfb :117 -screen 0 1280x800x24 -nolisten tcp &
//! DISPLAY=:117 cargo test -p plt-platform
//! ```
//!
//! A second connection (the [`Driver`]) plays the user and the window
//! manager. Tests that move the real pointer or change root-window
//! properties only run on a bare server without a window manager, so they
//! never disturb a desktop session.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use super::atoms::predefined;
use super::connection::Connection;
use super::display::DisplayName;
use super::keymap::Keymap;
use super::request::{self, WindowAttribute, event_mask, testing};
use super::server_event::ServerEvent;
use super::transport::Wait;
use super::{X11Backend, reply};
use crate::backend::{Backend, BackendKind};
use crate::error::{Error, X11Error};
use crate::event::{ButtonState, Event, MouseButton, ScrollDelta, WindowEvent};
use crate::event_loop::EventLoop;
use crate::frame::OwnedFrame;
use crate::geometry::{LogicalSize, PhysicalPosition, PhysicalRect, PhysicalSize};
use crate::keyboard::{Key, Modifiers};
use crate::window::{CursorIcon, Icon, WindowId, WindowOptions};

/// The tests share the server's pointer, focus and root properties, so
/// they run one at a time.
static SERVER: Mutex<()> = Mutex::new(());

/// A connection kept open for the whole test run: X servers reset when
/// their last client disconnects, dropping connection attempts meanwhile.
static ANCHOR: Mutex<Option<Connection>> = Mutex::new(None);

/// XTEST event codes for `FakeInput`.
const FAKE_KEY_PRESS: u8 = 2;
const FAKE_KEY_RELEASE: u8 = 3;
const FAKE_BUTTON_PRESS: u8 = 4;
const FAKE_BUTTON_RELEASE: u8 = 5;
const FAKE_MOTION: u8 = 6;

/// Keysyms the input test types.
const XK_A: u32 = 0x61;
const XK_ESCAPE: u32 = 0xff1b;
const XK_SHIFT_L: u32 = 0xffe1;

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

#[test]
fn window_properties_follow_the_options() {
    let Some(live) = live() else { return };
    let mut driver = Driver::connect(&live.name);
    let mut event_loop = x11_loop();
    let icon = Icon::from_rgba(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 128]).unwrap();
    let options = WindowOptions::new("Figure 1 – α")
        .with_app_id("plt-live")
        .with_inner_size(PhysicalSize::new(320, 200))
        .with_min_inner_size(PhysicalSize::new(100, 80))
        .with_icon(icon);
    let window = open_window(&mut event_loop, &options);
    let id = xid(window);
    let net_wm_name = driver.atom("_NET_WM_NAME");
    assert_eq!(driver.property(id, net_wm_name), "Figure 1 – α".as_bytes());
    assert_eq!(driver.property(id, predefined::WM_NAME), b"Figure 1 ? ?");
    assert_eq!(
        driver.property(id, predefined::WM_CLASS),
        b"plt-live\0plt-live\0"
    );
    let protocols = driver.atom("WM_PROTOCOLS");
    let expected = [driver.atom("WM_DELETE_WINDOW"), driver.atom("_NET_WM_PING")];
    assert_eq!(driver.property_u32(id, protocols), expected);
    let hints = driver.property_u32(id, predefined::WM_NORMAL_HINTS);
    assert_eq!((hints[5], hints[6]), (100, 80));
    let net_wm_icon = driver.atom("_NET_WM_ICON");
    assert_eq!(
        driver.property_u32(id, net_wm_icon),
        [2, 1, 0xffff_0000, 0x8000_00ff]
    );
    let net_wm_pid = driver.atom("_NET_WM_PID");
    assert_eq!(driver.property_u32(id, net_wm_pid), [std::process::id()]);

    let mut cx = event_loop.context();
    cx.set_title(window, "Renamed").unwrap();
    for cursor in [
        CursorIcon::Crosshair,
        CursorIcon::Pointer,
        CursorIcon::Text,
        CursorIcon::Default,
    ] {
        cx.set_cursor(window, cursor).unwrap();
    }
    assert_eq!(driver.property(id, net_wm_name), b"Renamed");
    cx.close_window(window).unwrap();
    assert_eq!(
        cx.set_title(window, "gone"),
        Err(Error::UnknownWindow(window))
    );
    let events = pump_until(&mut event_loop, "destruction", |e| {
        is(e, window, &WindowEvent::Destroyed)
    });
    assert_eq!(of(&events, window).last(), Some(&WindowEvent::Destroyed));
}

#[test]
fn real_input_reaches_the_focused_window() {
    let Some(live) = live() else { return };
    let mut driver = Driver::connect(&live.name);
    if !driver.is_bare_server() {
        return;
    }
    let keymap = Keymap::query(&mut driver.conn).unwrap();
    driver.fake(FAKE_MOTION, 0, 1000, 700);
    let mut event_loop = x11_loop();
    let options = WindowOptions::new("input").with_inner_size(PhysicalSize::new(300, 200));
    let window = open_window(&mut event_loop, &options);
    driver.send(&testing::set_input_focus(xid(window)));
    driver.fake(FAKE_MOTION, 0, 50, 60);
    let at = PhysicalPosition::new(50.0, 60.0);
    let events = pump_until(
        &mut event_loop,
        "pointer motion",
        |e| matches!(e, Event::Window { event: WindowEvent::CursorMoved { position, .. }, .. } if *position == at),
    );
    let events = of(&events, window);
    assert!(events.contains(&WindowEvent::Focused(true)), "{events:?}");
    assert!(
        events.contains(&WindowEvent::CursorEntered { position: at }),
        "{events:?}"
    );

    driver.fake(FAKE_BUTTON_PRESS, 1, 0, 0);
    driver.fake(FAKE_BUTTON_RELEASE, 1, 0, 0);
    driver.fake(FAKE_BUTTON_PRESS, 4, 0, 0);
    driver.fake(FAKE_BUTTON_RELEASE, 4, 0, 0);
    let events = pump_until(&mut event_loop, "a scroll", |e| {
        matches!(
            e,
            Event::Window {
                event: WindowEvent::MouseWheel { .. },
                ..
            }
        )
    });
    let events = of(&events, window);
    let press = WindowEvent::MouseInput {
        button: MouseButton::Left,
        state: ButtonState::Pressed,
        position: at,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    assert!(events.contains(&press), "{events:?}");
    let scroll = WindowEvent::MouseWheel {
        delta: ScrollDelta::Lines { x: 0.0, y: 1.0 },
        position: at,
        modifiers: Modifiers::NONE,
    };
    assert!(events.contains(&scroll), "{events:?}");

    driver.key(&keymap, XK_A, FAKE_KEY_PRESS);
    driver.key(&keymap, XK_A, FAKE_KEY_RELEASE);
    driver.key(&keymap, XK_SHIFT_L, FAKE_KEY_PRESS);
    driver.key(&keymap, XK_A, FAKE_KEY_PRESS);
    driver.key(&keymap, XK_A, FAKE_KEY_RELEASE);
    driver.key(&keymap, XK_SHIFT_L, FAKE_KEY_RELEASE);
    driver.key(&keymap, XK_ESCAPE, FAKE_KEY_PRESS);
    driver.key(&keymap, XK_ESCAPE, FAKE_KEY_RELEASE);
    let events = pump_until(
        &mut event_loop,
        "Escape",
        |e| matches!(e, Event::Window { event: WindowEvent::Key(key), .. } if key.key == Key::Escape),
    );
    let presses: Vec<(Key, Option<String>, Modifiers)> = of(&events, window)
        .into_iter()
        .filter_map(|event| match event {
            WindowEvent::Key(key) if key.state.is_pressed() => {
                Some((key.key, key.text, key.modifiers))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        presses,
        [
            (Key::Character('a'), Some("a".into()), Modifiers::NONE),
            (Key::Shift, None, Modifiers::NONE),
            (Key::Character('A'), Some("A".into()), Modifiers::SHIFT),
            (Key::Escape, None, Modifiers::NONE),
        ]
    );

    driver.fake(FAKE_MOTION, 0, 1000, 700);
    pump_until(&mut event_loop, "the pointer leaving", |e| {
        is(e, window, &WindowEvent::CursorLeft)
    });
}

#[test]
fn window_manager_messages_are_handled() {
    let Some(live) = live() else { return };
    let mut driver = Driver::connect(&live.name);
    let mut event_loop = x11_loop();
    let window = open_window(&mut event_loop, &WindowOptions::new("wm"));
    let (protocols, delete, ping) = (
        driver.atom("WM_PROTOCOLS"),
        driver.atom("WM_DELETE_WINDOW"),
        driver.atom("_NET_WM_PING"),
    );
    driver.client_message(xid(window), protocols, [delete, 0, 0, 0, 0]);
    pump_until(&mut event_loop, "a close request", |e| {
        is(e, window, &WindowEvent::CloseRequested)
    });

    let root = driver.root;
    let listen = WindowAttribute::EventMask(event_mask::SUBSTRUCTURE_NOTIFY);
    driver.send(&request::change_window_attributes(root, &[listen]));
    driver.client_message(xid(window), protocols, [ping, 4321, xid(window), 0, 0]);
    let deadline = Instant::now() + Duration::from_secs(5);
    let pong = loop {
        assert!(Instant::now() < deadline, "no answer to the ping");
        event_loop
            .pump_events(Some(Duration::ZERO), |_, _| {})
            .unwrap();
        let events = driver
            .conn
            .wait_for_events(Wait::Until(Instant::now() + Duration::from_millis(20)));
        let pong = events.unwrap().iter().map(ServerEvent::parse).find(|event| {
            matches!(event, ServerEvent::ClientMessage { window, data, .. } if *window == root && data[0] == ping)
        });
        if let Some(pong) = pong {
            break pong;
        }
    };
    assert_eq!(
        pong,
        ServerEvent::ClientMessage {
            window: root,
            message_type: protocols,
            data: [ping, 4321, xid(window), 0, 0],
        }
    );
}

/// Asks for the clipboard as `target` through `driver`'s `requestor`
/// window and returns the answer, pumping `event_loop` so that the owner
/// answers; `None` when the owner refuses.
fn paste(
    driver: &mut Driver,
    event_loop: &mut EventLoop,
    requestor: u32,
    target: &str,
) -> Option<Vec<u8>> {
    let (clipboard, target, property) = (
        driver.atom("CLIPBOARD"),
        driver.atom(target),
        driver.atom("PLT_PASTE"),
    );
    driver.send(&testing::convert_selection(
        requestor, clipboard, target, property,
    ));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(Instant::now() < deadline, "no answer to the paste");
        event_loop
            .pump_events(Some(Duration::ZERO), |_, _| {})
            .unwrap();
        let events = driver
            .conn
            .wait_for_events(Wait::Until(Instant::now() + Duration::from_millis(20)))
            .unwrap();
        // `SelectionNotify` (31), sent with `SendEvent`.
        if let Some(notify) = events.iter().find(|raw| raw[0] & 0x7f == 31) {
            let answered = u32::from_le_bytes([notify[20], notify[21], notify[22], notify[23]]);
            return (answered != 0).then(|| driver.property(requestor, property));
        }
    }
}

#[test]
fn the_clipboard_hands_out_its_image() {
    let Some(live) = live() else { return };
    let mut driver = Driver::connect(&live.name);
    let requestor = driver.conn.generate_id().unwrap();
    let root = driver.root;
    let window = request::NewWindow {
        id: requestor,
        parent: root,
        class: request::WindowClass::InputOnly,
        depth: 0,
        visual: 0,
        width: 1,
        height: 1,
    };
    driver.send(&request::create_window(&window, &[]));
    let mut event_loop = x11_loop();
    let owner = open_window(&mut event_loop, &WindowOptions::new("clipboard"));
    let png = b"\x89PNG\r\n\x1a\nnot really a PNG".to_vec();
    event_loop
        .context()
        .set_clipboard_image(owner, png.clone())
        .unwrap();

    let targets = paste(&mut driver, &mut event_loop, requestor, "TARGETS").expect("targets");
    let words: Vec<u32> = targets
        .as_chunks::<4>()
        .0
        .iter()
        .map(|w| u32::from_le_bytes(*w))
        .collect();
    assert_eq!(words, [driver.atom("TARGETS"), driver.atom("image/png")]);
    assert_eq!(
        paste(&mut driver, &mut event_loop, requestor, "image/png"),
        Some(png)
    );
    assert_eq!(
        paste(&mut driver, &mut event_loop, requestor, "UTF8_STRING"),
        None
    );
}

#[test]
fn scale_factor_follows_xft_dpi() {
    let Some(live) = live() else { return };
    let mut driver = Driver::connect(&live.name);
    if !driver.is_bare_server() {
        return;
    }
    let _reset = ResetResources(&live.name);
    driver.set_resources("Xft.antialias:\t1\nXft.dpi:\t192\n");
    let mut event_loop = x11_loop();
    let options = WindowOptions::new("hidpi").with_inner_size(LogicalSize::new(200.0, 100.0));
    let window = open_window(&mut event_loop, &options);
    assert_eq!(event_loop.context().scale_factor(window), Ok(2.0));
    assert_eq!(
        event_loop.context().inner_size(window),
        Ok(PhysicalSize::new(400, 200))
    );
    driver.set_resources("Xft.dpi:\t144\n");
    pump_until(&mut event_loop, "the new scale", |e| {
        is(e, window, &WindowEvent::ScaleFactorChanged(1.5))
    });
    driver.send(&testing::delete_property(
        driver.root,
        predefined::RESOURCE_MANAGER,
    ));
    pump_until(&mut event_loop, "the default scale", |e| {
        is(e, window, &WindowEvent::ScaleFactorChanged(1.0))
    });
}

#[test]
fn large_frames_are_split_into_requests_the_server_accepts() {
    let Some(live) = live() else { return };
    let mut backend = X11Backend::open(&live.name).unwrap();
    backend.conn.limit_request_bytes(16 * 1024);
    let size = PhysicalSize::new(203, 157);
    let window = backend
        .create_window(&WindowOptions::new("tiles").with_inner_size(size))
        .unwrap();
    let pixels: Vec<u8> = (0..size.area() as u32)
        .flat_map(|i| [(i % 251) as u8, (i % 241) as u8, (i % 239) as u8, 255])
        .collect();
    let frame = OwnedFrame::new(size.width, size.height, pixels).unwrap();
    let mut events = Vec::new();
    while !events.contains(&Event::Window {
        window,
        event: WindowEvent::RedrawRequested,
    }) {
        backend
            .poll(Some(Instant::now() + Duration::from_secs(5)), &mut events)
            .unwrap();
    }
    backend
        .present(window, &frame.as_frame(), PhysicalRect::from_size(size))
        .unwrap();
    assert_eq!(backend.read_pixels(window).unwrap(), frame);
}

#[test]
fn x_errors_become_typed_errors() {
    let Some(live) = live() else { return };
    let mut conn = Connection::connect(&live.name).unwrap();
    let unused = conn.generate_id().unwrap();
    let checked = conn.send_checked(&request::destroy_window(unused));
    assert!(
        matches!(checked, Err(Error::X11(X11Error { code: 3, major_opcode: 4, bad_value, .. })) if bad_value == unused),
        "{checked:?}"
    );

    let mut backend = X11Backend::open(&live.name).unwrap();
    let unused = backend.conn.generate_id().unwrap();
    backend.conn.send(&request::map_window(unused)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let polled = backend.poll(Some(deadline), &mut Vec::new());
    assert!(
        matches!(polled, Err(Error::X11(X11Error { code: 3, major_opcode: 8, bad_value, .. })) if bad_value == unused),
        "{polled:?}"
    );
}

#[test]
fn invalid_arguments_and_unknown_windows_are_rejected() {
    let Some(live) = live() else { return };
    let mut backend = X11Backend::open(&live.name).unwrap();
    let too_big = WindowOptions::new("huge").with_inner_size(PhysicalSize::new(70_000, 10));
    assert!(matches!(
        backend.create_window(&too_big),
        Err(Error::InvalidArgument(_))
    ));
    let window = backend.create_window(&WindowOptions::new("ok")).unwrap();
    assert_eq!(backend.window_ids(), [window]);
    backend.destroy_window(window).unwrap();
    assert!(!backend.has_window(window));
    assert_eq!(
        backend.destroy_window(window),
        Err(Error::UnknownWindow(window))
    );
}
