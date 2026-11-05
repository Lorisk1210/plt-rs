//! An interactive figure window: curves, an image and a time series in
//! three subplots, shown with pan/zoom navigation and a group of tools of
//! its own.
//!
//! ```text
//! cargo run -p pltrs --example figure_window
//! cargo run -p pltrs --example figure_window -- --screenshot window.png
//! cargo run -p pltrs --example figure_window -- --review DIR
//! cargo run -p pltrs --example figure_window -- --review-on-screen DIR
//! ```
//!
//! With `--screenshot PATH`, the window is drawn off screen (no display
//! server needed) by [`FigureWindow::screenshot`], saved as a PNG, and
//! the example quits.
//!
//! With `--review DIR`, every surface of the window chrome is drawn off
//! screen, captured by [`FigureWindow::capture`] and saved in `DIR` as
//! `LOOK-RATIOx-SURFACE.png`:
//! the window idle, with a button hovered, pressed and focused, panning
//! (with the pointer's coordinates) and zooming (with a rubber band); the
//! figure options on each tab, with the axes list open, on a datetime
//! axis (in UTC and in the `timezone` rcParam's Tokyo), and with a problem
//! in a field shown, on another tab and on other axes; the subplot tool; the save dialog, in a deep directory full of
//! files, without a file name and replacing a file; and the help, at the
//! top and scrolled to its end. Each is
//! drawn at pixel ratios 1 and 2, in the modern light and dark looks and
//! in matplotlib's. The dialogs open from the toolbar and the keyboard,
//! as a user would open them, and synthetic input drives them.
//!
//! With `--review-on-screen DIR`, the same surfaces show in windows of
//! the display, which open and close one after the other, and are saved
//! as their windows presented them, at the display's scale factor. So
//! are two things only windows on a display do: the window resized, and
//! closed by its shortcut (⌘W on macOS, Ctrl+W elsewhere). Synthetic
//! input reaches the windows as the platform hands them input; the
//! user's own pointer over them counts too.
//!
//! The figure and the window take pltrs' modern look; the `PLTRS_LOOK`,
//! `PLTRS_FIGURE_LOOK` and `PLTRS_CHROME_LOOK` environment variables choose
//! matplotlib's instead (see `pltrs::look`):
//!
//! ```text
//! PLTRS_LOOK=matplotlib cargo run -p pltrs --example figure_window
//! PLTRS_FIGURE_LOOK=matplotlib cargo run -p pltrs --example figure_window
//! PLTRS_FIGURE_LOOK=modern-dark cargo run -p pltrs --example figure_window
//! ```

#![expect(
    clippy::print_stdout,
    reason = "the example reports where the screenshots went"
)]

mod offscreen;
mod onscreen;
mod review;

use std::error::Error;
use std::path::{Path, PathBuf};

use pltrs::artist::ArtistHandle;
use pltrs::axes::ImshowOptions;
use pltrs::dates::{AutoDateLocator, ConciseDateFormatter, DateTime, TimeDelta};
use pltrs::figure::Figure;
use pltrs::gui::{FigureWindow, Icon, ToolKind, ToolSpec};
use pltrs::image::{ImsaveOptions, RgbaImage, imsave};
use pltrs::lines::LineProps;
use pltrs::numeric::{Grid, linspace};
use pltrs_core::{Path as Outline, Point};

