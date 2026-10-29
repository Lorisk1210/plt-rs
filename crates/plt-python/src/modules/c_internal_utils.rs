//! `_native.c_internal_utils`: whether a display server is reachable
//! (matplotlib's `_c_internal_utils`).
//!
//! matplotlib asks libX11 and libwayland-client to connect; plt-rs speaks
//! X11 over its socket itself (`plt_platform`), and a Wayland compositor is
//! reachable when its socket accepts a connection.

use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use plt_platform::{BackendKind, EventLoop};

use crate::binding::prelude::*;

/// Whether an X server accepts a connection (`$DISPLAY`).
fn xdisplay_is_valid() -> PyResult<bool> {
    Ok(BackendKind::detect() == Some(BackendKind::X11)
        && EventLoop::with_backend(BackendKind::X11).is_ok())
}

/// The socket of the Wayland compositor `$WAYLAND_DISPLAY` names (a path,
/// or a name in `$XDG_RUNTIME_DIR`), as libwayland-client finds it.
fn wayland_socket() -> Option<PathBuf> {
    let display = std::env::var_os("WAYLAND_DISPLAY").unwrap_or_else(|| "wayland-0".into());
    let display = PathBuf::from(display);
    if display.is_absolute() {
        return Some(display);
    }
    std::env::var_os("XDG_RUNTIME_DIR").map(|dir| PathBuf::from(dir).join(display))
}

/// Whether an X server or a Wayland compositor accepts a connection.
fn display_is_valid() -> PyResult<bool> {
    let wayland = wayland_socket().is_some_and(|socket| UnixStream::connect(socket).is_ok());
    Ok(wayland || xdisplay_is_valid()?)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("display_is_valid", display_is_valid)?;
    module.add("xdisplay_is_valid", xdisplay_is_valid)
}
