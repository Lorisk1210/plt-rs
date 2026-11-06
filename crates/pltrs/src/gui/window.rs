//! A figure in a window: the rendered figure above a toolbar with the
//! navigation tools and a coordinate readout (matplotlib's
//! `FigureManager` + `NavigationToolbar2`).

mod canvas;
mod dialogs;
mod input;
mod screenshot;
mod toolbar;

use pltrs_core::Point;
use pltrs_platform::{
    Context, CursorIcon, LogicalSize, PhysicalSize, TimerId, WindowEvent, WindowId,
};
use pltrs_ui::{Flex, Surface, SurfaceEvent, UiWindow, WidgetId};

use super::help_dialog::HelpDialog;
use super::options::{self, OptionsDialog};
use super::save_dialog::{Outcome, SaveDialog};
use super::{GuiError, dialog, tools};
use crate::events::{CursorShape, Event, EventSource, TimerKey};
use crate::figure::{Figure, Presenting};
use crate::navigation::{NavMode, Navigation};
use dialogs::SubplotTool;
use toolbar::{Tool, Toolbar};

pub(super) use canvas::present_figure;
pub(super) use toolbar::{button_row, height as toolbar_height, one_line, push_status};

/// What the widgets of a figure window report.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Message {
    /// A toolbar button was clicked.
    Tool(Tool),
    /// The pointer moved onto (`Some`) or off a toolbar button.
    Hint(Option<Tool>),
    /// Something happened on the figure canvas.
    Canvas(SurfaceEvent),
}

/// A window showing one figure, with pan/zoom navigation.
#[derive(Debug)]
pub struct FigureWindow {
    figure: Figure,
    /// Keeps the figure's blits and draws recorded for the window.
    _presenting: Presenting,
    window: UiWindow<Message>,
    canvas: WidgetId,
    toolbar: Toolbar,
    navigation: Navigation,
    /// Turns canvas input into figure events for connected callbacks.
    events: EventSource,
    /// The platform timers running the figure's timers.
    timers: Vec<(TimerId, TimerKey)>,
    canvas_size: PhysicalSize,
    /// The `x` or `y` key held to constrain dragging.
    held: Option<char>,
    /// The last pointer position over the canvas, in display coordinates.
    pointer: Option<Point>,
    /// The cursor shown, once set.
    cursor: Option<CursorIcon>,
    /// Whether the window was made full screen.
    fullscreen: bool,
    title: String,
    /// The subplot tool opened from the toolbar, while it is open.
    subplot_tool: Option<SubplotTool>,
    /// The save dialog opened from the toolbar, while it is open.
    save_dialog: Option<SaveDialog>,
    /// The figure options dialog opened from the toolbar, while it is open.
    options_dialog: Option<OptionsDialog>,
    /// The keyboard shortcuts, while they are shown.
    help_dialog: Option<HelpDialog>,
    /// Whether `keymap.quit_all` closed the window, asking for every
    /// figure window to close.
    quit_all: bool,
}

impl FigureWindow {
    /// Opens a window titled `title` showing `figure` at its size, above a
    /// toolbar.
    ///
    /// # Errors
    ///
    /// When the window cannot be created.
    pub fn open(cx: &mut Context<'_>, figure: &Figure, title: &str) -> Result<Self, GuiError> {
        let mut root = Flex::column();
        // The canvas keeps its keys while a toolbar button has the focus.
        let canvas = Surface::new(|event| Some(Message::Canvas(event))).with_unused_keys();
        let canvas = root.push(canvas, 1.0);
        let look = crate::look::appearance().chrome;
        let toolbar = Toolbar::build(&mut root, figure.tools(), look);
        let (width, height) = figure.size_inches();
        // The window is sized in logical pixels.
        let dpi = figure.dpi() / figure.canvas().device_pixel_ratio();
        let options = dialog::window_options(title)
            .with_inner_size(LogicalSize::new(
                width * dpi,
                height * dpi + toolbar::height(),
            ))
            .with_min_inner_size(LogicalSize::new(240.0, 160.0));
        let mut window = UiWindow::open(cx, &options, root)?;
        window.ui_mut().set_theme(dialog::chrome_theme(figure));
        // Keys reach the figure from the start, as in matplotlib's windows.
        window.ui_mut().set_focus(Some(canvas));
        let mut figure_window = Self {
            figure: figure.clone(),
            _presenting: Presenting::new(figure),
            window,
            canvas,
            toolbar,
            navigation: Navigation::new(),
            events: EventSource::new(),
            timers: Vec::new(),
            canvas_size: PhysicalSize::default(),
            held: None,
            pointer: None,
            cursor: None,
            fullscreen: false,
            title: title.to_owned(),
            subplot_tool: None,
            save_dialog: None,
            options_dialog: None,
            help_dialog: None,
            quit_all: false,
        };
        figure_window.refresh_tools(cx)?;
        figure_window.sync_timers(cx);
        // The initial layout reports the canvas size: render the figure
        // before the first frame.
        for message in figure_window.window.take_messages() {
            figure_window.apply(cx, message)?;
        }
        Ok(figure_window)
    }

