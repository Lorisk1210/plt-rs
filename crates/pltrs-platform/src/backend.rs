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
    /// macOS's `AppKit`, through the Objective-C runtime.
    MacOs,
    /// The Windows API (Win32).
    Windows,
    /// An in-memory backend for tests: records presented frames and lets
    /// tests inject input (see [`HeadlessHandle`](crate::HeadlessHandle)).
    Headless,
}

impl BackendKind {
    /// Every backend, in the order their names are matched.
    const ALL: [Self; 5] = [
        Self::X11,
        Self::Wayland,
        Self::MacOs,
        Self::Windows,
        Self::Headless,
    ];

    /// Picks the backend for this session, or `None` if no display server
    /// is reachable:
    ///
    /// * the one `$PLTRS_PLATFORM` names (`x11`, `wayland`, `macos`,
    ///   `windows` or `headless`), if it names one;
    /// * else the native one on macOS and Windows;
    /// * else Wayland when `$WAYLAND_DISPLAY` is set (on Linux);
    /// * else X11 when `$DISPLAY` is set (on other Unix systems).
    pub fn detect() -> Option<Self> {
        let set = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
        let requested = std::env::var("PLTRS_PLATFORM").ok();
        Self::choose(
            System::CURRENT,
            requested.as_deref(),
            set("WAYLAND_DISPLAY"),
            set("DISPLAY"),
        )
    }

    /// The backend on `os` for a `$PLTRS_PLATFORM` value and whether
    /// `$WAYLAND_DISPLAY` and `$DISPLAY` are set.
    fn choose(system: System, requested: Option<&str>, wayland: bool, x11: bool) -> Option<Self> {
        let named = Self::ALL
            .into_iter()
            .find(|kind| requested.is_some_and(|name| name.eq_ignore_ascii_case(kind.name())));
        named.or(match system {
            System::MacOs => Some(Self::MacOs),
            System::Windows => Some(Self::Windows),
            System::Linux if wayland => Some(Self::Wayland),
            System::Linux | System::OtherUnix if x11 => Some(Self::X11),
            System::Linux | System::OtherUnix | System::Other => None,
        })
    }

    /// A short lowercase name (`"x11"`, `"wayland"`, `"macos"`,
    /// `"windows"`, `"headless"`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::X11 => "x11",
            Self::Wayland => "wayland",
            Self::MacOs => "macos",
            Self::Windows => "windows",
            Self::Headless => "headless",
        }
    }
}

/// The operating systems backend detection tells apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum System {
    Linux,
    MacOs,
    Windows,
    /// A Unix system other than Linux and macOS (the BSDs, ...).
    OtherUnix,
    /// Anything else, where only the headless backend runs.
    Other,
}

impl System {
    /// The system this crate was compiled for.
    const CURRENT: Self = if cfg!(target_os = "linux") {
        Self::Linux
    } else if cfg!(target_os = "macos") {
        Self::MacOs
    } else if cfg!(windows) {
        Self::Windows
    } else if cfg!(unix) {
        Self::OtherUnix
    } else {
        Self::Other
    };
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
        assert_eq!(BackendKind::MacOs.to_string(), "macos");
        assert_eq!(BackendKind::Windows.to_string(), "windows");
        assert_eq!(BackendKind::Headless.name(), "headless");
    }

    #[test]
    fn detection_on_linux_prefers_the_request_then_wayland_then_x11() {
        let choose =
            |requested, wayland, x11| BackendKind::choose(System::Linux, requested, wayland, x11);
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
        assert_eq!(choose(None, true, true), Some(BackendKind::Wayland));
        assert_eq!(choose(None, false, true), Some(BackendKind::X11));
        assert_eq!(choose(None, false, false), None);
        assert_eq!(
            choose(Some("MACOS"), false, false),
            Some(BackendKind::MacOs)
        );
    }

    #[test]
    fn macos_and_windows_pick_their_native_backend_unless_asked() {
        for (system, native) in [
            (System::MacOs, BackendKind::MacOs),
            (System::Windows, BackendKind::Windows),
        ] {
            assert_eq!(BackendKind::choose(system, None, false, true), Some(native));
            assert_eq!(
                BackendKind::choose(system, Some("bogus"), true, true),
                Some(native)
            );
            assert_eq!(
                BackendKind::choose(system, Some("x11"), false, true),
                Some(BackendKind::X11)
            );
        }
    }

    #[test]
    fn other_systems_have_x11_at_most() {
        assert_eq!(
            BackendKind::choose(System::OtherUnix, None, true, false),
            None
        );
        assert_eq!(
            BackendKind::choose(System::OtherUnix, None, true, true),
            Some(BackendKind::X11)
        );
        assert_eq!(BackendKind::choose(System::Other, None, true, true), None);
        assert_eq!(
            BackendKind::choose(System::Other, Some("headless"), false, false),
            Some(BackendKind::Headless)
        );
    }
}
