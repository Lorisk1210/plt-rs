//! A matplotlib-style window built from `plt-ui` alone: a canvas showing a
//! figure (drawn here with `plt-raster`) above a toolbar with a status
//! line. Moving the pointer over the axes shows data coordinates; toolbar
//! clicks are printed. Close the window or press Escape to quit.
//!
//! ```text
//! cargo run -p plt-ui --example canvas_window
//! cargo run -p plt-ui --example canvas_window -- --screenshot figure.png
//! ```
//!
//! With `--screenshot PATH`, the window contents are saved as a PNG once
//! the figure has been shown, and the example quits.

#![expect(
    clippy::print_stdout,
    reason = "the example reports toolbar clicks on stdout"
)]

use std::error::Error;
use std::f64::consts::TAU;
use std::path::PathBuf;

use plt_codec::png::{Encoder, ImageRef};
use plt_core::{Path, Point, Rgba};
use plt_platform::{
    ButtonState, Context, Event, EventLoop, Key, LogicalSize, PhysicalSize, WindowEvent,
    WindowOptions,
};
use plt_raster::{Antialias, Clip, Painter, Pixmap, Rgba8, StrokeStyle};
use plt_ui::{Button, Flex, Label, Spacing, Surface, SurfaceEvent, UiEvent, UiWindow, WidgetId};

/// Where matplotlib puts the axes by default, as fractions of the figure:
/// left, bottom, width, height.
const AXES: [f64; 4] = [0.125, 0.11, 0.775, 0.77];
/// Data limits of the plotted sine wave.
const X_RANGE: (f64, f64) = (0.0, TAU);
const Y_RANGE: (f64, f64) = (-1.1, 1.1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Home,
    Back,
    Forward,
    Pan,
    Zoom,
    Save,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Message {
    Tool(Tool),
    CanvasResized(PhysicalSize),
    PointerAt(Point),
    PointerLeft,
    Escape,
}

/// Turns canvas events into messages.
fn canvas_message(event: SurfaceEvent) -> Option<Message> {
    match event {
        SurfaceEvent::Resized(size) => Some(Message::CanvasResized(size)),
        SurfaceEvent::Input(UiEvent::PointerMove { position, .. }) => {
            Some(Message::PointerAt(position))
        }
        SurfaceEvent::Input(UiEvent::PointerLeave) => Some(Message::PointerLeft),
        SurfaceEvent::Input(UiEvent::Key(key))
            if key.key == Key::Escape && key.state == ButtonState::Pressed =>
        {
            Some(Message::Escape)
        }
        SurfaceEvent::Input(_) => None,
    }
}

struct FigureWindow {
    window: UiWindow<Message>,
    canvas: WidgetId,
    status: WidgetId,
    canvas_size: PhysicalSize,
    scale_factor: f64,
}

impl FigureWindow {
    fn open(cx: &mut Context<'_>) -> Result<Self, Box<dyn Error>> {
        let mut root = Flex::column();
        let canvas = root.push(Surface::new(canvas_message), 1.0);
        let mut toolbar = Flex::row()
            .with_padding(Spacing::Xs)
            .with_spacing(Spacing::Xs);
        for (label, tool) in [
            ("Home", Tool::Home),
            ("Back", Tool::Back),
            ("Forward", Tool::Forward),
            ("Pan", Tool::Pan),
            ("Zoom", Tool::Zoom),
            ("Save", Tool::Save),
        ] {
            toolbar.push(Button::new(label, Message::Tool(tool)), 0.0);
        }
        let status = toolbar.push(Label::new(""), 1.0);
        root.push(toolbar, 0.0);
        let options = WindowOptions::new("Figure 1")
            .with_app_id("plt-ui-figure")
            .with_inner_size(LogicalSize::new(640.0, 480.0))
            .with_min_inner_size(LogicalSize::new(320.0, 200.0));
        let window = UiWindow::open(cx, &options, root)?;
        let scale_factor = window.ui().scale_factor();
        let mut figure_window = Self {
            window,
            canvas,
            status,
            canvas_size: PhysicalSize::default(),
            scale_factor,
        };
        // The initial layout reports the canvas size: draw the figure
        // before the first frame.
        for message in figure_window.window.take_messages() {
            figure_window.apply(cx, message)?;
        }
        Ok(figure_window)
    }

    /// Handles a message; returns `false` when the window should close.
    fn apply(&mut self, cx: &mut Context<'_>, message: Message) -> Result<bool, Box<dyn Error>> {
        match message {
            Message::CanvasResized(size) => {
                self.canvas_size = size;
                self.scale_factor = self.window.ui().scale_factor();
                let figure = render_figure(size, self.scale_factor)?;
                self.window
                    .update(cx, self.canvas, |canvas: &mut Surface<Message>| {
                        canvas.set_image(figure);
                    })?;
            }
            Message::PointerAt(position) => {
                let text = data_position(position, self.canvas_size)
                    .map(|(x, y)| format!("x={x:.3}  y={y:.3}"))
                    .unwrap_or_default();
                self.set_status(cx, text)?;
            }
            Message::PointerLeft => self.set_status(cx, String::new())?,
            Message::Tool(tool) => {
                println!("toolbar: {tool:?}");
                self.set_status(cx, format!("{tool:?} clicked"))?;
            }
            Message::Escape => return Ok(false),
        }
        Ok(true)
    }

    fn set_status(&mut self, cx: &mut Context<'_>, text: String) -> Result<(), Box<dyn Error>> {
        self.window
            .update(cx, self.status, |label: &mut Label| label.set_text(text))?;
        Ok(())
    }
}

/// The axes rectangle in canvas pixels (y down).
fn axes_rect(size: PhysicalSize) -> (f64, f64, f64, f64) {
    let (width, height) = (f64::from(size.width), f64::from(size.height));
    let [left, bottom, w, h] = AXES;
    (
        left * width,
        (1.0 - bottom - h) * height,
        w * width,
        h * height,
    )
}

/// Data coordinates under a canvas position, if it is inside the axes.
fn data_position(position: Point, size: PhysicalSize) -> Option<(f64, f64)> {
    let (x0, y0, width, height) = axes_rect(size);
    let u = (position.x - x0) / width;
    let v = 1.0 - (position.y - y0) / height;
    let data = (
        X_RANGE.0 + u * (X_RANGE.1 - X_RANGE.0),
        Y_RANGE.0 + v * (Y_RANGE.1 - Y_RANGE.0),
    );
    ((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)).then_some(data)
}

/// Draws a white figure with axes and a sine wave.
fn render_figure(size: PhysicalSize, scale_factor: f64) -> Result<Pixmap, Box<dyn Error>> {
    let mut figure = Pixmap::filled(size.width, size.height, Rgba8::new(255, 255, 255, 255))?;
    let mut painter = Painter::new();
    let (x0, y0, width, height) = axes_rect(size);
    let to_pixels = |x: f64, y: f64| {
        let u = (x - X_RANGE.0) / (X_RANGE.1 - X_RANGE.0);
        let v = (y - Y_RANGE.0) / (Y_RANGE.1 - Y_RANGE.0);
        Point::new(x0 + u * width, y0 + (1.0 - v) * height)
    };
    let wave = (0..=200).map(|i| {
        let x = X_RANGE.0 + (X_RANGE.1 - X_RANGE.0) * f64::from(i) / 200.0;
        to_pixels(x, x.sin())
    });
    let blue = Rgba::new(0.122, 0.467, 0.706, 1.0);
    let line = StrokeStyle::new(1.5 * scale_factor * 96.0 / 72.0);
    painter.stroke_path(
        &mut figure,
        &Path::polyline(wave.collect()),
        blue,
        &line,
        Antialias::On,
        &Clip::NONE,
    )?;
    let frame = Path::polygon([
        Point::new(x0, y0),
        Point::new(x0 + width, y0),
        Point::new(x0 + width, y0 + height),
        Point::new(x0, y0 + height),
    ]);
    let black = Rgba::new(0.0, 0.0, 0.0, 1.0);
    let spine = StrokeStyle::new(0.8 * scale_factor * 96.0 / 72.0);
    painter.stroke_path(
        &mut figure,
        &frame,
        black,
        &spine,
        Antialias::On,
        &Clip::NONE,
    )?;
    Ok(figure)
}

/// Parses the command line: nothing, or `--screenshot PATH`.
fn screenshot_path() -> Result<Option<PathBuf>, Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    match (args.next(), args.next(), args.next()) {
        (None, _, _) => Ok(None),
        (Some(flag), Some(path), None) if flag == "--screenshot" => Ok(Some(path.into())),
        _ => Err("usage: canvas_window [--screenshot PATH]".into()),
    }
}

