//! Events delivered by the event loop.

use crate::event_loop::TimerId;
use crate::geometry::{PhysicalPosition, PhysicalSize};
use crate::keyboard::{Key, Modifiers};
use crate::window::WindowId;

/// An event dispatched to the handler passed to
/// [`EventLoop::run`](crate::EventLoop::run) or
/// [`EventLoop::pump_events`](crate::EventLoop::pump_events).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// Delivered exactly once, before any other event, the first time the
    /// loop runs. Creating the initial windows here is the most portable
    /// choice.
    Started,
    /// Something happened to one of this loop's windows.
    Window {
        /// The affected window.
        window: WindowId,
        /// What happened.
        event: WindowEvent,
    },
    /// A timer started with
    /// [`Context::set_timeout`](crate::Context::set_timeout) or
    /// [`Context::set_interval`](crate::Context::set_interval) fired.
    Timer(TimerId),
    /// [`EventLoopProxy::wake_up`](crate::EventLoopProxy::wake_up) was
    /// called (several calls may be coalesced into one event).
    WakeUp,
    /// Every pending event of this loop iteration has been dispatched; the
    /// loop is about to wait for new input. Ideal for idle work such as
    /// coalesced redraws.
    AboutToWait,
}

/// An event concerning one window.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WindowEvent {
    /// The window's content must be (re)painted and presented. Delivered
    /// after a window becomes visible, after it was resized or its scale
    /// factor changed, when the platform lost its contents, and after
    /// [`Context::request_redraw`](crate::Context::request_redraw).
    /// Coalesced: at most one per window per loop iteration, after input
    /// events.
    RedrawRequested,
    /// The content area changed size (in physical pixels).
    Resized(PhysicalSize),
    /// The window's scale factor (physical / logical pixels) changed.
    ScaleFactorChanged(f64),
    /// The user asked to close the window (e.g. the title-bar close
    /// button). The window stays open until
    /// [`Context::close_window`](crate::Context::close_window) is called.
    CloseRequested,
    /// The window was closed and its id is no longer valid. This is the
    /// last event delivered for it.
    Destroyed,
    /// The window gained (`true`) or lost (`false`) keyboard focus.
    Focused(bool),
    /// The pointer entered the content area.
    CursorEntered {
        /// Pointer position.
        position: PhysicalPosition,
    },
    /// The pointer left the content area.
    CursorLeft,
    /// The pointer moved inside the content area (or anywhere while a
    /// button is held). Consecutive motions may be coalesced.
    CursorMoved {
        /// Pointer position.
        position: PhysicalPosition,
        /// Modifiers held during the motion.
        modifiers: Modifiers,
    },
    /// A mouse button was pressed or released.
    MouseInput {
        /// Which button.
        button: MouseButton,
        /// Pressed or released.
        state: ButtonState,
        /// Pointer position.
        position: PhysicalPosition,
        /// Modifiers held at the time.
        modifiers: Modifiers,
        /// 1 for a single click, 2 for a double click, 3 for a triple
        /// click, ... (always 1 on release events).
        click_count: u32,
    },
    /// The mouse wheel or touchpad scrolled.
    MouseWheel {
        /// Scroll amount.
        delta: ScrollDelta,
        /// Pointer position.
        position: PhysicalPosition,
        /// Modifiers held at the time.
        modifiers: Modifiers,
    },
    /// A key was pressed or released.
    Key(KeyEvent),
    /// The text on the clipboard, which
    /// [`Context::request_clipboard_text`](crate::Context::request_clipboard_text)
    /// asked for: `None` when the clipboard is empty or holds no text.
    ClipboardText(Option<String>),
}

/// Whether a button or key went down or up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonState {
    /// The button/key went down.
    Pressed,
    /// The button/key went up.
    Released,
}

impl ButtonState {
    /// Whether this is [`ButtonState::Pressed`].
    pub const fn is_pressed(self) -> bool {
        matches!(self, Self::Pressed)
    }
}

/// A mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Primary (usually left) button; matplotlib button 1.
    Left,
    /// Middle button / wheel click; matplotlib button 2.
    Middle,
    /// Secondary (usually right) button; matplotlib button 3.
    Right,
    /// "Back" side button; matplotlib button 8.
    Back,
    /// "Forward" side button; matplotlib button 9.
    Forward,
    /// Any other button, by platform number.
    Other(u16),
}

impl MouseButton {
    /// The matplotlib `MouseButton` number (1 left, 2 middle, 3 right,
    /// 8 back, 9 forward).
    pub const fn number(self) -> u16 {
        match self {
            Self::Left => 1,
            Self::Middle => 2,
            Self::Right => 3,
            Self::Back => 8,
            Self::Forward => 9,
            Self::Other(n) => n,
        }
    }
}

/// A scroll amount. Positive `y` scrolls up (away from the user), positive
/// `x` scrolls right; matplotlib maps `y > 0` to `button='up'`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollDelta {
    /// Discrete wheel notches (or "lines").
    Lines {
        /// Horizontal notches.
        x: f64,
        /// Vertical notches.
        y: f64,
    },
    /// Precise scrolling in physical pixels (touchpads).
    Pixels {
        /// Horizontal pixels.
        x: f64,
        /// Vertical pixels.
        y: f64,
    },
}

/// A keyboard event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    /// The logical key, with the layout and Shift/Caps Lock applied.
    pub key: Key,
    /// Pressed or released.
    pub state: ButtonState,
    /// Whether this press was generated by key auto-repeat.
    pub repeat: bool,
    /// Text the key press inserts, if any. `None` for releases,
    /// non-printable keys and presses with Control, Alt or Super held.
    pub text: Option<String>,
    /// Modifiers held *before* this event (pressing Shift reports
    /// `Key::Shift` with Shift not yet in the set, as matplotlib expects).
    pub modifiers: Modifiers,
    /// The platform's raw scan/key code: the keycode on X11 and Wayland,
    /// the hardware key code (`kVK_*`) on macOS, the scan code on Windows
    /// (`0xE0xx` for extended keys).
    pub scancode: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_button_numbers_match_matplotlib() {
        assert_eq!(MouseButton::Left.number(), 1);
        assert_eq!(MouseButton::Middle.number(), 2);
        assert_eq!(MouseButton::Right.number(), 3);
        assert_eq!(MouseButton::Back.number(), 8);
        assert_eq!(MouseButton::Forward.number(), 9);
        assert_eq!(MouseButton::Other(12).number(), 12);
    }

    #[test]
    fn button_state_predicate() {
        assert!(ButtonState::Pressed.is_pressed());
        assert!(!ButtonState::Released.is_pressed());
    }
}
