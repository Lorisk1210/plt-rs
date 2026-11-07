//! Input over a figure window's canvas in the terms of matplotlib's
//! events — button numbers, key names, modifier names and wheel steps —
//! and as matplotlib's Qt backend gives them, event for event:
//!
//! * a motion to where the last motion or button event was is no event,
//!   and a press where the pointer has not been reported yet comes after
//!   a motion there (Qt sends no event that changes both the place and
//!   the buttons);
//! * the pointer leaving the canvas while a button is held stays with
//!   the canvas (motion goes on being reported) until the last button is
//!   released, and leaves then; coming back before that is no event;
//! * leaving reports where the pointer went, outside the canvas, where
//!   the platform tells, and so do the keys typed meanwhile;
//! * the wheel turns by notches, vertically only: a horizontal turn is no
//!   event, and Alt held turns a vertical wheel horizontal and a
//!   horizontal one vertical (but on macOS);
//! * modifier keys count as held from their press to their release, for
//!   the pointer's events;
//! * keys are named as Qt's backend names them ([`keys::key_name`]).

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

/// Input over the canvas of a window.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasInput {
    /// What happened.
    pub kind: InputKind,
    /// Where the pointer is, in physical pixels from the canvas' top-left
    /// corner (the program counts matplotlib's `y` up from the figure's
    /// bottom), outside the canvas once it left; `None` when it was never
    /// over the canvas.
    pub position: Option<Point>,
    /// The modifier keys held, as matplotlib's Qt backend names them:
    /// `ctrl`, `alt`, `shift`, and `meta` (`cmd` on macOS) for the Super
    /// key.
    pub modifiers: Vec<&'static str>,
    /// The mouse buttons held, by matplotlib's numbers, in the order they
    /// went down.
    pub buttons: Vec<u16>,
}

/// matplotlib's names of the modifier keys `modifiers` holds.
fn modifier_names(modifiers: Modifiers) -> Vec<&'static str> {
    [
        (Modifiers::CONTROL, "ctrl"),
        (Modifiers::ALT, "alt"),
        (Modifiers::SHIFT, "shift"),
        (Modifiers::SUPER, keys::SUPER_NAME),
    ]
    .into_iter()
    .filter(|&(flag, _)| modifiers.contains(flag))
    .map(|(_, name)| name)
    .collect()
}

/// The vertical notches of a wheel turn: lines as they are, pixels as
/// matplotlib's Qt backend counts a notch of a fine-grained wheel. Alt
/// swaps the directions, as Qt does but on macOS.
fn wheel_steps(delta: ScrollDelta, modifiers: Modifiers) -> f64 {
    let (x, y) = match delta {
        ScrollDelta::Lines { x, y } => (x, y),
        ScrollDelta::Pixels { x, y } => (x / 40.0, y / 40.0),
    };
    if modifiers.alt() && !cfg!(target_os = "macos") {
        x
    } else {
        y
    }
}

/// What the canvas of a window remembers of the input: where the pointer
/// was last, where the last motion or button event was, the buttons and
/// the modifiers held, and whether the pointer left while a button was
/// held.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct InputState {
    pointer: Option<Point>,
    reported: Option<Point>,
    buttons: Vec<u16>,
    modifiers: Modifiers,
    left_while_held: bool,
}

impl InputState {
    /// The canvas input `event` reports, as matplotlib's Qt backend
    /// reports it: nothing for what it has no event for (keys it does not
    /// name, mouse buttons beyond the fifth, horizontal wheels, pastes),
    /// up to two events for a press where the pointer was not reported
    /// (a motion first) and the release of the last button after the
    /// pointer left (the leave after it).
    pub(crate) fn convert(&mut self, event: &UiEvent) -> Vec<CanvasInput> {
        let mut inputs = Vec::new();
        match event {
            UiEvent::PointerEnter(position) => {
                if !std::mem::take(&mut self.left_while_held) {
                    self.pointer = Some(*position);
                    inputs.push(self.input(InputKind::Enter));
                }
            }
            UiEvent::PointerLeave(position) => {
                if position.is_some() {
                    self.pointer = *position;
                }
                if self.buttons.is_empty() {
                    inputs.push(self.input(InputKind::Leave));
                } else {
                    self.left_while_held = true;
                }
            }
            UiEvent::PointerMove {
                position,
                modifiers,
            } => {
                self.modifiers = *modifiers;
                self.pointer = Some(*position);
                self.move_to(*position, &mut inputs);
            }
            UiEvent::PointerDown {
                button,
                position,
                modifiers,
                click_count,
            } => {
                self.modifiers = *modifiers;
                let Some(button) = known(*button) else {
                    return inputs;
                };
                self.pointer = Some(*position);
                self.move_to(*position, &mut inputs);
                if !self.buttons.contains(&button) {
                    self.buttons.push(button);
                }
                inputs.push(self.input(InputKind::Press {
                    button,
                    double: *click_count == 2,
                }));
            }
            UiEvent::PointerUp {
                button,
                position,
                modifiers,
            } => {
                self.modifiers = *modifiers;
                let Some(button) = known(*button) else {
                    return inputs;
                };
                self.pointer = Some(*position);
                self.reported = Some(*position);
                self.buttons.retain(|&held| held != button);
                inputs.push(self.input(InputKind::Release { button }));
                if self.buttons.is_empty() && std::mem::take(&mut self.left_while_held) {
                    inputs.push(self.input(InputKind::Leave));
                }
            }
            UiEvent::Wheel {
                delta, modifiers, ..
            } => {
                self.modifiers = *modifiers;
                let step = wheel_steps(*delta, *modifiers);
                if step != 0.0 {
                    inputs.push(self.input(InputKind::Scroll { step }));
                }
            }
            UiEvent::Key(key) => {
                let held = key.state == ButtonState::Pressed;
                self.modifiers = key.modifiers.with(keys::own_modifier(key.key), held);
                if let Some(name) = keys::key_name(key) {
                    inputs.push(self.input(match key.state {
                        ButtonState::Pressed => InputKind::KeyPress(name),
                        ButtonState::Released => InputKind::KeyRelease(name),
                    }));
                }
            }
            _ => {}
        }
        inputs
    }

    /// Reports a motion to `position` in `inputs`, unless the last motion
    /// or button event was there.
    fn move_to(&mut self, position: Point, inputs: &mut Vec<CanvasInput>) {
        if self.reported != Some(position) {
            self.reported = Some(position);
            inputs.push(self.input(InputKind::Motion));
        }
    }

    /// `kind` with the pointer, the modifiers and the buttons as they are.
    fn input(&self, kind: InputKind) -> CanvasInput {
        CanvasInput {
            kind,
            position: self.pointer,
            modifiers: modifier_names(self.modifiers),
            buttons: self.buttons.clone(),
        }
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
mod tests;
