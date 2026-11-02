//! [`WindowsBackend`]: the [`Backend`] on Win32 windows.

use std::sync::Arc;
use std::time::Instant;

use super::convert::{scale_factor, timeout_ms};
use super::procedure::{WindowState, with_windows};
use super::sys::{self, Clipboard, Hwnd, OwnedIcon, WindowClass};
use crate::backend::{Backend, BackendKind, Wait, Waker};
use crate::error::Error;
use crate::event::{Event, WindowEvent};
use crate::frame::{Frame, OwnedFrame};
use crate::geometry::{PhysicalRect, PhysicalSize, Size};
use crate::registry::{LoopId, Registry};
use crate::surface::{ChannelOrder, Surface};
use crate::wake::WakeFlags;
use crate::window::{CursorIcon, WindowId, WindowOptions};

/// The Windows [`Backend`].
#[derive(Debug)]
pub(crate) struct WindowsBackend {
    /// This loop among those of the thread.
    id: LoopId,
    class: WindowClass,
    waker: Arc<WindowsWaker>,
}

impl WindowsBackend {
    /// Prepares the thread for windows: per-monitor DPI awareness, the
    /// window class, and the window that receives wake-ups.
    pub(crate) fn connect() -> Result<Self, Error> {
        sys::use_per_monitor_dpi();
        let class = WindowClass::get()?;
        let messages = class.create_message_window()?;
        Ok(Self {
            id: with_windows(Registry::new_loop),
            class,
            waker: Arc::new(WindowsWaker {
                flags: WakeFlags::new(),
                window: messages,
            }),
        })
    }

    /// The handle of `window`, one of this loop's windows.
    fn hwnd(&self, window: WindowId) -> Result<Hwnd, Error> {
        with_windows(|windows| windows.get(self.id, window).map(|_| Hwnd::from_id(window)))
    }

    /// Runs `f` on the state of `window`, one of this loop's windows.
    fn with_state<T>(
        &self,
        window: WindowId,
        f: impl FnOnce(&mut WindowState) -> T,
    ) -> Result<T, Error> {
        with_windows(|windows| windows.get_mut(self.id, window).map(f))
    }

    /// Registers the new window `hwnd` and shows it, at the size and with
    /// the icon `options` give.
    fn adopt(
        &self,
        hwnd: Hwnd,
        options: &WindowOptions,
        icon: Option<OwnedIcon>,
    ) -> Result<WindowId, Error> {
        let dpi = hwnd.dpi();
        let size = options.physical_inner_size(scale_factor(dpi))?;
        let surface = Surface::new(size, ChannelOrder::Bgra)?;
        if let Some(icon) = &icon {
            hwnd.set_icon(icon);
        }
        let window = hwnd.id();
        let state = WindowState::new(surface, dpi, options.min_inner_size, icon);
        with_windows(|windows| windows.insert(self.id, window, state));
        hwnd.resize(hwnd.outer_size(size, dpi)?)?;
        hwnd.show();
        with_windows(|windows| windows.push(window, WindowEvent::RedrawRequested));
        Ok(window)
    }

    /// Puts `content` on the clipboard, owned by `window`.
    fn set_clipboard(&self, window: WindowId, content: Clipboard<'_>) -> Result<(), Error> {
        sys::set_clipboard(self.hwnd(window)?, content)
    }
}

impl Drop for WindowsBackend {
    fn drop(&mut self) {
        // Errors cannot be reported from `drop`; the windows and the
        // wake-up window are no longer used either way.
        for window in with_windows(|windows| windows.windows(self.id)) {
            let _ = self.destroy_window(window);
        }
        self.waker.flags.close();
        let _ = self.waker.window.destroy();
    }
}

