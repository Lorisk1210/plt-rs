//! The windows of programs that handle their figures' input themselves
//! ([`WindowHost`]).

mod input;
#[cfg(test)]
mod tests;
mod toolbar;
mod window;

use std::path::{Path, PathBuf};
use std::time::Duration;

use pltrs_core::{Bbox, Rgba};
use pltrs_platform::{
    Context, CursorIcon, Event, EventLoop, HeadlessHandle, LogicalSize, TimerId, WindowId,
};
use pltrs_ui::TitleBarPainter;

pub use input::{CanvasInput, InputKind};
pub use toolbar::{ToolImage, ToolItem};

use super::GuiError;
use crate::figure::Figure;
use toolbar::Toolbar;
use window::HostedWindow;

/// Identifies a window of a [`WindowHost`] from its creation to its
/// closing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HostedWindowId(u64);

impl HostedWindowId {
    /// The id whose number is `raw` (see [`HostedWindowId::to_raw`]).
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The window's number, unique in its host.
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

/// Identifies a timer of a [`WindowHost`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HostTimerId(u64);

impl HostTimerId {
    /// The id whose number is `raw` (see [`HostTimerId::to_raw`]).
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The timer's number, unique in its host.
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

/// A group of keyboard shortcuts for [`WindowHost::show_help`]: its
/// name, then what each shortcut does and its keys (`"Ctrl+S"`).
pub type ShortcutGroup = (String, Vec<(String, Vec<String>)>);

/// What happened in the windows of a [`WindowHost`].
#[derive(Clone, Debug, PartialEq)]
pub enum HostEvent {
    /// The canvas of `window` is `width × height` physical pixels at
    /// `scale_factor` physical pixels per logical pixel: when the window
    /// opens, is resized or moves to a screen of another density.
    Resized {
        /// The window.
        window: HostedWindowId,
        /// The canvas' width in physical pixels.
        width: u32,
        /// The canvas' height in physical pixels.
        height: u32,
        /// Physical pixels per logical pixel (matplotlib's
        /// `device_pixel_ratio`).
        scale_factor: f64,
    },
    /// Input over the canvas of `window`.
    Input {
        /// The window.
        window: HostedWindowId,
        /// What happened.
        input: CanvasInput,
    },
    /// The toolbar button `name` of `window` was clicked.
    ToolClicked {
        /// The window.
        window: HostedWindowId,
        /// The button's name ([`ToolItem::name`]).
        name: String,
    },
    /// The user asked to close `window`; it stays open until the program
    /// closes it ([`WindowHost::close`]).
    CloseRequested {
        /// The window.
        window: HostedWindowId,
    },
    /// The file chooser of `window` ([`WindowHost::choose_file`]) ended:
    /// with the file chosen, or `None` when it was cancelled.
    FileChosen {
        /// The window.
        window: HostedWindowId,
        /// The file, with the extension of the format chosen.
        path: Option<PathBuf>,
    },
    /// A timer started with [`WindowHost::start_timer`] fired.
    Timer(HostTimerId),
}

/// A timer of the program's.
#[derive(Clone, Copy, Debug)]
struct ProgramTimer {
    id: HostTimerId,
    platform: TimerId,
    repeat: bool,
}

/// The windows of a program handling its figures' input itself, as
/// matplotlib's backends do (pltrs (Python)'s `pltrs` backend), and the
/// event loop running them.
///
/// A [`FigureWindow`](super::FigureWindow) runs its figure on its own:
/// navigation, shortcuts and callbacks are pltrs (Rust)'s. A
/// `WindowHost` leaves them to the program. Each of its windows shows
/// what its figure's canvas holds (whatever the program drew or blitted
/// there) above a toolbar of the program's buttons ([`ToolItem`]) and a
/// status line, with the chrome of the current [look](crate::look); the
/// host reports what happens as [`HostEvent`]s — the canvas' size, the
/// pointer and keys over it in matplotlib's terms ([`CanvasInput`]), the
/// buttons clicked, the windows asked to close, the files chosen and the
/// timers that fired — which the program turns into matplotlib's events,
/// and the program asks for everything else: titles, sizes, cursors, the
/// zoom rectangle, checked buttons, messages, the file chooser of
/// `save_figure`, the keyboard shortcuts and timers.
///
/// A window is described before it is first shown, as matplotlib's
/// managers make their windows hidden: it opens on
/// [`show`](Self::show).
///
/// ```
/// use pltrs::figure::Figure;
/// use pltrs::gui::{HostEvent, WindowHost};
///
/// let figure = Figure::with_size(4.0, 3.0, 100.0);
/// let (mut host, _display) = WindowHost::headless();
/// let window = host.add_window(&figure, "Figure 1", (400.0, 300.0), false);
/// host.show(window)?;
/// let events = host.pump(Some(std::time::Duration::ZERO))?;
/// assert!(events.iter().any(|event| matches!(
///     event,
///     HostEvent::Resized { width: 400, height: 300, .. }
/// )));
/// # Ok::<(), pltrs::gui::GuiError>(())
/// ```
#[derive(Debug)]
pub struct WindowHost {
    event_loop: EventLoop,
    windows: Vec<HostedWindow>,
    timers: Vec<ProgramTimer>,
    /// Events from outside [`WindowHost::pump`] (a window opening), for
    /// the next pump to report first.
    pending: Vec<HostEvent>,
    next_window: u64,
    next_timer: u64,
}

impl WindowHost {
    /// A host on the system's window system. Where windows draw their own
    /// title bars (Wayland without server-side decorations), they take the
    /// chrome's look.
    ///
    /// # Errors
    ///
    /// When there is no window system to connect to.
    pub fn connect() -> Result<Self, GuiError> {
        let mut event_loop = EventLoop::new()?;
        let background = crate::rc::params()
            .figure_facecolor
            .resolve()
            .unwrap_or(Rgba::WHITE);
        let theme = crate::look::appearance().chrome.theme(background);
        event_loop.set_decoration_painter(TitleBarPainter::new(theme))?;
        Ok(Self::with_loop(event_loop))
    }

