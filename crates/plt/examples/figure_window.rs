//! An interactive figure window: curves, an image and a time series in
//! three subplots, shown with pan/zoom navigation and a group of tools of
//! its own.
//!
//! ```text
//! cargo run -p plt --example figure_window
//! cargo run -p plt --example figure_window -- --screenshot window.png
//! cargo run -p plt --example figure_window -- --review DIR
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
//! The figure and the window take plt-rs' modern look; the `PLT_LOOK`,
//! `PLT_FIGURE_LOOK` and `PLT_CHROME_LOOK` environment variables choose
//! matplotlib's instead (see `plt::look`):
//!
//! ```text
//! PLT_LOOK=matplotlib cargo run -p plt --example figure_window
//! PLT_FIGURE_LOOK=matplotlib cargo run -p plt --example figure_window
//! PLT_FIGURE_LOOK=modern-dark cargo run -p plt --example figure_window
//! ```

#![expect(
    clippy::print_stdout,
    reason = "the example reports where the screenshots went"
)]

use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use plt::artist::ArtistHandle;
use plt::axes::ImshowOptions;
use plt::dates::{AutoDateLocator, ConciseDateFormatter, DateTime, TimeDelta};
use plt::figure::Figure;
use plt::gui::{FigureWindow, Icon, ToolKind, ToolSpec};
use plt::image::{ImsaveOptions, RgbaImage, imsave};
use plt::lines::LineProps;
use plt::look::{Appearance, ChromeLook, FigureLook};
use plt::numeric::{Grid, linspace};
use plt_core::{Path as Outline, Point};
use plt_platform::{
    ButtonState, Event, EventLoop, HeadlessHandle, Key, KeyEvent, Modifiers, MouseButton,
    PhysicalPosition, WindowEvent, WindowId,
};

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
    /// Save every surface of the chrome to this directory.
    Review(PathBuf),
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
            _ => Err("usage: figure_window [--screenshot PATH | --review DIR]".into()),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    match Mode::from_args()? {
        Mode::Show => Ok(plt::gui::show(&[demo()?])?),
        Mode::Screenshot(path) => save(FigureWindow::screenshot(&demo()?, 1.0)?, &path),
        Mode::Review(dir) => review(&dir),
    }
}

/// The looks reviewed: a name for the files and the appearance.
const LOOKS: [(&str, Appearance); 3] = [
    ("modern", Appearance::MODERN),
    (
        "dark",
        Appearance {
            figures: FigureLook::ModernDark,
            chrome: ChromeLook::Modern,
        },
    ),
    ("matplotlib", Appearance::MATPLOTLIB),
];

/// Brings a shown figure window to the state of a surface; returns the
/// window to capture.
type Scene = fn(&mut Offscreen) -> Result<WindowId, Box<dyn Error>>;

