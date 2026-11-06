//! The review of the window chrome: every surface of the figure window
//! in every look, brought about by synthetic input as a user would bring
//! it about, and saved as PNG images; drawn off screen at pixel ratios 1
//! and 2 ([`review`]), or shown in windows of the display at its scale
//! factor ([`review_on_screen`]).

use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pltrs::gui::{FigureWindow, GuiError};
use pltrs::image::RgbaImage;
use pltrs::look::{Appearance, ChromeLook, FigureLook};
use pltrs_platform::{
    ButtonState, Context, Event, Key, KeyEvent, LogicalSize, Modifiers, MouseButton,
    PhysicalPosition, WindowEvent, WindowId,
};

use crate::offscreen::Offscreen;
use crate::onscreen::Onscreen;
use crate::{demo, save};

/// A figure window shown and driven by synthetic input: drawn off
/// screen, or in a window of the display.
pub(crate) trait Driver {
    /// The figure window.
    fn main(&self) -> WindowId;

    /// The scale factor of `window` and its height in pixels.
    fn geometry(&mut self, window: WindowId) -> (f64, f64);

    /// Hands `event` to `window`, as the platform hands it input.
    fn inject(&mut self, window: WindowId, event: WindowEvent) -> Result<(), Box<dyn Error>>;

    /// Handles the events queued and those they cause, and the timers
    /// due before `deadline`; returns the window opened last.
    fn run(&mut self, deadline: Instant) -> Result<WindowId, Box<dyn Error>>;

    /// What `window` shows.
    fn capture(&mut self, window: WindowId) -> Result<RgbaImage, Box<dyn Error>>;

    /// Handles the events queued; returns the window opened last.
    fn settle(&mut self) -> Result<WindowId, Box<dyn Error>> {
        self.run(Instant::now())
    }

    /// Handles the events queued until the tooltip of what the pointer
    /// rests on (or the keyboard focused) shows.
    fn wait_for_tooltip(&mut self) -> Result<WindowId, Box<dyn Error>> {
        let delay = pltrs_ui::Theme::default().tooltip_delay;
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
        &mut self,
        window: WindowId,
        count: usize,
        modifiers: Modifiers,
    ) -> Result<(), Box<dyn Error>> {
        (0..count).try_for_each(|_| self.press(window, Key::Tab, modifiers))
    }

    /// The physical position of `(x, y)` logical pixels in `window`,
    /// from the bottom edge when `y` is negative.
    fn at(&mut self, window: WindowId, (x, y): (f64, f64)) -> PhysicalPosition {
        let (scale, height) = self.geometry(window);
        let y = if y < 0.0 {
            height + y * scale
        } else {
            y * scale
        };
        PhysicalPosition::new(x * scale, y)
    }

    /// Moves the pointer to `point` (see [`Driver::at`]).
    fn point(&mut self, window: WindowId, point: (f64, f64)) -> Result<(), Box<dyn Error>> {
        let event = WindowEvent::CursorMoved {
            position: self.at(window, point),
            modifiers: Modifiers::NONE,
        };
        self.inject(window, event)
    }

    /// Presses or releases the left button at `point`.
    fn click(
        &mut self,
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
        self.inject(window, event)
    }

    /// Presses and releases `key` with `modifiers` held; characters typed
    /// without Control, Alt or Super type their text.
    fn press(
        &mut self,
        window: WindowId,
        key: Key,
        modifiers: Modifiers,
    ) -> Result<(), Box<dyn Error>> {
        let shortcut = modifiers.control() || modifiers.alt() || modifiers.super_key();
        let text = match key {
            Key::Character(c) if !shortcut => Some(c.to_string()),
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
            self.inject(window, WindowEvent::Key(event))?;
        }
        Ok(())
    }

    /// Types `text` into `window`.
    fn type_text(&mut self, window: WindowId, text: &str) -> Result<(), Box<dyn Error>> {
        text.chars()
            .try_for_each(|c| self.press(window, Key::Character(c), Modifiers::NONE))
    }

    /// Saves what `window` shows to `path` as a PNG.
    fn save(&mut self, window: WindowId, path: &Path) -> Result<(), Box<dyn Error>> {
        save(self.capture(window)?, path)
    }
}