    /// A host without a display, driven through the returned handle (for
    /// tests and off-screen use).
    pub fn headless() -> (Self, HeadlessHandle) {
        let (event_loop, handle) = EventLoop::headless();
        (Self::with_loop(event_loop), handle)
    }

    fn with_loop(event_loop: EventLoop) -> Self {
        Self {
            event_loop,
            windows: Vec::new(),
            timers: Vec::new(),
            pending: Vec::new(),
            next_window: 0,
            next_timer: 0,
        }
    }

    /// Describes a window titled `title` for `figure`, its canvas `size`
    /// logical pixels `(width, height)`, with an empty toolbar when
    /// `toolbar` (see [`WindowHost::add_tool`]); it opens when first
    /// shown.
    pub fn add_window(
        &mut self,
        figure: &Figure,
        title: &str,
        (width, height): (f64, f64),
        toolbar: bool,
    ) -> HostedWindowId {
        self.next_window += 1;
        let id = HostedWindowId(self.next_window);
        let toolbar = toolbar.then(|| Toolbar::new(crate::look::appearance().chrome));
        let size = LogicalSize::new(width, height);
        self.windows
            .push(HostedWindow::new(id, figure, title, size, toolbar));
        id
    }

    fn window(&self, id: HostedWindowId) -> Result<&HostedWindow, GuiError> {
        self.windows
            .iter()
            .find(|window| window.id() == id)
            .ok_or(GuiError::NotShown)
    }

