//! Window messages decoded from their raw `(message, wParam, lParam)`
//! form into what the window procedure acts on.

use super::keys::KeyMessage;
use crate::event::{ButtonState, MouseButton, ScrollDelta};
use crate::geometry::{PhysicalPosition, PhysicalSize};

/// Window message numbers (`WM_*`).
pub(crate) mod wm {
    pub(crate) const SIZE: u32 = 0x0005;
    pub(crate) const SETFOCUS: u32 = 0x0007;
    pub(crate) const KILLFOCUS: u32 = 0x0008;
    pub(crate) const PAINT: u32 = 0x000F;
    pub(crate) const CLOSE: u32 = 0x0010;
    pub(crate) const ERASEBKGND: u32 = 0x0014;
    pub(crate) const SETCURSOR: u32 = 0x0020;
    pub(crate) const GETMINMAXINFO: u32 = 0x0024;
    pub(crate) const KEYDOWN: u32 = 0x0100;
    pub(crate) const KEYUP: u32 = 0x0101;
    pub(crate) const CHAR: u32 = 0x0102;
    pub(crate) const DEADCHAR: u32 = 0x0103;
    pub(crate) const SYSKEYDOWN: u32 = 0x0104;
    pub(crate) const SYSKEYUP: u32 = 0x0105;
    pub(crate) const SYSCHAR: u32 = 0x0106;
    pub(crate) const MOUSEMOVE: u32 = 0x0200;
    pub(crate) const LBUTTONDOWN: u32 = 0x0201;
    pub(crate) const LBUTTONUP: u32 = 0x0202;
    pub(crate) const RBUTTONDOWN: u32 = 0x0204;
    pub(crate) const RBUTTONUP: u32 = 0x0205;
    pub(crate) const MBUTTONDOWN: u32 = 0x0207;
    pub(crate) const MBUTTONUP: u32 = 0x0208;
    pub(crate) const MOUSEWHEEL: u32 = 0x020A;
    pub(crate) const XBUTTONDOWN: u32 = 0x020B;
    pub(crate) const XBUTTONUP: u32 = 0x020C;
    pub(crate) const MOUSEHWHEEL: u32 = 0x020E;
    pub(crate) const MOUSELEAVE: u32 = 0x02A3;
    pub(crate) const DPICHANGED: u32 = 0x02E0;
    /// The private message that wakes the event loop (`WM_APP`).
    pub(crate) const WAKE: u32 = 0x8000;
}

/// `SIZE_MINIMIZED`: the `WM_SIZE` of a window being minimized.
const SIZE_MINIMIZED: usize = 1;
/// `HTCLIENT`: the client area, in `WM_SETCURSOR`'s hit test.
const HTCLIENT: isize = 1;
/// `WHEEL_DELTA`: the wheel rotation of one notch.
const WHEEL_DELTA: f64 = 120.0;
/// `MK_LBUTTON | MK_RBUTTON | MK_MBUTTON | MK_XBUTTON1 | MK_XBUTTON2`:
/// the buttons held, in the `wParam` of mouse messages.
const ANY_BUTTON: usize = 0x0001 | 0x0002 | 0x0010 | 0x0020 | 0x0040;
/// `XBUTTON1`, in the high word of an X button message's `wParam`.
const XBUTTON1: u16 = 1;

/// A rectangle in screen coordinates (`RECT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ScreenRect {
    /// Left edge.
    pub(crate) left: i32,
    /// Top edge.
    pub(crate) top: i32,
    /// Right edge (exclusive).
    pub(crate) right: i32,
    /// Bottom edge (exclusive).
    pub(crate) bottom: i32,
}

impl ScreenRect {
    /// The rectangle's width.
    pub(crate) const fn width(self) -> i32 {
        self.right - self.left
    }

    /// The rectangle's height.
    pub(crate) const fn height(self) -> i32 {
        self.bottom - self.top
    }
}

