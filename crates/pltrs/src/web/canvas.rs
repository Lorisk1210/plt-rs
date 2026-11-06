//! One figure as its browser clients see it (matplotlib's
//! `FigureCanvasWebAggCore`, `NavigationToolbar2WebAgg` and
//! `FigureManagerWebAgg`): client messages become figure events and
//! navigation, and the canvas answers with events and PNG frames for every
//! client of the figure.

use std::time::Instant;

use pltrs_core::Point;

use super::frames::FrameEncoder;
use super::protocol::{
    ClientMessage, ImageMode, MouseInput, MouseKind, ServerEvent, ToolbarButton,
};
use super::schedule::TimerSchedule;
use crate::events::{CursorShape, Event, EventSource};
use crate::figure::{
    CanvasError, Figure, Presentation, Presenting, SaveError, SaveFormat, SaveOptions,
};
use crate::gui::actions;
use crate::gui::keys::{self, HistoryStep, KeyAction};
use crate::gui::tools;
use crate::navigation::{DragKey, NavMode, Navigation, Rubberband, axes_at};

/// The longest canvas side in physical pixels a client may ask for.
const MAX_SIDE: f64 = 8192.0;

/// The largest canvas area in physical pixels a client may ask for.
const MAX_AREA: f64 = 16_777_216.0;

/// The smallest device pixel ratio a client may ask for.
const MIN_PIXEL_RATIO: f64 = 0.25;

/// The largest device pixel ratio a client may ask for.
const MAX_PIXEL_RATIO: f64 = 8.0;

/// What a canvas sends its clients.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Output {
    /// A JSON event.
    Event(ServerEvent),
    /// A PNG frame.
    Frame(Vec<u8>),
}

/// What the server does after a message, besides sending the outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Next {
    /// Keep serving the figure.
    Continue,
    /// Close the figure (`keymap.quit`).
    Close,
    /// Close every figure (`keymap.quit_all`).
    CloseAll,
}

/// What the clients were last told, so that unchanged state is not sent
/// again.
#[derive(Debug, Default)]
struct Told {
    message: String,
    cursor: Option<CursorShape>,
    history: Option<(bool, bool)>,
    /// The canvas size in physical pixels.
    size: Option<(f64, f64)>,
    rubberband: bool,
}

/// A figure served to browsers.
#[derive(Debug)]
pub(crate) struct WebCanvas {
    figure: Figure,
    /// Keeps the figure's blits and draws recorded for the clients.
    _presenting: Presenting,
    title: String,
    navigation: Navigation,
    events: EventSource,
    frames: FrameEncoder,
    image_mode: ImageMode,
    told: Told,
    /// A `draw` event went out and the clients have not asked for the
    /// frame or acknowledged one since.
    draw_requested: bool,
    /// The last pointer position, in display pixels.
    pointer: Option<Point>,
    /// The key held, in matplotlib's naming.
    held: Option<String>,
    timers: TimerSchedule,
    outputs: Vec<Output>,
}

impl WebCanvas {
    /// Serves `figure` under `title`.
    pub(crate) fn new(figure: &Figure, title: &str) -> Self {
        let mut timers = TimerSchedule::default();
        timers.sync(figure, Instant::now());
        Self {
            figure: figure.clone(),
            _presenting: Presenting::new(figure),
            title: title.to_owned(),
            navigation: Navigation::new(),
            events: EventSource::new(),
            frames: FrameEncoder::new(),
            image_mode: ImageMode::Full,
            told: Told::default(),
            draw_requested: false,
            pointer: None,
            held: None,
            timers,
            outputs: Vec::new(),
        }
    }

    /// The figure.
    pub(crate) fn figure(&self) -> &Figure {
        &self.figure
    }

    /// Renames the figure; connected clients show the new title.
    pub(crate) fn set_title(&mut self, title: &str) {
        if self.title != title {
            title.clone_into(&mut self.title);
            self.push(ServerEvent::FigureLabel(self.title.clone()));
        }
    }

