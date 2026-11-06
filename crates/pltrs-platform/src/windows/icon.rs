//! A window's icon: drawn at the sizes of the window's DPI, shown with
//! `WM_SETICON`, and drawn again when the window moves to a monitor of
//! another DPI.

use super::convert::window_icon_images;
use super::sys::{self, Hwnd, OwnedIcon};
use crate::error::Error;
use crate::icon::Icon;

/// The icon of one window, and the handles drawn from it that the window
/// shows, which live as long as it shows them.
#[derive(Debug)]
pub(super) struct WindowIcon {
    icon: Icon,
    /// The title bar's and the task bar's icons.
    _shown: (OwnedIcon, OwnedIcon),
}

impl WindowIcon {
    /// Shows `icon` on `window` at the sizes of `dpi`; `None` when the
    /// icon draws none of them.
    pub(super) fn show(icon: &Icon, window: Hwnd, dpi: u32) -> Result<Option<Self>, Error> {
        let Some((small, big)) = window_icon_images(icon, sys::icon_sides(dpi)) else {
            return Ok(None);
        };
        let shown = (OwnedIcon::new(&small)?, OwnedIcon::new(&big)?);
        window.set_icons(&shown.0, &shown.1);
        Ok(Some(Self {
            icon: icon.clone(),
            _shown: shown,
        }))
    }

    /// The icon shown.
    pub(super) fn icon(&self) -> &Icon {
        &self.icon
    }
}
