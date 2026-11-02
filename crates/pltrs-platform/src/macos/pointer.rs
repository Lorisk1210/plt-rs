//! macOS pointer input as pltrs events: `NSEvent` types and button
//! numbers, the buttons a one-button mouse emulates, and scrolling.

use crate::event::{ButtonState, MouseButton, ScrollDelta};
use crate::geometry::PhysicalPosition;
use crate::keyboard::Modifiers;

/// The `NSEventType`s of pointer events.
pub(crate) mod event_type {
    pub(crate) const LEFT_MOUSE_DOWN: u64 = 1;
    pub(crate) const LEFT_MOUSE_UP: u64 = 2;
    pub(crate) const RIGHT_MOUSE_DOWN: u64 = 3;
    pub(crate) const RIGHT_MOUSE_UP: u64 = 4;
    pub(crate) const MOUSE_MOVED: u64 = 5;
    pub(crate) const LEFT_MOUSE_DRAGGED: u64 = 6;
    pub(crate) const RIGHT_MOUSE_DRAGGED: u64 = 7;
    pub(crate) const MOUSE_ENTERED: u64 = 8;
    pub(crate) const MOUSE_EXITED: u64 = 9;
    pub(crate) const OTHER_MOUSE_DOWN: u64 = 25;
    pub(crate) const OTHER_MOUSE_UP: u64 = 26;
    pub(crate) const OTHER_MOUSE_DRAGGED: u64 = 27;
}

/// What a pointer event reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pointer {
    /// A button went down or up.
    Button {
        /// The button `AppKit` reports (see [`emulated_button`] for what
        /// the left one stands for).
        button: MouseButton,
        /// Down or up.
        state: ButtonState,
    },
    /// The pointer moved (with or without a button held).
    Moved,
    /// The pointer entered the view.
    Entered,
    /// The pointer left the view.
    Exited,
}

/// What the pointer event of `kind` reports; `button_number` is the
/// event's `buttonNumber` (0 left, 1 right, 2 middle, then the side
/// buttons).
pub(crate) fn pointer(kind: u64, button_number: i64) -> Option<Pointer> {
    use event_type::{
        LEFT_MOUSE_DOWN, LEFT_MOUSE_DRAGGED, LEFT_MOUSE_UP, MOUSE_ENTERED, MOUSE_EXITED,
        MOUSE_MOVED, OTHER_MOUSE_DOWN, OTHER_MOUSE_DRAGGED, OTHER_MOUSE_UP, RIGHT_MOUSE_DOWN,
        RIGHT_MOUSE_DRAGGED, RIGHT_MOUSE_UP,
    };
    let button = |button, state| Some(Pointer::Button { button, state });
    let other = match button_number {
        2 => MouseButton::Middle,
        3 => MouseButton::Back,
        4 => MouseButton::Forward,
        n => MouseButton::Other(
            n.checked_add(1)
                .and_then(|number| u16::try_from(number).ok())
                .unwrap_or(u16::MAX),
        ),
    };
    match kind {
        LEFT_MOUSE_DOWN => button(MouseButton::Left, ButtonState::Pressed),
        LEFT_MOUSE_UP => button(MouseButton::Left, ButtonState::Released),
        RIGHT_MOUSE_DOWN => button(MouseButton::Right, ButtonState::Pressed),
        RIGHT_MOUSE_UP => button(MouseButton::Right, ButtonState::Released),
        OTHER_MOUSE_DOWN => button(other, ButtonState::Pressed),
        OTHER_MOUSE_UP => button(other, ButtonState::Released),
        MOUSE_MOVED | LEFT_MOUSE_DRAGGED | RIGHT_MOUSE_DRAGGED | OTHER_MOUSE_DRAGGED => {
            Some(Pointer::Moved)
        }
        MOUSE_ENTERED => Some(Pointer::Entered),
        MOUSE_EXITED => Some(Pointer::Exited),
        _ => None,
    }
}

/// The button a left-button press stands for: Control+click is a right
/// click and Option+click a middle click, as on one-button mice and in
/// matplotlib's macOS backend.
pub(crate) fn emulated_button(modifiers: Modifiers) -> MouseButton {
    if modifiers.control() {
        MouseButton::Right
    } else if modifiers.alt() {
        MouseButton::Middle
    } else {
        MouseButton::Left
    }
}

