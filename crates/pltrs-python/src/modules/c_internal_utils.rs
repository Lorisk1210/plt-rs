//! `_native.c_internal_utils`: whether a display server is reachable
//! (matplotlib's `_c_internal_utils`).
//!
//! matplotlib asks libX11 and libwayland-client to connect; pltrs (Rust) speaks
//! X11 over its socket itself (`pltrs_platform`), and a Wayland compositor is
//! reachable when its socket accepts a connection. As in matplotlib, only
//! Linux asks: there is always a display on the other systems.

use pltrs_platform::{BackendKind, EventLoop};

use crate::binding::prelude::*;

/// Whether this system tells whether there is a display (matplotlib's
/// `__linux__`).
const ASKS: bool = cfg!(target_os = "linux");

/// Whether an X server accepts a connection (`$DISPLAY`).
fn x_server_is_reachable() -> bool {
    BackendKind::detect() == Some(BackendKind::X11)
        && EventLoop::with_backend(BackendKind::X11).is_ok()
}

/// Whether an X server accepts a connection; always off Linux.
fn xdisplay_is_valid() -> PyResult<bool> {
    Ok(!ASKS || x_server_is_reachable())
}

/// Whether an X server or a Wayland compositor accepts a connection;
/// always off Linux.
fn display_is_valid() -> PyResult<bool> {
    Ok(!ASKS || wayland::is_reachable() || x_server_is_reachable())
}

/// The Wayland compositor, reached over a Unix socket.
#[cfg(unix)]
mod wayland {
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;

    /// The socket of the Wayland compositor `$WAYLAND_DISPLAY` names (a
    /// path, or a name in `$XDG_RUNTIME_DIR`), as libwayland-client finds
    /// it.
    fn socket() -> Option<PathBuf> {
        let display = std::env::var_os("WAYLAND_DISPLAY").unwrap_or_else(|| "wayland-0".into());
        let display = PathBuf::from(display);
        if display.is_absolute() {
            return Some(display);
        }
        std::env::var_os("XDG_RUNTIME_DIR").map(|dir| PathBuf::from(dir).join(display))
    }

    /// Whether the compositor accepts a connection.
    pub(super) fn is_reachable() -> bool {
        socket().is_some_and(|socket| UnixStream::connect(socket).is_ok())
    }
}

/// Without Unix sockets there is no Wayland compositor.
#[cfg(not(unix))]
mod wayland {
    /// Never.
    pub(super) fn is_reachable() -> bool {
        false
    }
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("display_is_valid", display_is_valid)?;
    module.add("xdisplay_is_valid", xdisplay_is_valid)
}
