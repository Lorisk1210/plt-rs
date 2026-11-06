//! Input posted as the window server delivers it: mouse buttons, drags,
//! the pointer crossing the view, scroll wheels and touchpads, keys and
//! modifier keys, and the menu's key equivalents.

use std::time::{Duration, Instant};

use pltrs_platform::{
    ButtonState, Event, EventLoop, Key, KeyEvent, Modifiers, MouseButton, PhysicalPosition,
    PhysicalSize, ScrollDelta, WindowEvent, WindowId,
};

use crate::appkit::{self, Action, KeyAction, Keystroke, Mouse, Scroll};
use crate::{appkit_loop, common, open_still};

/// `NSEventModifierFlagCapsLock`.
const CAPS_LOCK: u64 = 1 << 16;
/// `NSEventModifierFlagShift`.
const SHIFT: u64 = 1 << 17;
/// `NSEventModifierFlagControl`.
const CONTROL: u64 = 1 << 18;
/// `NSEventModifierFlagOption`.
const OPTION: u64 = 1 << 19;
/// `NSEventModifierFlagCommand`.
const COMMAND: u64 = 1 << 20;
/// The device-dependent flags of the left and right Shift keys and the
/// left Command key.
const LEFT_SHIFT: u64 = 0x2;
const RIGHT_SHIFT: u64 = 0x4;
const LEFT_COMMAND: u64 = 0x8;

/// An event loop with a window of 600 × 400 pixels, opened, and the
/// window's scale factor.
fn window() -> (EventLoop, WindowId, f64) {
    let mut event_loop = appkit_loop();
    let window = open_still(&mut event_loop, PhysicalSize::new(600, 400));
    let scale = appkit::backing_scale(&mut event_loop, window);
    (event_loop, window, scale)
}

/// Pumps until `window` receives an event `wanted` accepts; returns it.
fn receive(
    event_loop: &mut EventLoop,
    window: WindowId,
    wanted: impl Fn(&WindowEvent) -> bool,
) -> WindowEvent {
    common::pump_until(event_loop, window, &wanted)
        .into_iter()
        .find(wanted)
        .expect("the event was seen")
}

/// Posts `mouse` and returns the button event it causes.
fn click(event_loop: &mut EventLoop, window: WindowId, mouse: Mouse) -> WindowEvent {
    appkit::post_mouse(event_loop, window, mouse);
    receive(event_loop, window, |e| {
        matches!(e, WindowEvent::MouseInput { .. })
    })
}

/// The button event of `button` at `(x, y)` points.
fn button_event(
    button: MouseButton,
    state: ButtonState,
    (x, y): (f64, f64),
    scale: f64,
    modifiers: Modifiers,
    click_count: u32,
) -> WindowEvent {
    WindowEvent::MouseInput {
        button,
        state,
        position: PhysicalPosition::new(x * scale, y * scale),
        modifiers,
        click_count,
    }
}

/// Buttons arrive pressed and released where they were, in pixels, with
/// their click counts; `AppKit`'s button numbers become pltrs's buttons.
pub(crate) fn buttons_press_and_release_where_posted() {
    let (mut event_loop, window, scale) = window();
    let at = (30.25, 40.5);
    let (pressed, released) = (ButtonState::Pressed, ButtonState::Released);
    for (number, button) in [
        (0, MouseButton::Left),
        (1, MouseButton::Right),
        (2, MouseButton::Middle),
        (3, MouseButton::Back),
        (4, MouseButton::Forward),
        (7, MouseButton::Other(8)),
    ] {
        let press = click(
            &mut event_loop,
            window,
            Mouse::new(Action::Press, number, at),
        );
        assert_eq!(
            press,
            button_event(button, pressed, at, scale, Modifiers::NONE, 1)
        );
        let release = click(
            &mut event_loop,
            window,
            Mouse::new(Action::Release, number, at),
        );
        assert_eq!(
            release,
            button_event(button, released, at, scale, Modifiers::NONE, 1)
        );
    }
    // The second click of a double click, with Shift held.
    let double = Mouse::new(Action::Press, 0, at)
        .with_clicks(2)
        .with_flags(SHIFT);
    assert_eq!(
        click(&mut event_loop, window, double),
        button_event(MouseButton::Left, pressed, at, scale, Modifiers::SHIFT, 2)
    );
    let release = Mouse {
        action: Action::Release,
        ..double
    };
    assert_eq!(
        click(&mut event_loop, window, release),
        button_event(MouseButton::Left, released, at, scale, Modifiers::SHIFT, 1)
    );
}

