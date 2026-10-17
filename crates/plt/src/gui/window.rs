//! A figure in a window: the rendered figure above a toolbar with the
//! navigation tools and a coordinate readout (matplotlib's
//! `FigureManager` + `NavigationToolbar2`).

use plt_core::Point;
use plt_platform::{
    ButtonState, Context, CursorIcon, Key, KeyEvent, LogicalSize, Modifiers, MouseButton,
    PhysicalSize, ScrollDelta, TimerId, WindowEvent, WindowId, WindowOptions,
};
use plt_ui::{
    Flex, Label, Rect, Separator, Surface, SurfaceEvent, ToolButton, UiEvent, UiWindow, WidgetId,
};

use super::GuiError;
use super::actions;
use super::icons;
use super::keys::{self, KeyAction};
use crate::events::{Event, EventSource, TimerKey};
use crate::figure::Figure;
use crate::navigation::{DragKey, NavMode, Navigation, PointerButton, Rubberband};

/// Height of the toolbar in logical pixels.
const TOOLBAR_HEIGHT: f64 = 37.0;

/// A toolbar button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Tool {
    /// Restore the original view.
    Home,
    /// Previous view.
    Back,
    /// Next view.
    Forward,
    /// Toggle the pan tool.
    Pan,
    /// Toggle the zoom tool.
    Zoom,
    /// Save the figure as PNG.
    Save,
}

/// What the widgets of a figure window report.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Message {
    /// A toolbar button was clicked.
    Tool(Tool),
    /// Something happened on the figure canvas.
    Canvas(SurfaceEvent),
}

/// The toolbar buttons whose look follows the navigation state.
#[derive(Debug, Clone, Copy)]
struct ToolIds {
    back: WidgetId,
    forward: WidgetId,
    pan: WidgetId,
    zoom: WidgetId,
}

/// A window showing one figure, with pan/zoom navigation.
#[derive(Debug)]
pub struct FigureWindow {
    figure: Figure,
    window: UiWindow<Message>,
    canvas: WidgetId,
    status: WidgetId,
    tools: ToolIds,
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
}