    /// The outputs produced since the last call.
    pub(crate) fn take_outputs(&mut self) -> Vec<Output> {
        std::mem::take(&mut self.outputs)
    }

    fn push(&mut self, event: ServerEvent) {
        self.outputs.push(Output::Event(event));
    }

    /// The canvas size in physical pixels.
    fn size(&self) -> (f64, f64) {
        self.figure.size_pixels()
    }

    /// Physical pixels per CSS pixel of the clients' display.
    fn device_pixel_ratio(&self) -> f64 {
        self.figure.canvas().device_pixel_ratio()
    }

    /// Tells the clients the canvas size in CSS pixels.
    fn tell_size(&mut self, forward: bool) {
        let (width, height) = self.size();
        let ratio = self.device_pixel_ratio();
        self.told.size = Some((width, height));
        self.push(ServerEvent::Resize {
            width: width / ratio,
            height: height / ratio,
            forward,
        });
    }

    /// A client connected (`add_web_socket`): it is told the canvas size,
    /// and asked for a draw if the figure changed.
    pub(crate) fn connected(&mut self) {
        self.tell_size(true);
        self.draw_requested = false;
    }

    /// Handles a message of a client.
    pub(crate) fn handle(&mut self, message: ClientMessage) -> Next {
        match message {
            ClientMessage::Refresh => self.refresh(),
            ClientMessage::Draw => {
                self.draw_requested = false;
                self.render();
            }
            ClientMessage::Ack => self.draw_requested = false,
            ClientMessage::SendImageMode => self.push(ServerEvent::ImageMode(self.image_mode)),
            ClientMessage::Resize { width, height } => self.resize(width, height),
            ClientMessage::SetDevicePixelRatio(ratio) => self.set_device_pixel_ratio(ratio),
            ClientMessage::Mouse(input) => {
                self.mouse(&input);
                self.sync_tools();
            }
            ClientMessage::Key { pressed, key } => {
                let next = self.key(pressed, key);
                self.sync_tools();
                return next;
            }
            ClientMessage::Toolbar(Some(button)) => {
                self.toolbar(button);
                self.sync_tools();
            }
            ClientMessage::Toolbar(None)
            | ClientMessage::SupportsBinary(_)
            | ClientMessage::Unknown => {}
        }
        Next::Continue
    }

    /// Asks the clients for a draw when the figure changed (matplotlib's
    /// `draw_idle`, which a stale figure triggers), or else sends them
    /// what the program drew or blitted on the figure's canvas since the
    /// last frame (matplotlib's `blit`, whose frame holds only the pixels
    /// that changed).
    pub(crate) fn idle(&mut self) {
        if self.figure.is_stale() {
            if !self.draw_requested {
                self.request_draw();
            }
        } else if self.figure.canvas().take_presentation() != Presentation::Nothing
            && let Err(problem) = self.send_frame()
        {
            self.set_message(problem);
        }
    }

    fn request_draw(&mut self) {
        self.draw_requested = true;
        self.push(ServerEvent::Draw);
    }

    /// Runs the figure's timers due at `now`.
    pub(crate) fn fire_timers(&mut self, now: Instant) {
        self.timers.fire_due(&self.figure, now);
    }

    /// When the next timer of the figure fires.
    pub(crate) fn next_timer(&self) -> Option<Instant> {
        self.timers.next_deadline()
    }

    /// The figure in `format`, saved as `savefig` saves it, at the
    /// resolution the figure was made at rather than the device pixel
    /// ratio's (matplotlib's `Download` handler).
    pub(crate) fn download(&self, format: SaveFormat) -> Result<Vec<u8>, SaveError> {
        self.figure.to_format(format, SaveOptions::default())
    }