    /// Runs `change` on window `id` with the loop's context.
    fn with_window<R>(
        &mut self,
        id: HostedWindowId,
        change: impl FnOnce(&mut HostedWindow, &mut Context<'_>) -> Result<R, GuiError>,
    ) -> Result<R, GuiError> {
        change(find(&mut self.windows, id)?, &mut self.event_loop.context())
    }

    /// Opens window `id`, or raises it when it is open and
    /// `figure.raise_window` is on (matplotlib's `manager.show`).
    ///
    /// # Errors
    ///
    /// [`GuiError::NotShown`] for a window closed or never added, and when
    /// the window cannot be opened.
    pub fn show(&mut self, id: HostedWindowId) -> Result<(), GuiError> {
        find(&mut self.windows, id)?.show(&mut self.event_loop.context(), &mut self.pending)
    }

    /// Whether window `id` is open on screen.
    pub fn is_shown(&self, id: HostedWindowId) -> bool {
        self.window(id).is_ok_and(HostedWindow::is_shown)
    }

    /// The window system's window showing window `id`, once shown (to
    /// drive a [headless](WindowHost::headless) host's windows).
    pub fn window_system_id(&self, id: HostedWindowId) -> Option<WindowId> {
        self.window(id).ok().and_then(HostedWindow::window_id)
    }

    /// Closes window `id` and the windows it opened, and forgets it
    /// (matplotlib's `manager.destroy`); a window closed already is left
    /// alone.
    ///
    /// # Errors
    ///
    /// When the window system fails to close a window.
    pub fn close(&mut self, id: HostedWindowId) -> Result<(), GuiError> {
        match self.windows.iter().position(|window| window.id() == id) {
            Some(index) => self
                .windows
                .remove(index)
                .close(&mut self.event_loop.context()),
            None => Ok(()),
        }
    }

    /// The title of window `id`.
    ///
    /// # Errors
    ///
    /// [`GuiError::NotShown`] for a window closed or never added.
    pub fn title(&self, id: HostedWindowId) -> Result<&str, GuiError> {
        Ok(self.window(id)?.title())
    }

    /// Renames window `id` (matplotlib's `set_window_title`).
    ///
    /// # Errors
    ///
    /// [`GuiError::NotShown`] for a window closed or never added, and when
    /// the window system fails.
    pub fn set_title(&mut self, id: HostedWindowId, title: &str) -> Result<(), GuiError> {
        self.with_window(id, |window, cx| window.set_title(cx, title))
    }

    /// Asks for the canvas of window `id` to be `width × height` physical
    /// pixels (matplotlib's `manager.resize`): the window grows by its
    /// toolbar, and the program learns of the size the window system gave
    /// it as [`HostEvent::Resized`].
    ///
    /// # Errors
    ///
    /// As [`WindowHost::set_title`].
    pub fn resize(&mut self, id: HostedWindowId, width: u32, height: u32) -> Result<(), GuiError> {
        self.with_window(id, |window, cx| window.resize(cx, width, height))
    }

    /// Whether window `id` is full screen.
    pub fn is_fullscreen(&self, id: HostedWindowId) -> bool {
        self.window(id).is_ok_and(HostedWindow::is_fullscreen)
    }

    /// Makes window `id` full screen, or not (matplotlib's
    /// `full_screen_toggle`).
    ///
    /// # Errors
    ///
    /// As [`WindowHost::set_title`].
    pub fn set_fullscreen(&mut self, id: HostedWindowId, fullscreen: bool) -> Result<(), GuiError> {
        self.with_window(id, |window, cx| window.set_fullscreen(cx, fullscreen))
    }

    /// Shows `cursor` over window `id` (matplotlib's `set_cursor`).
    ///
    /// # Errors
    ///
    /// As [`WindowHost::set_title`].
    pub fn set_cursor(&mut self, id: HostedWindowId, cursor: CursorIcon) -> Result<(), GuiError> {
        self.with_window(id, |window, cx| window.set_cursor(cx, cursor))
    }

    /// Shows the zoom rectangle `rubberband` over the canvas of window
    /// `id`, in physical pixels from the canvas' top-left corner with its
    /// corners in any order, or hides it with `None` (matplotlib's
    /// `draw_rubberband` and `remove_rubberband`).
    ///
    /// # Errors
    ///
    /// As [`WindowHost::set_title`].
    pub fn set_rubberband(
        &mut self,
        id: HostedWindowId,
        rubberband: Option<Bbox>,
    ) -> Result<(), GuiError> {
        self.with_window(id, |window, cx| window.set_rubberband(cx, rubberband))
    }

    /// Adds the button `item` to the toolbar of window `id`, at `position`
    /// among the buttons of its group (at the end with `None`), unchecked
    /// and enabled (matplotlib's `add_toolitem`).
    ///
    /// # Errors
    ///
    /// [`GuiError::NotShown`] for a window closed, never added or without
    /// a toolbar, and when the toolbar cannot be updated.
    pub fn add_tool(
        &mut self,
        id: HostedWindowId,
        item: ToolItem,
        position: Option<usize>,
    ) -> Result<(), GuiError> {
        self.with_window(id, |window, cx| window.add_tool(cx, item, position))
    }

    /// Removes the toolbar buttons called `name` of window `id`; returns
    /// whether there was one (matplotlib's `remove_toolitem`).
    ///
    /// # Errors
    ///
    /// As [`WindowHost::set_title`].
    pub fn remove_tool(&mut self, id: HostedWindowId, name: &str) -> Result<bool, GuiError> {
        self.with_window(id, |window, cx| window.remove_tool(cx, name))
    }

    /// Checks or unchecks the toolbar buttons called `name` of window
    /// `id` (matplotlib's `toggle_toolitem`).
    ///
    /// # Errors
    ///
    /// As [`WindowHost::set_title`].
    pub fn set_tool_checked(
        &mut self,
        id: HostedWindowId,
        name: &str,
        checked: bool,
    ) -> Result<(), GuiError> {
        self.with_window(id, |window, cx| window.set_tool_checked(cx, name, checked))
    }

    /// Enables or disables the toolbar buttons called `name` of window
    /// `id` (the history buttons of `set_history_buttons`).
    ///
    /// # Errors
    ///
    /// As [`WindowHost::set_title`].
    pub fn set_tool_enabled(
        &mut self,
        id: HostedWindowId,
        name: &str,
        enabled: bool,
    ) -> Result<(), GuiError> {
        self.with_window(id, |window, cx| window.set_tool_enabled(cx, name, enabled))
    }

    /// The groups of the toolbar of window `id` and the names of their
    /// buttons, in order (none without a toolbar).
    ///
    /// # Errors
    ///
    /// As [`WindowHost::title`].
    pub fn toolbar_layout(
        &self,
        id: HostedWindowId,
    ) -> Result<Vec<(String, Vec<String>)>, GuiError> {
        Ok(self
            .window(id)?
            .toolbar()
            .map(Toolbar::layout)
            .unwrap_or_default())
    }

    /// Whether the toolbar button `name` of window `id` is checked, if
    /// there is one.
    ///
    /// # Errors
    ///
    /// As [`WindowHost::title`].
    pub fn tool_checked(&self, id: HostedWindowId, name: &str) -> Result<Option<bool>, GuiError> {
        Ok(self.window(id)?.toolbar().and_then(|t| t.checked(name)))
    }

    /// Shows `text` in the status line of window `id` (matplotlib's
    /// `set_message`).
    ///
    /// # Errors
    ///
    /// As [`WindowHost::set_title`].
    pub fn set_message(&mut self, id: HostedWindowId, text: &str) -> Result<(), GuiError> {
        self.with_window(id, |window, cx| window.set_message(cx, text))
    }

    /// The message of the status line of window `id`.
    ///
    /// # Errors
    ///
    /// As [`WindowHost::title`].
    pub fn message(&self, id: HostedWindowId) -> Result<String, GuiError> {
        Ok(self
            .window(id)?
            .toolbar()
            .map_or_else(String::new, |toolbar| toolbar.message().to_owned()))
    }

    /// Opens the file chooser of window `id` in `directory` (else in
    /// `savefig.directory`, or the working directory) proposing the file
    /// name `name` (whose extension picks the format), or raises it: the
    /// window takes no input until it ends, as [`HostEvent::FileChosen`]
    /// (matplotlib's `save_figure` dialog). It writes nothing.
    ///
    /// # Errors
    ///
    /// [`GuiError::NotShown`] when the window is not shown, and when the
    /// dialog cannot be opened.
    pub fn choose_file(
        &mut self,
        id: HostedWindowId,
        directory: Option<&Path>,
        name: &str,
    ) -> Result<(), GuiError> {
        self.with_window(id, |window, cx| window.choose_file(cx, directory, name))
    }

    /// Shows the keyboard shortcuts `groups` lists for window `id` (each
    /// group's name, then what each shortcut does and its keys), or raises
    /// them (matplotlib's help tool).
    ///
    /// # Errors
    ///
    /// As [`WindowHost::choose_file`].
    pub fn show_help(
        &mut self,
        id: HostedWindowId,
        groups: Vec<ShortcutGroup>,
    ) -> Result<(), GuiError> {
        self.with_window(id, |window, cx| window.show_help(cx, groups))
    }

    /// Puts what the canvas of window `id` shows on the clipboard, as a
    /// PNG image (matplotlib's copy tool).
    ///
    /// # Errors
    ///
    /// When the window is not shown, its canvas was never drawn, or the
    /// window system fails.
    pub fn copy_to_clipboard(&mut self, id: HostedWindowId) -> Result<(), GuiError> {
        self.with_window(id, HostedWindow::copy_to_clipboard)
    }

    /// Shows in window `id` what changed on its figure's canvas since it
    /// was last shown: all of it after a draw, the blitted regions after
    /// blits (matplotlib's `blit`); [`WindowHost::pump`] does so for every
    /// window.
    ///
    /// # Errors
    ///
    /// When the window cannot be updated.
    pub fn present(&mut self, id: HostedWindowId) -> Result<(), GuiError> {
        self.with_window(id, HostedWindow::present)
    }

    /// Starts a timer firing every `interval`, or once after it unless
    /// `repeat` ([`HostEvent::Timer`]).
    pub fn start_timer(&mut self, interval: Duration, repeat: bool) -> HostTimerId {
        self.next_timer += 1;
        let id = HostTimerId(self.next_timer);
        let mut cx = self.event_loop.context();
        let platform = if repeat {
            cx.set_interval(interval)
        } else {
            cx.set_timeout(interval)
        };
        self.timers.push(ProgramTimer {
            id,
            platform,
            repeat,
        });
        id
    }

    /// Stops timer `id`; returns whether it was running.
    pub fn stop_timer(&mut self, id: HostTimerId) -> bool {
        let Some(index) = self.timers.iter().position(|timer| timer.id == id) else {
            return false;
        };
        let timer = self.timers.remove(index);
        self.event_loop.context().cancel_timer(timer.platform);
        true
    }

    /// Shows what changed in every window, waits up to `timeout` (forever
    /// with `None`) for something to happen, and returns what happened,
    /// with what happened outside of pumping first.
    ///
    /// # Errors
    ///
    /// When the window system fails or a window cannot be updated.
    pub fn pump(&mut self, timeout: Option<Duration>) -> Result<Vec<HostEvent>, GuiError> {
        self.present_all()?;
        let mut events = std::mem::take(&mut self.pending);
        // Something to report already: look at what is queued, no more.
        let timeout = if events.is_empty() {
            timeout
        } else {
            Some(Duration::ZERO)
        };
        let mut failure = None;
        let (windows, timers) = (&mut self.windows, &mut self.timers);
        self.event_loop.pump_events(timeout, |cx, event| {
            if let Err(err) = dispatch(windows, timers, cx, event, &mut events) {
                failure.get_or_insert(err);
            }
        })?;
        match failure {
            Some(err) => Err(err),
            None => Ok(events),
        }
    }

    /// Shows what changed in every shown window.
    fn present_all(&mut self) -> Result<(), GuiError> {
        let mut cx = self.event_loop.context();
        self.windows
            .iter_mut()
            .try_for_each(|window| window.present(&mut cx))
    }
}

/// Window `id` of `windows`.
fn find(windows: &mut [HostedWindow], id: HostedWindowId) -> Result<&mut HostedWindow, GuiError> {
    windows
        .iter_mut()
        .find(|window| window.id() == id)
        .ok_or(GuiError::NotShown)
}

/// Delivers a platform event to the window or timer it belongs to,
/// putting what the program is to know in `events`.
fn dispatch(
    windows: &mut [HostedWindow],
    timers: &mut Vec<ProgramTimer>,
    cx: &mut Context<'_>,
    event: Event,
    events: &mut Vec<HostEvent>,
) -> Result<(), GuiError> {
    match event {
        Event::Timer(timer) => {
            if let Some(window) = windows.iter_mut().find(|w| w.owns_timer(timer)) {
                return window.tick(cx, timer);
            }
            if let Some(index) = timers.iter().position(|t| t.platform == timer) {
                let fired = timers[index];
                if !fired.repeat {
                    timers.remove(index);
                }
                events.push(HostEvent::Timer(fired.id));
            }
            Ok(())
        }
        Event::Window { window, event } => match windows.iter_mut().find(|w| w.owns_window(window))
        {
            Some(hosted) => hosted.handle(cx, window, &event, events),
            None => Ok(()),
        },
        _ => Ok(()),
    }
}