impl FigureWindow {
    /// Opens a window titled `title` showing `figure` at its size.
    ///
    /// # Errors
    ///
    /// When the window cannot be created.
    pub fn open(cx: &mut Context<'_>, figure: &Figure, title: &str) -> Result<Self, GuiError> {
        let mut root = Flex::column();
        let canvas = root.push(Surface::new(|event| Some(Message::Canvas(event))), 1.0);
        let mut toolbar = Flex::row().with_padding(2.0).with_spacing(2.0);
        let button = |toolbar: &mut Flex<Message>, icon, tool| {
            toolbar.push(ToolButton::new(icon, Message::Tool(tool)), 0.0)
        };
        button(&mut toolbar, icons::home(), Tool::Home);
        let back = button(&mut toolbar, icons::back(), Tool::Back);
        let forward = button(&mut toolbar, icons::forward(), Tool::Forward);
        toolbar.push(Separator::vertical(), 0.0);
        let pan = button(&mut toolbar, icons::pan(), Tool::Pan);
        let zoom = button(&mut toolbar, icons::zoom(), Tool::Zoom);
        toolbar.push(Separator::vertical(), 0.0);
        button(&mut toolbar, icons::save(), Tool::Save);
        let status = toolbar.push(Label::new(""), 1.0);
        root.push(Separator::horizontal(), 0.0);
        root.push(toolbar, 0.0);
        let (width, height) = figure.size_inches();
        let dpi = figure.dpi();
        let options = WindowOptions::new(title)
            .with_app_id("plt-rs")
            .with_inner_size(LogicalSize::new(width * dpi, height * dpi + TOOLBAR_HEIGHT))
            .with_min_inner_size(LogicalSize::new(240.0, 160.0));
        let window = UiWindow::open(cx, &options, root)?;
        let mut figure_window = Self {
            figure: figure.clone(),
            window,
            canvas,
            status,
            tools: ToolIds {
                back,
                forward,
                pan,
                zoom,
            },
            navigation: Navigation::new(),
            events: EventSource::new(),
            timers: Vec::new(),
            base_dpi: dpi,
            canvas_size: PhysicalSize::default(),
            held: None,
            pointer: None,
            title: title.to_owned(),
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

    /// Handles an event of this window; returns `false` once the window
    /// should close.
    ///
    /// # Errors
    ///
    /// When rendering the figure or updating the window fails.
    pub fn handle(&mut self, cx: &mut Context<'_>, event: &WindowEvent) -> Result<bool, GuiError> {
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

    fn apply(&mut self, cx: &mut Context<'_>, message: Message) -> Result<bool, GuiError> {
        let open = match message {
            Message::Tool(tool) => {
                self.use_tool(cx, tool)?;
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
        self.sync_timers(cx);
        if open && self.figure.is_stale() {
            self.render(cx)?;
        }
        Ok(open)
    }

    /// Whether platform timer `timer` belongs to this window.
    pub fn owns_timer(&self, timer: TimerId) -> bool {
        self.timers.iter().any(|(id, _)| *id == timer)
    }

    /// Runs the figure timer behind platform timer `timer` and redraws if
    /// the tick changed the figure.
    ///
    /// # Errors
    ///
    /// When rendering the figure fails.
    pub fn tick(&mut self, cx: &mut Context<'_>, timer: TimerId) -> Result<(), GuiError> {
        if let Some(&(_, key)) = self.timers.iter().find(|(id, _)| *id == timer) {
            self.figure.fire_timer(key);
        }
        self.sync_timers(cx);
        if self.figure.is_stale() {
            self.render(cx)?;
        }
        Ok(())
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

    /// Display coordinates (from the bottom-left corner of the figure) of
    /// a canvas position (from its top-left corner).
    fn to_display(&self, position: Point) -> Point {
        Point::new(position.x, f64::from(self.canvas_size.height) - position.y)
    }

    fn drag_key(&self, modifiers: Modifiers) -> DragKey {
        match self.held {
            Some('x') => DragKey::X,
            Some('y') => DragKey::Y,
            _ if modifiers.control() => DragKey::Control,
            _ if modifiers.shift() => DragKey::Shift,
            _ => DragKey::None,
        }
    }

    fn canvas_input(&mut self, cx: &mut Context<'_>, event: &UiEvent) -> Result<bool, GuiError> {
        match event {
            UiEvent::PointerEnter(position) => {
                let at = self.to_display(*position);
                self.events.figure_enter(&self.figure, at.x, at.y);
            }
            UiEvent::PointerMove {
                position,
                modifiers,
            } => {
                let at = self.to_display(*position);
                self.pointer = Some(at);
                self.events.motion(&self.figure, at.x, at.y);
                if self.navigation.is_dragging() {
                    let rubberband = self.navigation.drag(at, self.drag_key(*modifiers));
                    self.show_rubberband(cx, rubberband)?;
                }
                let readout = self.navigation.readout(&self.figure, at);
                self.set_status(cx, readout)?;
            }
            UiEvent::PointerDown {
                button,
                position,
                click_count,
                ..
            } => {
                if let Some(button) = pointer_button(*button) {
                    let at = self.to_display(*position);
                    self.events
                        .button_press(&self.figure, at.x, at.y, button, *click_count == 2);
                    self.navigation.press(&self.figure, at, button);
                }
            }
            UiEvent::PointerUp {
                button,
                position,
                modifiers,
            } => {
                let at = self.to_display(*position);
                if let Some(button) = pointer_button(*button) {
                    self.events.button_release(&self.figure, at.x, at.y, button);
                }
                if self.navigation.is_dragging() {
                    self.navigation
                        .release(&self.figure, at, self.drag_key(*modifiers));
                    self.show_rubberband(cx, None)?;
                    self.refresh_tools(cx)?;
                }
            }
            UiEvent::Wheel {
                delta, position, ..
            } => {
                let steps = match *delta {
                    ScrollDelta::Lines { y, .. } => y,
                    ScrollDelta::Pixels { y, .. } => y / 40.0,
                };
                let at = self.to_display(*position);
                self.events.scroll(&self.figure, at.x, at.y, steps);
                self.navigation.wheel(&self.figure, at, steps);
                self.refresh_tools(cx)?;
            }
            UiEvent::PointerLeave => {
                self.pointer = None;
                self.events.figure_leave(&self.figure);
                self.set_status(cx, String::new())?;
            }
            UiEvent::Key(key) => return self.key(cx, key),
            _ => {}
        }
        Ok(true)
    }

    fn key(&mut self, cx: &mut Context<'_>, event: &KeyEvent) -> Result<bool, GuiError> {
        if let Key::Character(c @ ('x' | 'y')) = event.key {
            self.held = (event.state == ButtonState::Pressed).then_some(c);
        }
        let Some(name) = keys::key_name(event) else {
            return Ok(true);
        };
        let at = self.pointer.unwrap_or(Point::new(f64::NAN, f64::NAN));
        if event.state != ButtonState::Pressed {
            self.events.key_release(&self.figure, &name, at.x, at.y);
            return Ok(true);
        }
        self.events.key_press(&self.figure, &name, at.x, at.y);
        let rc = crate::rc::params();
        let Some(action) = keys::action(&name, &rc) else {
            return Ok(true);
        };
        match action {
            KeyAction::Quit => return Ok(false),
            KeyAction::Home => self.use_tool(cx, Tool::Home)?,
            KeyAction::Back => self.use_tool(cx, Tool::Back)?,
            KeyAction::Forward => self.use_tool(cx, Tool::Forward)?,
            KeyAction::Pan => self.use_tool(cx, Tool::Pan)?,
            KeyAction::Zoom => self.use_tool(cx, Tool::Zoom)?,
            KeyAction::Save => self.use_tool(cx, Tool::Save)?,
            KeyAction::Grid | KeyAction::GridMinor | KeyAction::LogX | KeyAction::LogY => {
                let target = self
                    .pointer
                    .and_then(|at| crate::navigation::axes_at(&self.figure, at));
                if let Some(ax) = target {
                    actions::apply_to_axes(&ax, action);
                }
            }
        }
        Ok(true)
    }

    /// Shows the zoom rectangle (display coordinates) on the canvas.
    fn show_rubberband(
        &mut self,
        cx: &mut Context<'_>,
        rubberband: Option<Rubberband>,
    ) -> Result<(), GuiError> {
        let height = f64::from(self.canvas_size.height);
        let selection = rubberband.map(|band| {
            let b = band.bbox;
            Rect::new(b.x0(), height - b.y0(), b.x1() - b.x0(), b.y0() - b.y1())
        });
        self.window
            .update(cx, self.canvas, |canvas: &mut Surface<Message>| {
                canvas.set_selection(selection);
            })?;
        Ok(())
    }

    /// Updates the history buttons, the checked tool and the cursor.
    fn refresh_tools(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        let mode = self.navigation.mode();
        let states = [
            (self.tools.back, self.navigation.can_go_back(), false),
            (self.tools.forward, self.navigation.can_go_forward(), false),
            (self.tools.pan, true, mode == NavMode::Pan),
            (self.tools.zoom, true, mode == NavMode::Zoom),
        ];
        for (id, enabled, checked) in states {
            self.window
                .update(cx, id, |button: &mut ToolButton<Message>| {
                    button.set_enabled(enabled);
                    button.set_checked(checked);
                })?;
        }
        let cursor = match mode {
            NavMode::None => CursorIcon::Default,
            NavMode::Pan => CursorIcon::Move,
            NavMode::Zoom => CursorIcon::Crosshair,
        };
        cx.set_cursor(self.window.id(), cursor)?;
        Ok(())
    }

    fn set_status(&mut self, cx: &mut Context<'_>, text: String) -> Result<(), GuiError> {
        self.window
            .update(cx, self.status, |label: &mut Label| label.set_text(text))?;
        Ok(())
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

/// The navigation button of a mouse button.
fn pointer_button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Left),
        MouseButton::Middle => Some(PointerButton::Middle),
        MouseButton::Right => Some(PointerButton::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use plt_platform::{Event, EventLoop, HeadlessHandle, PhysicalPosition};

    use super::*;

    /// Runs one loop iteration, feeding the window's events to `window`.
    fn pump(event_loop: &mut EventLoop, window: &mut FigureWindow) -> bool {
        let mut open = true;
        event_loop
            .pump_events(Some(Duration::ZERO), |cx, event| {
                if let Event::Window { window: id, event } = event
                    && id == window.id()
                {
                    open &= window
                        .handle(cx, &event)
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
}