    /// The window.
    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Renames the window (matplotlib's `manager.set_window_title`).
    ///
    /// # Errors
    ///
    /// When the window system fails to rename it.
    pub fn retitle(&mut self, cx: &mut Context<'_>, title: &str) -> Result<(), GuiError> {
        if self.title != title {
            cx.set_title(self.id(), title)?;
            title.clone_into(&mut self.title);
        }
        Ok(())
    }

    /// Asks the window system for a canvas of `width × height` physical
    /// pixels, the window growing by its toolbar (matplotlib's
    /// `FigureManagerBase.resize`). The figure takes the new size, and
    /// resize callbacks run, once the window system has resized the
    /// window; it may refuse or adjust the size, as window managers do.
    ///
    /// # Errors
    ///
    /// When a side is zero or the window system fails.
    pub fn resize(
        &mut self,
        cx: &mut Context<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), GuiError> {
        let scale = cx.scale_factor(self.id())?;
        let size = LogicalSize::new(
            f64::from(width) / scale,
            f64::from(height) / scale + toolbar::height(),
        );
        cx.set_inner_size(self.id(), size)?;
        Ok(())
    }

    /// The figure shown.
    pub fn figure(&self) -> &Figure {
        &self.figure
    }

    /// Whether the window closed because `keymap.quit_all` asks for every
    /// figure window to close (matplotlib's `Gcf.destroy_all`).
    pub fn quit_all_requested(&self) -> bool {
        self.quit_all
    }

    /// Whether `window` is this window or a tool window it opened.
    pub fn owns_window(&self, window: WindowId) -> bool {
        self.id() == window
            || self
                .save_dialog
                .as_ref()
                .is_some_and(|dialog| dialog.id() == window)
            || self
                .options_dialog
                .as_ref()
                .is_some_and(|dialog| dialog.id() == window)
            || self
                .help_dialog
                .as_ref()
                .is_some_and(|dialog| dialog.id() == window)
            || self
                .subplot_tool
                .as_ref()
                .is_some_and(|tool| tool.dialog.id() == window)
    }

    /// Handles an event of this window or of a tool window it opened (see
    /// [`FigureWindow::owns_window`]); returns `false` once this window
    /// should close, which [`FigureWindow::close`] does.
    ///
    /// # Errors
    ///
    /// When rendering a figure or updating a window fails.
    pub fn handle(
        &mut self,
        cx: &mut Context<'_>,
        window: WindowId,
        event: &WindowEvent,
    ) -> Result<bool, GuiError> {
        if let Some(dialog) = self.save_dialog.as_mut().filter(|d| d.id() == window) {
            return match dialog.handle(cx, event)? {
                Outcome::Open => Ok(true),
                Outcome::Done(path) => {
                    self.close_save_dialog(cx)?;
                    self.set_status(cx, format!("Saved {}", path.display()))?;
                    Ok(true)
                }
                Outcome::Cancelled => {
                    self.close_save_dialog(cx)?;
                    Ok(true)
                }
            };
        }
        if let Some(dialog) = self.help_dialog.as_mut().filter(|d| d.id() == window) {
            if !dialog.handle(cx, event)? {
                self.close_help_dialog(cx)?;
            }
            return Ok(true);
        }
        if let Some(dialog) = self.options_dialog.as_mut().filter(|d| d.id() == window) {
            match dialog.handle(cx, event)? {
                options::Outcome::Open => {}
                options::Outcome::Applied {
                    limits_changed,
                    close,
                } => {
                    if limits_changed {
                        self.navigation.push_current(&self.figure);
                        self.refresh_tools(cx)?;
                    }
                    if close {
                        self.close_options_dialog(cx)?;
                    }
                    self.refresh(cx)?;
                }
                options::Outcome::Cancelled => self.close_options_dialog(cx)?,
            }
            return Ok(true);
        }
        if self
            .subplot_tool
            .as_ref()
            .is_some_and(|tool| tool.dialog.id() == window)
        {
            self.subplot_tool_event(cx, event)?;
            return Ok(true);
        }
        if window != self.id() {
            return Ok(true);
        }
        if *event == WindowEvent::CloseRequested {
            self.figure.process_event(&Event::Close);
            return Ok(false);
        }
        for message in self.window.handle(cx, event)? {
            if !self.apply(cx, message)? {
                self.figure.process_event(&Event::Close);
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Closes the window and the tool windows it opened, and stops the
    /// platform timers of its figure.
    ///
    /// # Errors
    ///
    /// When the window system fails to close a window.
    pub fn close(mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        self.close_subplot_tool(cx)?;
        self.close_save_dialog(cx)?;
        self.close_options_dialog(cx)?;
        self.close_help_dialog(cx)?;
        for (timer, _) in self.timers.drain(..) {
            cx.cancel_timer(timer);
        }
        cx.close_window(self.id())?;
        Ok(())
    }

    fn apply(&mut self, cx: &mut Context<'_>, message: Message) -> Result<bool, GuiError> {
        let open = match message {
            Message::Tool(tool) => {
                self.use_tool(cx, tool)?;
                true
            }
            Message::Hint(tool) => {
                let hint = tool.map_or_else(String::new, |tool| self.toolbar.hint(tool));
                self.set_status(cx, hint)?;
                true
            }
            Message::Canvas(SurfaceEvent::Resized(size)) => {
                // Programs learn of the new size before the figure is
                // drawn at it (matplotlib's `resize_event`, then a draw).
                self.canvas_size = size;
                self.fit_figure();
                self.announce_size();
                self.render(cx)?;
                true
            }
            Message::Canvas(SurfaceEvent::Input(event)) => self.canvas_input(cx, &event)?,
        };
        // Callbacks and tools may have changed the figure and its timers.
        if open {
            self.refresh(cx)?;
        }
        Ok(open)
    }

    /// Keeps platform timers running for the figure's timers and redraws
    /// the figure if it changed. Call it when the figure may have been
    /// changed from elsewhere (another window's callbacks, a timer).
    ///
    /// # Errors
    ///
    /// When rendering a figure fails.
    pub fn refresh(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        if self.figure.is_stale() {
            self.render(cx)?;
        } else {
            self.present(cx)?;
        }
        // Drawing may start animations.
        self.sync_timers(cx);
        // The program may have switched on a tool taking turns with pan
        // and zoom.
        if tools::yield_navigation(&self.figure, &mut self.navigation) {
            self.refresh_tools(cx)
        } else {
            self.sync_cursor(cx)
        }
    }

    /// Whether platform timer `timer` belongs to this window: its
    /// figure's timers and its tooltips.
    pub fn owns_timer(&self, timer: TimerId) -> bool {
        self.timers.iter().any(|(id, _)| *id == timer) || self.window.owns_timer(timer)
    }

    /// Runs the figure timer behind platform timer `timer` (or shows the
    /// tooltip it was waiting for) and redraws what the tick changed.
    ///
    /// # Errors
    ///
    /// When rendering a figure fails.
    pub fn tick(&mut self, cx: &mut Context<'_>, timer: TimerId) -> Result<(), GuiError> {
        if self.window.owns_timer(timer) {
            return Ok(self.window.handle_timer(cx, timer)?);
        }
        if let Some(&(_, key)) = self.timers.iter().find(|(id, _)| *id == timer) {
            self.figure.fire_timer(key);
        }
        self.refresh(cx)
    }

    /// Starts platform timers for new figure timers and cancels those of
    /// removed ones.
    fn sync_timers(&mut self, cx: &mut Context<'_>) {
        let wanted = self.figure.timers();
        self.timers.retain(|(id, key)| {
            let keep = wanted.iter().any(|(k, _)| k == key);
            if !keep {
                cx.cancel_timer(*id);
            }
            keep
        });
        for (key, interval) in wanted {
            if !self.timers.iter().any(|(_, k)| *k == key) {
                self.timers.push((cx.set_interval(interval), key));
            }
        }
    }

    fn use_tool(&mut self, cx: &mut Context<'_>, tool: Tool) -> Result<(), GuiError> {
        match tool {
            Tool::Home => self.navigation.home(&self.figure),
            Tool::Back => self.navigation.back(&self.figure),
            Tool::Forward => self.navigation.forward(&self.figure),
            Tool::Pan => return self.toggle_mode(cx, NavMode::Pan, "pan"),
            Tool::Zoom => return self.toggle_mode(cx, NavMode::Zoom, "zoom"),
            Tool::Subplots => return self.configure_subplots(cx),
            Tool::Options => return self.edit_parameters(cx),
            Tool::Save => return self.save_figure(cx),
            Tool::Help => return self.show_help(cx),
            Tool::User(index) => {
                let name = self.toolbar.user_tool(index).map(str::to_owned);
                if let Some(name) = name {
                    self.figure.trigger_tool(&name);
                }
            }
        }
        self.refresh_tools(cx)
    }

    /// Turns a navigation mode on or off, unless a widget holds the
    /// pointer.
    fn toggle_mode(
        &mut self,
        cx: &mut Context<'_>,
        mode: NavMode,
        name: &str,
    ) -> Result<(), GuiError> {
        if self.navigation.toggle(&self.figure, mode).is_err() {
            return self.set_status(cx, format!("{name} unavailable"));
        }
        tools::navigation_switched(&self.figure, &self.navigation);
        self.refresh_tools(cx)
    }

    /// Updates the toolbar buttons and the cursor, after pan or zoom gave
    /// way to a tool of their radio group switched on since, and shows the
    /// message a tool sent since ([`Figure::tool_message`]).
    fn refresh_tools(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        tools::yield_navigation(&self.figure, &mut self.navigation);
        let toggled: Vec<String> = self
            .figure
            .tool_names()
            .into_iter()
            .filter(|name| self.figure.tool_toggled(name) == Some(true))
            .collect();
        self.toolbar
            .refresh(&mut self.window, cx, &self.navigation, &toggled)?;
        if let Some(message) = self.figure.take_tool_message() {
            self.set_status(cx, message)?;
        }
        self.sync_cursor(cx)
    }

    /// Shows the cursor of the navigation mode over axes that navigate,
    /// as matplotlib does, else the one the figure's widgets ask for.
    fn sync_cursor(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        let in_mode = self
            .pointer
            .is_some_and(|at| self.navigation.shows_mode_cursor(&self.figure, at));
        let cursor = match self.navigation.mode() {
            NavMode::Pan if in_mode => CursorIcon::Move,
            NavMode::Zoom if in_mode => CursorIcon::Crosshair,
            _ => cursor_icon(tools::idle_cursor(&self.figure, self.pointer)),
        };
        if self.cursor != Some(cursor) {
            cx.set_cursor(self.window.id(), cursor)?;
            self.cursor = Some(cursor);
        }
        Ok(())
    }

    /// Shows `text` in the status line.
    fn set_status(&mut self, cx: &mut Context<'_>, text: String) -> Result<(), GuiError> {
        self.toolbar.set_status(&mut self.window, cx, text)
    }

    /// Puts the figure, as the window shows it, on the clipboard as a PNG
    /// image (matplotlib's `ToolCopyToClipboardBase`, bound to `keymap.copy`).
    fn copy_to_clipboard(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        let status = match self.figure.canvas().to_png() {
            Ok(png) => {
                cx.set_clipboard_image(self.window.id(), png)?;
                "Copied the figure to the clipboard".to_owned()
            }
            Err(problem) => problem.to_string(),
        };
        self.set_status(cx, status)
    }
}

/// The platform cursor of a cursor shape.
fn cursor_icon(shape: CursorShape) -> CursorIcon {
    match shape {
        CursorShape::Pointer => CursorIcon::Default,
        CursorShape::Hand => CursorIcon::Pointer,
        CursorShape::SelectRegion => CursorIcon::Crosshair,
        CursorShape::Move => CursorIcon::Move,
        CursorShape::Wait => CursorIcon::Wait,
        CursorShape::ResizeHorizontal => CursorIcon::ResizeHorizontal,
        CursorShape::ResizeVertical => CursorIcon::ResizeVertical,
    }
}

#[cfg(test)]
mod fuzz_tests;
#[cfg(test)]
mod keyboard_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tool_tests;