/// A point of the view (in points, flipped so that y grows downwards) in
/// physical pixels.
pub(crate) fn position(x: f64, y: f64, scale: f64) -> PhysicalPosition {
    PhysicalPosition::new(x * scale, y * scale)
}

/// A scroll of `scrollingDeltaX`/`scrollingDeltaY`: precise deltas
/// (touchpads, Magic Mouse) are points, made physical pixels, others
/// lines. `AppKit` counts x positive leftwards, pltrs rightwards; `None`
/// for no movement (the end of a gesture).
pub(crate) fn scroll(dx: f64, dy: f64, precise: bool, scale: f64) -> Option<ScrollDelta> {
    if dx == 0.0 && dy == 0.0 {
        return None;
    }
    Some(if precise {
        ScrollDelta::Pixels {
            x: -dx * scale,
            y: dy * scale,
        }
    } else {
        ScrollDelta::Lines { x: -dx, y: dy }
    })
}

#[cfg(test)]
mod tests {
    use super::event_type::*;
    use super::*;

    #[test]
    fn event_types_decode() {
        assert_eq!(
            pointer(LEFT_MOUSE_DOWN, 0),
            Some(Pointer::Button {
                button: MouseButton::Left,
                state: ButtonState::Pressed
            })
        );
        assert_eq!(
            pointer(RIGHT_MOUSE_UP, 1),
            Some(Pointer::Button {
                button: MouseButton::Right,
                state: ButtonState::Released
            })
        );
        assert!(pointer(LEFT_MOUSE_UP, 0).is_some());
        assert!(pointer(RIGHT_MOUSE_DOWN, 1).is_some());
        for kind in [
            MOUSE_MOVED,
            LEFT_MOUSE_DRAGGED,
            RIGHT_MOUSE_DRAGGED,
            OTHER_MOUSE_DRAGGED,
        ] {
            assert_eq!(pointer(kind, 0), Some(Pointer::Moved));
        }
        assert_eq!(pointer(MOUSE_ENTERED, 0), Some(Pointer::Entered));
        assert_eq!(pointer(MOUSE_EXITED, 0), Some(Pointer::Exited));
        assert_eq!(pointer(10, 0), None, "a key event");
    }

    #[test]
    fn other_buttons_by_number() {
        let button = |number| match pointer(OTHER_MOUSE_DOWN, number) {
            Some(Pointer::Button { button, .. }) => button,
            other => panic!("{other:?}"),
        };
        assert_eq!(button(2), MouseButton::Middle);
        assert_eq!(button(3), MouseButton::Back);
        assert_eq!(button(4), MouseButton::Forward);
        assert_eq!(button(7), MouseButton::Other(8));
        assert_eq!(button(i64::MAX), MouseButton::Other(u16::MAX));
        assert!(matches!(
            pointer(OTHER_MOUSE_UP, 2),
            Some(Pointer::Button {
                state: ButtonState::Released,
                ..
            })
        ));
    }

    #[test]
    fn control_and_option_clicks_emulate_buttons() {
        assert_eq!(emulated_button(Modifiers::NONE), MouseButton::Left);
        assert_eq!(emulated_button(Modifiers::CONTROL), MouseButton::Right);
        assert_eq!(emulated_button(Modifiers::ALT), MouseButton::Middle);
        assert_eq!(emulated_button(Modifiers::SHIFT), MouseButton::Left);
    }

    #[test]
    fn positions_and_scrolls_are_scaled() {
        assert_eq!(position(10.5, 20.0, 2.0), PhysicalPosition::new(21.0, 40.0));
        assert_eq!(
            scroll(1.0, -3.0, false, 2.0),
            Some(ScrollDelta::Lines { x: -1.0, y: -3.0 })
        );
        assert_eq!(
            scroll(0.0, 4.5, true, 2.0),
            Some(ScrollDelta::Pixels { x: -0.0, y: 9.0 })
        );
        assert_eq!(scroll(0.0, 0.0, true, 2.0), None);
    }
}