/// The demo figure: curves, an image and a time series, with a radio
/// group of two tools of its own, the first one on.
fn demo() -> Result<Figure, Box<dyn Error>> {
    let fig = Figure::with_size(9.0, 4.0, 100.0);
    let grid = fig.subplots(1, 3)?;
    let (waves, field, series) = (&grid[(0, 0)], &grid[(0, 1)], &grid[(0, 2)]);
    let x = linspace(0.0, 10.0, 200);
    for (phase, label) in [(0.0, "first"), (0.8, "second"), (1.6, "third")] {
        let y: Vec<f64> = x
            .iter()
            .map(|x| (x + phase).sin() * (-x / 8.0).exp())
            .collect();
        waves.plot_with(x.clone(), y, LineProps::default().label(label))?;
    }
    let dots: Vec<f64> = (0..20).map(|i| f64::from(i) * 0.5).collect();
    let heights: Vec<f64> = dots.iter().map(|x| (x * 1.3).cos() * 0.4).collect();
    waves.scatter(dots, heights)?;
    waves.set_title("Damped waves");
    waves.set_xlabel("time").set_ylabel("amplitude");
    let (rows, cols) = (linspace(0.0, 6.0, 24), linspace(0.0, 4.8, 24));
    let values = Grid::from_fn(24, 24, |row, col| rows[row].sin() * cols[col].cos());
    field
        .imshow(values, ImshowOptions::default())?
        .set_label("interference");
    field.set_title("Field");
    let start = DateTime::new(2026, 1, 1, 0, 0, 0, 0)?;
    let days = (0..120)
        .map(|day| start.checked_add(TimeDelta::new(day, 0, 0)?))
        .collect::<Result<Vec<_>, _>>()?;
    let temperature: Vec<f64> = (0..120)
        .map(|day| 4.0 + 6.0 * (f64::from(day) / 40.0).sin())
        .collect();
    series.plot_with(
        &days[..],
        &temperature,
        LineProps::default().label("temperature"),
    )?;
    series
        .xaxis()
        .set_major_locator(AutoDateLocator::new())
        .set_major_formatter(ConciseDateFormatter::new());
    series.set_title("Daily temperature");
    for (name, description, icon, key, on) in [
        ("pen", "Mark points with the pen", pen(), "m", true),
        ("ruler", "Measure distances", ruler(), "d", false),
    ] {
        let spec = ToolSpec {
            name: name.to_owned(),
            description: description.to_owned(),
            icon,
            keys: vec![key.to_owned()],
            kind: ToolKind::Toggle,
            radio_group: Some("annotate".to_owned()),
            default_toggled: on,
            ..ToolSpec::default()
        };
        fig.add_tool(spec, |_| ())?;
    }
    Ok(fig)
}

/// A pen, the icon of a tool of the demo's own.
fn pen() -> Icon {
    Icon::new(24.0).stroke(Outline::polyline(vec![
        Point::new(5.0, 19.0),
        Point::new(6.0, 15.0),
        Point::new(16.0, 5.0),
        Point::new(19.0, 8.0),
        Point::new(9.0, 18.0),
        Point::new(5.0, 19.0),
    ]))
}

/// A ruler, the icon of a tool of the demo's own.
fn ruler() -> Icon {
    let ticks = [7.0, 11.0, 15.0]
        .map(|x| Outline::polyline(vec![Point::new(x, 15.0), Point::new(x, 12.0)]));
    ticks.into_iter().fold(
        Icon::new(24.0).stroke(Outline::polyline(vec![
            Point::new(3.5, 9.0),
            Point::new(20.5, 9.0),
            Point::new(20.5, 15.0),
            Point::new(3.5, 15.0),
            Point::new(3.5, 9.0),
        ])),
        Icon::stroke,
    )
}

/// What the command line asks for.
enum Mode {
    /// Show the window.
    Show,
    /// Save the idle window to this file.
    Screenshot(PathBuf),
    /// Save every surface of the chrome, drawn off screen, to this
    /// directory.
    Review(PathBuf),
    /// Save every surface of the chrome, shown on the display, to this
    /// directory.
    ReviewOnScreen(PathBuf),
}

impl Mode {
    fn from_args() -> Result<Self, Box<dyn Error>> {
        let mut args = std::env::args_os().skip(1);
        match (args.next(), args.next(), args.next()) {
            (None, _, _) => Ok(Self::Show),
            (Some(flag), Some(path), None) if flag == "--screenshot" => {
                Ok(Self::Screenshot(path.into()))
            }
            (Some(flag), Some(dir), None) if flag == "--review" => Ok(Self::Review(dir.into())),
            (Some(flag), Some(dir), None) if flag == "--review-on-screen" => {
                Ok(Self::ReviewOnScreen(dir.into()))
            }
            _ => Err(
                "usage: figure_window [--screenshot PATH | --review DIR | --review-on-screen DIR]"
                    .into(),
            ),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    match Mode::from_args()? {
        Mode::Show => Ok(pltrs::gui::show(&[demo()?])?),
        Mode::Screenshot(path) => save(FigureWindow::screenshot(&demo()?, 1.0)?, &path),
        Mode::Review(dir) => review::review(&dir),
        Mode::ReviewOnScreen(dir) => review::review_on_screen(&dir),
    }
}

/// Saves a window's `image` to `path` as a PNG.
fn save(image: RgbaImage, path: &Path) -> Result<(), Box<dyn Error>> {
    imsave(path, image, ImsaveOptions::default())?;
    println!("saved {}", path.display());
    Ok(())
}