    /// The client needs everything again (`handle_refresh`).
    fn refresh(&mut self) {
        self.push(ServerEvent::FigureLabel(self.title.clone()));
        self.frames.force_full();
        self.told.history = None;
        self.sync_history();
        // The wheel zooms only with a key held, as in matplotlib: the
        // page scrolls with it otherwise.
        self.push(ServerEvent::CaptureScroll(false));
        self.push(ServerEvent::NavigateMode(self.navigation.mode()));
        self.told.cursor = None;
        self.told.message.clear();
        self.request_draw();
    }

    /// Renders the figure and sends the frame (`get_diff_image`); problems
    /// drawing it go to the status line.
    fn render(&mut self) {
        let (width, height) = self.size();
        if self.told.size != Some((width, height)) {
            // The program resized the figure: the clients follow.
            self.tell_size(true);
        }
        if let Err(problem) = self.draw_frame() {
            self.set_message(problem);
        }
    }

    /// Draws the figure onto its canvas and queues the frame; artists
    /// that cannot be drawn are skipped and reported.
    fn draw_frame(&mut self) -> Result<(), String> {
        let figure_canvas = self.figure.canvas();
        let drawn = match figure_canvas.draw() {
            Err(CanvasError::Draw(problem)) => Err(problem.to_string()),
            Err(err) => return Err(err.to_string()),
            Ok(()) => Ok(()),
        };
        // The clients are sent the whole canvas now.
        figure_canvas.take_presentation();
        self.send_frame()?;
        drawn
    }

    /// Queues a frame of the figure's canvas as it is (matplotlib's
    /// `get_diff_image`).
    fn send_frame(&mut self) -> Result<(), String> {
        let encoded = self
            .figure
            .canvas()
            .with_pixels(|pixmap| {
                self.frames
                    .encode(pixmap.width(), pixmap.height(), pixmap.data())
            })
            .ok_or_else(|| CanvasError::NotDrawn.to_string())?;
        let (mode, png) = encoded.map_err(|err| err.to_string())?;
        if mode != self.image_mode {
            self.image_mode = mode;
            self.push(ServerEvent::ImageMode(mode));
        }
        self.outputs.push(Output::Frame(png));
        Ok(())
    }

    /// Gives the figure `width × height` physical pixels, within the
    /// limits a client may ask for.
    fn set_size(&mut self, width: f64, height: f64) {
        let (mut width, mut height) = (width.clamp(1.0, MAX_SIDE), height.clamp(1.0, MAX_SIDE));
        if width * height > MAX_AREA {
            let shrink = (MAX_AREA / (width * height)).sqrt();
            width = (width * shrink).floor().max(1.0);
            height = (height * shrink).floor().max(1.0);
        }
        let dpi = self.figure.dpi();
        // At a degenerate resolution the size is invalid: the figure keeps
        // its size.
        let _ = self.figure.set_size_inches(width / dpi, height / dpi);
    }

    /// The client's canvas is `width × height` CSS pixels
    /// (`handle_resize`): the figure takes that size.
    fn resize(&mut self, width: f64, height: f64) {
        let ratio = self.device_pixel_ratio();
        self.set_size(width.trunc() * ratio, height.trunc() * ratio);
        self.tell_size(false);
        let (width, height) = self.size();
        self.figure.process_event(&Event::Resize { width, height });
        self.request_draw();
    }

    /// Physical pixels per CSS pixel, within bounds and to two decimals
    /// as in matplotlib: the figure renders at that multiple of the
    /// resolution it was made at.
    fn set_device_pixel_ratio(&mut self, ratio: f64) {
        let ratio = ((ratio * 100.0).round() / 100.0).clamp(MIN_PIXEL_RATIO, MAX_PIXEL_RATIO);
        // A ratio that is not a number changes nothing.
        if !matches!(self.figure.canvas().set_device_pixel_ratio(ratio), Ok(true)) {
            return;
        }
        let (width, height) = self.size();
        if width > MAX_SIDE || height > MAX_SIDE || width * height > MAX_AREA {
            self.set_size(width, height);
        }
        self.frames.force_full();
        self.request_draw();
    }

