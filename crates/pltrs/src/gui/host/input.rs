//! Input over a hosted window's canvas, in the terms of matplotlib's
//! events: button numbers, key names, modifier names and wheel steps.

use pltrs_core::Point;
use pltrs_platform::{ButtonState, Modifiers, MouseButton, ScrollDelta};
use pltrs_ui::UiEvent;

use crate::gui::keys;

/// What happened over the canvas.
#[derive(Clone, Debug, PartialEq)]
pub enum InputKind {
    /// The pointer moved onto the canvas (`figure_enter_event`).
    Enter,
    /// The pointer left the canvas (`figure_leave_event`).
    Leave,
    /// The pointer moved (`motion_notify_event`).
    Motion,
    /// A mouse button went down (`button_press_event`); `double` for the
    /// second press of a double click (matplotlib's `dblclick`).
    Press {
        /// matplotlib's number of the button (`MouseButton`).
        button: u16,
        /// Whether the press completes a double click.
        double: bool,
    },
    /// A mouse button went up (`button_release_event`).
    Release {
        /// matplotlib's number of the button.
        button: u16,
    },
    /// The wheel turned by `step` notches, up positive (`scroll_event`).
    Scroll {
        /// Notches, fractional for fine-grained wheels and touchpads.
        step: f64,
    },
    /// A key went down, named as matplotlib names it (`"a"`, `"ctrl+s"`,
    /// `"left"`; `key_press_event`).
    KeyPress(String),
    /// A key went up (`key_release_event`).
    KeyRelease(String),
}

/// Input over the canvas of a hosted window.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasInput {
    /// What happened.
    pub kind: InputKind,
    /// Where the pointer is, in physical pixels from the canvas' top-left
    /// corner (the program counts matplotlib's `y` up from the figure's
    /// bottom); `None` when it was never over the canvas.
    pub position: Option<Point>,
    /// The modifier keys held, as matplotlib names them: `ctrl`, `alt`,
    /// `shift` and `cmd` (macOS) or `super`.
    pub modifiers: Vec<&'static str>,
    /// The mouse buttons held, by matplotlib's numbers, in the order they
    /// went down.
    pub buttons: Vec<u16>,
}

/// matplotlib's names of the modifier keys `modifiers` holds.
fn modifier_names(modifiers: Modifiers) -> Vec<&'static str> {
    let super_name = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "super"
    };
    [
        (Modifiers::CONTROL, "ctrl"),
        (Modifiers::ALT, "alt"),
        (Modifiers::SHIFT, "shift"),
        (Modifiers::SUPER, super_name),
    ]
    .into_iter()
    .filter(|&(flag, _)| modifiers.contains(flag))
    .map(|(_, name)| name)
    .collect()
}

/// Wheel notches of a scroll: lines as they are, pixels as matplotlib's
/// Qt backend counts a notch of a fine-grained wheel.
fn wheel_steps(delta: ScrollDelta) -> f64 {
    match delta {
        ScrollDelta::Lines { y, .. } => y,
        ScrollDelta::Pixels { y, .. } => y / 40.0,
    }
}

/// What the canvas of a hosted window remembers of the input: where the
/// pointer was last, the buttons and the modifiers held.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct InputState {
    pointer: Option<Point>,
    buttons: Vec<u16>,
    modifiers: Modifiers,
}

impl InputState {
    /// The canvas input `event` reports, if matplotlib has an event for
    /// it (keys it does not name, mouse buttons beyond the fifth and
    /// pastes have none).
    pub(super) fn convert(&mut self, event: &UiEvent) -> Option<CanvasInput> {
        if let Some(position) = event.position() {
            self.pointer = Some(position);
        }
        let kind = match event {
            UiEvent::PointerEnter(_) => InputKind::Enter,
            UiEvent::PointerLeave => InputKind::Leave,
            UiEvent::PointerMove { modifiers, .. } => {
                self.modifiers = *modifiers;
                InputKind::Motion
            }
            UiEvent::PointerDown {
                button,
                modifiers,
                click_count,
                ..
            } => {
                self.modifiers = *modifiers;
                let button = known(*button)?;
                if !self.buttons.contains(&button) {
                    self.buttons.push(button);
                }
                InputKind::Press {
                    button,
                    double: *click_count == 2,
                }
            }
            UiEvent::PointerUp {
                button, modifiers, ..
            } => {
                self.modifiers = *modifiers;
                let button = known(*button)?;
                self.buttons.retain(|&held| held != button);
                InputKind::Release { button }
            }
            UiEvent::Wheel {
                delta, modifiers, ..
            } => {
                self.modifiers = *modifiers;
                InputKind::Scroll {
                    step: wheel_steps(*delta),
                }
            }
            UiEvent::Key(key) => {
                self.modifiers = key.modifiers;
                let name = keys::key_name(key)?;
                match key.state {
                    ButtonState::Pressed => InputKind::KeyPress(name),
                    ButtonState::Released => InputKind::KeyRelease(name),
                }
            }
            _ => return None,
        };
        Some(CanvasInput {
            kind,
            position: self.pointer,
            modifiers: modifier_names(self.modifiers),
            buttons: self.buttons.clone(),
        })
    }
}

