//! Live test of real (XTEST-faked) pointer, button, wheel and keyboard
//! input reaching the focused window, dead keys included.

use super::super::connection::Connection;
use super::super::keymap::Keymap;
use super::super::request::testing;
use super::{Driver, is, live, of, open_window, pump_until, x11_loop, xid};
use crate::compose::Composer;
use crate::event::{ButtonState, Event, MouseButton, ScrollDelta, WindowEvent};
use crate::geometry::{PhysicalPosition, PhysicalSize};
use crate::keyboard::{Key, Modifiers};
use crate::window::WindowOptions;

/// XTEST event codes for `FakeInput`.
const FAKE_KEY_PRESS: u8 = 2;
const FAKE_KEY_RELEASE: u8 = 3;
const FAKE_BUTTON_PRESS: u8 = 4;
const FAKE_BUTTON_RELEASE: u8 = 5;
const FAKE_MOTION: u8 = 6;

/// Keysyms the input tests type.
const XK_A: u32 = 0x61;
const XK_E: u32 = 0x65;
const XK_ESCAPE: u32 = 0xff1b;
const XK_SHIFT_L: u32 = 0xffe1;
const XK_DEAD_ACUTE: u32 = 0xfe51;

/// A keycode given other keysyms for a test, given its own back when
/// dropped (also when the test fails).
struct Remapped {
    conn: Connection,
    keycode: u8,
    original: Vec<u32>,
}

impl Remapped {
    /// Maps `keycode` of `keymap` to `keysym` (at every level).
    fn new(conn: Connection, keymap: &Keymap, keycode: u8, keysym: u32) -> Self {
        let original = keymap.keysyms_of(keycode).to_vec();
        let mut remapped = Self {
            conn,
            keycode,
            original,
        };
        let keysyms = vec![keysym; remapped.original.len()];
        remapped.change(&keysyms);
        remapped
    }

    fn change(&mut self, keysyms: &[u32]) {
        self.conn
            .send_checked(&testing::change_keyboard_mapping(self.keycode, keysyms))
            .expect("the server changes the mapping");
    }
}

impl Drop for Remapped {
    fn drop(&mut self) {
        let original = std::mem::take(&mut self.original);
        self.change(&original);
    }
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
    pump_until(
        &mut event_loop,
        "the pointer leaving",
        |e| matches!(e, Event::Window { window: w, event: WindowEvent::CursorLeft { position: Some(_) } } if *w == window),
    );
}

#[test]
fn dead_keys_compose_with_the_next_key() {
    let Some(live) = live() else { return };
    let mut driver = Driver::connect(&live.name);
    if !driver.is_bare_server() {
        return;
    }
    let keymap = Keymap::query(&mut driver.conn).unwrap();
    let free = keymap
        .keycode_of(crate::keysym::NO_SYMBOL)
        .expect("a keycode without keysyms");
    let conn = Connection::connect(&live.name).expect("the mapping's connection");
    let _remapped = Remapped::new(conn, &keymap, free, XK_DEAD_ACUTE);
    let mut event_loop = x11_loop();
    let options = WindowOptions::new("dead keys").with_inner_size(PhysicalSize::new(300, 200));
    let window = open_window(&mut event_loop, &options);
    driver.send(&testing::set_input_focus(xid(window)));
    pump_until(&mut event_loop, "the focus", |e| {
        is(e, window, &WindowEvent::Focused(true))
    });
    driver.fake(FAKE_KEY_PRESS, free, 0, 0);
    driver.fake(FAKE_KEY_RELEASE, free, 0, 0);
    driver.key(&keymap, XK_E, FAKE_KEY_PRESS);
    driver.key(&keymap, XK_E, FAKE_KEY_RELEASE);
    driver.key(&keymap, XK_ESCAPE, FAKE_KEY_PRESS);
    driver.key(&keymap, XK_ESCAPE, FAKE_KEY_RELEASE);
    let events = pump_until(
        &mut event_loop,
        "Escape",
        |e| matches!(e, Event::Window { event: WindowEvent::Key(key), .. } if key.key == Key::Escape),
    );
    let presses: Vec<(Key, Option<String>)> = of(&events, window)
        .into_iter()
        .filter_map(|event| match event {
            WindowEvent::Key(key) if key.state.is_pressed() => Some((key.key, key.text)),
            _ => None,
        })
        .collect();
    // What this system's Compose file makes of ´ then e: é wherever X's
    // locale data is installed.
    let mut composer = Composer::default();
    let press = |composer: &mut Composer, keysym| {
        composer
            .key_event(keysym, ButtonState::Pressed, Modifiers::NONE, false, 0)
            .text
    };
    press(&mut composer, XK_DEAD_ACUTE);
    let composed = press(&mut composer, XK_E);
    assert_eq!(
        presses,
        [
            (Key::Dead('´'), None),
            (Key::Character('e'), composed),
            (Key::Escape, None),
        ]
    );
}
