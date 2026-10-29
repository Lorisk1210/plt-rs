//! Fractional scaling: buffers cropped and scaled by the compositor
//! (`viewporter`) and the compositor's preferred scale per surface
//! (`fractional-scale-v1`).

use super::Event;
use crate::error::Error;
use crate::wayland::wire::{Args, Request};

/// `wp_viewporter.get_viewport`.
pub(crate) fn get_viewport(viewporter: u32, viewport: u32, surface: u32) -> Request {
    Request::new(viewporter, 1).uint(viewport).uint(surface)
}

/// `wp_viewport.destroy`.
pub(crate) fn destroy_viewport(viewport: u32) -> Request {
    Request::new(viewport, 0)
}

/// `wp_viewport.set_destination`: the surface size the buffer is scaled
/// to, in surface coordinates.
pub(crate) fn set_destination(viewport: u32, width: i32, height: i32) -> Request {
    Request::new(viewport, 2).int(width).int(height)
}

/// `wp_fractional_scale_manager_v1.get_fractional_scale`.
pub(crate) fn get_fractional_scale(manager: u32, scale: u32, surface: u32) -> Request {
    Request::new(manager, 1).uint(scale).uint(surface)
}

/// `wp_fractional_scale_v1.destroy`.
pub(crate) fn destroy_fractional_scale(scale: u32) -> Request {
    Request::new(scale, 0)
}

/// Parses the events of `wp_fractional_scale_v1`.
pub(super) fn parse(opcode: u16, args: &mut Args<'_>) -> Result<Event, Error> {
    Ok(match opcode {
        0 => Event::PreferredScale(args.uint()?),
        _ => Event::Ignored,
    })
}
