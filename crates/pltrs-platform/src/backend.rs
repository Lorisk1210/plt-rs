//! The internal interface every platform backend implements, and runtime
//! backend selection.

use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use crate::decoration::DecorationPainter;
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
    /// A Wayland compositor (Linux), spoken over its wire protocol.
    Wayland,
    /// An in-memory backend for tests: records presented frames and lets
    /// tests inject input (see [`HeadlessHandle`](crate::HeadlessHandle)).
    Headless,
}

impl BackendKind {
    /// Picks the backend for this session, or `None` if no display server
    /// is reachable:
    ///
    /// * the one `$PLTRS_PLATFORM` names (`x11`, `wayland` or `headless`), if
    ///   it names one;
    /// * else Wayland when `$WAYLAND_DISPLAY` is set (on Linux);
    /// * else X11 when `$DISPLAY` is set (on Unix).
    pub fn detect() -> Option<Self> {
        let set = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
        let requested = std::env::var("PLTRS_PLATFORM").ok();
        Self::choose(requested.as_deref(), set("WAYLAND_DISPLAY"), set("DISPLAY"))
    }

    /// The backend for a `$PLTRS_PLATFORM` value and whether
    /// `$WAYLAND_DISPLAY` and `$DISPLAY` are set.
    fn choose(requested: Option<&str>, wayland: bool, x11: bool) -> Option<Self> {
        let named = [Self::X11, Self::Wayland, Self::Headless]
            .into_iter()
            .find(|kind| requested.is_some_and(|name| name.eq_ignore_ascii_case(kind.name())));
        named.or_else(|| {
            if cfg!(target_os = "linux") && wayland {
                Some(Self::Wayland)
            } else {
                (cfg!(unix) && x11).then_some(Self::X11)
            }
        })
    }

    /// A short lowercase name (`"x11"`, `"wayland"`, `"headless"`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::X11 => "x11",
            Self::Wayland => "wayland",
            Self::Headless => "headless",
        }
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How long a backend may block waiting for its display server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wait {
    /// Return at once when no data is available.
    Poll,
    /// Block until data arrives or the instant passes.
    Until(Instant),
    /// Block until data arrives.
    Forever,
}

impl Wait {
    /// The wait until `deadline` ([`Backend::poll`]'s argument): forever
    /// without one, not at all once it passed.
    pub(crate) fn until(deadline: Option<Instant>) -> Self {
        match deadline {
            None => Self::Forever,
            Some(deadline) if deadline <= Instant::now() => Self::Poll,
            Some(deadline) => Self::Until(deadline),
        }
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

    /// Brings the window above the others and asks for focus.
    fn raise_window(&mut self, window: WindowId) -> Result<(), Error>;

    /// Puts the PNG image `png` on the clipboard, owned by `window`, until
    /// another client takes the clipboard.
    fn set_clipboard_image(&mut self, window: WindowId, png: Vec<u8>) -> Result<(), Error>;

    /// Puts `text` on the clipboard, owned by `window`, until another
    /// client takes the clipboard.
    fn set_clipboard_text(&mut self, window: WindowId, text: String) -> Result<(), Error>;

    /// Asks for the clipboard's text, which arrives later as
    /// [`WindowEvent::ClipboardText`](crate::WindowEvent::ClipboardText)
    /// for `window`.
    fn request_clipboard_text(&mut self, window: WindowId) -> Result<(), Error>;

    /// Makes the window cover the screen without decorations, or restores
    /// it.
    fn set_fullscreen(&mut self, window: WindowId, fullscreen: bool) -> Result<(), Error>;

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

    /// Installs the painter of client-side decorations; backends whose
    /// windows the desktop decorates ignore it.
    fn set_decoration_painter(
        &mut self,
        _painter: Box<dyn DecorationPainter>,
    ) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_follow_the_deadline() {
        assert_eq!(Wait::until(None), Wait::Forever);
        let past = Instant::now();
        assert_eq!(Wait::until(Some(past)), Wait::Poll);
        let later = past + std::time::Duration::from_secs(60);
        assert_eq!(Wait::until(Some(later)), Wait::Until(later));
    }

    #[test]
    fn names() {
        assert_eq!(BackendKind::X11.to_string(), "x11");
        assert_eq!(BackendKind::Wayland.to_string(), "wayland");
        assert_eq!(BackendKind::Headless.name(), "headless");
    }

    #[test]
    fn detection_prefers_the_request_then_wayland_then_x11() {
        let choose = BackendKind::choose;
        assert_eq!(choose(Some("x11"), true, true), Some(BackendKind::X11));
        assert_eq!(
            choose(Some("Wayland"), false, false),
            Some(BackendKind::Wayland)
        );
        assert_eq!(
            choose(Some("headless"), true, true),
            Some(BackendKind::Headless)
        );
        assert_eq!(choose(Some("bogus"), false, true), Some(BackendKind::X11));
        let wayland = cfg!(target_os = "linux").then_some(BackendKind::Wayland);
        assert_eq!(choose(None, true, true), wayland.or(Some(BackendKind::X11)));
        assert_eq!(choose(None, false, true), Some(BackendKind::X11));
        assert_eq!(choose(None, false, false), None);
    }
}