/// Control-click is a right click and Option-click a middle one, from
/// the press to the release (even once the modifier is let go).
pub(crate) fn control_and_option_clicks_emulate_buttons() {
    let (mut event_loop, window, scale) = window();
    let at = (100.0, 50.0);
    for (flags, modifiers, button) in [
        (CONTROL, Modifiers::CONTROL, MouseButton::Right),
        (OPTION, Modifiers::ALT, MouseButton::Middle),
    ] {
        let press = Mouse::new(Action::Press, 0, at).with_flags(flags);
        assert_eq!(
            click(&mut event_loop, window, press),
            button_event(button, ButtonState::Pressed, at, scale, modifiers, 1)
        );
        let release = Mouse::new(Action::Release, 0, at);
        assert_eq!(
            click(&mut event_loop, window, release),
            button_event(button, ButtonState::Released, at, scale, Modifiers::NONE, 1)
        );
    }
}

/// Dragging with any button held moves the pointer, in pixels.
pub(crate) fn drags_move_the_pointer() {
    let (mut event_loop, window, scale) = window();
    for (button, x) in [(0, 20.5), (1, 21.5), (2, 22.5)] {
        click(
            &mut event_loop,
            window,
            Mouse::new(Action::Press, button, (10.0, 10.0)),
        );
        let to = (x, 30.0);
        let drag = Mouse::new(Action::Drag, button, to).with_flags(SHIFT);
        appkit::post_mouse(&mut event_loop, window, drag);
        let expected = WindowEvent::CursorMoved {
            position: PhysicalPosition::new(to.0 * scale, to.1 * scale),
            modifiers: Modifiers::SHIFT,
        };
        receive(&mut event_loop, window, |e| *e == expected);
        click(
            &mut event_loop,
            window,
            Mouse::new(Action::Release, button, to),
        );
    }
}

/// Moves the pointer into `window`'s view at `at`, out of it first (the
/// user's pointer may be over the window), and waits for it to enter.
pub(crate) fn enter(event_loop: &mut EventLoop, window: WindowId, at: (f64, f64)) {
    let scale = appkit::backing_scale(event_loop, window);
    appkit::post_crossing(event_loop, window, false, (0.0, 0.0));
    appkit::post_crossing(event_loop, window, true, at);
    let entered = WindowEvent::CursorEntered {
        position: PhysicalPosition::new(at.0 * scale, at.1 * scale),
    };
    receive(event_loop, window, |e| *e == entered);
}

/// The pointer enters the view where it crossed, and leaves it.
pub(crate) fn the_pointer_enters_and_leaves() {
    let (mut event_loop, window, _) = window();
    let at = (5.0, 6.5);
    enter(&mut event_loop, window, at);
    appkit::post_crossing(&mut event_loop, window, false, at);
    receive(&mut event_loop, window, |e| *e == WindowEvent::CursorLeft);
}

/// Wheels scroll lines and touchpads pixels, rightwards positive, where
/// the pointer is; Shift turns a wheel sideways, as everywhere on macOS.
pub(crate) fn wheels_scroll_lines_and_touchpads_pixels() {
    let (mut event_loop, window, scale) = window();
    let at = (40.0, 50.5);
    let position = PhysicalPosition::new(at.0 * scale, at.1 * scale);
    let cases = [
        ((-2, 3), false, 0, ScrollDelta::Lines { x: 2.0, y: 3.0 }),
        ((0, 3), false, SHIFT, ScrollDelta::Lines { x: -3.0, y: 0.0 }),
        (
            (-20, -30),
            true,
            SHIFT,
            ScrollDelta::Pixels {
                x: 20.0 * scale,
                y: -30.0 * scale,
            },
        ),
    ];
    for (delta, precise, flags, expected) in cases {
        let scroll = Scroll {
            at,
            delta,
            precise,
            flags,
        };
        appkit::post_scroll(&mut event_loop, window, scroll);
        let seen = receive(&mut event_loop, window, |e| {
            matches!(e, WindowEvent::MouseWheel { .. })
        });
        let modifiers = if flags == SHIFT {
            Modifiers::SHIFT
        } else {
            Modifiers::NONE
        };
        let expected = WindowEvent::MouseWheel {
            delta: expected,
            position,
            modifiers,
        };
        assert_eq!(seen, expected, "{scroll:?}");
    }
}

