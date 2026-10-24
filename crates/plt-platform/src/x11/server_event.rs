//! Decoding of the 32-byte core events this crate selects.

use super::connection::RawEvent;
use super::wire::{i16_at, u16_at, u32_at};
use crate::event::ButtonState;

const KEY_PRESS: u8 = 2;
const KEY_RELEASE: u8 = 3;
const BUTTON_PRESS: u8 = 4;
const BUTTON_RELEASE: u8 = 5;
const MOTION_NOTIFY: u8 = 6;
const ENTER_NOTIFY: u8 = 7;
const LEAVE_NOTIFY: u8 = 8;
const FOCUS_IN: u8 = 9;
const FOCUS_OUT: u8 = 10;
const EXPOSE: u8 = 12;
const CONFIGURE_NOTIFY: u8 = 22;
const PROPERTY_NOTIFY: u8 = 28;
const SELECTION_CLEAR: u8 = 29;
const SELECTION_REQUEST: u8 = 30;
const SELECTION_NOTIFY: u8 = 31;
const CLIENT_MESSAGE: u8 = 33;
const MAPPING_NOTIFY: u8 = 34;

/// `FocusIn`/`FocusOut` detail of focus moving with the pointer, which
/// does not give the window keyboard input.
const NOTIFY_POINTER: u8 = 5;
/// Focus mode of changes caused by keyboard grabs (e.g. a window
/// manager's Alt+Tab), which are transient.
const NOTIFY_GRAB: u8 = 1;
/// Focus mode of the end of a keyboard grab.
const NOTIFY_UNGRAB: u8 = 2;

/// What another client asked for, from a `SelectionRequest` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectionRequest {
    /// The time of the request.
    pub(crate) time: u32,
    /// The client window asking.
    pub(crate) requestor: u32,
    /// The selection asked for.
    pub(crate) selection: u32,
    /// The form asked for (an atom such as `image/png`).
    pub(crate) target: u32,
    /// Where to put it on the requestor (0 from obsolete clients).
    pub(crate) property: u32,
}

/// The answer to a request for a selection, from a `SelectionNotify`
/// event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectionNotify {
    /// The window that asked.
    pub(crate) requestor: u32,
    /// The selection asked for.
    pub(crate) selection: u32,
    /// The form asked for.
    pub(crate) target: u32,
    /// The property of the requestor holding the selection, 0 when the
    /// owner refused (or there is no owner).
    pub(crate) property: u32,
}

/// Where and when a pointer or key event happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PointerInfo {
    /// The window the event is reported to.
    pub(crate) window: u32,
    /// Server time in milliseconds.
    pub(crate) time: u32,
    /// Pointer x relative to the window.
    pub(crate) x: i16,
    /// Pointer y relative to the window.
    pub(crate) y: i16,
    /// Modifier and button state before the event (`SETofKEYBUTMASK`).
    pub(crate) mask: u16,
}

impl PointerInfo {
    fn parse(raw: &RawEvent) -> Self {
        Self {
            window: u32_at(raw, 12),
            time: u32_at(raw, 4),
            x: i16_at(raw, 24),
            y: i16_at(raw, 26),
            mask: u16_at(raw, 28),
        }
    }
}

/// A decoded server event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServerEvent {
    /// `KeyPress` / `KeyRelease`.
    Key {
        /// Pressed or released.
        state: ButtonState,
        /// The hardware keycode.
        keycode: u8,
        /// Window, time, position and modifiers.
        pointer: PointerInfo,
    },
    /// `ButtonPress` / `ButtonRelease`.
    Button {
        /// Pressed or released.
        state: ButtonState,
        /// The button number (1 left, 2 middle, 3 right, 4-7 wheel, ...).
        button: u8,
        /// Window, time, position and modifiers.
        pointer: PointerInfo,
    },
    /// `MotionNotify`.
    Motion(PointerInfo),
    /// `EnterNotify` / `LeaveNotify`.
    Crossing {
        /// Whether the pointer entered (or left) the window.
        entered: bool,
        /// Window, time, position and modifiers.
        pointer: PointerInfo,
    },
    /// A real keyboard focus change (pointer-tracking and grab-related
    /// changes are dropped while decoding).
    Focus {
        /// The window.
        window: u32,
        /// Whether it gained (or lost) focus.
        focused: bool,
    },
    /// `Expose`: part of the window must be repainted.
    Expose {
        /// The window.
        window: u32,
    },
    /// `ConfigureNotify`: the window's geometry changed.
    Configure {
        /// The window.
        window: u32,
        /// New width.
        width: u16,
        /// New height.
        height: u16,
    },
    /// `PropertyNotify`.
    Property {
        /// The window whose property changed.
        window: u32,
        /// The property.
        atom: u32,
    },
    /// `ClientMessage` with 32-bit data.
    ClientMessage {
        /// The target window.
        window: u32,
        /// The message type atom.
        message_type: u32,
        /// The data.
        data: [u32; 5],
    },
    /// `SelectionRequest`: a client asks for a selection we own.
    SelectionRequest(SelectionRequest),
    /// `SelectionNotify`: the answer to a request for a selection.
    SelectionNotify(SelectionNotify),
    /// `SelectionClear`: another client took a selection we owned.
    SelectionClear {
        /// The selection.
        selection: u32,
    },
    /// `MappingNotify` for the keyboard or modifier mapping.
    KeyboardMappingChanged,
    /// Anything this crate does not act on.
    Ignored,
}

