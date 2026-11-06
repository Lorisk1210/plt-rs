//! Input against the scripted compositor: the keyboard with its keymap
//! and repeat, the pointer over content, and the pointer acting on the
//! client-side decorations.

use std::time::{Duration, Instant};

use super::super::fake::{Fake, Options};
use super::{expect, of, open, poll_until, surface, wait_requests};
use crate::backend::Backend;
use crate::decoration::{ButtonLayout, TitleBarAction, TitleBarSettings};
use crate::event::{ButtonState, MouseButton, ScrollDelta, WindowEvent};
use crate::geometry::PhysicalPosition;
use crate::keyboard::{Key, Modifiers};
use crate::wayland::decoration::layout::{Edges, TITLE_BAR_HEIGHT};
use crate::wayland::wire::Request;
use crate::window::{CursorIcon, WindowId};

/// `KEY_A`.
const KEY_A: u32 = 30;
/// `BTN_LEFT`.
const BTN_LEFT: u32 = 0x110;
/// `BTN_RIGHT`.
const BTN_RIGHT: u32 = 0x111;
/// `BTN_MIDDLE`.
const BTN_MIDDLE: u32 = 0x112;
/// `BTN_SIDE`.
const BTN_SIDE: u32 = 0x113;

/// A device's id, once the backend created it.
fn device(fake: &Fake, interface: &str) -> u32 {
    fake.wait_for(interface, |state| state.id_of(interface).is_some());
    fake.state().id_of(interface).expect("the device exists")
}

/// Whether `event` is a key event for `key`.
fn key_event(event: &WindowEvent, key: Key) -> bool {
    matches!(event, WindowEvent::Key(k) if k.key == key && k.state == ButtonState::Pressed)
}

#[test]
fn keys_follow_the_keymap_modifiers_and_focus() {
    let (mut backend, fake) = Fake::start(Options::default());
    let window = open(&mut backend);
    let keyboard = device(&fake, "wl_keyboard");
    let serial = fake.state().next_serial();
    fake.send(
        Request::new(keyboard, 1)
            .uint(serial)
            .uint(surface(window))
            .array(&[]),
    );
    expect(&mut backend, window, &WindowEvent::Focused(true));
    fake.send(
        Request::new(keyboard, 3)
            .uint(serial + 1)
            .uint(0)
            .uint(KEY_A)
            .uint(1),
    );
    let events = poll_until(&mut backend, "a", |events| {
        of(events, window)
            .iter()
            .any(|e| key_event(e, Key::Character('a')))
    });
    let Some(WindowEvent::Key(a)) = of(&events, window)
        .into_iter()
        .find(|e| matches!(e, WindowEvent::Key(_)))
    else {
        panic!("a key event");
    };
    assert_eq!(
        (a.text.as_deref(), a.scancode, a.repeat),
        (Some("a"), 38, false)
    );
    fake.send(
        Request::new(keyboard, 3)
            .uint(serial + 2)
            .uint(0)
            .uint(KEY_A)
            .uint(0),
    );
    fake.send(
        Request::new(keyboard, 4)
            .uint(serial + 3)
            .uint(1 | 4)
            .uint(0)
            .uint(0)
            .uint(0),
    );
    fake.send(
        Request::new(keyboard, 3)
            .uint(serial + 4)
            .uint(0)
            .uint(KEY_A)
            .uint(1),
    );
    let events = poll_until(&mut backend, "Ctrl+Shift+A", |events| {
        of(events, window)
            .iter()
            .any(|e| key_event(e, Key::Character('A')))
    });
    let shifted = of(&events, window)
        .into_iter()
        .find_map(|e| match e {
            WindowEvent::Key(k) if k.key == Key::Character('A') => Some(k),
            _ => None,
        })
        .unwrap();
    assert_eq!(shifted.modifiers, Modifiers::SHIFT | Modifiers::CONTROL);
    assert_eq!(shifted.text, None, "a shortcut");
    fake.send(
        Request::new(keyboard, 2)
            .uint(serial + 5)
            .uint(surface(window)),
    );
    expect(&mut backend, window, &WindowEvent::Focused(false));
}