    /// The constraint the held key or modifiers put on dragging.
    fn drag_key(&self, input: &MouseInput) -> DragKey {
        match self.held.as_deref() {
            Some("x") => DragKey::X,
            Some("y") => DragKey::Y,
            Some("control") => DragKey::Control,
            Some("shift") => DragKey::Shift,
            _ if input.ctrl => DragKey::Control,
            _ if input.shift => DragKey::Shift,
            _ => DragKey::None,
        }
    }

    /// Mouse input: figure events first, then navigation, as in the
    /// figure windows.
    fn mouse(&mut self, input: &MouseInput) {
        let figure = self.figure.clone();
        // The client measures from the top; matplotlib from the bottom of
        // the renderer's whole pixels.
        let height = self.size().1.trunc();
        let at = Point::new(input.x, height - input.y);
        self.pointer = Some(at);
        match input.kind {
            MouseKind::Press => {
                if let Some(button) = input.button {
                    self.events.button_press(&figure, at.x, at.y, button, false);
                    self.navigation.press(&figure, at, button);
                    match keys::history_step(button, &crate::rc::params()) {
                        Some(HistoryStep::Back) => self.toolbar(ToolbarButton::Back),
                        Some(HistoryStep::Forward) => self.toolbar(ToolbarButton::Forward),
                        None => {}
                    }
                }
            }
            MouseKind::DoubleClick => {
                if let Some(button) = input.button {
                    self.events.double_click(&figure, at.x, at.y, button);
                }
            }
            MouseKind::Release => {
                if let Some(button) = input.button {
                    self.events.button_release(&figure, at.x, at.y, button);
                }
                if self.navigation.is_dragging() {
                    self.navigation.release(&figure, at, self.drag_key(input));
                    self.show_rubberband(None);
                    self.sync_history();
                }
            }
            MouseKind::Motion => {
                self.events.motion(&figure, at.x, at.y);
                if self.navigation.is_dragging() {
                    let band = self.navigation.drag(at, self.drag_key(input));
                    self.show_rubberband(band);
                }
                let readout = self.navigation.readout(&figure, at);
                self.set_message(readout);
            }
            MouseKind::Scroll => {
                self.events.scroll(&figure, at.x, at.y, input.step);
                self.navigation
                    .wheel(&figure, at, input.wheel, self.drag_key(input));
                self.sync_history();
            }
            MouseKind::Enter => self.events.figure_enter(&figure, at.x, at.y),
            MouseKind::Leave => {
                self.pointer = None;
                self.events.figure_leave(&figure);
                self.set_message(String::new());
            }
        }
    }

    /// Shows or removes the zoom rectangle.
    fn show_rubberband(&mut self, band: Option<Rubberband>) {
        if band.is_some() || self.told.rubberband {
            self.told.rubberband = band.is_some();
            self.push(ServerEvent::Rubberband(band.map(|band| band.bbox)));
        }
    }