impl ServerEvent {
    /// Decodes a raw event. The high bit of the code (set for events sent
    /// with `SendEvent`) is ignored.
    pub(crate) fn parse(raw: &RawEvent) -> Self {
        match raw[0] & 0x7f {
            code @ (KEY_PRESS | KEY_RELEASE) => Self::Key {
                state: press_state(code == KEY_PRESS),
                keycode: raw[1],
                pointer: PointerInfo::parse(raw),
            },
            code @ (BUTTON_PRESS | BUTTON_RELEASE) => Self::Button {
                state: press_state(code == BUTTON_PRESS),
                button: raw[1],
                pointer: PointerInfo::parse(raw),
            },
            MOTION_NOTIFY => Self::Motion(PointerInfo::parse(raw)),
            code @ (ENTER_NOTIFY | LEAVE_NOTIFY) => Self::Crossing {
                entered: code == ENTER_NOTIFY,
                pointer: PointerInfo::parse(raw),
            },
            code @ (FOCUS_IN | FOCUS_OUT) => {
                let (detail, mode) = (raw[1], raw[8]);
                if detail == NOTIFY_POINTER || matches!(mode, NOTIFY_GRAB | NOTIFY_UNGRAB) {
                    Self::Ignored
                } else {
                    Self::Focus {
                        window: u32_at(raw, 4),
                        focused: code == FOCUS_IN,
                    }
                }
            }
            EXPOSE => Self::Expose {
                window: u32_at(raw, 4),
            },
            CONFIGURE_NOTIFY => Self::Configure {
                window: u32_at(raw, 8),
                width: u16_at(raw, 20),
                height: u16_at(raw, 22),
            },
            PROPERTY_NOTIFY => Self::Property {
                window: u32_at(raw, 4),
                atom: u32_at(raw, 8),
            },
            CLIENT_MESSAGE if raw[1] == 32 => Self::ClientMessage {
                window: u32_at(raw, 4),
                message_type: u32_at(raw, 8),
                data: [12, 16, 20, 24, 28].map(|at| u32_at(raw, at)),
            },
            SELECTION_REQUEST => Self::SelectionRequest(SelectionRequest {
                time: u32_at(raw, 4),
                requestor: u32_at(raw, 12),
                selection: u32_at(raw, 16),
                target: u32_at(raw, 20),
                property: u32_at(raw, 24),
            }),
            SELECTION_NOTIFY => Self::SelectionNotify(SelectionNotify {
                requestor: u32_at(raw, 8),
                selection: u32_at(raw, 12),
                target: u32_at(raw, 16),
                property: u32_at(raw, 20),
            }),
            SELECTION_CLEAR => Self::SelectionClear {
                selection: u32_at(raw, 12),
            },
            MAPPING_NOTIFY if matches!(raw[4], 0 | 1) => Self::KeyboardMappingChanged,
            _ => Self::Ignored,
        }
    }
}