/// A message to a window, as the window procedure handles it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Message {
    /// `WM_PAINT`: repaint the invalid part of the client area.
    Paint,
    /// `WM_ERASEBKGND`: painting covers the whole client area, so the
    /// background is never erased.
    EraseBackground,
    /// `WM_SIZE`: the client area's new size, `None` while minimized.
    Resized(Option<PhysicalSize>),
    /// `WM_DPICHANGED`: the window moved to a monitor of another DPI; the
    /// system suggests a new window rectangle, of the same logical size.
    DpiChanged {
        /// The new DPI.
        dpi: u32,
        /// The suggested window rectangle.
        suggested: ScreenRect,
    },
    /// `WM_GETMINMAXINFO`: the system asks for the size limits.
    MinMaxInfo,
    /// `WM_CLOSE`: the user asked to close the window.
    Close,
    /// `WM_SETFOCUS` (`true`) or `WM_KILLFOCUS` (`false`).
    Focus(bool),
    /// `WM_SETCURSOR`: the pointer moved; `client` when it is over the
    /// client area, where the window sets the cursor.
    SetCursor {
        /// Whether the pointer is over the client area.
        client: bool,
    },
    /// `WM_MOUSEMOVE`.
    MouseMove(PhysicalPosition),
    /// `WM_MOUSELEAVE`: the pointer left the client area.
    MouseLeave,
    /// A mouse button went down or up.
    Button {
        /// Which button.
        button: MouseButton,
        /// Down or up.
        state: ButtonState,
        /// The pointer position.
        position: PhysicalPosition,
        /// Whether any button is held after this message.
        any_held: bool,
        /// The message time, in milliseconds.
        time: u32,
    },
    /// `WM_MOUSEWHEEL` or `WM_MOUSEHWHEEL`.
    Wheel {
        /// Notches turned (fractions of one for precise touchpads).
        delta: ScrollDelta,
        /// The pointer position, in screen coordinates.
        screen: (i32, i32),
    },
    /// A key went down or up; `system` for `WM_SYSKEYDOWN` and
    /// `WM_SYSKEYUP` (Alt held, or F10).
    Key {
        /// The key.
        key: KeyMessage,
        /// Whether this is a system key message.
        system: bool,
    },
    /// `WM_SYSCHAR`, with the character code.
    SystemChar(usize),
    /// `WM_APP`: wakes the event loop.
    Wake,
    /// Anything else, left to `DefWindowProcW`.
    Other,
}

impl Message {
    /// Decodes a message that carries no pointer (all but
    /// `WM_DPICHANGED` and `WM_GETMINMAXINFO`, whose pointers the caller
    /// reads; `WM_GETMINMAXINFO` decodes as [`Message::MinMaxInfo`]).
    /// `time` is the message time, from `GetMessageTime`.
    pub(crate) fn decode(message: u32, wparam: usize, lparam: isize, time: u32) -> Self {
        let position = || {
            let (x, y) = point(lparam);
            PhysicalPosition::new(f64::from(x), f64::from(y))
        };
        let button = |button, state| Self::Button {
            button,
            state,
            position: position(),
            any_held: wparam & ANY_BUTTON != 0,
            time,
        };
        match message {
            wm::PAINT => Self::Paint,
            wm::ERASEBKGND => Self::EraseBackground,
            wm::SIZE => Self::Resized((wparam != SIZE_MINIMIZED).then(|| {
                let (width, height) = words(lparam as usize);
                PhysicalSize::new(width.into(), height.into())
            })),
            wm::GETMINMAXINFO => Self::MinMaxInfo,
            wm::CLOSE => Self::Close,
            wm::SETFOCUS => Self::Focus(true),
            wm::KILLFOCUS => Self::Focus(false),
            wm::SETCURSOR => Self::SetCursor {
                client: lparam & 0xFFFF == HTCLIENT,
            },
            wm::MOUSEMOVE => Self::MouseMove(position()),
            wm::MOUSELEAVE => Self::MouseLeave,
            wm::LBUTTONDOWN => button(MouseButton::Left, ButtonState::Pressed),
            wm::LBUTTONUP => button(MouseButton::Left, ButtonState::Released),
            wm::RBUTTONDOWN => button(MouseButton::Right, ButtonState::Pressed),
            wm::RBUTTONUP => button(MouseButton::Right, ButtonState::Released),
            wm::MBUTTONDOWN => button(MouseButton::Middle, ButtonState::Pressed),
            wm::MBUTTONUP => button(MouseButton::Middle, ButtonState::Released),
            wm::XBUTTONDOWN | wm::XBUTTONUP => {
                let side = if words(wparam).1 == XBUTTON1 {
                    MouseButton::Back
                } else {
                    MouseButton::Forward
                };
                let state = if message == wm::XBUTTONDOWN {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                };
                button(side, state)
            }
            wm::MOUSEWHEEL | wm::MOUSEHWHEEL => {
                // Positive turns are up (away from the user) for the
                // vertical wheel, right for the horizontal one.
                let notches = f64::from(words(wparam).1 as i16) / WHEEL_DELTA;
                let delta = if message == wm::MOUSEHWHEEL {
                    ScrollDelta::Lines { x: notches, y: 0.0 }
                } else {
                    ScrollDelta::Lines { x: 0.0, y: notches }
                };
                Self::Wheel {
                    delta,
                    screen: point(lparam),
                }
            }
            wm::KEYDOWN | wm::KEYUP | wm::SYSKEYDOWN | wm::SYSKEYUP => Self::Key {
                key: key_message(message, wparam, lparam, time),
                system: matches!(message, wm::SYSKEYDOWN | wm::SYSKEYUP),
            },
            wm::SYSCHAR => Self::SystemChar(wparam),
            wm::WAKE => Self::Wake,
            _ => Self::Other,
        }
    }

