//! The seat: its pointer and keyboard come and go with its capabilities,
//! and its latest input serial authorizes clipboard, move and focus
//! requests.

use super::WaylandBackend;
use super::keyboard::Keyboard;
use super::pointer::Pointer;
use super::protocol::Interface;
use super::protocol::data;
use super::protocol::input::{self, CAPABILITY_KEYBOARD, CAPABILITY_POINTER};
use crate::compose::Composer;
use crate::error::Error;

/// The first seat the compositor announced.
#[derive(Debug)]
pub(crate) struct Seat {
    /// The `wl_seat` object.
    pub(crate) id: u32,
    /// The global's numeric name.
    pub(crate) name: u32,
    /// The version bound.
    pub(crate) version: u32,
    /// The pointer, while the seat has one.
    pub(crate) pointer: Option<Pointer>,
    /// The keyboard, while the seat has one.
    pub(crate) keyboard: Option<Keyboard>,
    /// The serial of the latest input event (press, key, focus).
    pub(crate) serial: u32,
}

impl Seat {
    /// A seat without devices yet.
    pub(crate) fn new(id: u32, name: u32, version: u32) -> Self {
        Self {
            id,
            name,
            version,
            pointer: None,
            keyboard: None,
            serial: 0,
        }
    }
}

impl WaylandBackend {
    /// Creates or releases the pointer and keyboard as the seat gains or
    /// loses them.
    pub(super) fn seat_capabilities(&mut self, capabilities: u32) -> Result<(), Error> {
        let Some(mut seat) = self.seat.take() else {
            return Ok(());
        };
        let result = self.update_devices(&mut seat, capabilities);
        self.seat = Some(seat);
        result
    }

    fn update_devices(&mut self, seat: &mut Seat, capabilities: u32) -> Result<(), Error> {
        let wants_pointer = capabilities & CAPABILITY_POINTER != 0;
        if wants_pointer && seat.pointer.is_none() {
            let pointer = self.conn.new_id(Interface::Pointer)?;
            self.conn.send(input::get_pointer(seat.id, pointer))?;
            if let Some(manager) = self.globals.get(Interface::CursorShapeManager) {
                let device = self.conn.new_id(Interface::CursorShapeDevice)?;
                self.conn
                    .send(input::get_cursor_shape_device(manager, device, pointer))?;
                self.cursors.shape_device = Some(device);
            }
            seat.pointer = Some(Pointer::new(pointer));
        } else if !wants_pointer && let Some(pointer) = seat.pointer.take() {
            if let Some(device) = self.cursors.shape_device.take() {
                self.conn.send(input::destroy_cursor_shape_device(device))?;
            }
            if seat.version >= 3 {
                self.conn.send(input::release_pointer(pointer.id))?;
            }
        }
        let wants_keyboard = capabilities & CAPABILITY_KEYBOARD != 0;
        if wants_keyboard && seat.keyboard.is_none() {
            let keyboard = self.conn.new_id(Interface::Keyboard)?;
            self.conn.send(input::get_keyboard(seat.id, keyboard))?;
            seat.keyboard = Some(Keyboard::new(keyboard, Composer::default()));
        } else if !wants_keyboard
            && let Some(keyboard) = seat.keyboard.take()
            && seat.version >= 3
        {
            self.conn.send(input::release_keyboard(keyboard.id))?;
        }
        Ok(())
    }

    /// Creates the seat's clipboard device once both the seat and the
    /// data device manager are known.
    pub(super) fn ensure_data_device(&mut self) -> Result<(), Error> {
        let (Some(seat), Some(manager)) = (
            self.seat.as_ref(),
            self.globals.get(Interface::DataDeviceManager),
        ) else {
            return Ok(());
        };
        if self.clipboard.device.is_none() {
            let device = self.conn.new_id(Interface::DataDevice)?;
            self.conn
                .send(data::get_data_device(manager, device, seat.id))?;
            self.clipboard.device = Some(device);
        }
        Ok(())
    }

    /// Records the serial of an input event.
    pub(super) fn note_serial(&mut self, serial: u32) {
        if let Some(seat) = &mut self.seat {
            seat.serial = serial;
        }
    }
}
