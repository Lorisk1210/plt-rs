//! The platform-independent event loop: dispatch order, redraw coalescing,
//! timers, and the blocking / non-blocking entry points.
//!
//! # Dispatch contract
//!
//! Each loop *iteration*:
//!
//! 1. waits for platform input, but never past the earliest timer deadline
//!    (and does not wait at all if redraws or internal events are pending);
//! 2. dispatches every platform event in arrival order ([`Event::Window`],
//!    [`Event::WakeUp`]), a window's input events (pointer, buttons,
//!    wheel, keys) after the redraw the platform asked for before them
//!    (`Resized`, `ScaleFactorChanged`, expose), as toolkits paint a
//!    window when it is exposed: a program sees its figure drawn at the
//!    new size before the input that follows, however the platform's
//!    events were batched (matplotlib's `QtAgg` draws a shrunk window
//!    before the pointer it shrank away from leaves);
//! 3. dispatches every due [`Event::Timer`];
//! 4. dispatches at most one [`WindowEvent::RedrawRequested`] per window
//!    that still needs one (after `Resized`, `ScaleFactorChanged`, expose,
//!    or [`Context::request_redraw`]);
//! 5. dispatches [`Event::AboutToWait`].
//!
//! [`Event::Started`] is dispatched once before the first iteration, and
//! [`WindowEvent::Destroyed`] right after the handler action that closed a
//! window.

mod context;
mod timers;

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use context::Context;
pub use timers::TimerId;

use crate::backend::{Backend, BackendKind, Waker};
use crate::decoration::DecorationPainter;
use crate::error::Error;
use crate::event::{Event, WindowEvent};
use crate::headless::{HeadlessBackend, HeadlessHandle};
use crate::window::WindowId;
use timers::Timers;

/// The outcome of [`EventLoop::pump_events`] and [`EventLoop::run_until`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PumpStatus {
    /// The loop may be pumped again.
    Continue,
    /// [`Context::exit`] was called during this call.
    Exit,
}

/// Owns the connection to the windowing system and dispatches its events.
///
/// Create it on the main thread and keep it there: it is neither `Send`
/// nor `Sync`, because some platforms (macOS) only allow windowing on the
/// main thread. Use an [`EventLoopProxy`] to wake it from other threads.
///
/// Two ways to drive it:
///
/// * [`run`](Self::run) blocks until every window is closed or
///   [`Context::exit`] is called — matplotlib's `plt.show()`.
/// * [`pump_events`](Self::pump_events) / [`run_until`](Self::run_until)
///   process events for a bounded time and return — `plt.pause()` and
///   interactive mode.
///
/// ```
/// use pltrs_platform::{Event, EventLoop, WindowEvent, WindowOptions};
///
/// let (mut event_loop, handle) = EventLoop::headless();
/// event_loop.run(|cx, event| match event {
///     Event::Started => {
///         cx.create_window(&WindowOptions::new("demo")).unwrap();
///     }
///     Event::Window { window, event: WindowEvent::RedrawRequested } => {
///         let size = cx.inner_size(window).unwrap();
///         let pixels = vec![255; (size.width * size.height * 4) as usize];
///         let frame = pltrs_platform::Frame::new(size.width, size.height, &pixels).unwrap();
///         cx.present(window, &frame).unwrap();
///         cx.close_window(window).unwrap();
///     }
///     _ => {}
/// })?;
/// assert_eq!(handle.take_frames().len(), 1);
/// # Ok::<(), pltrs_platform::Error>(())
/// ```
pub struct EventLoop {
    backend: Box<dyn Backend>,
    state: LoopState,
}

/// Loop bookkeeping shared by [`EventLoop`] and [`Context`].
#[derive(Debug)]
struct LoopState {
    started: bool,
    exit_requested: bool,
    timers: Timers,
    /// Windows awaiting a `RedrawRequested`, in request order, no
    /// duplicates.
    redraws: Vec<WindowId>,
    /// The windows of `redraws` whose redraw the platform asked for (an
    /// expose, a resize, a new scale factor), dispatched before their next
    /// input event.
    exposed: Vec<WindowId>,
    /// Events produced by handler actions (e.g. `Destroyed`).
    pending: VecDeque<Event>,
    waker: Arc<dyn Waker>,
}

