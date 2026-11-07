//! Input events as widgets receive them.

use pltrs_core::Point;
use pltrs_platform::{KeyEvent, Modifiers, MouseButton, ScrollDelta};

/// An input event routed to one widget, with positions in window pixels.
///
/// Pointer events go to the widget under the pointer, except that a
/// widget pressed with a button receives every pointer event until all
/// buttons are released (pointer capture). Wheel events the widget under
/// the pointer leaves unused go on to its ancestors (see
/// [`EventCx::set_handled`](crate::EventCx::set_handled)). Key events go
/// to the focused widget: the last clicked widget that accepts focus.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum UiEvent {
    /// The pointer moved onto the widget.
    PointerEnter(Point),
    /// The pointer moved off the widget (or left the window), to the
    /// position given where it is known (outside the widget; the platform
    /// may not tell where the pointer went when it left the window).
    PointerLeave(Option<Point>),
    /// The pointer moved.
    PointerMove {
        /// Pointer position.
        position: Point,
        /// Held modifiers.
        modifiers: Modifiers,
    },
    /// A mouse button was pressed.
    PointerDown {
        /// The button.
        button: MouseButton,
        /// Pointer position.
        position: Point,
        /// Held modifiers.
        modifiers: Modifiers,
        /// 1 for a single click, 2 for a double click, ...
        click_count: u32,
    },
    /// A mouse button was released.
    PointerUp {
        /// The button.
        button: MouseButton,
        /// Pointer position.
        position: Point,
        /// Held modifiers.
        modifiers: Modifiers,
    },
    /// The wheel or touchpad scrolled.
    Wheel {
        /// Scroll amount.
        delta: ScrollDelta,
        /// Pointer position.
        position: Point,
        /// Held modifiers.
        modifiers: Modifiers,
    },
    /// A key was pressed or released.
    Key(KeyEvent),
    /// The clipboard's text, which the widget asked for with
    /// [`EventCx::request_paste`](crate::EventCx::request_paste).
    Paste(String),
}

impl UiEvent {
    /// The pointer position, for pointer events.
    pub fn position(&self) -> Option<Point> {
        match self {
            Self::PointerEnter(position)
            | Self::PointerMove { position, .. }
            | Self::PointerDown { position, .. }
            | Self::PointerUp { position, .. }
            | Self::Wheel { position, .. } => Some(*position),
            Self::PointerLeave(position) => *position,
            Self::Key(_) | Self::Paste(_) => None,
        }
    }

    /// The same event with its position moved by `(-origin.x, -origin.y)`,
    /// i.e. relative to `origin`.
    #[must_use]
    pub fn relative_to(&self, origin: Point) -> Self {
        let shift = |p: &Point| Point::new(p.x - origin.x, p.y - origin.y);
        let mut event = self.clone();
        match &mut event {
            Self::PointerEnter(position)
            | Self::PointerMove { position, .. }
            | Self::PointerDown { position, .. }
            | Self::PointerUp { position, .. }
            | Self::Wheel { position, .. }
            | Self::PointerLeave(Some(position)) => *position = shift(position),
            Self::PointerLeave(None) | Self::Key(_) | Self::Paste(_) => {}
        }
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_and_relative_events() {
        let down = UiEvent::PointerDown {
            button: MouseButton::Left,
            position: Point::new(15.0, 25.0),
            modifiers: Modifiers::NONE,
            click_count: 1,
        };
        assert_eq!(down.position(), Some(Point::new(15.0, 25.0)));
        let local = down.relative_to(Point::new(10.0, 20.0));
        assert_eq!(local.position(), Some(Point::new(5.0, 5.0)));
        assert_eq!(UiEvent::PointerLeave(None).position(), None);
        assert_eq!(
            UiEvent::PointerLeave(None).relative_to(Point::new(1.0, 1.0)),
            UiEvent::PointerLeave(None)
        );
        let left = UiEvent::PointerLeave(Some(Point::new(-3.0, 4.0)));
        assert_eq!(
            left.relative_to(Point::new(1.0, 1.0)).position(),
            Some(Point::new(-4.0, 3.0))
        );
    }
}