/// The key event of `key`.
fn key_event(
    key: Key,
    state: ButtonState,
    text: Option<&str>,
    modifiers: Modifiers,
    code: u16,
) -> WindowEvent {
    WindowEvent::Key(KeyEvent {
        key,
        state,
        repeat: false,
        text: text.map(str::to_owned),
        modifiers,
        scancode: u32::from(code),
    })
}

/// Posts `stroke` and returns the key event it causes.
fn strike(event_loop: &mut EventLoop, window: WindowId, stroke: Keystroke<'_>) -> WindowEvent {
    appkit::post_key(event_loop, window, stroke);
    receive(event_loop, window, |e| matches!(e, WindowEvent::Key(_)))
}

/// Characters type their text, named keys none; Control keeps the
/// text from a key, Option types its characters (`@` on German layouts).
pub(crate) fn keys_type_text_and_name_keys() {
    let (mut event_loop, window, _) = window();
    let pressed = ButtonState::Pressed;
    let cases = [
        (
            Keystroke::new(KeyAction::Press, 0x00, "a"),
            key_event(
                Key::Character('a'),
                pressed,
                Some("a"),
                Modifiers::NONE,
                0x00,
            ),
        ),
        (
            Keystroke::new(KeyAction::Release, 0x00, "a"),
            key_event(
                Key::Character('a'),
                ButtonState::Released,
                None,
                Modifiers::NONE,
                0x00,
            ),
        ),
        (
            Keystroke::new(KeyAction::Press, 0x00, "A").with_flags(SHIFT),
            key_event(
                Key::Character('A'),
                pressed,
                Some("A"),
                Modifiers::SHIFT,
                0x00,
            ),
        ),
        (
            Keystroke::new(KeyAction::Press, 0x0E, "é"),
            key_event(
                Key::Character('é'),
                pressed,
                Some("é"),
                Modifiers::NONE,
                0x0E,
            ),
        ),
        (
            Keystroke::new(KeyAction::Press, 0x08, "\u{3}")
                .unmodified("c")
                .with_flags(CONTROL),
            key_event(Key::Character('c'), pressed, None, Modifiers::CONTROL, 0x08),
        ),
        (
            Keystroke::new(KeyAction::Press, 0x25, "@")
                .unmodified("l")
                .with_flags(OPTION),
            key_event(
                Key::Character('l'),
                pressed,
                Some("@"),
                Modifiers::ALT,
                0x25,
            ),
        ),
        (
            Keystroke::new(KeyAction::Press, 0x24, "\r"),
            key_event(Key::Enter, pressed, None, Modifiers::NONE, 0x24),
        ),
        (
            Keystroke::new(KeyAction::Press, 0x35, "\u{1b}"),
            key_event(Key::Escape, pressed, None, Modifiers::NONE, 0x35),
        ),
        (
            Keystroke::new(KeyAction::Press, 0x7B, "\u{F702}"),
            key_event(Key::Left, pressed, None, Modifiers::NONE, 0x7B),
        ),
        (
            Keystroke::new(KeyAction::Press, 0x60, "\u{F708}"),
            key_event(Key::F(5), pressed, None, Modifiers::NONE, 0x60),
        ),
    ];
    for (stroke, expected) in cases {
        assert_eq!(
            strike(&mut event_loop, window, stroke),
            expected,
            "{stroke:?}"
        );
    }
    let repeated = strike(
        &mut event_loop,
        window,
        Keystroke::new(KeyAction::Press, 0x00, "a").repeated(),
    );
    assert!(
        matches!(&repeated, WindowEvent::Key(KeyEvent { repeat: true, .. })),
        "{repeated:?}"
    );
}

/// Dead keys put their accent on the next key, composed with the
/// keyboard layout in use: Option+e on American and most other layouts,
/// the ´ key on German ones. `AppKit` gives a dead key no characters.
pub(crate) fn dead_keys_compose_with_the_layout() {
    let (mut event_loop, window, _) = window();
    let candidates = [
        Keystroke::new(KeyAction::Press, 0x0E, "")
            .unmodified("e")
            .with_flags(OPTION),
        Keystroke::new(KeyAction::Press, 0x18, "").unmodified("´"),
    ];
    let mut composed = 0;
    for dead in candidates {
        let WindowEvent::Key(accent) = strike(&mut event_loop, window, dead) else {
            unreachable!("strike returns key events");
        };
        if accent.key != Key::Dead('´') {
            continue;
        }
        assert_eq!(accent.text, None, "{dead:?}");
        let e = strike(
            &mut event_loop,
            window,
            Keystroke::new(KeyAction::Press, 0x0E, "e"),
        );
        assert_eq!(
            e,
            key_event(
                Key::Character('e'),
                ButtonState::Pressed,
                Some("é"),
                Modifiers::NONE,
                0x0E
            ),
            "{dead:?}"
        );
        composed += 1;
    }
    if composed == 0 {
        println!("skipped: no dead acute on Option+e or ´ in this keyboard layout");
    }
}