    /// A key: figure events first, then the figure's own tools and the
    /// `keymap.*` shortcuts, unless a widget captures the keys
    /// (matplotlib's `key_press_handler`).
    fn key(&mut self, pressed: bool, key: String) -> Next {
        let figure = self.figure.clone();
        let at = self.pointer.unwrap_or(Point::new(f64::NAN, f64::NAN));
        if !pressed {
            self.held = None;
            self.events.key_release(&figure, &key, at.x, at.y);
            return Next::Continue;
        }
        self.events.key_press(&figure, &key, at.x, at.y);
        let action = if figure.captures_keys() {
            None
        } else if let Some(tool) = figure.tool_for_key(&key) {
            // The figure's own tools come first, as they were bound on
            // purpose.
            figure.trigger_tool(&tool);
            None
        } else {
            keys::action(&key, &crate::rc::params())
        };
        self.held = Some(key);
        match action {
            // The browser has its own full screen, help and clipboard.
            None | Some(KeyAction::Fullscreen | KeyAction::Help | KeyAction::Copy) => {}
            Some(KeyAction::Quit) => return Next::Close,
            Some(KeyAction::QuitAll) => return Next::CloseAll,
            Some(
                action @ (KeyAction::Grid
                | KeyAction::GridMinor
                | KeyAction::LogX
                | KeyAction::LogY),
            ) => {
                if let Some(ax) = self.pointer.and_then(|at| axes_at(&figure, at)) {
                    actions::apply_to_axes(&ax, action);
                }
            }
            Some(KeyAction::Home) => self.toolbar(ToolbarButton::Home),
            Some(KeyAction::Back) => self.toolbar(ToolbarButton::Back),
            Some(KeyAction::Forward) => self.toolbar(ToolbarButton::Forward),
            Some(KeyAction::Pan) => self.toolbar(ToolbarButton::Pan),
            Some(KeyAction::Zoom) => self.toolbar(ToolbarButton::Zoom),
            // The browser downloads the figure in the chosen format.
            Some(KeyAction::Save) => self.push(ServerEvent::Save),
        }
        Next::Continue
    }

    /// A toolbar button (`handle_toolbar_button`).
    fn toolbar(&mut self, button: ToolbarButton) {
        let figure = self.figure.clone();
        match button {
            ToolbarButton::Home => self.navigation.home(&figure),
            ToolbarButton::Back => self.navigation.back(&figure),
            ToolbarButton::Forward => self.navigation.forward(&figure),
            ToolbarButton::Pan => return self.toggle(NavMode::Pan, "pan"),
            ToolbarButton::Zoom => return self.toggle(NavMode::Zoom, "zoom"),
        }
        self.sync_history();
    }

    /// Turns a navigation mode on or off, unless a widget holds the
    /// pointer.
    fn toggle(&mut self, mode: NavMode, name: &str) {
        if self.navigation.toggle(&self.figure, mode).is_err() {
            self.set_message(format!("{name} unavailable"));
        }
        tools::navigation_switched(&self.figure, &self.navigation);
        self.push(ServerEvent::NavigateMode(self.navigation.mode()));
    }

    /// Lets pan or zoom give way to a tool of their radio group switched
    /// on since, then updates the cursor.
    fn sync_tools(&mut self) {
        if tools::yield_navigation(&self.figure, &mut self.navigation) {
            self.push(ServerEvent::NavigateMode(self.navigation.mode()));
        }
        self.sync_cursor();
    }

    /// Updates the back and forward buttons if they changed.
    fn sync_history(&mut self) {
        let history = (
            self.navigation.can_go_back(),
            self.navigation.can_go_forward(),
        );
        if self.told.history != Some(history) {
            self.told.history = Some(history);
            self.push(ServerEvent::HistoryButtons {
                back: history.0,
                forward: history.1,
            });
        }
    }

    /// Shows the cursor of the navigation mode over navigable axes, or the
    /// one the figure's widgets ask for (`_update_cursor`).
    fn sync_cursor(&mut self) {
        let over_axes = self
            .pointer
            .and_then(|at| axes_at(&self.figure, at))
            .is_some_and(|ax| ax.navigate());
        let cursor = match (self.navigation.mode(), over_axes) {
            (NavMode::Pan, true) => CursorShape::Move,
            (NavMode::Zoom, true) => CursorShape::SelectRegion,
            _ => tools::idle_cursor(&self.figure, self.pointer),
        };
        if self.told.cursor != Some(cursor) {
            self.told.cursor = Some(cursor);
            self.push(ServerEvent::Cursor(cursor));
        }
    }

    /// Sets the status line if it changed (`set_message`).
    fn set_message(&mut self, text: String) {
        if self.told.message != text {
            self.told.message.clone_from(&text);
            self.push(ServerEvent::Message(text));
        }
    }
}

#[cfg(test)]
mod tests;
