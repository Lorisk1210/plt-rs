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

use super::GuiError;
use super::actions;
use crate::events::{Event, EventSource, TimerKey};
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
    title: String,
    /// The subplot tool opened from the toolbar, while it is open.
    subplot_tool: Option<SubplotToolWindow>,
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
            title: title.to_owned(),
            subplot_tool: None,
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
            Tool::Pan => self.navigation.toggle(NavMode::Pan),
            Tool::Zoom => self.navigation.toggle(NavMode::Zoom),
            Tool::Subplots => return self.configure_subplots(cx),
            Tool::Save => {
                let message = match actions::save(&self.figure, &self.title) {
                    Ok(path) => format!("Saved {}", path.display()),
                    Err(err) => format!("Could not save the figure: {err}"),
                };
                return self.set_status(cx, message);
            }
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
        let cursor = match self.navigation.mode() {
            NavMode::None => CursorIcon::Default,
            NavMode::Pan => CursorIcon::Move,
            NavMode::Zoom => CursorIcon::Crosshair,
        };
        cx.set_cursor(self.window.id(), cursor)?;
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use plt_platform::{
        ButtonState, Event, EventLoop, HeadlessHandle, Key, KeyEvent, Modifiers, MouseButton,
        PhysicalPosition,
    };

    use super::*;

    /// Runs one loop iteration, feeding the events of `window` and its
    /// tool windows to it.
    fn pump(event_loop: &mut EventLoop, window: &mut FigureWindow) -> bool {
        let mut open = true;
        event_loop
            .pump_events(Some(Duration::ZERO), |cx, event| {
                if let Event::Window { window: id, event } = event
                    && window.owns_window(id)
                {
                    open &= window
                        .handle(cx, id, &event)
                        .expect("the window handles events");
                }
            })
            .expect("the headless loop runs");
        open
    }

    fn mouse(handle: &HeadlessHandle, id: WindowId, state: ButtonState, (x, y): (f64, f64)) {
        let position = PhysicalPosition::new(x, y);
        handle
            .inject(
                id,
                WindowEvent::CursorMoved {
                    position,
                    modifiers: Modifiers::NONE,
                },
            )
            .expect("the window exists");
        let click = WindowEvent::MouseInput {
            button: MouseButton::Left,
            state,
            position,
            modifiers: Modifiers::NONE,
            click_count: 1,
        };
        handle.inject(id, click).expect("the window exists");
    }

    fn key(handle: &HeadlessHandle, id: WindowId, c: char) {
        for state in [ButtonState::Pressed, ButtonState::Released] {
            let event = KeyEvent {
                key: Key::Character(c),
                state,
                repeat: false,
                text: None,
                modifiers: Modifiers::NONE,
                scancode: 0,
            };
            handle
                .inject(id, WindowEvent::Key(event))
                .expect("the window exists");
        }
    }

    #[test]
    fn shows_the_figure_and_pans_it() {
        let figure = Figure::with_size(4.0, 3.0, 100.0);
        let ax = figure.subplot();
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
        let presses = std::rc::Rc::new(std::cell::Cell::new(0));
        let counter = std::rc::Rc::clone(&presses);
        figure.connect(crate::events::EventKind::ButtonPress, move |event| {
            if event.location().is_some_and(|at| at.inaxes.is_some()) {
                counter.set(counter.get() + 1);
            }
        });
        let (mut event_loop, handle) = EventLoop::headless();
        let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
            .expect("the window opens");
        assert!(pump(&mut event_loop, &mut window));
        let frames = handle.take_frames();
        let frame = frames.last().expect("a frame is presented");
        // The canvas shows the white figure above the toolbar.
        assert_eq!(frame.image.pixel(5, 5), Some([255, 255, 255, 255]));
        assert_eq!(window.canvas_size, PhysicalSize::new(400, 300));

        let id = window.id();
        mouse(&handle, id, ButtonState::Pressed, (200.0, 150.0));
        mouse(&handle, id, ButtonState::Released, (200.0, 150.0));
        key(&handle, id, 'p');
        pump(&mut event_loop, &mut window);
        assert_eq!(window.navigation.mode(), NavMode::Pan);
        assert_eq!(presses.get(), 1, "callbacks see clicks in the axes");
        mouse(&handle, id, ButtonState::Pressed, (200.0, 150.0));
        mouse(&handle, id, ButtonState::Released, (231.0, 150.0));
        pump(&mut event_loop, &mut window);
        let (x0, x1) = ax.xlim();
        assert!(
            x0 < -0.5 && (x1 - x0 - 10.0).abs() < 1e-9,
            "panned to ({x0}, {x1})"
        );
        key(&handle, id, 'h');
        pump(&mut event_loop, &mut window);
        assert_eq!(ax.xlim(), (0.0, 10.0), "home restores the view");
        key(&handle, id, 'q');
        assert!(!pump(&mut event_loop, &mut window), "q closes the window");
    }

    #[test]
    fn the_subplot_tool_adjusts_the_figure() {
        let figure = Figure::with_size(4.0, 3.0, 100.0);
        figure.subplots(2, 2).expect("a 2×2 grid has cells");
        let (mut event_loop, handle) = EventLoop::headless();
        let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
            .expect("the window opens");
        let main = window.id();
        window
            .use_tool(&mut event_loop.context(), Tool::Subplots)
            .expect("the tool opens");
        let tool = window.subplot_tool.as_ref().expect("the tool is open");
        let tool_id = tool.window.id();
        let snapshot = handle.window(tool_id).expect("the tool window exists");
        assert_eq!(snapshot.title, "Subplot configuration tool");
        assert_eq!(snapshot.size, PhysicalSize::new(600, 300), "no toolbar");
        assert!(window.owns_window(tool_id));
        pump(&mut event_loop, &mut window);
        handle.take_frames();

        // Click 30% along the "left" slider.
        let tool = window.subplot_tool.as_ref().expect("the tool is open");
        let slider = tool.window.figure().axes()[0].position();
        let at = (
            600.0 * (slider.x0() + 0.3 * slider.width()),
            300.0 * (1.0 - (slider.y0() + 0.5 * slider.height())),
        );
        mouse(&handle, tool_id, ButtonState::Pressed, at);
        mouse(&handle, tool_id, ButtonState::Released, at);
        pump(&mut event_loop, &mut window);
        let left = figure.subplot_params().left;
        assert!((left - 0.3).abs() < 0.01, "left is {left}");
        let redrawn: Vec<_> = handle.take_frames().iter().map(|f| f.window).collect();
        assert!(redrawn.contains(&main), "the figure is redrawn");
        assert!(redrawn.contains(&tool_id), "the slider is redrawn");

        // Clicking the tool again raises its window.
        event_loop
            .context()
            .raise_window(main)
            .expect("the window exists");
        window
            .use_tool(&mut event_loop.context(), Tool::Subplots)
            .expect("the tool is raised");
        assert_eq!(handle.stacking_order(), [main, tool_id]);

        handle
            .inject(tool_id, WindowEvent::CloseRequested)
            .expect("the window exists");
        assert!(pump(&mut event_loop, &mut window));
        assert!(window.subplot_tool.is_none());
        assert_eq!(handle.windows(), [main]);
        window
            .close(&mut event_loop.context())
            .expect("the window closes");
        assert_eq!(handle.windows(), Vec::<WindowId>::new());
    }
}