/// The surfaces reviewed: a name for the files and how to show them.
const SURFACES: [(&str, Scene); 21] = [
    ("window-idle", |shown| Ok(shown.main())),
    ("window-hover", |shown| {
        shown.point(shown.main(), HOME_BUTTON)?;
        shown.wait_for_tooltip()
    }),
    ("window-pressed", |shown| {
        shown.point(shown.main(), HOME_BUTTON)?;
        shown.click(shown.main(), HOME_BUTTON, ButtonState::Pressed)?;
        shown.settle()
    }),
    ("window-focus", |shown| {
        // Ctrl+Tab focuses Home; Tab skips the disabled history buttons.
        shown.press(shown.main(), Key::Tab, Modifiers::CONTROL)?;
        shown.press(shown.main(), Key::Tab, Modifiers::NONE)?;
        shown.wait_for_tooltip()
    }),
    ("window-pan", |shown| {
        shown.type_text(shown.main(), "p")?;
        shown.point(shown.main(), IN_AXES)?;
        shown.settle()
    }),
    ("window-zoom", |shown| {
        shown.type_text(shown.main(), "o")?;
        shown.point(shown.main(), IN_AXES)?;
        shown.click(shown.main(), IN_AXES, ButtonState::Pressed)?;
        shown.point(shown.main(), (IN_AXES.0 + 120.0, IN_AXES.1 + 90.0))?;
        shown.settle()
    }),
    ("options-axes", |shown| shown.open_tool(OPTIONS_TABS)),
    ("options-curves", |shown| {
        let dialog = shown.open_tool(OPTIONS_TABS)?;
        shown.press(dialog, Key::Tab, Modifiers::CONTROL)?;
        shown.settle()
    }),
    ("options-axes-list", |shown| {
        let dialog = shown.open_axes_list()?;
        shown.type_text(dialog, " ")?;
        shown.settle()
    }),
    ("options-image", |shown| {
        let dialog = shown.open_axes_list()?;
        shown.press(dialog, Key::Down, Modifiers::NONE)?;
        shown.press(dialog, Key::Tab, Modifiers::CONTROL)?;
        shown.settle()
    }),
    ("options-dates", |shown| {
        let dialog = shown.open_axes_list()?;
        shown.press(dialog, Key::End, Modifiers::NONE)?;
        shown.settle()
    }),
    ("options-dates-zone", |shown| {
        plt::rc::context(|| {
            plt::rc::set("timezone", "Asia/Tokyo")?;
            let dialog = shown.open_axes_list()?;
            shown.press(dialog, Key::End, Modifiers::NONE)?;
            shown.settle()
        })
    }),
    ("options-problem", |shown| {
        let dialog = shown.open_tool(OPTIONS_TABS)?;
        // From the title to the x axis' minimum.
        shown.press(dialog, Key::Tab, Modifiers::NONE)?;
        shown.type_text(dialog, "ten")?;
        shown.press(dialog, Key::Enter, Modifiers::NONE)?;
        shown.settle()
    }),
    ("options-problem-tab", |shown| {
        // The second curve's width refused from the Axes tab, the first
        // curve selected.
        let dialog = shown.open_tool(OPTIONS_TABS)?;
        shown.press(dialog, Key::Tab, Modifiers::CONTROL)?;
        shown.press(dialog, Key::Down, Modifiers::NONE)?;
        shown.tab(dialog, CURVE_WIDTH_TABS, Modifiers::NONE)?;
        shown.type_text(dialog, "thick")?;
        shown.tab(dialog, CURVE_WIDTH_TABS, Modifiers::SHIFT)?;
        shown.press(dialog, Key::Up, Modifiers::NONE)?;
        shown.press(dialog, Key::Tab, Modifiers::CONTROL)?;
        shown.press(dialog, Key::Enter, Modifiers::NONE)?;
        shown.settle()
    }),
    ("options-problem-axes", |shown| {
        // The first axes' minimum refused while the last axes show.
        let dialog = shown.open_tool(OPTIONS_TABS)?;
        shown.press(dialog, Key::Tab, Modifiers::NONE)?;
        shown.type_text(dialog, "ten")?;
        // Back past the title and the tabs to the axes list.
        shown.tab(dialog, 3, Modifiers::SHIFT)?;
        shown.press(dialog, Key::End, Modifiers::NONE)?;
        shown.press(dialog, Key::Tab, Modifiers::NONE)?;
        shown.press(dialog, Key::Enter, Modifiers::NONE)?;
        shown.settle()
    }),
    ("subplots", |shown| shown.open_tool(SUBPLOTS_TABS)),
    ("save", |shown| {
        shown.type_text(shown.main(), "s")?;
        shown.settle()
    }),
    ("save-problem", |shown| {
        shown.type_text(shown.main(), "s")?;
        let dialog = shown.settle()?;
        shown.press(dialog, Key::Character('a'), Modifiers::CONTROL)?;
        shown.press(dialog, Key::Backspace, Modifiers::NONE)?;
        shown.press(dialog, Key::Enter, Modifiers::NONE)?;
        shown.settle()
    }),
    ("save-replace", |shown| {
        shown.type_text(shown.main(), "s")?;
        let dialog = shown.settle()?;
        shown.press(dialog, Key::Character('a'), Modifiers::CONTROL)?;
        shown.type_text(dialog, LONG_FILE)?;
        shown.press(dialog, Key::Enter, Modifiers::NONE)?;
        shown.settle()
    }),
    ("help", |shown| {
        shown.press(shown.main(), Key::F(1), Modifiers::NONE)?;
        shown.settle()
    }),
    ("help-end", |shown| {
        shown.press(shown.main(), Key::F(1), Modifiers::NONE)?;
        let help = shown.settle()?;
        shown.press(help, Key::End, Modifiers::NONE)?;
        shown.settle()
    }),
];

