//! Desktop windows: `xdg-shell` (`xdg_wm_base`, `xdg_surface`,
//! `xdg_toplevel`), server-side decorations (`xdg-decoration-unstable-v1`)
//! and focus requests (`xdg-activation-v1`).

use super::{Event, Interface, words};
use crate::error::Error;
use crate::wayland::wire::{Args, Request};

/// `xdg_toplevel.state.maximized`.
pub(crate) const STATE_MAXIMIZED: u32 = 1;
/// `xdg_toplevel.state.fullscreen`.
pub(crate) const STATE_FULLSCREEN: u32 = 2;
/// `xdg_toplevel.state.activated`.
pub(crate) const STATE_ACTIVATED: u32 = 4;
/// `xdg_toplevel.state.tiled_left` ..= `tiled_bottom`.
pub(crate) const STATES_TILED: [u32; 4] = [5, 6, 7, 8];

/// `xdg_toplevel.wm_capabilities.maximize`.
pub(crate) const CAPABILITY_MAXIMIZE: u32 = 2;
/// `xdg_toplevel.wm_capabilities.minimize`.
pub(crate) const CAPABILITY_MINIMIZE: u32 = 4;

/// `zxdg_toplevel_decoration_v1.mode.client_side`.
pub(crate) const MODE_CLIENT_SIDE: u32 = 1;
/// `zxdg_toplevel_decoration_v1.mode.server_side`.
pub(crate) const MODE_SERVER_SIDE: u32 = 2;

/// `xdg_wm_base.get_xdg_surface`.
pub(crate) fn get_xdg_surface(wm_base: u32, xdg_surface: u32, surface: u32) -> Request {
    Request::new(wm_base, 2).uint(xdg_surface).uint(surface)
}

/// `xdg_wm_base.pong`.
pub(crate) fn pong(wm_base: u32, serial: u32) -> Request {
    Request::new(wm_base, 3).uint(serial)
}

/// `xdg_surface.destroy`.
pub(crate) fn destroy_xdg_surface(xdg_surface: u32) -> Request {
    Request::new(xdg_surface, 0)
}

/// `xdg_surface.get_toplevel`.
pub(crate) fn get_toplevel(xdg_surface: u32, toplevel: u32) -> Request {
    Request::new(xdg_surface, 1).uint(toplevel)
}

