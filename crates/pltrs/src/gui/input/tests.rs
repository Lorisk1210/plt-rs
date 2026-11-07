use pltrs_platform::{Key, KeyEvent};

use super::*;

fn press(button: MouseButton, at: (f64, f64), click_count: u32) -> UiEvent {
    UiEvent::PointerDown {
        button,
        position: Point::new(at.0, at.1),
        modifiers: Modifiers::CONTROL | Modifiers::SHIFT,
        click_count,
    }
}

fn release(button: MouseButton, at: (f64, f64)) -> UiEvent {
    UiEvent::PointerUp {
        button,
        position: Point::new(at.0, at.1),
        modifiers: Modifiers::NONE,
    }
}

fn motion(at: (f64, f64)) -> UiEvent {
    UiEvent::PointerMove {
        position: Point::new(at.0, at.1),
        modifiers: Modifiers::NONE,
    }
}

fn key(key: Key, state: ButtonState, modifiers: Modifiers) -> UiEvent {
    UiEvent::Key(KeyEvent {
        key,
        state,
        repeat: false,
        text: None,
        modifiers,
        scancode: 0,
    })
}

fn kinds(inputs: &[CanvasInput]) -> Vec<InputKind> {
    inputs.iter().map(|input| input.kind.clone()).collect()
}

/// Checks that `event` reports nothing.
fn nothing(state: &mut InputState, event: &UiEvent) {
    assert_eq!(state.convert(event), Vec::new(), "{event:?}");
}

/// The one input `event` reports.
fn one(state: &mut InputState, event: &UiEvent) -> CanvasInput {
    let mut inputs = state.convert(event);
    assert_eq!(inputs.len(), 1, "{inputs:?}");
    inputs.remove(0)
}

#[test]
fn buttons_are_numbered_and_held_until_released() {
    let mut state = InputState::default();
    state.convert(&motion((10.0, 20.0)));
    let input = one(&mut state, &press(MouseButton::Left, (10.0, 20.0), 1));
    assert_eq!(
        input.kind,
        InputKind::Press {
            button: 1,
            double: false
        }
    );
    assert_eq!(input.position, Some(Point::new(10.0, 20.0)));
    assert_eq!(input.modifiers, ["ctrl", "shift"]);
    assert_eq!(input.buttons, [1]);
    let second = one(&mut state, &press(MouseButton::Right, (10.0, 20.0), 2));
    assert_eq!(
        second.kind,
        InputKind::Press {
            button: 3,
            double: true
        }
    );
    assert_eq!(second.buttons, [1, 3]);
    let released = state.convert(&release(MouseButton::Left, (12.0, 22.0)));
    assert_eq!(kinds(&released), [InputKind::Release { button: 1 }]);
    assert_eq!(released[0].buttons, [3]);
    assert_eq!(released[0].modifiers, Vec::<&str>::new());
    nothing(&mut state, &press(MouseButton::Other(12), (1.0, 1.0), 1));
}

#[test]
fn motion_that_does_not_move_is_no_event_and_presses_move_first() {
    let mut state = InputState::default();
    // Entering is no motion.
    state.convert(&UiEvent::PointerEnter(Point::new(5.0, 5.0)));
    assert_eq!(
        kinds(&state.convert(&motion((5.0, 5.0)))),
        [InputKind::Motion]
    );
    nothing(&mut state, &motion((5.0, 5.0)));
    let pressed = state.convert(&press(MouseButton::Left, (7.0, 5.0), 1));
    assert_eq!(
        kinds(&pressed),
        [
            InputKind::Motion,
            InputKind::Press {
                button: 1,
                double: false
            }
        ]
    );
    assert_eq!(pressed[0].position, Some(Point::new(7.0, 5.0)));
    assert_eq!(pressed[0].buttons, Vec::<u16>::new(), "before the press");
}

