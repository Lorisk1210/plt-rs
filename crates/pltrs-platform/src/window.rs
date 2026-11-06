//! Window identifiers, creation options and cursors.

use std::fmt;

use crate::error::Error;
use crate::geometry::{LogicalSize, PhysicalSize, Size};
use crate::icon::Icon;

/// Identifies a window of an [`EventLoop`](crate::EventLoop).
///
/// Ids are unique among the live windows of one loop; a closed window's
/// id may be reused later by some backends.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(u64);

impl WindowId {
    pub(crate) const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The backend-specific raw value: the window's XID on X11, its
    /// `HWND` on Windows, the address of its view on macOS.
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

impl fmt::Debug for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WindowId({:#x})", self.0)
    }
}

/// Options for [`Context::create_window`](crate::Context::create_window).
///
/// ```
/// use pltrs_platform::{LogicalSize, WindowOptions};
/// let options = WindowOptions::new("Figure 1")
///     .with_inner_size(LogicalSize::new(640.0, 480.0))
///     .with_min_inner_size(LogicalSize::new(200.0, 150.0));
/// assert_eq!(options.title, "Figure 1");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct WindowOptions {
    /// Title shown in the title bar / task bar.
    pub title: String,
    /// Initial size of the content area.
    pub inner_size: Size,
    /// Minimum size of the content area the user may resize to.
    pub min_inner_size: Option<Size>,
    /// Application identifier used by the desktop to group windows and
    /// pick icons (X11 `WM_CLASS`, Wayland `app_id`; macOS and Windows
    /// group windows by process).
    pub app_id: String,
    /// Window icon. macOS windows have none: there it becomes the
    /// application's icon in the Dock.
    pub icon: Option<Icon>,
}

impl WindowOptions {
    /// Options with the given title, a 640×480 logical size and default
    /// settings for everything else.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    /// Sets the initial content size.
    #[must_use]
    pub fn with_inner_size(mut self, size: impl Into<Size>) -> Self {
        self.inner_size = size.into();
        self
    }

    /// Sets the minimum content size.
    #[must_use]
    pub fn with_min_inner_size(mut self, size: impl Into<Size>) -> Self {
        self.min_inner_size = Some(size.into());
        self
    }

    /// Sets the application identifier.
    #[must_use]
    pub fn with_app_id(mut self, app_id: impl Into<String>) -> Self {
        self.app_id = app_id.into();
        self
    }

    /// Sets the window icon.
    #[must_use]
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Resolves the initial size, rejecting empty sizes.
    pub(crate) fn physical_inner_size(&self, scale_factor: f64) -> Result<PhysicalSize, Error> {
        let size = self.inner_size.to_physical(scale_factor);
        if size.is_empty() {
            return Err(Error::InvalidArgument("window size must be non-zero"));
        }
        Ok(size)
    }
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: String::from("pltrs"),
            inner_size: Size::Logical(LogicalSize::new(640.0, 480.0)),
            min_inner_size: None,
            app_id: String::from("pltrs"),
            icon: None,
        }
    }
}

/// The mouse cursor shape shown over a window.
///
/// Mirrors matplotlib's `backend_tools.Cursors` plus a text cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CursorIcon {
    /// The platform's default arrow (matplotlib `POINTER`).
    #[default]
    Default,
    /// A pointing hand (matplotlib `HAND`).
    Pointer,
    /// A crosshair (matplotlib `SELECT_REGION`).
    Crosshair,
    /// A four-way move cursor (matplotlib `MOVE`).
    Move,
    /// A busy/wait cursor (matplotlib `WAIT`).
    Wait,
    /// A text I-beam.
    Text,
    /// Horizontal resize arrows (matplotlib `RESIZE_HORIZONTAL`).
    ResizeHorizontal,
    /// Vertical resize arrows (matplotlib `RESIZE_VERTICAL`).
    ResizeVertical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_builder_and_size_resolution() {
        let options = WindowOptions::new("t")
            .with_inner_size(PhysicalSize::new(300, 200))
            .with_app_id("demo");
        assert_eq!(options.app_id, "demo");
        assert_eq!(
            options.physical_inner_size(2.0),
            Ok(PhysicalSize::new(300, 200))
        );
        let options = WindowOptions::default();
        assert_eq!(
            options.physical_inner_size(2.0),
            Ok(PhysicalSize::new(1280, 960))
        );
        let empty = WindowOptions::new("t").with_inner_size(LogicalSize::new(0.0, 10.0));
        assert!(matches!(
            empty.physical_inner_size(1.0),
            Err(Error::InvalidArgument(_))
        ));
    }

    #[test]
    fn window_id_debug_is_hex() {
        assert_eq!(format!("{:?}", WindowId::from_raw(255)), "WindowId(0xff)");
        assert_eq!(WindowId::from_raw(7).to_raw(), 7);
    }
}
