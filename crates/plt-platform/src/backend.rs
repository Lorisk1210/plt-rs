//! The internal interface every platform backend implements, and runtime
//! backend selection.

use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use crate::error::Error;
use crate::event::Event;
use crate::frame::{Frame, OwnedFrame};
use crate::geometry::{PhysicalRect, PhysicalSize, Size};
use crate::window::{CursorIcon, WindowId, WindowOptions};

/// The windowing backends this crate can drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BackendKind {
    /// The X Window System (also `XWayland`), spoken over its wire protocol.
    X11,
    /// An in-memory backend for tests: records presented frames and lets
    /// tests inject input (see [`HeadlessHandle`](crate::HeadlessHandle)).
    Headless,
}

impl BackendKind {
    /// Picks the best backend for this session, or `None` if no display
    /// server is reachable: X11 when `$DISPLAY` is set (on Unix).
    pub fn detect() -> Option<Self> {
        let has_display = std::env::var_os("DISPLAY").is_some_and(|name| !name.is_empty());
        (cfg!(unix) && has_display).then_some(Self::X11)
    }

    /// A short lowercase name (`"x11"`, `"headless"`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::X11 => "x11",
            Self::Headless => "headless",
        }
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Wakes a backend's event loop from any thread.
pub(crate) trait Waker: Send + Sync + fmt::Debug {
    /// Makes the loop deliver [`Event::WakeUp`] soon, interrupting a
    /// blocking wait.
    fn wake(&self) -> Result<(), Error>;
}

/// A platform implementation. The shared loop logic in
/// [`crate::event_loop`] (timers, redraw coalescing, dispatch) drives it.
///
/// Every method taking a [`WindowId`] returns [`Error::UnknownWindow`] for
/// ids that are not live windows of this backend.
pub(crate) trait Backend {
    /// Which backend this is.
    fn kind(&self) -> BackendKind;

    /// Creates and shows a window. The backend later reports
    /// `RedrawRequested` once the window is visible.
    fn create_window(&mut self, options: &WindowOptions) -> Result<WindowId, Error>;

    /// Destroys a window immediately.
    fn destroy_window(&mut self, window: WindowId) -> Result<(), Error>;

    /// Ids of all live windows.
    fn window_ids(&self) -> Vec<WindowId>;

    /// Whether `window` is a live window.
    fn has_window(&self, window: WindowId) -> bool;

    /// Current content size in physical pixels.
    fn inner_size(&self, window: WindowId) -> Result<PhysicalSize, Error>;

    /// Current scale factor (physical / logical pixels).
    fn scale_factor(&self, window: WindowId) -> Result<f64, Error>;

    /// Changes the title.
    fn set_title(&mut self, window: WindowId, title: &str) -> Result<(), Error>;

    /// Changes the cursor shown over the window.
    fn set_cursor(&mut self, window: WindowId, cursor: CursorIcon) -> Result<(), Error>;

    /// Asks for a new content size; the outcome is reported as `Resized`.
    fn set_inner_size(&mut self, window: WindowId, size: Size) -> Result<(), Error>;

    /// Copies `region` of `frame` (already clipped to the frame) to the
    /// same position of the window.
    fn present(
        &mut self,
        window: WindowId,
        frame: &Frame<'_>,
        region: PhysicalRect,
    ) -> Result<(), Error>;

    /// Reads back the window's current contents as opaque RGBA8.
    fn read_pixels(&mut self, window: WindowId) -> Result<OwnedFrame, Error>;

    /// Waits until input is available or `deadline` passes (`None`: no
    /// deadline; a past deadline: do not wait), then appends every
    /// available event to `events`.
    fn poll(&mut self, deadline: Option<Instant>, events: &mut Vec<Event>) -> Result<(), Error>;

    /// A thread-safe handle that interrupts [`Backend::poll`].
    fn waker(&self) -> Arc<dyn Waker>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(BackendKind::X11.to_string(), "x11");
        assert_eq!(BackendKind::Headless.name(), "headless");
    }
}