impl LoopState {
    fn new(waker: Arc<dyn Waker>) -> Self {
        Self {
            started: false,
            exit_requested: false,
            timers: Timers::default(),
            redraws: Vec::new(),
            exposed: Vec::new(),
            pending: VecDeque::new(),
            waker,
        }
    }

    fn schedule_redraw(&mut self, window: WindowId) {
        if !self.redraws.contains(&window) {
            self.redraws.push(window);
        }
    }

    /// Schedules the redraw the platform asked for `window`'s contents.
    fn schedule_exposed(&mut self, window: WindowId) {
        self.schedule_redraw(window);
        if !self.exposed.contains(&window) {
            self.exposed.push(window);
        }
    }

    /// Takes `window`'s redraw if the platform asked for it.
    fn take_exposed(&mut self, window: WindowId) -> bool {
        let exposed = self.exposed.contains(&window);
        if exposed {
            self.exposed.retain(|&id| id != window);
            self.redraws.retain(|&id| id != window);
        }
        exposed
    }

    /// Drops the redraws of `window`, which closed.
    fn forget(&mut self, window: WindowId) {
        self.redraws.retain(|&id| id != window);
        self.exposed.retain(|&id| id != window);
    }

    fn take_exit(&mut self) -> bool {
        std::mem::take(&mut self.exit_requested)
    }

    /// Whether the next iteration must not block.
    fn has_pending_work(&self) -> bool {
        self.exit_requested || !self.redraws.is_empty() || !self.pending.is_empty()
    }
}

impl EventLoop {
    /// Connects to the best available windowing system (see
    /// [`BackendKind::detect`]): the native one on macOS and Windows. When
    /// the Wayland compositor cannot be reached (a stale
    /// `$WAYLAND_DISPLAY`) but `$DISPLAY` names an X server, it connects to
    /// that instead, unless `$PLTRS_PLATFORM` asked for Wayland.
    ///
    /// On macOS, call it on the main thread, as `AppKit` requires.
    pub fn new() -> Result<Self, Error> {
        let kind = BackendKind::detect().ok_or_else(|| {
            Error::BackendUnavailable(String::from(
                "no display server found ($WAYLAND_DISPLAY and $DISPLAY are unset)",
            ))
        })?;
        let connected = Self::with_backend(kind);
        let wayland_asked_for = std::env::var("PLTRS_PLATFORM")
            .is_ok_and(|name| name.eq_ignore_ascii_case(BackendKind::Wayland.name()));
        let x11 = std::env::var_os("DISPLAY").is_some_and(|name| !name.is_empty());
        if connected.is_err() && kind == BackendKind::Wayland && x11 && !wayland_asked_for {
            return Self::with_backend(BackendKind::X11).or(connected);
        }
        connected
    }

    /// Connects to a specific backend. For [`BackendKind::Headless`] prefer
    /// [`EventLoop::headless`], which also returns the test handle.
    ///
    /// The macOS backend must be created on the main thread, as `AppKit`
    /// requires.
    pub fn with_backend(kind: BackendKind) -> Result<Self, Error> {
        match kind {
            BackendKind::X11 => Self::x11(),
            BackendKind::Wayland => Self::wayland(),
            BackendKind::MacOs => Self::macos(),
            BackendKind::Windows => Self::windows(),
            BackendKind::Headless => Ok(Self::headless().0),
        }
    }

    #[cfg(target_os = "macos")]
    fn macos() -> Result<Self, Error> {
        Ok(Self::from_backend(Box::new(
            crate::macos::MacBackend::connect()?,
        )))
    }

    #[cfg(not(target_os = "macos"))]
    fn macos() -> Result<Self, Error> {
        Err(Error::BackendUnavailable(String::from(
            "AppKit is only available on macOS",
        )))
    }

    #[cfg(windows)]
    fn windows() -> Result<Self, Error> {
        Ok(Self::from_backend(Box::new(
            crate::windows::WindowsBackend::connect()?,
        )))
    }

    #[cfg(not(windows))]
    fn windows() -> Result<Self, Error> {
        Err(Error::BackendUnavailable(String::from(
            "Win32 is only available on Windows",
        )))
    }

    #[cfg(target_os = "linux")]
    fn wayland() -> Result<Self, Error> {
        Ok(Self::from_backend(Box::new(
            crate::wayland::WaylandBackend::connect()?,
        )))
    }