/// Modifier keys report going down and up, left and right apart, with
/// the modifiers held before.
pub(crate) fn modifier_keys_go_down_and_up() {
    let (mut event_loop, window, _) = window();
    let (pressed, released) = (ButtonState::Pressed, ButtonState::Released);
    let cases = [
        (
            Keystroke::flags_changed(0x38, SHIFT | LEFT_SHIFT),
            key_event(Key::Shift, pressed, None, Modifiers::NONE, 0x38),
        ),
        (
            Keystroke::flags_changed(0x3C, SHIFT | LEFT_SHIFT | RIGHT_SHIFT),
            key_event(Key::Shift, pressed, None, Modifiers::SHIFT, 0x3C),
        ),
        (
            Keystroke::flags_changed(0x38, SHIFT | RIGHT_SHIFT),
            key_event(Key::Shift, released, None, Modifiers::SHIFT, 0x38),
        ),
        (
            Keystroke::flags_changed(0x3C, 0),
            key_event(Key::Shift, released, None, Modifiers::SHIFT, 0x3C),
        ),
        (
            Keystroke::flags_changed(0x37, COMMAND | LEFT_COMMAND),
            key_event(Key::Super, pressed, None, Modifiers::NONE, 0x37),
        ),
        (
            Keystroke::flags_changed(0x37, 0),
            key_event(Key::Super, released, None, Modifiers::SUPER, 0x37),
        ),
        (
            Keystroke::flags_changed(0x39, CAPS_LOCK),
            key_event(Key::CapsLock, pressed, None, Modifiers::NONE, 0x39),
        ),
        (
            Keystroke::flags_changed(0x39, 0),
            key_event(Key::CapsLock, released, None, Modifiers::NONE, 0x39),
        ),
    ];
    for (stroke, expected) in cases {
        assert_eq!(
            strike(&mut event_loop, window, stroke),
            expected,
            "{stroke:?}"
        );
    }
}

/// ⌘W is no menu item: it reaches the window as a key, which pltrs's
/// figure windows close on (matplotlib's `keymap.quit` has `cmd+w`).
pub(crate) fn command_w_reaches_the_window() {
    let (mut event_loop, window, _) = window();
    let stroke = Keystroke::new(KeyAction::Press, 0x0D, "w").with_flags(COMMAND);
    assert_eq!(
        strike(&mut event_loop, window, stroke),
        key_event(
            Key::Character('w'),
            ButtonState::Pressed,
            None,
            Modifiers::SUPER,
            0x0D
        )
    );
}

/// ⌘Q goes to the menu's Quit, which asks every window to close and
/// leaves the application running; the windows stay open until closed.
pub(crate) fn command_q_asks_every_window_to_close() {
    let (mut event_loop, first, _) = window();
    let second = common::open(&mut event_loop, PhysicalSize::new(200, 100));
    let stroke = Keystroke::new(KeyAction::Press, 0x0C, "q").with_flags(COMMAND);
    appkit::post_key(&mut event_loop, first, stroke);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut asked = Vec::new();
    let mut keys = Vec::new();
    while asked.len() < 2 && Instant::now() < deadline {
        event_loop
            .pump_events(Some(Duration::from_millis(20)), |_, event| match event {
                Event::Window {
                    window,
                    event: WindowEvent::CloseRequested,
                } => asked.push(window),
                Event::Window {
                    event: WindowEvent::Key(key),
                    ..
                } => keys.push(key),
                _ => {}
            })
            .expect("pumping events succeeds");
    }
    asked.sort_unstable();
    let mut both = vec![first, second];
    both.sort_unstable();
    assert_eq!(asked, both);
    assert_eq!(keys, [], "the menu takes ⌘Q");
    let mut open = event_loop.context().windows();
    open.sort_unstable();
    assert_eq!(open, both);
}