/// Saves what the window shows as a PNG.
fn save_screenshot(
    cx: &mut Context<'_>,
    app: &FigureWindow,
    path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    let shown = cx.read_pixels(app.window.id())?;
    let size = shown.size();
    let png = Encoder::new().encode(ImageRef::rgba8(size.width, size.height, shown.pixels())?)?;
    std::fs::write(path, png)?;
    println!("saved {}", path.display());
    Ok(())
}

/// Routes one event; returns `false` when the window should close.
fn handle(
    cx: &mut Context<'_>,
    app: &mut FigureWindow,
    event: &WindowEvent,
    screenshot: &mut Option<PathBuf>,
) -> Result<bool, Box<dyn Error>> {
    if *event == WindowEvent::CloseRequested {
        return Ok(false);
    }
    let has_figure = app.canvas_size.area() > 0;
    for message in app.window.handle(cx, event)? {
        if !app.apply(cx, message)? {
            return Ok(false);
        }
    }
    let figure_shown = *event == WindowEvent::RedrawRequested && has_figure;
    if figure_shown && let Some(path) = screenshot.take() {
        save_screenshot(cx, app, &path)?;
        return Ok(false);
    }
    Ok(true)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut screenshot = screenshot_path()?;
    let mut event_loop = EventLoop::new()?;
    let mut app: Option<FigureWindow> = None;
    let mut failure: Option<Box<dyn Error>> = None;
    event_loop.run(|cx, event| {
        let result = match (&event, app.as_mut()) {
            (Event::Started, _) => FigureWindow::open(cx).map(|window| {
                app = Some(window);
                true
            }),
            (Event::Window { window, event }, Some(state)) if *window == state.window.id() => {
                handle(cx, state, event, &mut screenshot)
            }
            _ => Ok(true),
        };
        let keep_open = result.unwrap_or_else(|error| {
            failure = Some(error);
            false
        });
        if !keep_open
            && let Some(state) = app.take()
            && let Err(error) = cx.close_window(state.window.id())
        {
            failure.get_or_insert(error.into());
        }
        if failure.is_some() {
            cx.exit();
        }
    })?;
    failure.map_or(Ok(()), Err)
}
