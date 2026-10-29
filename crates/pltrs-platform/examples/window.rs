//! Opens a resizable window showing an animated gradient that follows the
//! window size, and prints the input events it receives. Close the window
//! or press Escape to quit.
//!
//! ```text
//! cargo run -p pltrs-platform --example window
//! cargo run -p pltrs-platform --example window -- --screenshot window.png
//! ```
//!
//! With `--screenshot PATH`, the window contents are read back after one
//! second of animation, saved as a PNG, and the example quits.

#![expect(clippy::print_stdout, reason = "the example reports events on stdout")]

use std::error::Error;
use std::f64::consts::TAU;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use pltrs_codec::png::{Encoder, ImageRef};
use pltrs_platform::{
    ButtonState, Context, Event, EventLoop, Frame, Key, LogicalSize, PhysicalSize, WindowEvent,
    WindowId, WindowOptions,
};

/// Time between animation frames (about 60 per second).
const FRAME_INTERVAL: Duration = Duration::from_millis(16);
/// How long to animate before taking a screenshot.
const SCREENSHOT_DELAY: Duration = Duration::from_secs(1);

/// The example's state between events.
struct Demo {
    started: Instant,
    window: Option<WindowId>,
    /// The frame buffer, reused between frames.
    pixels: Vec<u8>,
    screenshot: Option<PathBuf>,
}

impl Demo {
    fn handle(&mut self, cx: &mut Context<'_>, event: Event) -> Result<(), Box<dyn Error>> {
        match event {
            Event::Started => {
                let options = WindowOptions::new("pltrs-platform demo")
                    .with_app_id("pltrs-platform-demo")
                    .with_inner_size(LogicalSize::new(640.0, 400.0))
                    .with_min_inner_size(LogicalSize::new(160.0, 100.0));
                let window = cx.create_window(&options)?;
                println!(
                    "{} window {window:?}, scale factor {}",
                    cx.backend_kind(),
                    cx.scale_factor(window)?
                );
                cx.set_interval(FRAME_INTERVAL);
                self.window = Some(window);
            }
            Event::Timer(_) => {
                if let Some(window) = self.window {
                    cx.request_redraw(window)?;
                }
            }
            Event::Window { window, event } => self.window_event(cx, window, event)?,
            _ => {}
        }
        Ok(())
    }

    fn window_event(
        &mut self,
        cx: &mut Context<'_>,
        window: WindowId,
        event: WindowEvent,
    ) -> Result<(), Box<dyn Error>> {
        match event {
            WindowEvent::RedrawRequested => self.redraw(cx, window)?,
            WindowEvent::CloseRequested => cx.close_window(window)?,
            WindowEvent::Key(key)
                if key.key == Key::Escape && key.state == ButtonState::Pressed =>
            {
                println!("Escape pressed: closing");
                cx.close_window(window)?;
            }
            WindowEvent::Destroyed => {
                println!("window closed");
                self.window = None;
            }
            event => println!("{event:?}"),
        }
        Ok(())
    }

    fn redraw(&mut self, cx: &mut Context<'_>, window: WindowId) -> Result<(), Box<dyn Error>> {
        let size = cx.inner_size(window)?;
        let seconds = self.started.elapsed().as_secs_f64();
        paint_gradient(&mut self.pixels, size, seconds);
        cx.present(window, &Frame::new(size.width, size.height, &self.pixels)?)?;
        if let Some(path) = self
            .screenshot
            .take_if(|_| seconds >= SCREENSHOT_DELAY.as_secs_f64())
        {
            let shown = cx.read_pixels(window)?;
            let png = Encoder::new().encode(ImageRef::rgba8(
                shown.size().width,
                shown.size().height,
                shown.pixels(),
            )?)?;
            std::fs::write(&path, png)?;
            println!("saved {}", path.display());
            cx.close_window(window)?;
        }
        Ok(())
    }
}

/// Fills `pixels` with an opaque gradient of `size` whose colour bands
/// drift with time.
fn paint_gradient(pixels: &mut Vec<u8>, size: PhysicalSize, seconds: f64) {
    let wave = |position: u32, extent: u32, speed: f64| {
        let phase = f64::from(position) / f64::from(extent.max(1)) + seconds * speed;
        0.5 + 0.5 * (TAU * phase).sin()
    };
    let columns: Vec<f64> = (0..size.width).map(|x| wave(x, size.width, 0.25)).collect();
    let rows: Vec<f64> = (0..size.height)
        .map(|y| wave(y, size.height, -0.15))
        .collect();
    pixels.clear();
    for &row in &rows {
        for &column in &columns {
            let blue = 1.0 - f64::midpoint(row, column);
            pixels.extend([to_byte(column), to_byte(row), to_byte(blue), u8::MAX]);
        }
    }
}

fn to_byte(intensity: f64) -> u8 {
    (intensity.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Parses the command line: nothing, or `--screenshot PATH`.
fn screenshot_path() -> Result<Option<PathBuf>, Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    match (args.next(), args.next(), args.next()) {
        (None, _, _) => Ok(None),
        (Some(flag), Some(path), None) if flag == "--screenshot" => Ok(Some(path.into())),
        _ => Err("usage: window [--screenshot PATH]".into()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut demo = Demo {
        started: Instant::now(),
        window: None,
        pixels: Vec::new(),
        screenshot: screenshot_path()?,
    };
    let mut event_loop = EventLoop::new()?;
    let mut failure = None;
    event_loop.run(|cx, event| {
        if let Err(error) = demo.handle(cx, event) {
            failure = Some(error);
            cx.exit();
        }
    })?;
    failure.map_or(Ok(()), Err)
}