/// matplotlib's number of `button`, for the five buttons it knows.
fn known(button: MouseButton) -> Option<u16> {
    match button {
        MouseButton::Other(_) => None,
        button => Some(button.number()),
    }
}

#[cfg(test)]
mod tests {
    use pltrs_platform::{Key, KeyEvent};

    use super::*;

    fn press(button: MouseButton, click_count: u32) -> UiEvent {
        UiEvent::PointerDown {
            button,
            position: Point::new(10.0, 20.0),
            modifiers: Modifiers::CONTROL | Modifiers::SHIFT,
            click_count,
        }
    }

    #[test]
    fn buttons_are_numbered_and_held_until_released() {
        let mut state = InputState::default();
        let input = state.convert(&press(MouseButton::Left, 1)).unwrap();
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
        let second = state.convert(&press(MouseButton::Right, 2)).unwrap();
        assert_eq!(
            second.kind,
            InputKind::Press {
                button: 3,
                double: true
            }
        );
        assert_eq!(second.buttons, [1, 3]);
        let release = UiEvent::PointerUp {
            button: MouseButton::Left,
            position: Point::new(12.0, 22.0),
            modifiers: Modifiers::NONE,
        };
        let released = state.convert(&release).unwrap();
        assert_eq!(released.kind, InputKind::Release { button: 1 });
        assert_eq!(released.buttons, [3]);
        assert_eq!(released.modifiers, Vec::<&str>::new());
        assert!(state.convert(&press(MouseButton::Other(12), 1)).is_none());
    }

    #[test]
    fn leaving_and_keys_report_where_the_pointer_was() {
        let mut state = InputState::default();
        let key = |state| {
            UiEvent::Key(KeyEvent {
                key: Key::Character('s'),
                state,
                repeat: false,
                text: None,
                modifiers: Modifiers::CONTROL,
                scancode: 0,
            })
        };
        let unseen = state.convert(&key(ButtonState::Pressed)).unwrap();
        assert_eq!(unseen.kind, InputKind::KeyPress("ctrl+s".into()));
        assert_eq!(unseen.position, None);
        state.convert(&UiEvent::PointerEnter(Point::new(5.0, 6.0)));
        let left = state.convert(&UiEvent::PointerLeave).unwrap();
        assert_eq!(left.kind, InputKind::Leave);
        assert_eq!(left.position, Some(Point::new(5.0, 6.0)));
        let released = state.convert(&key(ButtonState::Released)).unwrap();
        assert_eq!(released.kind, InputKind::KeyRelease("ctrl+s".into()));
        let menu = UiEvent::Key(KeyEvent {
            key: Key::Menu,
            state: ButtonState::Pressed,
            repeat: false,
            text: None,
            modifiers: Modifiers::NONE,
            scancode: 0,
        });
        assert!(
            state.convert(&menu).is_none(),
            "matplotlib names no such key"
        );
    }

    #[test]
    fn wheels_turn_by_notches() {
        let mut state = InputState::default();
        let wheel = |delta| UiEvent::Wheel {
            delta,
            position: Point::new(1.0, 1.0),
            modifiers: Modifiers::NONE,
        };
        let lines = state
            .convert(&wheel(ScrollDelta::Lines { x: 0.0, y: -1.0 }))
            .unwrap();
        assert_eq!(lines.kind, InputKind::Scroll { step: -1.0 });
        let pixels = state
            .convert(&wheel(ScrollDelta::Pixels { x: 0.0, y: 20.0 }))
            .unwrap();
        assert_eq!(pixels.kind, InputKind::Scroll { step: 0.5 });
    }
}
