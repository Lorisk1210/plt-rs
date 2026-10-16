//! [`Context`]: the operations available to event handlers.

use std::time::{Duration, Instant};

use super::{EventLoopProxy, LoopState, TimerId};
use crate::backend::{Backend, BackendKind};
use crate::error::Error;
use crate::event::{Event, WindowEvent};
use crate::frame::{Frame, OwnedFrame};
use crate::geometry::{PhysicalRect, PhysicalSize, Size};
use crate::window::{CursorIcon, WindowId, WindowOptions};

/// Mutable access to the event loop: create and close windows, present
/// frames, change titles and cursors, start timers, request exit.
///
/// Handlers receive one with every event; [`EventLoop::context`]
/// (crate::EventLoop::context) provides one outside of a handler.
pub struct Context<'a> {
    backend: &'a mut dyn Backend,
    state: &'a mut LoopState,
}

impl<'a> Context<'a> {
    pub(super) fn new(backend: &'a mut dyn Backend, state: &'a mut LoopState) -> Self {
        Self { backend, state }
    }

    /// Which backend the loop drives.
    pub fn backend_kind(&self) -> BackendKind {
        self.backend.kind()
    }

    /// Creates and shows a window. It receives
    /// [`WindowEvent::RedrawRequested`] once visible.
    pub fn create_window(&mut self, options: &WindowOptions) -> Result<WindowId, Error> {
        self.backend.create_window(options)
    }

    /// Closes a window. [`WindowEvent::Destroyed`] is dispatched right after
    /// the current handler call returns, and no other event follows it.
    pub fn close_window(&mut self, window: WindowId) -> Result<(), Error> {
        self.backend.destroy_window(window)?;
        self.state.redraws.retain(|&id| id != window);
        let event = WindowEvent::Destroyed;
        self.state.pending.push_back(Event::Window { window, event });
        Ok(())
    }

    /// Ids of all open windows.
    pub fn windows(&self) -> Vec<WindowId> {
        self.backend.window_ids()
    }

    /// Current content size of a window in physical pixels.
    pub fn inner_size(&self, window: WindowId) -> Result<PhysicalSize, Error> {
        self.backend.inner_size(window)
    }

    /// Current scale factor of a window (physical / logical pixels).
    pub fn scale_factor(&self, window: WindowId) -> Result<f64, Error> {
        self.backend.scale_factor(window)
    }

    /// Changes a window's title.
    pub fn set_title(&mut self, window: WindowId, title: &str) -> Result<(), Error> {
        self.backend.set_title(window, title)
    }

    /// Changes the cursor shown while the pointer is over a window.
    pub fn set_cursor(&mut self, window: WindowId, cursor: CursorIcon) -> Result<(), Error> {
        self.backend.set_cursor(window, cursor)
    }

    /// Asks for a new content size. The window manager may adjust or
    /// refuse it; the actual size arrives as [`WindowEvent::Resized`].
    pub fn set_inner_size(&mut self, window: WindowId, size: impl Into<Size>) -> Result<(), Error> {
        let size = size.into();
        if size.to_physical(self.backend.scale_factor(window)?).is_empty() {
            return Err(Error::InvalidArgument("window size must be non-zero"));
        }
        self.backend.set_inner_size(window, size)
    }

    /// Schedules a [`WindowEvent::RedrawRequested`] for the next loop
    /// iteration (coalesced with other requests).
    pub fn request_redraw(&mut self, window: WindowId) -> Result<(), Error> {
        if !self.backend.has_window(window) {
            return Err(Error::UnknownWindow(window));
        }
        self.state.schedule_redraw(window);
        Ok(())
    }

    /// Shows `frame` in the window (see [`Frame`] for the pixel
    /// convention).
    ///
    /// The frame is drawn at the top-left corner; a frame smaller than the
    /// window leaves the rest unchanged, a larger one is clipped. This
    /// tolerates the frame lagging behind a resize.
    pub fn present(&mut self, window: WindowId, frame: &Frame<'_>) -> Result<(), Error> {
        self.present_region(window, frame, PhysicalRect::from_size(frame.size()))
    }

    /// Updates only `region` of the window from the same region of
    /// `frame` (matplotlib's blitting). The region is clipped to the frame.
    pub fn present_region(
        &mut self,
        window: WindowId,
        frame: &Frame<'_>,
        region: PhysicalRect,
    ) -> Result<(), Error> {
        let region = region.clip_to(frame.size());
        if region.is_empty() {
            return if self.backend.has_window(window) {
                Ok(())
            } else {
                Err(Error::UnknownWindow(window))
            };
        }
        self.backend.present(window, frame, region)
    }

    /// Reads back what the window currently shows, as opaque RGBA8 (alpha
    /// 255). On X11 this queries the server; regions covered by other
    /// windows may read back as undefined.
    pub fn read_pixels(&mut self, window: WindowId) -> Result<OwnedFrame, Error> {
        self.backend.read_pixels(window)
    }

    /// Starts a one-shot timer: [`Event::Timer`] fires once after `delay`.
    pub fn set_timeout(&mut self, delay: Duration) -> TimerId {
        self.state.timers.add(Instant::now(), delay, None)
    }

    /// Starts a repeating timer: [`Event::Timer`] fires every `period`
    /// (missed ticks are skipped, not queued). A zero period fires once per
    /// loop iteration.
    pub fn set_interval(&mut self, period: Duration) -> TimerId {
        self.state.timers.add(Instant::now(), period, Some(period))
    }

    /// Stops a timer; returns whether it was still active.
    pub fn cancel_timer(&mut self, timer: TimerId) -> bool {
        self.state.timers.cancel(timer)
    }

    /// Makes [`EventLoop::run`](crate::EventLoop::run) return, and
    /// [`EventLoop::pump_events`](crate::EventLoop::pump_events) report
    /// [`PumpStatus::Exit`](crate::PumpStatus::Exit), after the current
    /// iteration. Open windows stay open.
    pub fn exit(&mut self) {
        self.state.exit_requested = true;
    }

    /// A thread-safe handle that wakes the loop.
    pub fn proxy(&self) -> EventLoopProxy {
        EventLoopProxy {
            waker: std::sync::Arc::clone(&self.state.waker),
        }
    }
}

impl std::fmt::Debug for Context<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("backend", &self.backend.kind())
            .finish_non_exhaustive()
    }
}