#[test]
fn held_keys_repeat_until_released() {
    let (mut backend, fake) = Fake::start(Options::default());
    let window = open(&mut backend);
    let keyboard = device(&fake, "wl_keyboard");
    fake.send(
        Request::new(keyboard, 1)
            .uint(1)
            .uint(surface(window))
            .array(&[]),
    );
    fake.send(
        Request::new(keyboard, 3)
            .uint(2)
            .uint(0)
            .uint(KEY_A)
            .uint(1),
    );
    let started = Instant::now();
    let events = poll_until(&mut backend, "a repeat", |events| {
        of(events, window)
            .iter()
            .any(|e| matches!(e, WindowEvent::Key(k) if k.repeat))
    });
    assert!(
        started.elapsed() >= Duration::from_millis(350),
        "the delay is 400 ms"
    );
    let presses = of(&events, window)
        .iter()
        .filter(|e| key_event(e, Key::Character('a')))
        .count();
    assert!(presses >= 2);
    fake.send(
        Request::new(keyboard, 3)
            .uint(3)
            .uint(0)
            .uint(KEY_A)
            .uint(0),
    );
    poll_until(&mut backend, "the release", |events| {
        of(events, window)
            .iter()
            .any(|e| matches!(e, WindowEvent::Key(k) if k.state == ButtonState::Released))
    });
    assert_eq!(backend.next_repeat(), None, "no key held");
}

#[test]
fn the_pointer_reports_motion_buttons_and_scrolling() {
    let (mut backend, fake) = Fake::start(Options::default());
    let window = open(&mut backend);
    let pointer = device(&fake, "wl_pointer");
    let id = surface(window);
    fake.send(
        Request::new(pointer, 0)
            .uint(9)
            .uint(id)
            .fixed(10.5)
            .fixed(20.0),
    );
    let at = PhysicalPosition::new(10.5, 20.0);
    expect(
        &mut backend,
        window,
        &WindowEvent::CursorEntered { position: at },
    );
    wait_requests(&mut backend, &fake, "wl_pointer", 0, 1);
    for x in [30.0, 40.0] {
        fake.send(Request::new(pointer, 2).uint(0).fixed(x).fixed(5.0));
    }
    fake.send(Request::new(pointer, 5));
    let moved = WindowEvent::CursorMoved {
        position: PhysicalPosition::new(40.0, 5.0),
        modifiers: Modifiers::NONE,
    };
    let events = expect(&mut backend, window, &moved);
    let motions = events
        .iter()
        .filter(|e| matches!(e, WindowEvent::CursorMoved { .. }))
        .count();
    assert_eq!(motions, 1, "consecutive motions are coalesced");
    let click = |time: u32, button: u32, state: u32| {
        Request::new(pointer, 3)
            .uint(time)
            .uint(time)
            .uint(button)
            .uint(state)
    };
    fake.send(click(100, BTN_LEFT, 1));
    fake.send(click(110, BTN_LEFT, 0));
    fake.send(click(200, BTN_LEFT, 1));
    let double = WindowEvent::MouseInput {
        button: MouseButton::Left,
        state: ButtonState::Pressed,
        position: PhysicalPosition::new(40.0, 5.0),
        modifiers: Modifiers::NONE,
        click_count: 2,
    };
    expect(&mut backend, window, &double);
    fake.send(click(300, BTN_SIDE, 1));
    poll_until(&mut backend, "the back button", |events| {
        of(events, window).iter().any(|e| {
            matches!(
                e,
                WindowEvent::MouseInput {
                    button: MouseButton::Back,
                    ..
                }
            )
        })
    });
    fake.send(Request::new(pointer, 9).uint(0).int(120));
    fake.send(Request::new(pointer, 5));
    let wheel = |delta| WindowEvent::MouseWheel {
        delta,
        position: PhysicalPosition::new(40.0, 5.0),
        modifiers: Modifiers::NONE,
    };
    expect(
        &mut backend,
        window,
        &wheel(ScrollDelta::Lines { x: 0.0, y: -1.0 }),
    );
    fake.send(Request::new(pointer, 4).uint(0).uint(1).fixed(-2.5));
    fake.send(Request::new(pointer, 5));
    expect(
        &mut backend,
        window,
        &wheel(ScrollDelta::Pixels { x: -2.5, y: 0.0 }),
    );
    fake.send(Request::new(pointer, 1).uint(10).uint(id));
    expect(&mut backend, window, &WindowEvent::CursorLeft);
}