/// The centre of the Home button, in logical pixels from the window's
/// bottom-left corner.
const HOME_BUTTON: (f64, f64) = (24.0, -20.0);
/// A point in the first axes, in logical pixels from the top-left corner.
const IN_AXES: (f64, f64) = (150.0, 150.0);
/// How many Tabs lead from Home to the figure options button: Pan, Zoom,
/// Subplots, Figure options (the history buttons are disabled).
const OPTIONS_TABS: usize = 4;
/// How many Tabs lead from Home to the subplot tool's button.
const SUBPLOTS_TABS: usize = 3;
/// How many Tabs lead from the list of curves to the width of the curve
/// selected: past its label, line style and draw style.
const CURVE_WIDTH_TABS: usize = 4;
/// A file of the save dialog's directory with a long name.
const LONG_FILE: &str = "quarterly-results-with-a-rather-long-descriptive-name-final-v2.png";

/// Saves every surface of every look at pixel ratios 1 and 2 in `dir`.
fn review(dir: &Path) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(dir)?;
    let files = save_dialog_directory(dir)?;
    for (look, appearance) in LOOKS {
        plt::look::set_appearance(appearance);
        // The look restarted the parameters.
        plt::rc::update(|rc| rc.savefig_directory = files.display().to_string());
        for ratio in [1.0, 2.0] {
            for (surface, scene) in SURFACES {
                let mut shown = Offscreen::open(&demo()?, ratio)?;
                let window = scene(&mut shown)?;
                shown.save(window, &dir.join(format!("{look}-{ratio}x-{surface}.png")))?;
            }
        }
    }
    Ok(())
}

/// Makes a deep directory under `dir` holding subdirectories and many
/// files, for the save dialog to start in; returns it.
fn save_dialog_directory(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let files = dir
        .join(".save-dialog")
        .join("Quarterly reports and supplementary figures")
        .join("Second half of the year, revised after the review");
    for sub in ["drafts", "exports for the printed edition", "archive"] {
        std::fs::create_dir_all(files.join(sub))?;
    }
    let names = (1..=24)
        .map(|n| format!("figure-{n:02}.png"))
        .chain([LONG_FILE.to_owned()]);
    for name in names {
        std::fs::write(files.join(name), b"")?;
    }
    Ok(files)
}

/// A figure window drawn off screen, driven by synthetic input.
struct Offscreen {
    event_loop: EventLoop,
    handle: HeadlessHandle,
    window: FigureWindow,
}

impl Offscreen {
    /// Shows `figure` at pixel ratio `ratio`.
    fn open(figure: &Figure, ratio: f64) -> Result<Self, Box<dyn Error>> {
        let (mut event_loop, handle) = EventLoop::headless();
        handle.set_scale_factor(ratio)?;
        let window = FigureWindow::open(&mut event_loop.context(), figure, "Figure 1")?;
        let mut shown = Self {
            event_loop,
            handle,
            window,
        };
        shown.settle()?;
        Ok(shown)
    }

    /// The figure window.
    fn main(&self) -> WindowId {
        self.window.id()
    }