/// `xdg_surface.set_window_geometry`: the window's visible bounds in
/// surface coordinates (decorations included, shadows and resize margins
/// excluded).
pub(crate) fn set_window_geometry(
    xdg_surface: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Request {
    Request::new(xdg_surface, 3)
        .int(x)
        .int(y)
        .int(width)
        .int(height)
}

/// `xdg_surface.ack_configure`.
pub(crate) fn ack_configure(xdg_surface: u32, serial: u32) -> Request {
    Request::new(xdg_surface, 4).uint(serial)
}

/// `xdg_toplevel.destroy`.
pub(crate) fn destroy_toplevel(toplevel: u32) -> Request {
    Request::new(toplevel, 0)
}

/// `xdg_toplevel.set_title`.
pub(crate) fn set_title(toplevel: u32, title: &str) -> Request {
    Request::new(toplevel, 2).string(title)
}

/// `xdg_toplevel.set_app_id`.
pub(crate) fn set_app_id(toplevel: u32, app_id: &str) -> Request {
    Request::new(toplevel, 3).string(app_id)
}

/// `xdg_toplevel.show_window_menu` at a position in surface coordinates.
pub(crate) fn show_window_menu(toplevel: u32, seat: u32, serial: u32, x: i32, y: i32) -> Request {
    Request::new(toplevel, 4)
        .uint(seat)
        .uint(serial)
        .int(x)
        .int(y)
}

/// `xdg_toplevel.move`: an interactive move started by the press with
/// `serial`.
pub(crate) fn start_move(toplevel: u32, seat: u32, serial: u32) -> Request {
    Request::new(toplevel, 5).uint(seat).uint(serial)
}

/// `xdg_toplevel.resize` of the edges `edges` (`xdg_toplevel.resize_edge`).
pub(crate) fn start_resize(toplevel: u32, seat: u32, serial: u32, edges: u32) -> Request {
    Request::new(toplevel, 6)
        .uint(seat)
        .uint(serial)
        .uint(edges)
}

/// `xdg_toplevel.set_min_size`, of the window geometry.
pub(crate) fn set_min_size(toplevel: u32, width: i32, height: i32) -> Request {
    Request::new(toplevel, 8).int(width).int(height)
}

/// `xdg_toplevel.set_maximized` or `unset_maximized`.
pub(crate) fn set_maximized(toplevel: u32, maximized: bool) -> Request {
    Request::new(toplevel, if maximized { 9 } else { 10 })
}

/// `xdg_toplevel.set_minimized`.
pub(crate) fn set_minimized(toplevel: u32) -> Request {
    Request::new(toplevel, 13)
}

/// `xdg_toplevel.set_fullscreen` (on an output of the compositor's
/// choice) or `unset_fullscreen`.
pub(crate) fn set_fullscreen(toplevel: u32, fullscreen: bool) -> Request {
    if fullscreen {
        Request::new(toplevel, 11).uint(0)
    } else {
        Request::new(toplevel, 12)
    }
}

/// `zxdg_decoration_manager_v1.get_toplevel_decoration`.
pub(crate) fn get_toplevel_decoration(manager: u32, decoration: u32, toplevel: u32) -> Request {
    Request::new(manager, 1).uint(decoration).uint(toplevel)
}

/// `zxdg_toplevel_decoration_v1.destroy`.
pub(crate) fn destroy_decoration(decoration: u32) -> Request {
    Request::new(decoration, 0)
}

/// `zxdg_toplevel_decoration_v1.set_mode`.
pub(crate) fn set_decoration_mode(decoration: u32, mode: u32) -> Request {
    Request::new(decoration, 1).uint(mode)
}

/// `xdg_activation_v1.get_activation_token`.
pub(crate) fn get_activation_token(activation: u32, token: u32) -> Request {
    Request::new(activation, 1).uint(token)
}

/// `xdg_activation_v1.activate`: focuses `surface` with a token.
pub(crate) fn activate(activation: u32, token: &str, surface: u32) -> Request {
    Request::new(activation, 2).string(token).uint(surface)
}

/// `xdg_activation_token_v1.set_serial`: the input event that justifies
/// the activation.
pub(crate) fn token_set_serial(token: u32, serial: u32, seat: u32) -> Request {
    Request::new(token, 0).uint(serial).uint(seat)
}

/// `xdg_activation_token_v1.set_surface`.
pub(crate) fn token_set_surface(token: u32, surface: u32) -> Request {
    Request::new(token, 2).uint(surface)
}

/// `xdg_activation_token_v1.commit`.
pub(crate) fn token_commit(token: u32) -> Request {
    Request::new(token, 3)
}

/// `xdg_activation_token_v1.destroy`.
pub(crate) fn destroy_token(token: u32) -> Request {
    Request::new(token, 4)
}

/// Parses the events of the shell interfaces.
pub(super) fn parse(
    interface: Interface,
    opcode: u16,
    args: &mut Args<'_>,
) -> Result<Event, Error> {
    Ok(match (interface, opcode) {
        (Interface::WmBase, 0) => Event::Ping(args.uint()?),
        (Interface::XdgSurface, 0) => Event::Configure(args.uint()?),
        (Interface::Toplevel, 0) => Event::ToplevelConfigure {
            width: args.int()?,
            height: args.int()?,
            states: words(args.array()?),
        },
        (Interface::Toplevel, 1) => Event::Close,
        (Interface::Toplevel, 3) => Event::WmCapabilities(words(args.array()?)),
        (Interface::Decoration, 0) => Event::DecorationMode(args.uint()?),
        (Interface::ActivationToken, 0) => Event::ActivationToken(args.string()?),
        _ => Event::Ignored,
    })
}
