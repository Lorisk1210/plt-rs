//! Live test of real (XTEST-faked) pointer, button, wheel and keyboard
//! input reaching the focused window.

use super::super::keymap::Keymap;
use super::super::request::testing;
use super::{Driver, is, live, of, open_window, pump_until, x11_loop, xid};
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

/// Keysyms the input test types.
const XK_A: u32 = 0x61;
const XK_ESCAPE: u32 = 0xff1b;
const XK_SHIFT_L: u32 = 0xffe1;

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
