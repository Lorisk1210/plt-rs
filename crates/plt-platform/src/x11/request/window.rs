//! Requests that create, configure and destroy windows.

use super::{Request, request};
use crate::x11::wire::WireWrite;

const CREATE_WINDOW: u8 = 1;
const CHANGE_WINDOW_ATTRIBUTES: u8 = 2;
const DESTROY_WINDOW: u8 = 4;
const MAP_WINDOW: u8 = 8;
const CONFIGURE_WINDOW: u8 = 12;
const CREATE_COLORMAP: u8 = 78;

/// Window event masks (`SETofEVENT`).
pub(crate) mod event_mask {
    /// `KeyPress`.
    pub(crate) const KEY_PRESS: u32 = 1;
    /// `KeyRelease`.
    pub(crate) const KEY_RELEASE: u32 = 1 << 1;
    /// `ButtonPress`.
    pub(crate) const BUTTON_PRESS: u32 = 1 << 2;
    /// `ButtonRelease`.
    pub(crate) const BUTTON_RELEASE: u32 = 1 << 3;
    /// `EnterNotify`.
    pub(crate) const ENTER_WINDOW: u32 = 1 << 4;
    /// `LeaveNotify`.
    pub(crate) const LEAVE_WINDOW: u32 = 1 << 5;
    /// `MotionNotify`.
    pub(crate) const POINTER_MOTION: u32 = 1 << 6;
    /// `Expose`.
    pub(crate) const EXPOSURE: u32 = 1 << 15;
    /// `ConfigureNotify`, `MapNotify`, `DestroyNotify`, ...
    pub(crate) const STRUCTURE_NOTIFY: u32 = 1 << 17;
    /// Structure changes of children (used for messages to the root).
    pub(crate) const SUBSTRUCTURE_NOTIFY: u32 = 1 << 19;
    /// Redirected structure requests of children (window managers).
    pub(crate) const SUBSTRUCTURE_REDIRECT: u32 = 1 << 20;
    /// `FocusIn` and `FocusOut`.
    pub(crate) const FOCUS_CHANGE: u32 = 1 << 21;
    /// `PropertyNotify`.
    pub(crate) const PROPERTY_CHANGE: u32 = 1 << 22;
}

/// A window attribute set by `CreateWindow` / `ChangeWindowAttributes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowAttribute {
    /// No background: the server never clears exposed areas, which avoids
    /// flicker for clients that repaint everything anyway.
    BackgroundNone,
    /// Border colour (required when the visual differs from the parent's).
    BorderPixel(u32),
    /// Keep the contents at the top-left corner when resizing.
    BitGravityNorthWest,
    /// The events to receive.
    EventMask(u32),
    /// The colormap (required for non-default visuals).
    Colormap(u32),
    /// The cursor shown over the window (0: inherit the parent's).
    Cursor(u32),
}

impl WindowAttribute {
    /// The attribute's bit in the value mask and its value.
    const fn bit_and_value(self) -> (u32, u32) {
        match self {
            Self::BackgroundNone => (1, 0),
            Self::BorderPixel(pixel) => (1 << 3, pixel),
            Self::BitGravityNorthWest => (1 << 4, 1),
            Self::EventMask(mask) => (1 << 11, mask),
            Self::Colormap(colormap) => (1 << 13, colormap),
            Self::Cursor(cursor) => (1 << 14, cursor),
        }
    }
}

/// Appends a value mask and its values, sorted by bit as the protocol
/// requires.
fn put_values(buf: &mut Vec<u8>, attributes: &[WindowAttribute]) {
    let mut values: Vec<(u32, u32)> = attributes.iter().map(|a| a.bit_and_value()).collect();
    values.sort_unstable_by_key(|&(bit, _)| bit);
    values.dedup_by_key(|&mut (bit, _)| bit);
    buf.put_u32(values.iter().fold(0, |mask, &(bit, _)| mask | bit));
    for (_, value) in values {
        buf.put_u32(value);
    }
}

/// The class of a new window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowClass {
    /// A window that is drawn into.
    InputOutput,
    /// An invisible window that only receives events.
    InputOnly,
}

/// Geometry and appearance of a new window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NewWindow {
    /// The id to give the window.
    pub(crate) id: u32,
    /// Its parent (the root window for top-level windows).
    pub(crate) parent: u32,
    /// Window class.
    pub(crate) class: WindowClass,
    /// Depth (0: copy from parent).
    pub(crate) depth: u8,
    /// Visual id (0: copy from parent).
    pub(crate) visual: u32,
    /// Width in pixels.
    pub(crate) width: u16,
    /// Height in pixels.
    pub(crate) height: u16,
}