const fn press_state(pressed: bool) -> ButtonState {
    if pressed {
        ButtonState::Pressed
    } else {
        ButtonState::Released
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(bytes: &[(usize, &[u8])]) -> RawEvent {
        let mut event = [0; 32];
        for (at, data) in bytes {
            event[*at..*at + data.len()].copy_from_slice(data);
        }
        event
    }

    #[test]
    fn key_and_button_events() {
        let event = raw(&[
            (0, &[KEY_PRESS, 38]),
            (4, &1000_u32.to_le_bytes()),
            (12, &0x0040_0001_u32.to_le_bytes()),
            (24, &[10, 0, 0xfb, 0xff]),
            (28, &[0x41, 0]),
        ]);
        let pointer = PointerInfo {
            window: 0x0040_0001,
            time: 1000,
            x: 10,
            y: -5,
            mask: 0x41,
        };
        assert_eq!(
            ServerEvent::parse(&event),
            ServerEvent::Key {
                state: ButtonState::Pressed,
                keycode: 38,
                pointer
            }
        );
        let mut synthetic = event;
        synthetic[0] = BUTTON_RELEASE | 0x80;
        synthetic[1] = 3;
        assert_eq!(
            ServerEvent::parse(&synthetic),
            ServerEvent::Button {
                state: ButtonState::Released,
                button: 3,
                pointer
            }
        );
    }

    #[test]
    fn focus_changes_from_grabs_and_pointer_are_ignored() {
        let focus =
            |code, detail, mode| raw(&[(0, &[code, detail]), (4, &[7, 0, 0, 0]), (8, &[mode])]);
        assert_eq!(
            ServerEvent::parse(&focus(FOCUS_IN, 3, 0)),
            ServerEvent::Focus {
                window: 7,
                focused: true
            }
        );
        assert_eq!(
            ServerEvent::parse(&focus(FOCUS_OUT, NOTIFY_POINTER, 0)),
            ServerEvent::Ignored
        );
        assert_eq!(
            ServerEvent::parse(&focus(FOCUS_OUT, 3, NOTIFY_GRAB)),
            ServerEvent::Ignored
        );
    }

    #[test]
    fn structure_and_protocol_events() {
        let configure = raw(&[
            (0, &[CONFIGURE_NOTIFY]),
            (8, &[9, 0, 0, 0]),
            (20, &[0x20, 3, 0x58, 2]),
        ]);
        assert_eq!(
            ServerEvent::parse(&configure),
            ServerEvent::Configure {
                window: 9,
                width: 800,
                height: 600
            }
        );
        let message = raw(&[
            (0, &[CLIENT_MESSAGE, 32]),
            (4, &[9, 0, 0, 0]),
            (8, &[5, 1, 0, 0]),
            (12, &[42]),
        ]);
        assert_eq!(
            ServerEvent::parse(&message),
            ServerEvent::ClientMessage {
                window: 9,
                message_type: 0x105,
                data: [42, 0, 0, 0, 0]
            }
        );
        let bytes_message = raw(&[(0, &[CLIENT_MESSAGE, 8])]);
        assert_eq!(ServerEvent::parse(&bytes_message), ServerEvent::Ignored);
        let keyboard = raw(&[(0, &[MAPPING_NOTIFY]), (4, &[1, 8, 248])]);
        assert_eq!(
            ServerEvent::parse(&keyboard),
            ServerEvent::KeyboardMappingChanged
        );
        let pointer = raw(&[(0, &[MAPPING_NOTIFY]), (4, &[2])]);
        assert_eq!(ServerEvent::parse(&pointer), ServerEvent::Ignored);
        let expose = raw(&[(0, &[EXPOSE]), (4, &[3, 0, 0, 0])]);
        assert_eq!(
            ServerEvent::parse(&expose),
            ServerEvent::Expose { window: 3 }
        );
        let property = raw(&[(0, &[PROPERTY_NOTIFY]), (4, &[1, 2, 0, 0]), (8, &[23])]);
        assert_eq!(
            ServerEvent::parse(&property),
            ServerEvent::Property {
                window: 0x201,
                atom: 23
            }
        );
    }

    #[test]
    fn selection_answers_including_sent_ones() {
        let notify = raw(&[
            (0, &[SELECTION_NOTIFY | 0x80]),
            (8, &[1, 0, 0, 0]),
            (12, &[2, 0, 0, 0]),
            (16, &[3, 0, 0, 0]),
            (20, &[4, 0, 0, 0]),
        ]);
        assert_eq!(
            ServerEvent::parse(&notify),
            ServerEvent::SelectionNotify(SelectionNotify {
                requestor: 1,
                selection: 2,
                target: 3,
                property: 4,
            })
        );
    }
}