#[test]
fn cursor_shapes_go_by_name_when_offered() {
    let options = Options {
        globals: vec![("wl_seat", 8), ("wp_cursor_shape_manager_v1", 1)],
        ..Options::default()
    };
    let (mut backend, fake) = Fake::start(options);
    let window = open(&mut backend);
    let pointer = device(&fake, "wl_pointer");
    fake.send(
        Request::new(pointer, 0)
            .uint(4)
            .uint(surface(window))
            .fixed(1.0)
            .fixed(1.0),
    );
    poll_until(&mut backend, "the entry", |events| {
        of(events, window)
            .iter()
            .any(|e| matches!(e, WindowEvent::CursorEntered { .. }))
    });
    backend.set_cursor(window, CursorIcon::Crosshair).unwrap();
    wait_requests(&mut backend, &fake, "wp_cursor_shape_device_v1", 1, 2);
    let shapes: Vec<(u32, u32)> = fake
        .state()
        .requests("wp_cursor_shape_device_v1", 1)
        .iter()
        .map(|r| (r.word(0), r.word(1)))
        .collect();
    assert_eq!(
        shapes,
        [(4, 1), (4, 8)],
        "default, then crosshair, for the entry's serial"
    );
    assert!(
        fake.state().requests("wl_pointer", 0).is_empty(),
        "nothing drawn"
    );
}

/// The title bar and border surfaces of a client-side decorated window.
fn decoration_surfaces(backend: &crate::wayland::WaylandBackend, window: WindowId) -> (u32, u32) {
    backend.windows[&surface(window)]
        .decorations
        .as_ref()
        .expect("client-side decorations")
        .surfaces()
}

#[test]
fn the_title_bar_moves_maximizes_and_closes() {
    let (mut backend, fake) = Fake::start(Options::default());
    let window = open(&mut backend);
    let (title_bar, border) = decoration_surfaces(&backend, window);
    let pointer = device(&fake, "wl_pointer");
    let mid = f64::from(TITLE_BAR_HEIGHT) / 2.0;
    let press = |serial: u32, time: u32, button: u32, state: u32| {
        Request::new(pointer, 3)
            .uint(serial)
            .uint(time)
            .uint(button)
            .uint(state)
    };
    fake.send(
        Request::new(pointer, 0)
            .uint(1)
            .uint(title_bar)
            .fixed(20.0)
            .fixed(mid),
    );
    fake.send(press(2, 0, BTN_LEFT, 1));
    fake.send(press(3, 10, BTN_LEFT, 0));
    wait_requests(&mut backend, &fake, "xdg_toplevel", 5, 1);
    fake.send(press(4, 2000, BTN_RIGHT, 1));
    wait_requests(&mut backend, &fake, "xdg_toplevel", 4, 1);
    fake.send(press(5, 4000, BTN_LEFT, 1));
    fake.send(press(6, 4100, BTN_LEFT, 1));
    wait_requests(&mut backend, &fake, "xdg_toplevel", 9, 1);
    let maximize_x = 320.0 - 1.5 * f64::from(TITLE_BAR_HEIGHT);
    fake.send(
        Request::new(pointer, 2)
            .uint(0)
            .fixed(maximize_x)
            .fixed(mid),
    );
    fake.send(press(7, 6000, BTN_LEFT, 1));
    fake.send(press(8, 6050, BTN_LEFT, 0));
    wait_requests(&mut backend, &fake, "xdg_toplevel", 9, 2);
    fake.send(Request::new(pointer, 2).uint(0).fixed(315.0).fixed(mid));
    fake.send(press(9, 8000, BTN_LEFT, 1));
    fake.send(press(10, 8050, BTN_LEFT, 0));
    let events = expect(&mut backend, window, &WindowEvent::CloseRequested);
    assert!(
        !events.iter().any(|e| matches!(
            e,
            WindowEvent::MouseInput { .. } | WindowEvent::CursorEntered { .. }
        )),
        "the decorations keep their input: {events:?}"
    );
    fake.send(Request::new(pointer, 1).uint(11).uint(title_bar));
    fake.send(
        Request::new(pointer, 0)
            .uint(12)
            .uint(border)
            .fixed(2.0)
            .fixed(100.0),
    );
    fake.send(press(13, 9000, BTN_LEFT, 1));
    wait_requests(&mut backend, &fake, "xdg_toplevel", 6, 1);
    let resize = fake.state().requests("xdg_toplevel", 6)[0].clone();
    assert_eq!((resize.word(1), resize.word(2)), (13, Edges::LEFT));
    let shown_cursors = fake.state().requests("wl_pointer", 0).len();
    assert!(shown_cursors >= 2, "a drawn resize cursor over the border");
}

