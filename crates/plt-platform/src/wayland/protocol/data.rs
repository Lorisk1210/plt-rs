//! The clipboard: `wl_data_device_manager`, `wl_data_device`,
//! `wl_data_source` (what this client offers) and `wl_data_offer` (what
//! another client offers).

use std::os::fd::OwnedFd;

use super::{Event, Interface};
use crate::error::Error;
use crate::wayland::wire::{Args, Request};

/// `wl_data_device_manager.create_data_source`.
pub(crate) fn create_data_source(manager: u32, source: u32) -> Request {
    Request::new(manager, 0).uint(source)
}

/// `wl_data_device_manager.get_data_device` for `seat`.
pub(crate) fn get_data_device(manager: u32, device: u32, seat: u32) -> Request {
    Request::new(manager, 1).uint(device).uint(seat)
}

/// `wl_data_device.set_selection`: puts `source` on the clipboard, for
/// the input event with `serial`.
pub(crate) fn set_selection(device: u32, source: u32, serial: u32) -> Request {
    Request::new(device, 1).uint(source).uint(serial)
}

/// `wl_data_source.offer`: adds a MIME type the source can provide.
pub(crate) fn offer(source: u32, mime: &str) -> Request {
    Request::new(source, 0).string(mime)
}

/// `wl_data_source.destroy`.
pub(crate) fn destroy_source(source: u32) -> Request {
    Request::new(source, 1)
}

/// `wl_data_offer.receive`: asks the offering client to write `mime` data
/// to `fd`.
pub(crate) fn receive(offer: u32, mime: &str, fd: OwnedFd) -> Request {
    Request::new(offer, 1).string(mime).fd(fd)
}

/// `wl_data_offer.destroy`.
pub(crate) fn destroy_offer(offer: u32) -> Request {
    Request::new(offer, 2)
}

/// Parses the events of the clipboard interfaces.
pub(super) fn parse(
    interface: Interface,
    opcode: u16,
    args: &mut Args<'_>,
) -> Result<Event, Error> {
    Ok(match (interface, opcode) {
        (Interface::DataDevice, 0) => Event::DataOffer(args.uint()?),
        (Interface::DataDevice, 1) => {
            for _ in 0..4 {
                args.uint()?;
            }
            Event::DragEnter(args.uint()?)
        }
        (Interface::DataDevice, 5) => Event::Selection(args.uint()?),
        (Interface::DataOffer, 0) => Event::Offer(args.string()?),
        (Interface::DataSource, 1) => Event::Send {
            mime: args.string()?,
            fd: args.fd()?,
        },
        (Interface::DataSource, 2) => Event::Cancelled,
        _ => Event::Ignored,
    })
}
