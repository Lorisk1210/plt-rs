//! Window icons: `xdg-toplevel-icon-v1` (`xdg_toplevel_icon_manager_v1`,
//! `xdg_toplevel_icon_v1`).

use super::Event;
use crate::error::Error;
use crate::wayland::wire::{Args, Request};

/// `xdg_toplevel_icon_manager_v1.create_icon`.
pub(crate) fn create_icon(manager: u32, icon: u32) -> Request {
    Request::new(manager, 1).uint(icon)
}

/// `xdg_toplevel_icon_manager_v1.set_icon`: the toplevel shows `icon`
/// from its surface's next commit.
pub(crate) fn set_icon(manager: u32, toplevel: u32, icon: u32) -> Request {
    Request::new(manager, 2).uint(toplevel).uint(icon)
}

/// `xdg_toplevel_icon_v1.destroy` (the toplevels keep the icon).
pub(crate) fn destroy_icon(icon: u32) -> Request {
    Request::new(icon, 0)
}

/// `xdg_toplevel_icon_v1.add_buffer`: a square `wl_shm` buffer showing
/// the icon at `scale`.
pub(crate) fn add_buffer(icon: u32, buffer: u32, scale: i32) -> Request {
    Request::new(icon, 2).uint(buffer).int(scale)
}

/// Parses the events of `xdg_toplevel_icon_manager_v1`.
pub(super) fn parse(opcode: u16, args: &mut Args<'_>) -> Result<Event, Error> {
    Ok(match opcode {
        0 => Event::IconSize(args.int()?),
        1 => Event::IconSizesDone,
        _ => Event::Ignored,
    })
}