    /// Decodes `WM_DPICHANGED`, given the rectangle its `lParam` points
    /// to.
    pub(crate) const fn dpi_changed(wparam: usize, suggested: ScreenRect) -> Self {
        Self::DpiChanged {
            dpi: words(wparam).0 as u32,
            suggested,
        }
    }
}

/// The key message of `WM_KEYDOWN`, `WM_KEYUP`, `WM_SYSKEYDOWN` or
/// `WM_SYSKEYUP` with these parameters.
pub(crate) fn key_message(message: u32, wparam: usize, lparam: isize, time: u32) -> KeyMessage {
    let flags = lparam as usize;
    let state = if matches!(message, wm::KEYDOWN | wm::SYSKEYDOWN) {
        ButtonState::Pressed
    } else {
        ButtonState::Released
    };
    KeyMessage {
        vk: (wparam & 0xFFFF) as u16,
        scancode: ((flags >> 16) & 0xFF) as u16,
        extended: flags & (1 << 24) != 0,
        state,
        repeat: state.is_pressed() && flags & (1 << 30) != 0,
        time,
    }
}

/// Whether `message` is a key press or release (as opposed to the
/// character messages between them).
pub(crate) const fn is_key(message: u32) -> bool {
    matches!(
        message,
        wm::KEYDOWN | wm::KEYUP | wm::SYSKEYDOWN | wm::SYSKEYUP
    )
}

/// The low and high words of a parameter.
const fn words(value: usize) -> (u16, u16) {
    (value as u16, (value >> 16) as u16)
}