#[test]
fn the_title_bar_follows_the_desktops_settings() {
    // A layout with the close button on the left, and other actions.
    let options = Options {
        title_bar: TitleBarSettings {
            layout: ButtonLayout::parse("close:minimize"),
            double_click: TitleBarAction::Minimize,
            middle_click: TitleBarAction::Menu,
            right_click: TitleBarAction::Nothing,
        },
        ..Options::default()
    };
    let (mut backend, fake) = Fake::start(options);
    let window = open(&mut backend);
    let (title_bar, _) = decoration_surfaces(&backend, window);
    let pointer = device(&fake, "wl_pointer");
    let mid = f64::from(TITLE_BAR_HEIGHT) / 2.0;
    let press = |serial: u32, time: u32, button: u32, state: u32| {
        Request::new(pointer, 3)
            .uint(serial)
            .uint(time)
            .uint(button)
            .uint(state)
    };
    fake.send(
        Request::new(pointer, 0)
            .uint(1)
            .uint(title_bar)
            .fixed(150.0)
            .fixed(mid),
    );
    // A double click minimizes (xdg_toplevel.set_minimized), the middle
    // button opens the window menu, the right one does nothing.
    fake.send(press(2, 1000, BTN_LEFT, 1));
    fake.send(press(3, 1050, BTN_LEFT, 0));
    fake.send(press(4, 1100, BTN_LEFT, 1));
    wait_requests(&mut backend, &fake, "xdg_toplevel", 13, 1);
    fake.send(press(5, 3000, BTN_RIGHT, 1));
    fake.send(press(6, 3100, BTN_MIDDLE, 1));
    wait_requests(&mut backend, &fake, "xdg_toplevel", 4, 1);
    assert_eq!(
        fake.state().requests("xdg_toplevel", 4)[0].word(1),
        6,
        "the middle click's serial: the right click asked nothing"
    );
    // The minimize button is at the right end, the close button at the
    // left end.
    fake.send(Request::new(pointer, 2).uint(0).fixed(315.0).fixed(mid));
    fake.send(press(7, 5000, BTN_LEFT, 1));
    fake.send(press(8, 5050, BTN_LEFT, 0));
    wait_requests(&mut backend, &fake, "xdg_toplevel", 13, 2);
    fake.send(Request::new(pointer, 2).uint(0).fixed(5.0).fixed(mid));
    fake.send(press(9, 7000, BTN_LEFT, 1));
    fake.send(press(10, 7050, BTN_LEFT, 0));
    expect(&mut backend, window, &WindowEvent::CloseRequested);
}