#[test]
fn leaving_with_a_button_held_waits_for_the_release() {
    let mut state = InputState::default();
    state.convert(&UiEvent::PointerEnter(Point::new(5.0, 6.0)));
    state.convert(&press(MouseButton::Left, (5.0, 6.0), 1));
    let outside = Point::new(-30.0, 6.0);
    nothing(&mut state, &UiEvent::PointerLeave(Some(outside)));
    assert_eq!(
        kinds(&state.convert(&motion((-40.0, 6.0)))),
        [InputKind::Motion]
    );
    let released = state.convert(&release(MouseButton::Left, (-40.0, 6.0)));
    assert_eq!(
        kinds(&released),
        [InputKind::Release { button: 1 }, InputKind::Leave]
    );
    assert_eq!(released[1].position, Some(Point::new(-40.0, 6.0)));
    // Back in before the release: neither the leave nor the enter.
    state.convert(&UiEvent::PointerEnter(Point::new(5.0, 6.0)));
    state.convert(&press(MouseButton::Left, (5.0, 6.0), 1));
    nothing(&mut state, &UiEvent::PointerLeave(None));
    nothing(&mut state, &UiEvent::PointerEnter(Point::new(4.0, 6.0)));
    let released = state.convert(&release(MouseButton::Left, (4.0, 6.0)));
    assert_eq!(kinds(&released), [InputKind::Release { button: 1 }]);
}

#[test]
fn leaving_and_keys_report_where_the_pointer_went() {
    let mut state = InputState::default();
    let ctrl_s = |state| key(Key::Character('s'), state, Modifiers::CONTROL);
    let unseen = one(&mut state, &ctrl_s(ButtonState::Pressed));
    assert_eq!(unseen.kind, InputKind::KeyPress("ctrl+s".into()));
    assert_eq!(unseen.position, None);
    state.convert(&UiEvent::PointerEnter(Point::new(5.0, 6.0)));
    let left = one(
        &mut state,
        &UiEvent::PointerLeave(Some(Point::new(-2.0, 6.0))),
    );
    assert_eq!(left.kind, InputKind::Leave);
    assert_eq!(left.position, Some(Point::new(-2.0, 6.0)));
    let released = one(&mut state, &ctrl_s(ButtonState::Released));
    assert_eq!(released.kind, InputKind::KeyRelease("ctrl+s".into()));
    assert_eq!(released.position, Some(Point::new(-2.0, 6.0)));
    // Where the platform does not tell, the last place known.
    state.convert(&UiEvent::PointerEnter(Point::new(5.0, 6.0)));
    let left = one(&mut state, &UiEvent::PointerLeave(None));
    assert_eq!(left.position, Some(Point::new(5.0, 6.0)));
    let menu = key(Key::Menu, ButtonState::Pressed, Modifiers::NONE);
    nothing(&mut state, &menu);
}

#[test]
fn modifier_keys_count_as_held_until_released() {
    let mut state = InputState::default();
    let shift = |state| key(Key::Shift, state, Modifiers::NONE);
    assert_eq!(
        one(&mut state, &shift(ButtonState::Pressed)).kind,
        InputKind::KeyPress("shift".into())
    );
    let entered = one(&mut state, &UiEvent::PointerEnter(Point::new(1.0, 2.0)));
    assert_eq!(entered.modifiers, ["shift"]);
    state.convert(&shift(ButtonState::Released));
    let moved = one(&mut state, &motion((3.0, 2.0)));
    assert_eq!(moved.modifiers, Vec::<&str>::new());
}

#[test]
fn wheels_turn_vertically_by_notches() {
    let mut state = InputState::default();
    let wheel = |delta, modifiers| UiEvent::Wheel {
        delta,
        position: Point::new(1.0, 1.0),
        modifiers,
    };
    let lines = one(
        &mut state,
        &wheel(ScrollDelta::Lines { x: 0.0, y: -1.0 }, Modifiers::NONE),
    );
    assert_eq!(lines.kind, InputKind::Scroll { step: -1.0 });
    let pixels = one(
        &mut state,
        &wheel(ScrollDelta::Pixels { x: 0.0, y: 20.0 }, Modifiers::NONE),
    );
    assert_eq!(pixels.kind, InputKind::Scroll { step: 0.5 });
    let sideways = wheel(ScrollDelta::Lines { x: 1.0, y: 0.0 }, Modifiers::NONE);
    nothing(&mut state, &sideways);
    // Alt turns the wheels, but on macOS.
    let alt_vertical = wheel(ScrollDelta::Lines { x: 0.0, y: 1.0 }, Modifiers::ALT);
    let alt_sideways = wheel(ScrollDelta::Lines { x: -1.0, y: 0.0 }, Modifiers::ALT);
    if cfg!(target_os = "macos") {
        assert_eq!(
            kinds(&state.convert(&alt_vertical)),
            [InputKind::Scroll { step: 1.0 }]
        );
        nothing(&mut state, &alt_sideways);
    } else {
        nothing(&mut state, &alt_vertical);
        assert_eq!(
            kinds(&state.convert(&alt_sideways)),
            [InputKind::Scroll { step: -1.0 }]
        );
    }
}