/// The signed coordinates packed into an `lParam` (`GET_X_LPARAM`,
/// `GET_Y_LPARAM`).
const fn point(lparam: isize) -> (i32, i32) {
    let (x, y) = words(lparam as usize);
    (x as i16 as i32, y as i16 as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(message: u32, wparam: usize, lparam: isize) -> Message {
        Message::decode(message, wparam, lparam, 0)
    }

    /// Packs two coordinates as `MAKELPARAM` does.
    fn make_lparam(x: i16, y: i16) -> isize {
        (x as u16 as isize) | ((y as u16 as isize) << 16)
    }

    #[test]
    fn sizes_ignore_minimizing() {
        let size = (480 << 16) | 0x280;
        assert_eq!(
            decode(wm::SIZE, 0, size),
            Message::Resized(Some(PhysicalSize::new(640, 480)))
        );
        assert_eq!(decode(wm::SIZE, SIZE_MINIMIZED, 0), Message::Resized(None));
    }

    #[test]
    fn mouse_positions_are_signed() {
        assert_eq!(
            decode(wm::MOUSEMOVE, 0, make_lparam(-5, 300)),
            Message::MouseMove(PhysicalPosition::new(-5.0, 300.0))
        );
    }

    #[test]
    fn buttons_report_what_stays_held() {
        assert_eq!(
            Message::decode(wm::RBUTTONDOWN, 0x0002, make_lparam(1, 2), 99),
            Message::Button {
                button: MouseButton::Right,
                state: ButtonState::Pressed,
                position: PhysicalPosition::new(1.0, 2.0),
                any_held: true,
                time: 99,
            }
        );
        let Message::Button {
            button, any_held, ..
        } = decode(wm::LBUTTONUP, 0, 0)
        else {
            panic!("a button message");
        };
        assert_eq!((button, any_held), (MouseButton::Left, false));
        let side = |wparam| match decode(wm::XBUTTONDOWN, wparam, 0) {
            Message::Button { button, .. } => button,
            other => panic!("{other:?}"),
        };
        assert_eq!(side(1 << 16), MouseButton::Back);
        assert_eq!(side(2 << 16), MouseButton::Forward);
        assert!(matches!(
            decode(wm::MBUTTONUP, 0, 0),
            Message::Button {
                button: MouseButton::Middle,
                state: ButtonState::Released,
                ..
            }
        ));
    }

    #[test]
    fn wheels_count_notches() {
        let down = (-240_i16 as u16 as usize) << 16;
        assert_eq!(
            decode(wm::MOUSEWHEEL, down, make_lparam(100, -20)),
            Message::Wheel {
                delta: ScrollDelta::Lines { x: 0.0, y: -2.0 },
                screen: (100, -20),
            }
        );
        let right = 60 << 16;
        assert_eq!(
            decode(wm::MOUSEHWHEEL, right, 0),
            Message::Wheel {
                delta: ScrollDelta::Lines { x: 0.5, y: 0.0 },
                screen: (0, 0),
            }
        );
    }

    #[test]
    fn key_messages_carry_scan_codes_and_repeats() {
        // Right Control, extended, held down already.
        let lparam = (1 << 30) | (1 << 24) | (0x1D << 16) | 1;
        let Message::Key { key, system } = Message::decode(wm::KEYDOWN, 0x11, lparam, 77) else {
            panic!("a key message");
        };
        assert!(!system);
        assert_eq!(key.vk, 0x11);
        assert_eq!(key.scancode, 0x1D);
        assert!(key.extended && key.repeat);
        assert_eq!(key.state, ButtonState::Pressed);
        assert_eq!(key.time, 77);
        // Releases are never repeats (bit 30 is always set on them).
        let release = key_message(wm::SYSKEYUP, 0x12, (1 << 31) | (1 << 30), 0);
        assert_eq!(release.state, ButtonState::Released);
        assert!(!release.repeat);
        assert!(matches!(
            decode(wm::SYSKEYDOWN, 0x73, 0),
            Message::Key { system: true, .. }
        ));
        assert!(is_key(wm::KEYUP) && !is_key(wm::CHAR) && !is_key(wm::DEADCHAR));
    }

    #[test]
    fn other_messages() {
        assert_eq!(decode(wm::PAINT, 0, 0), Message::Paint);
        assert_eq!(decode(wm::ERASEBKGND, 0, 0), Message::EraseBackground);
        assert_eq!(decode(wm::GETMINMAXINFO, 0, 0), Message::MinMaxInfo);
        assert_eq!(decode(wm::CLOSE, 0, 0), Message::Close);
        assert_eq!(decode(wm::SETFOCUS, 0, 0), Message::Focus(true));
        assert_eq!(decode(wm::KILLFOCUS, 0, 0), Message::Focus(false));
        assert_eq!(decode(wm::MOUSELEAVE, 0, 0), Message::MouseLeave);
        assert_eq!(
            decode(wm::SETCURSOR, 0, (0x0200 << 16) | 1),
            Message::SetCursor { client: true }
        );
        assert_eq!(
            decode(wm::SETCURSOR, 0, 2),
            Message::SetCursor { client: false }
        );
        assert_eq!(
            decode(wm::SYSCHAR, usize::from(b' '), 0),
            Message::SystemChar(32)
        );
        assert_eq!(decode(wm::WAKE, 0, 0), Message::Wake);
        assert_eq!(decode(wm::DPICHANGED, 0, 0), Message::Other);
        assert_eq!(decode(0x0001, 0, 0), Message::Other);
        let rect = ScreenRect {
            left: -10,
            top: 5,
            right: 90,
            bottom: 25,
        };
        assert_eq!((rect.width(), rect.height()), (100, 20));
        assert_eq!(
            Message::dpi_changed(0x0090_0090, rect),
            Message::DpiChanged {
                dpi: 144,
                suggested: rect
            }
        );
    }
}