    #[cfg(not(target_os = "linux"))]
    fn wayland() -> Result<Self, Error> {
        Err(Error::BackendUnavailable(String::from(
            "Wayland is only supported on Linux",
        )))
    }

    #[cfg(unix)]
    fn x11() -> Result<Self, Error> {
        Ok(Self::from_backend(Box::new(
            crate::x11::X11Backend::connect()?,
        )))
    }

    #[cfg(not(unix))]
    fn x11() -> Result<Self, Error> {
        Err(Error::BackendUnavailable(String::from(
            "X11 is only supported on Unix",
        )))
    }

    /// Creates an in-memory event loop for tests, plus the handle used to
    /// inject input and inspect presented frames.
    pub fn headless() -> (Self, HeadlessHandle) {
        let (backend, handle) = HeadlessBackend::new();
        (Self::from_backend(Box::new(backend)), handle)
    }

    fn from_backend(backend: Box<dyn Backend>) -> Self {
        let state = LoopState::new(backend.waker());
        Self { backend, state }
    }

    /// Which backend this loop drives.
    pub fn backend_kind(&self) -> BackendKind {
        self.backend.kind()
    }

    /// A handle for creating windows, presenting frames, etc. outside of
    /// an event handler (e.g. from `plt.figure()` in interactive mode).
    pub fn context(&mut self) -> Context<'_> {
        Context::new(&mut *self.backend, &mut self.state)
    }

    /// Installs the painter of client-side window decorations, which
    /// Wayland compositors without server-side decorations (GNOME's) leave
    /// to applications; see [`DecorationPainter`]. Other backends ignore
    /// it.
    pub fn set_decoration_painter(
        &mut self,
        painter: impl DecorationPainter + 'static,
    ) -> Result<(), Error> {
        self.backend.set_decoration_painter(Box::new(painter))
    }

    /// A thread-safe handle that wakes this loop.
    pub fn proxy(&self) -> EventLoopProxy {
        EventLoopProxy {
            waker: Arc::clone(&self.state.waker),
        }
    }

    /// Dispatches events until every window has been closed or
    /// [`Context::exit`] is called (`plt.show()` semantics).
    ///
    /// Returns immediately (after [`Event::Started`], if not yet
    /// delivered) when no window is open. The loop can be run again
    /// afterwards.
    pub fn run<F>(&mut self, mut handler: F) -> Result<(), Error>
    where
        F: FnMut(&mut Context<'_>, Event),
    {
        self.start(&mut handler);
        loop {
            if self.state.take_exit() || self.backend.window_ids().is_empty() {
                return Ok(());
            }
            self.iterate(None, &mut handler)?;
        }
    }

    /// Runs one loop iteration: waits at most `timeout` for new input
    /// (`None`: wait indefinitely; `Some(Duration::ZERO)`: do not wait),
    /// dispatches everything available, and returns.
    pub fn pump_events<F>(
        &mut self,
        timeout: Option<Duration>,
        mut handler: F,
    ) -> Result<PumpStatus, Error>
    where
        F: FnMut(&mut Context<'_>, Event),
    {
        self.start(&mut handler);
        if !self.state.exit_requested {
            // A timeout too large to represent waits indefinitely.
            let deadline = timeout.and_then(|timeout| Instant::now().checked_add(timeout));
            self.iterate(deadline, &mut handler)?;
        }
        Ok(self.status())
    }

    /// Dispatches events until `deadline` passes or [`Context::exit`] is
    /// called (`plt.pause(interval)` semantics: windows stay responsive
    /// while the caller waits).
    pub fn run_until<F>(&mut self, deadline: Instant, mut handler: F) -> Result<PumpStatus, Error>
    where
        F: FnMut(&mut Context<'_>, Event),
    {
        self.start(&mut handler);
        while !self.state.exit_requested && Instant::now() < deadline {
            self.iterate(Some(deadline), &mut handler)?;
        }
        Ok(self.status())
    }

    fn status(&mut self) -> PumpStatus {
        if self.state.take_exit() {
            PumpStatus::Exit
        } else {
            PumpStatus::Continue
        }
    }

    fn start<F>(&mut self, handler: &mut F)
    where
        F: FnMut(&mut Context<'_>, Event),
    {
        if !self.state.started {
            self.state.started = true;
            self.dispatch(handler, Event::Started);
        }
    }

    /// One loop iteration; see the module documentation for the order.
    fn iterate<F>(&mut self, deadline: Option<Instant>, handler: &mut F) -> Result<(), Error>
    where
        F: FnMut(&mut Context<'_>, Event),
    {
        let now = Instant::now();
        let wait_until = if self.state.has_pending_work() {
            Some(now)
        } else {
            earliest(deadline, self.state.timers.next_deadline())
        };
        let mut events = Vec::new();
        self.backend.poll(wait_until, &mut events)?;
        for event in events {
            self.deliver(handler, event);
        }
        let now = Instant::now();
        for timer in self.state.timers.due(now) {
            if self.state.timers.fire(timer, now) {
                self.dispatch(handler, Event::Timer(timer));
            }
        }
        self.state.exposed.clear();
        for window in std::mem::take(&mut self.state.redraws) {
            if self.backend.has_window(window) {
                let event = WindowEvent::RedrawRequested;
                self.dispatch(handler, Event::Window { window, event });
            }
        }
        self.dispatch(handler, Event::AboutToWait);
        Ok(())
    }

    /// Routes one backend event: redraw requests are coalesced (but those
    /// of the platform come before the window's next input event), events
    /// for windows closed earlier in this iteration are dropped.
    fn deliver<F>(&mut self, handler: &mut F, event: Event)
    where
        F: FnMut(&mut Context<'_>, Event),
    {
        if let Event::Window {
            window,
            event: ref window_event,
        } = event
        {
            if !self.backend.has_window(window) && *window_event != WindowEvent::Destroyed {
                return;
            }
            match window_event {
                WindowEvent::RedrawRequested => {
                    self.state.schedule_exposed(window);
                    return;
                }
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged(_) => {
                    self.state.schedule_exposed(window);
                }
                input if is_input(input) => self.redraw_exposed(handler, window),
                _ => {}
            }
        }
        self.dispatch(handler, event);
    }

    /// Dispatches `window`'s redraw now if the platform asked for it.
    fn redraw_exposed<F>(&mut self, handler: &mut F, window: WindowId)
    where
        F: FnMut(&mut Context<'_>, Event),
    {
        if self.state.take_exposed(window) {
            let event = WindowEvent::RedrawRequested;
            self.dispatch(handler, Event::Window { window, event });
        }
    }

    /// Calls the handler, then flushes events its actions produced.
    fn dispatch<F>(&mut self, handler: &mut F, event: Event)
    where
        F: FnMut(&mut Context<'_>, Event),
    {
        let mut next = Some(event);
        while let Some(event) = next {
            handler(&mut self.context(), event);
            next = self.state.pending.pop_front();
        }
    }
}

impl std::fmt::Debug for EventLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventLoop")
            .field("backend", &self.backend.kind())
            .field("state", &self.state)
            .finish()
    }
}

/// Whether `event` is the user's input: the pointer, a button, the wheel
/// or a key.
fn is_input(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::CursorEntered { .. }
            | WindowEvent::CursorLeft { .. }
            | WindowEvent::CursorMoved { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::Key(_)
    )
}

fn earliest(a: Option<Instant>, b: Option<Instant>) -> Option<Instant> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// Wakes an [`EventLoop`] from any thread, making it deliver
/// [`Event::WakeUp`].
///
/// Combine it with a channel to send data to the loop: push the message,
/// then call [`wake_up`](Self::wake_up).
#[derive(Clone)]
pub struct EventLoopProxy {
    waker: Arc<dyn Waker>,
}

impl EventLoopProxy {
    /// Interrupts the loop's wait. Fails with [`Error::EventLoopClosed`]
    /// once the loop was dropped.
    pub fn wake_up(&self) -> Result<(), Error> {
        self.waker.wake()
    }
}

impl std::fmt::Debug for EventLoopProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventLoopProxy").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earliest_deadline() {
        let t = Instant::now();
        let later = t + Duration::from_secs(1);
        assert_eq!(earliest(None, None), None);
        assert_eq!(earliest(Some(t), None), Some(t));
        assert_eq!(earliest(None, Some(later)), Some(later));
        assert_eq!(earliest(Some(later), Some(t)), Some(t));
    }

    #[test]
    fn proxy_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync + Clone>() {}
        assert_send_sync::<EventLoopProxy>();
    }
}
