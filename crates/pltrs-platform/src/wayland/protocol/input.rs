//! Input devices: `wl_seat`, `wl_pointer`, `wl_keyboard`, and cursor
//! shapes by name (`cursor-shape-v1`).

use super::{Event, Interface};
use crate::error::Error;
use crate::wayland::wire::{Args, Request};

/// `wl_seat.capability.pointer`.
pub(crate) const CAPABILITY_POINTER: u32 = 1;
/// `wl_seat.capability.keyboard`.
pub(crate) const CAPABILITY_KEYBOARD: u32 = 2;

/// `wl_keyboard.keymap_format.xkb_v1`.
pub(crate) const KEYMAP_XKB_V1: u32 = 1;

/// `wl_seat.get_pointer`.
pub(crate) fn get_pointer(seat: u32, pointer: u32) -> Request {
    Request::new(seat, 0).uint(pointer)
}

/// `wl_seat.get_keyboard`.
pub(crate) fn get_keyboard(seat: u32, keyboard: u32) -> Request {
    Request::new(seat, 1).uint(keyboard)
}

/// `wl_pointer.set_cursor`: shows `surface` (0: no cursor) as the
/// cursor, its hot spot at `hotspot` in surface coordinates.
pub(crate) fn set_cursor(pointer: u32, serial: u32, surface: u32, hotspot: (i32, i32)) -> Request {
    Request::new(pointer, 0)
        .uint(serial)
        .uint(surface)
        .int(hotspot.0)
        .int(hotspot.1)
}

/// `wl_pointer.release` (version 3).
pub(crate) fn release_pointer(pointer: u32) -> Request {
    Request::new(pointer, 1)
}

/// `wl_keyboard.release` (version 3).
pub(crate) fn release_keyboard(keyboard: u32) -> Request {
    Request::new(keyboard, 0)
}

/// `wp_cursor_shape_manager_v1.get_pointer`.
pub(crate) fn get_cursor_shape_device(manager: u32, device: u32, pointer: u32) -> Request {
    Request::new(manager, 1).uint(device).uint(pointer)
}

/// `wp_cursor_shape_device_v1.destroy`.
pub(crate) fn destroy_cursor_shape_device(device: u32) -> Request {
    Request::new(device, 0)
}

/// `wp_cursor_shape_device_v1.set_shape` (`wp_cursor_shape_device_v1.shape`).
pub(crate) fn set_shape(device: u32, serial: u32, shape: u32) -> Request {
    Request::new(device, 1).uint(serial).uint(shape)
}

/// Parses the events of the seat and its devices.
pub(super) fn parse(
    interface: Interface,
    opcode: u16,
    args: &mut Args<'_>,
) -> Result<Event, Error> {
    Ok(match (interface, opcode) {
        (Interface::Seat, 0) => Event::Capabilities(args.uint()?),
        (Interface::Pointer, 0) => Event::PointerEnter {
            serial: args.uint()?,
            surface: args.uint()?,
            x: args.fixed()?,
            y: args.fixed()?,
        },
        (Interface::Pointer, 1) => {
            args.uint()?;
            args.uint()?;
            Event::PointerLeave
        }
        (Interface::Pointer, 2) => {
            args.uint()?;
            Event::Motion {
                x: args.fixed()?,
                y: args.fixed()?,
            }
        }
        (Interface::Pointer, 3) => Event::Button {
            serial: args.uint()?,
            time: args.uint()?,
            button: args.uint()?,
            pressed: args.uint()? == 1,
        },
        (Interface::Pointer, 4) => {
            args.uint()?;
            Event::Axis {
                axis: args.uint()?,
                value: args.fixed()?,
            }
        }
        (Interface::Pointer, 5) => Event::Frame,
        (Interface::Pointer, 8) => Event::AxisDiscrete {
            axis: args.uint()?,
            discrete: args.int()?,
        },
        (Interface::Pointer, 9) => Event::AxisValue120 {
            axis: args.uint()?,
            value120: args.int()?,
        },
        (Interface::Keyboard, 0) => Event::Keymap {
            format: args.uint()?,
            fd: args.fd()?,
            size: args.uint()?,
        },
        (Interface::Keyboard, 1) => Event::KeyboardEnter {
            serial: args.uint()?,
            surface: args.uint()?,
        },
        (Interface::Keyboard, 2) => {
            args.uint()?;
            Event::KeyboardLeave {
                surface: args.uint()?,
            }
        }
        (Interface::Keyboard, 3) => {
            let serial = args.uint()?;
            args.uint()?;
            Event::Key {
                serial,
                key: args.uint()?,
                pressed: args.uint()? != 0,
            }
        }
        (Interface::Keyboard, 4) => {
            args.uint()?;
            Event::Modifiers {
                depressed: args.uint()?,
                latched: args.uint()?,
                locked: args.uint()?,
                group: args.uint()?,
            }
        }
        (Interface::Keyboard, 5) => Event::RepeatInfo {
            rate: args.int()?,
            delay: args.int()?,
        },
        _ => Event::Ignored,
    })
}