/// `CreateWindow` at position (0, 0) without a border.
pub(crate) fn create_window(window: &NewWindow, attributes: &[WindowAttribute]) -> Request {
    request(CREATE_WINDOW, window.depth, |b| {
        b.put_u32(window.id);
        b.put_u32(window.parent);
        b.put_i16(0);
        b.put_i16(0);
        b.put_u16(window.width);
        b.put_u16(window.height);
        b.put_u16(0);
        b.put_u16(match window.class {
            WindowClass::InputOutput => 1,
            WindowClass::InputOnly => 2,
        });
        b.put_u32(window.visual);
        put_values(b, attributes);
    })
}

/// `ChangeWindowAttributes`.
pub(crate) fn change_window_attributes(window: u32, attributes: &[WindowAttribute]) -> Request {
    request(CHANGE_WINDOW_ATTRIBUTES, 0, |b| {
        b.put_u32(window);
        put_values(b, attributes);
    })
}

/// `DestroyWindow`.
pub(crate) fn destroy_window(window: u32) -> Request {
    request(DESTROY_WINDOW, 0, |b| b.put_u32(window))
}

/// `MapWindow`.
pub(crate) fn map_window(window: u32) -> Request {
    request(MAP_WINDOW, 0, |b| b.put_u32(window))
}

/// `ConfigureWindow` changing the size.
pub(crate) fn resize_window(window: u32, width: u16, height: u16) -> Request {
    const WIDTH: u16 = 1 << 2;
    const HEIGHT: u16 = 1 << 3;
    request(CONFIGURE_WINDOW, 0, |b| {
        b.put_u32(window);
        b.put_u16(WIDTH | HEIGHT);
        b.put_u16(0);
        b.put_u32(u32::from(width));
        b.put_u32(u32::from(height));
    })
}

/// `CreateColormap` without allocated entries (enough for `TrueColor`).
pub(crate) fn create_colormap(colormap: u32, window: u32, visual: u32) -> Request {
    request(CREATE_COLORMAP, 0, |b| {
        b.put_u32(colormap);
        b.put_u32(window);
        b.put_u32(visual);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x11::request::wire_bytes;

    #[test]
    fn create_window_sorts_values() {
        let window = NewWindow {
            id: 0x0020_0001,
            parent: 0x1e6,
            class: WindowClass::InputOutput,
            depth: 24,
            visual: 0x21,
            width: 640,
            height: 480,
        };
        let attributes = [
            WindowAttribute::EventMask(event_mask::EXPOSURE),
            WindowAttribute::BackgroundNone,
            WindowAttribute::BitGravityNorthWest,
        ];
        let bytes = wire_bytes(create_window(&window, &attributes).encoded());
        #[rustfmt::skip]
        let expected = [
            1, 24, 11, 0,
            0x01, 0, 0x20, 0,  0xe6, 0x01, 0, 0,
            0, 0, 0, 0,  0x80, 0x02, 0xe0, 0x01,
            0, 0, 1, 0,  0x21, 0, 0, 0,
            0x11, 0x08, 0, 0,
            0, 0, 0, 0,  1, 0, 0, 0,  0, 0x80, 0, 0,
        ];
        assert_eq!(bytes, expected);
    }

    #[test]
    fn small_window_requests() {
        assert_eq!(
            wire_bytes(map_window(0x0102_0304).encoded()),
            [8, 0, 2, 0, 4, 3, 2, 1]
        );
        assert_eq!(
            wire_bytes(destroy_window(7).encoded()),
            [4, 0, 2, 0, 7, 0, 0, 0]
        );
        #[rustfmt::skip]
        let resize = [12, 0, 5, 0,  1, 0, 0, 0,  12, 0, 0, 0,  44, 1, 0, 0,  200, 0, 0, 0];
        assert_eq!(wire_bytes(resize_window(1, 300, 200).encoded()), resize);
        let attributes = change_window_attributes(9, &[WindowAttribute::Cursor(3)]);
        assert_eq!(
            wire_bytes(attributes.encoded()),
            [2, 0, 4, 0, 9, 0, 0, 0, 0, 0x40, 0, 0, 3, 0, 0, 0]
        );
    }
}