    /// Handles the events queued and those they cause, and the timers
    /// due before `deadline`; returns the window opened last.
    fn run(&mut self, deadline: Instant) -> Result<WindowId, Box<dyn Error>> {
        let window = &mut self.window;
        let mut failure = None;
        let mut dispatch = |cx: &mut plt_platform::Context<'_>, event: Event| {
            let result = match event {
                Event::Window { window: id, event } if window.owns_window(id) => {
                    window.handle(cx, id, &event).map(drop)
                }
                Event::Timer(timer) if window.owns_timer(timer) => window.tick(cx, timer),
                _ => Ok(()),
            };
            if let Err(err) = result {
                failure.get_or_insert(err);
            }
        };
        // Redraws follow the events that asked for them.
        for _ in 0..3 {
            self.event_loop
                .pump_events(Some(Duration::ZERO), &mut dispatch)?;
        }
        self.event_loop.run_until(deadline, &mut dispatch)?;
        match failure {
            Some(err) => Err(err.into()),
            None => Ok(self
                .handle
                .windows()
                .into_iter()
                .last()
                .unwrap_or(self.main())),
        }
    }

    /// Handles the events queued; returns the window opened last.
    fn settle(&mut self) -> Result<WindowId, Box<dyn Error>> {
        self.run(Instant::now())
    }

    /// Handles the events queued until the tooltip of what the pointer
    /// rests on (or the keyboard focused) shows.
    fn wait_for_tooltip(&mut self) -> Result<WindowId, Box<dyn Error>> {
        let delay = plt_ui::Theme::default().tooltip_delay;
        self.run(Instant::now() + delay + Duration::from_millis(100))
    }

    /// Opens a dialog from the toolbar: Ctrl+Tab focuses its first button,
    /// `tabs` Tabs lead to the dialog's and Space presses it. Returns the
    /// dialog.
    fn open_tool(&mut self, tabs: usize) -> Result<WindowId, Box<dyn Error>> {
        let main = self.main();
        self.press(main, Key::Tab, Modifiers::CONTROL)?;
        self.tab(main, tabs, Modifiers::NONE)?;
        self.type_text(main, " ")?;
        self.settle()
    }

    /// Opens the figure options with the focus on the list of axes: back
    /// from the title past the tabs.
    fn open_axes_list(&mut self) -> Result<WindowId, Box<dyn Error>> {
        let dialog = self.open_tool(OPTIONS_TABS)?;
        self.tab(dialog, 2, Modifiers::SHIFT)?;
        Ok(dialog)
    }

    /// Presses Tab `count` times with `modifiers` held (Shift to go
    /// back).
    fn tab(
        &self,
        window: WindowId,
        count: usize,
        modifiers: Modifiers,
    ) -> Result<(), Box<dyn Error>> {
        (0..count).try_for_each(|_| self.press(window, Key::Tab, modifiers))
    }

    /// The physical position of `(x, y)` logical pixels in `window`,
    /// from the bottom edge when `y` is negative.
    fn at(&self, window: WindowId, (x, y): (f64, f64)) -> PhysicalPosition {
        let (scale, height) = self.handle.window(window).map_or((1.0, 0.0), |snapshot| {
            (snapshot.scale_factor, f64::from(snapshot.size.height))
        });
        let y = if y < 0.0 {
            height + y * scale
        } else {
            y * scale
        };
        PhysicalPosition::new(x * scale, y)
    }

    /// Moves the pointer to `point` (see [`Offscreen::at`]).
    fn point(&self, window: WindowId, point: (f64, f64)) -> Result<(), Box<dyn Error>> {
        let event = WindowEvent::CursorMoved {
            position: self.at(window, point),
            modifiers: Modifiers::NONE,
        };
        Ok(self.handle.inject(window, event)?)
    }

    /// Presses or releases the left button at `point`.
    fn click(
        &self,
        window: WindowId,
        point: (f64, f64),
        state: ButtonState,
    ) -> Result<(), Box<dyn Error>> {
        let event = WindowEvent::MouseInput {
            button: MouseButton::Left,
            state,
            position: self.at(window, point),
            modifiers: Modifiers::NONE,
            click_count: 1,
        };
        Ok(self.handle.inject(window, event)?)
    }

    /// Presses and releases `key` with `modifiers` held; characters typed
    /// without Control or Alt type their text.
    fn press(
        &self,
        window: WindowId,
        key: Key,
        modifiers: Modifiers,
    ) -> Result<(), Box<dyn Error>> {
        let text = match key {
            Key::Character(c) if !modifiers.control() && !modifiers.alt() => Some(c.to_string()),
            _ => None,
        };
        for state in [ButtonState::Pressed, ButtonState::Released] {
            let event = KeyEvent {
                key,
                state,
                repeat: false,
                text: text.clone().filter(|_| state == ButtonState::Pressed),
                modifiers,
                scancode: 0,
            };
            self.handle.inject(window, WindowEvent::Key(event))?;
        }
        Ok(())
    }

    /// Types `text` into `window`.
    fn type_text(&self, window: WindowId, text: &str) -> Result<(), Box<dyn Error>> {
        text.chars()
            .try_for_each(|c| self.press(window, Key::Character(c), Modifiers::NONE))
    }

    /// Saves what `window` shows to `path` as a PNG.
    fn save(&self, window: WindowId, path: &Path) -> Result<(), Box<dyn Error>> {
        save(FigureWindow::capture(&self.handle, window)?, path)
    }
}

/// Saves a window's `image` to `path` as a PNG.
fn save(image: RgbaImage, path: &Path) -> Result<(), Box<dyn Error>> {
    imsave(path, image, ImsaveOptions::default())?;
    println!("saved {}", path.display());
    Ok(())
}
