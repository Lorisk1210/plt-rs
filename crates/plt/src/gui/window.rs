//! A figure in a window: the rendered figure above a toolbar with the
//! navigation tools and a coordinate readout (matplotlib's
//! `FigureManager` + `NavigationToolbar2`).

mod input;
mod toolbar;

use plt_core::Point;
use plt_platform::{
    Context, CursorIcon, LogicalSize, PhysicalSize, TimerId, WindowEvent, WindowId, WindowOptions,
};
use plt_ui::{Flex, Surface, SurfaceEvent, UiWindow, WidgetId};

use super::save_dialog::{Outcome, SaveDialog};
use super::{GuiError, icons};
use crate::events::{CursorShape, Event, EventSource, TimerKey};
use crate::figure::Figure;
use crate::gridspec::SubplotOverrides;
use crate::navigation::{NavMode, Navigation};
use crate::widgets::SubplotTool;
use toolbar::{Tool, Toolbar};

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

/// The subplot configuration tool of a figure window, in its own window.
#[derive(Debug)]
struct SubplotToolWindow {
    window: Box<FigureWindow>,
    /// Keeps the sliders connected while the window is open.
    _tool: SubplotTool,
}

/// A window showing one figure, with pan/zoom navigation.
#[derive(Debug)]
pub struct FigureWindow {
    figure: Figure,
    window: UiWindow<Message>,
    canvas: WidgetId,
    /// The toolbar; tool windows have none.
    toolbar: Option<Toolbar>,
    navigation: Navigation,
    /// Turns canvas input into figure events for connected callbacks.
    events: EventSource,
    /// The platform timers running the figure's timers.
    timers: Vec<(TimerId, TimerKey)>,
    /// The figure's resolution at scale factor 1.
    base_dpi: f64,
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
    subplot_tool: Option<SubplotToolWindow>,
    /// The save dialog opened from the toolbar, while it is open.
    save_dialog: Option<SaveDialog>,
}

impl FigureWindow {
    /// Opens a window titled `title` showing `figure` at its size, above a
    /// toolbar.
    ///
    /// # Errors
    ///
    /// When the window cannot be created.
    pub fn open(cx: &mut Context<'_>, figure: &Figure, title: &str) -> Result<Self, GuiError> {
        Self::build(cx, figure, title, true)
    }