/// Hands `event` to the figure window `window` if it concerns one of its
/// windows or timers; returns whether the figure window stays open.
pub(crate) fn dispatch(
    window: &mut FigureWindow,
    cx: &mut Context<'_>,
    event: Event,
) -> Result<bool, GuiError> {
    match event {
        Event::Window { window: id, event } if window.owns_window(id) => {
            window.handle(cx, id, &event)
        }
        Event::Timer(timer) if window.owns_timer(timer) => window.tick(cx, timer).map(|()| true),
        _ => Ok(true),
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
type Scene = fn(&mut dyn Driver) -> Result<WindowId, Box<dyn Error>>;

/// The surfaces reviewed: a name for the files and how to show them.
const SURFACES: [(&str, Scene); 23] = [
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
        pltrs::rc::context(|| {
            pltrs::rc::set("timezone", "Asia/Tokyo")?;
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
    ("subplots-switched", |shown| {
        // The first control has the focus: under constrained layout, the
        // switch, whose Right shows the subplot parameters; else the top
        // border's slider, which Right moves.
        let tool = shown.open_tool(SUBPLOTS_TABS)?;
        shown.press(tool, Key::Right, Modifiers::NONE)?;
        shown.settle()
    }),
    ("subplots-figure", |shown| {
        // The figure after the change above.
        let tool = shown.open_tool(SUBPLOTS_TABS)?;
        shown.press(tool, Key::Right, Modifiers::NONE)?;
        shown.settle()?;
        Ok(shown.main())
    }),
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
/// The size the window is resized to on screen, in points.
const RESIZED: LogicalSize = LogicalSize::new(720.0, 420.0);
/// The modifier of the shortcut that closes a window: matplotlib's
/// `keymap.quit` has `cmd+w` and `ctrl+w`.
const CLOSE_SHORTCUT: Modifiers = if cfg!(target_os = "macos") {
    Modifiers::SUPER
} else {
    Modifiers::CONTROL
};
/// A file of the save dialog's directory with a long name.
const LONG_FILE: &str = "quarterly-results-with-a-rather-long-descriptive-name-final-v2.png";

/// Saves every surface of every look in `dir`, drawn off screen at pixel
/// ratios 1 and 2.
pub(crate) fn review(dir: &Path) -> Result<(), Box<dyn Error>> {
    in_every_look(dir, |look| {
        for ratio in [1.0, 2.0] {
            for (surface, scene) in SURFACES {
                let mut shown = Offscreen::open(&demo()?, ratio)?;
                let window = scene(&mut shown)?;
                shown.save(window, &dir.join(format!("{look}-{ratio}x-{surface}.png")))?;
            }
        }
        Ok(())
    })
}

/// Saves every surface of every look in `dir`, shown in windows of the
/// display at its scale factor, and the window resized; checks that the
/// close shortcut closes it.
pub(crate) fn review_on_screen(dir: &Path) -> Result<(), Box<dyn Error>> {
    in_every_look(dir, |look| {
        for (surface, scene) in SURFACES {
            let mut shown = Onscreen::open(&demo()?)?;
            let window = scene(&mut shown)?;
            let (ratio, _) = shown.geometry(window);
            shown.save(window, &dir.join(format!("{look}-{ratio}x-{surface}.png")))?;
        }
        let mut shown = Onscreen::open(&demo()?)?;
        let main = shown.resize(RESIZED)?;
        let (ratio, _) = shown.geometry(main);
        shown.save(
            main,
            &dir.join(format!("{look}-{ratio}x-window-resized.png")),
        )?;
        shown.press(main, Key::Character('w'), CLOSE_SHORTCUT)?;
        shown.settle()?;
        if shown.closing() {
            Ok(())
        } else {
            Err("the close shortcut left the window open".into())
        }
    })
}

/// Runs `review` for every look (named as in the files), with the save
/// dialog starting in a deep directory of `dir`.
fn in_every_look(
    dir: &Path,
    mut review: impl FnMut(&str) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(dir)?;
    let files = save_dialog_directory(dir)?;
    for (look, appearance) in LOOKS {
        pltrs::look::set_appearance(appearance);
        // The look restarted the parameters.
        pltrs::rc::update(|rc| rc.savefig_directory = files.display().to_string());
        review(look)?;
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
