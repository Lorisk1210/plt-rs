//! An interactive figure window: two subplots with lines, a scatter and
//! bars, shown with pan/zoom navigation.
//!
//! ```text
//! cargo run -p plt --example figure_window
//! cargo run -p plt --example figure_window -- --screenshot window.png
//! ```
//!
//! With `--screenshot PATH`, the window contents are saved as a PNG once
//! the figure has been shown, and the example quits.

#![expect(
    clippy::print_stdout,
    reason = "the example reports where the screenshot went"
)]

use std::error::Error;
use std::path::PathBuf;

use plt::axes::BarOptions;
use plt::figure::Figure;
use plt::gui::FigureWindow;
use plt::numeric::linspace;
use plt_platform::{Event, EventLoop, WindowEvent};

/// The demo figure.
fn demo() -> Result<Figure, Box<dyn Error>> {
    let fig = Figure::with_size(8.0, 4.0, 100.0);
    let grid = fig.subplots(1, 2)?;
    let (left, right) = (&grid[(0, 0)], &grid[(0, 1)]);
    let x = linspace(0.0, 10.0, 200);
    for phase in [0.0, 0.8, 1.6] {
        let y: Vec<f64> = x
            .iter()
            .map(|x| (x + phase).sin() * (-x / 8.0).exp())
            .collect();
        left.plot(x.clone(), y)?;
    }
    let dots: Vec<f64> = (0..20).map(|i| f64::from(i) * 0.5).collect();
    let heights: Vec<f64> = dots.iter().map(|x| (x * 1.3).cos() * 0.4).collect();
    left.scatter(dots, heights)?;
    left.set_title("Damped waves");
    left.set_xlabel("time").set_ylabel("amplitude");
    left.grid(true);
    right.bar(
        vec![1.0, 2.0, 3.0, 4.0],
        [3.0, 5.0, 2.0, 4.0],
        BarOptions::default(),
    )?;
    right.set_title("Bars");
    Ok(fig)
}

/// `--screenshot PATH`, if given.
fn screenshot_path() -> Result<Option<PathBuf>, Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    match (args.next(), args.next(), args.next()) {
        (None, _, _) => Ok(None),
        (Some(flag), Some(path), None) if flag == "--screenshot" => Ok(Some(path.into())),
        _ => Err("usage: figure_window [--screenshot PATH]".into()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let figure = demo()?;
    let mut screenshot = screenshot_path()?;
    if screenshot.is_none() {
        return Ok(plt::gui::show(&[figure])?);
    }
    let mut event_loop = EventLoop::new()?;
    let mut window: Option<FigureWindow> = None;
    let mut failure: Option<Box<dyn Error>> = None;
    event_loop.run(|cx, event| {
        let result: Result<bool, Box<dyn Error>> = match (event, window.as_mut()) {
            (Event::Started, _) => FigureWindow::open(cx, &figure, "Figure 1")
                .map(|opened| {
                    window = Some(opened);
                    true
                })
                .map_err(Into::into),
            (Event::Window { window: id, event }, Some(shown)) if id == shown.id() => shown
                .handle(cx, &event)
                .map_err(Into::into)
                .and_then(|open| {
                    if event == WindowEvent::RedrawRequested
                        && let Some(path) = screenshot.take()
                    {
                        let pixels = cx.read_pixels(id)?;
                        let size = pixels.size();
                        let png = plt_codec::png::Encoder::new().encode(
                            plt_codec::png::ImageRef::rgba8(
                                size.width,
                                size.height,
                                pixels.pixels(),
                            )?,
                        )?;
                        std::fs::write(&path, png)?;
                        println!("saved {}", path.display());
                        return Ok(false);
                    }
                    Ok(open)
                }),
            _ => Ok(true),
        };
        match result {
            Ok(true) => {}
            Ok(false) => cx.exit(),
            Err(err) => {
                failure = Some(err);
                cx.exit();
            }
        }
    })?;
    failure.map_or(Ok(()), Err)
}