    /// Opens a window showing `figure`, with or without the toolbar.
    fn build(
        cx: &mut Context<'_>,
        figure: &Figure,
        title: &str,
        with_toolbar: bool,
    ) -> Result<Self, GuiError> {
        let mut root = Flex::column();
        let canvas = root.push(Surface::new(|event| Some(Message::Canvas(event))), 1.0);
        let toolbar = with_toolbar.then(|| Toolbar::build(&mut root));
        let (width, height) = figure.size_inches();
        let dpi = figure.dpi();
        let toolbar_height = if with_toolbar { toolbar::HEIGHT } else { 0.0 };
        let options = WindowOptions::new(title)
            .with_app_id("plt-rs")
            .with_icon(icons::window_icon()?)
            .with_inner_size(LogicalSize::new(width * dpi, height * dpi + toolbar_height))
            .with_min_inner_size(LogicalSize::new(240.0, 160.0));
        let window = UiWindow::open(cx, &options, root)?;
        let mut figure_window = Self {
            figure: figure.clone(),
            window,
            canvas,
            toolbar,
            navigation: Navigation::new(),
            events: EventSource::new(),
            timers: Vec::new(),
            base_dpi: dpi,
            canvas_size: PhysicalSize::default(),
            held: None,
            pointer: None,
            cursor: None,
            fullscreen: false,
            title: title.to_owned(),
            subplot_tool: None,
            save_dialog: None,
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

    /// The figure shown.
    pub fn figure(&self) -> &Figure {
        &self.figure
    }

    /// Whether `window` is this window or a tool window it opened.
    pub fn owns_window(&self, window: WindowId) -> bool {
        self.id() == window
            || self
                .save_dialog
                .as_ref()
                .is_some_and(|dialog| dialog.id() == window)
            || self
                .subplot_tool
                .as_ref()
                .is_some_and(|tool| tool.window.owns_window(window))
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
                Outcome::Saved(path) => {
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
        if window != self.id() {
            if let Some(tool) = &mut self.subplot_tool
                && tool.window.owns_window(window)
                && !tool.window.handle(cx, window, event)?
            {
                self.close_subplot_tool(cx)?;
            }
            // The tool adjusts this window's figure.
            self.refresh(cx)?;
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
                self.set_status(cx, tool.map_or("", Tool::hint).to_owned())?;
                true
            }
            Message::Canvas(SurfaceEvent::Resized(size)) => {
                self.canvas_size = size;
                self.render(cx)?;
                self.figure.process_event(&Event::Resize {
                    width: f64::from(size.width),
                    height: f64::from(size.height),
                });
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
    /// the figure if it changed, here and in the tool windows. Call it when
    /// the figure may have been changed from elsewhere (another window's
    /// callbacks, a timer).
    ///
    /// # Errors
    ///
    /// When rendering a figure fails.
    pub fn refresh(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        self.sync_timers(cx);
        self.sync_cursor(cx)?;
        if self.figure.is_stale() {
            self.render(cx)?;
        }
        match &mut self.subplot_tool {
            Some(tool) => tool.window.refresh(cx),
            None => Ok(()),
        }
    }

    /// Whether platform timer `timer` belongs to this window or its tool
    /// windows.
    pub fn owns_timer(&self, timer: TimerId) -> bool {
        self.timers.iter().any(|(id, _)| *id == timer)
            || self
                .subplot_tool
                .as_ref()
                .is_some_and(|tool| tool.window.owns_timer(timer))
    }

    /// Runs the figure timer behind platform timer `timer` and redraws
    /// what the tick changed.
    ///
    /// # Errors
    ///
    /// When rendering a figure fails.
    pub fn tick(&mut self, cx: &mut Context<'_>, timer: TimerId) -> Result<(), GuiError> {
        match self.timers.iter().find(|(id, _)| *id == timer) {
            Some(&(_, key)) => {
                self.figure.fire_timer(key);
            }
            None => {
                if let Some(tool) = &mut self.subplot_tool {
                    tool.window.tick(cx, timer)?;
                }
            }
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
            Tool::Save => return self.save_figure(cx),
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
        self.refresh_tools(cx)
    }

    /// Opens a window with sliders for the figure's subplot parameters,
    /// or raises it when it is open (matplotlib's `configure_subplots`).
    fn configure_subplots(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        if let Some(tool) = &self.subplot_tool {
            cx.raise_window(tool.window.id())?;
            return Ok(());
        }
        let figure = Figure::with_size(6.0, 3.0, crate::rc::params().figure_dpi);
        let top = SubplotOverrides {
            top: Some(0.9),
            ..SubplotOverrides::default()
        };
        figure
            .subplots_adjust(&top)
            .expect("0.9 is a valid top margin");
        let tool = SubplotTool::new(&self.figure, &figure);
        let window = Self::build(cx, &figure, "Subplot configuration tool", false)?;
        self.subplot_tool = Some(SubplotToolWindow {
            window: Box::new(window),
            _tool: tool,
        });
        Ok(())
    }

    /// Opens the save dialog, or raises it when it is open (matplotlib's
    /// `save_figure`).
    fn save_figure(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        match &self.save_dialog {
            Some(dialog) => cx.raise_window(dialog.id())?,
            None => self.save_dialog = Some(SaveDialog::open(cx, &self.figure, &self.title)?),
        }
        Ok(())
    }

    fn close_save_dialog(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        match self.save_dialog.take() {
            Some(dialog) => dialog.close(cx),
            None => Ok(()),
        }
    }

    fn close_subplot_tool(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        match self.subplot_tool.take() {
            Some(tool) => tool.window.close(cx),
            None => Ok(()),
        }
    }

    /// Updates the toolbar buttons and the cursor.
    fn refresh_tools(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        if let Some(toolbar) = self.toolbar {
            toolbar.refresh(&mut self.window, cx, &self.navigation)?;
        }
        self.sync_cursor(cx)
    }

    /// Shows the cursor of the navigation mode, or the one the figure's
    /// widgets ask for.
    fn sync_cursor(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        let cursor = match self.navigation.mode() {
            NavMode::None => cursor_icon(self.figure.cursor()),
            NavMode::Pan => CursorIcon::Move,
            NavMode::Zoom => CursorIcon::Crosshair,
        };
        if self.cursor != Some(cursor) {
            cx.set_cursor(self.window.id(), cursor)?;
            self.cursor = Some(cursor);
        }
        Ok(())
    }

    /// Shows `text` in the status line, if there is a toolbar.
    fn set_status(&mut self, cx: &mut Context<'_>, text: String) -> Result<(), GuiError> {
        match self.toolbar {
            Some(toolbar) => toolbar.set_status(&mut self.window, cx, text),
            None => Ok(()),
        }
    }

    /// Renders the figure at the canvas size and shows it; artists that
    /// cannot be drawn are skipped and the first problem is shown in the
    /// status line.
    fn render(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        let size = self.canvas_size;
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let dpi = self.base_dpi * self.window.ui().scale_factor();
        self.figure
            .set_dpi(dpi)
            .set_size_inches(f64::from(size.width) / dpi, f64::from(size.height) / dpi);
        let mut canvas = self.figure.raster_canvas()?;
        let problem = self.figure.draw(&mut canvas).err();
        let image = canvas.into_pixmap();
        self.window
            .update(cx, self.canvas, |canvas: &mut Surface<Message>| {
                canvas.set_image(image);
            })?;
        if let Some(problem) = problem {
            self.set_status(cx, problem.to_string())?;
        }
        Ok(())
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
mod tests;