impl Backend for WindowsBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Windows
    }

    fn create_window(&mut self, options: &WindowOptions) -> Result<WindowId, Error> {
        let icon = options.icon.as_ref().map(OwnedIcon::new).transpose()?;
        let hwnd = self.class.create_window(&options.title)?;
        self.adopt(hwnd, options, icon).inspect_err(|_| {
            // The creation error matters more than a failure to clean up.
            let _ = hwnd.destroy();
            with_windows(|windows| windows.remove(hwnd.id()));
        })
    }

    fn destroy_window(&mut self, window: WindowId) -> Result<(), Error> {
        let destroyed = self.hwnd(window)?.destroy();
        with_windows(|windows| windows.remove(window));
        destroyed
    }

    fn window_ids(&self) -> Vec<WindowId> {
        with_windows(|windows| windows.windows(self.id))
    }

    fn has_window(&self, window: WindowId) -> bool {
        self.hwnd(window).is_ok()
    }

    fn inner_size(&self, window: WindowId) -> Result<PhysicalSize, Error> {
        self.with_state(window, |state| state.surface.size())
    }

    fn scale_factor(&self, window: WindowId) -> Result<f64, Error> {
        self.with_state(window, |state| scale_factor(state.dpi))
    }

    fn set_title(&mut self, window: WindowId, title: &str) -> Result<(), Error> {
        self.hwnd(window)?.set_title(title)
    }

    fn set_cursor(&mut self, window: WindowId, cursor: CursorIcon) -> Result<(), Error> {
        let inside = self.with_state(window, |state| {
            state.cursor = cursor;
            state.pointer_inside
        })?;
        if inside {
            sys::show_cursor(cursor);
        }
        Ok(())
    }

    fn set_inner_size(&mut self, window: WindowId, size: Size) -> Result<(), Error> {
        let hwnd = self.hwnd(window)?;
        let dpi = self.with_state(window, |state| state.dpi)?;
        hwnd.resize(hwnd.outer_size(size.to_physical(scale_factor(dpi)), dpi)?)
    }

    fn raise_window(&mut self, window: WindowId) -> Result<(), Error> {
        self.hwnd(window)?.raise();
        Ok(())
    }

    fn set_clipboard_image(&mut self, window: WindowId, png: Vec<u8>) -> Result<(), Error> {
        self.set_clipboard(window, Clipboard::Png(&png))
    }

    fn set_clipboard_text(&mut self, window: WindowId, text: String) -> Result<(), Error> {
        self.set_clipboard(window, Clipboard::Text(&text))
    }

    fn request_clipboard_text(&mut self, window: WindowId) -> Result<(), Error> {
        let text = sys::clipboard_text(self.hwnd(window)?)?;
        with_windows(|windows| windows.push(window, WindowEvent::ClipboardText(text)));
        Ok(())
    }

    fn set_fullscreen(&mut self, window: WindowId, fullscreen: bool) -> Result<(), Error> {
        let hwnd = self.hwnd(window)?;
        let restore = self.with_state(window, |state| state.fullscreen.take())?;
        let restore = match (restore, fullscreen) {
            (None, true) => Some(hwnd.enter_fullscreen()?),
            (Some(restore), false) => hwnd
                .leave_fullscreen(&restore)
                .map_or(Some(restore), |()| None),
            (unchanged, _) => unchanged,
        };
        self.with_state(window, |state| state.fullscreen = restore)
    }

    fn present(
        &mut self,
        window: WindowId,
        frame: &Frame<'_>,
        region: PhysicalRect,
    ) -> Result<(), Error> {
        let hwnd = self.hwnd(window)?;
        let (region, pixels) = self.with_state(window, |state| {
            let copied = state.surface.blit(frame, region);
            state.surface.crop(copied)
        })?;
        sys::draw(hwnd, region, &pixels)
    }

    fn read_pixels(&mut self, window: WindowId) -> Result<OwnedFrame, Error> {
        self.with_state(window, |state| state.surface.to_rgba())
    }

    fn poll(&mut self, deadline: Option<Instant>, events: &mut Vec<Event>) -> Result<(), Error> {
        let wait = Wait::until(deadline);
        loop {
            sys::dispatch_messages();
            events.extend(with_windows(|windows| windows.take_events(self.id)));
            if self.waker.flags.take() {
                events.push(Event::WakeUp);
            }
            if !events.is_empty() {
                return Ok(());
            }
            let Some(timeout) = timeout_ms(wait, Instant::now()) else {
                return Ok(());
            };
            sys::wait_for_messages(timeout)?;
        }
    }

    fn waker(&self) -> Arc<dyn Waker> {
        Arc::clone(&self.waker) as Arc<dyn Waker>
    }
}

/// Wakes a Windows event loop by posting a message to its message-only
/// window, from any thread.
#[derive(Debug)]
struct WindowsWaker {
    flags: WakeFlags,
    window: Hwnd,
}

impl Waker for WindowsWaker {
    fn wake(&self) -> Result<(), Error> {
        self.flags.wake(|| sys::post_wake(self.window))
    }
}
