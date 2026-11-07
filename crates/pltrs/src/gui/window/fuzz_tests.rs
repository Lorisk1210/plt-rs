//! Randomised robustness tests of figure windows and their dialogs: the
//! save dialog, the subplot tool, the figure options and the keyboard
//! shortcuts are opened over figures of several kinds, then random keys
//! (text editing, undo and redo, selection, Tab and Ctrl+Tab, type-ahead
//! in lists and dropdowns), typing, clipboard pastes and answers, mouse
//! input, resizes, scale changes and closes reach the window on top or
//! the figure window, and a session ends with Enter (OK, Apply, Save or
//! Close). Nothing may panic.
//!
//! Saves go to a sandbox directory dozens of levels deep, and nothing
//! typed or pasted holds a path separator (`/`, or on Windows `\`) or a
//! drive's colon, so that no file is written outside it.

use std::path::PathBuf;
use std::time::Duration;

use pltrs_platform::{
    ButtonState, Event, EventLoop, HeadlessHandle, Key, KeyEvent, Modifiers, MouseButton,
    PhysicalPosition, ScrollDelta,
};

use super::*;
use crate::axes::{BarOptions, ImshowOptions};
use crate::fuzz::{Gen, fuzz};
use crate::legend::LegendOptions;
use crate::lines::LineProps;
use crate::numeric::Grid;
use crate::widgets::{TextBox, TextBoxOptions};

/// How many directories deep the save dialog's sandbox lies, so that
/// walking up its listing never leaves it.
const SANDBOX_DEPTH: usize = 40;

/// The figures windows show.
#[derive(Clone, Copy, Debug)]
enum FigureKind {
    /// A labelled curve with a title and a legend.
    Curve,
    /// Four axes: a curve, log-scaled points, an image and bars.
    Grid,
    /// Polar axes.
    Polar,
    /// 3-D axes.
    ThreeD,
    /// No axes at all.
    Empty,
    /// A text box widget, which takes the keys while it is active.
    TextBox,
}

/// The figure of `kind`, and what must live as long as it is shown.
fn figure(kind: FigureKind) -> (Figure, Option<TextBox>) {
    let figure = Figure::with_size(3.0, 2.2, 60.0);
    let mut text_box = None;
    match kind {
        FigureKind::Curve => {
            let ax = figure.subplot();
            let line = LineProps::default().label("signal");
            let _ = ax.plot_with(vec![0.0, 1.0, 2.0], vec![1.0, 3.0, 2.0], line);
            ax.set_title("t");
            let _ = ax.legend(LegendOptions::default());
        }
        FigureKind::Grid => {
            if let Ok(axs) = figure.subplots(2, 2) {
                let _ = axs[(0, 0)].plot(vec![0.0, 1.0, 2.0], vec![1.0, 3.0, 2.0]);
                let _ = axs[(0, 1)].scatter(vec![1.0, 10.0, 100.0], vec![1.0, 5.0, 50.0]);
                let _ = axs[(0, 1)].set_xscale("log");
                let image = Grid::from_fn(4, 4, |i, j| (i * j) as f64);
                let _ = axs[(1, 0)].imshow(image, ImshowOptions::default());
                let _ = axs[(1, 1)].bar(vec![0.0, 1.0], [3.0, 1.0], BarOptions::default());
            }
        }
        FigureKind::Polar => {
            let _ = figure
                .polar_subplot()
                .plot(vec![0.0, 1.0, 2.0], vec![1.0, 2.0, 1.5]);
        }
        FigureKind::ThreeD => {
            let _ = figure.subplot_3d().plot([0.0, 1.0], [1.0, 0.0], [0.0, 1.0]);
        }
        FigureKind::Empty => {}
        FigureKind::TextBox => {
            let plot = figure.subplot();
            let _ = plot.plot(vec![0.0, 1.0], vec![1.0, 0.0]);
            if let Ok(ax) = figure.add_axes([0.2, 0.02, 0.6, 0.1]) {
                text_box = Some(TextBox::new(&ax, "t", "x", TextBoxOptions::default()));
            }
        }
    }
    (figure, text_box)
}

/// Which window an input goes to.
#[derive(Clone, Copy, Debug)]
enum Target {
    /// The window on top: the dialog opened last, if any is open.
    Top,
    /// The figure window.
    Main,
}

/// One input of a session.
#[derive(Clone, Debug)]
enum Step {
    /// A key pressed and released with modifiers held.
    Key(Key, Modifiers, Target),
    /// Text typed a character at a time.
    Type(String, Target),
    /// Text put on the clipboard, then pasted with Ctrl+V.
    Paste(String, Target),
    /// A clipboard answer nobody asked for.
    Answer(Option<String>, Target),
    /// A click (or double click) at a fraction of the window's size.
    Click(f64, f64, MouseButton, u32, Target),
    /// A press there, a move and a release elsewhere.
    Drag([f64; 4], Target),
    /// A wheel turn there.
    Wheel(f64, f64, f64, Target),
    /// A window event: a resize, a scale change, focus or a close.
    Window(WindowEvent, Target),
    /// A dialog opened from the toolbar.
    Open(Tool),
}

/// Keys of the dialogs and the canvas.
const KEYS: [Key; 18] = [
    Key::Tab,
    Key::Tab,
    Key::Enter,
    Key::Escape,
    Key::Backspace,
    Key::Delete,
    Key::Left,
    Key::Right,
    Key::Up,
    Key::Down,
    Key::Home,
    Key::End,
    Key::PageUp,
    Key::PageDown,
    Key::Insert,
    Key::F(1),
    Key::Shift,
    Key::Control,
];

/// Characters typed: field contents (numbers, names, extensions),
/// shortcuts with Control (select all, copy, cut, paste, undo, redo),
/// matplotlib's keys, and characters whose case changes their length.
const CHARS: [char; 32] = [
    'a', 'c', 'x', 'v', 'z', 'y', 'g', 'l', 'k', 'L', 'p', 'o', 'h', 's', 'f', '0', '1', '9', '.',
    '-', '+', 'e', ' ', ',', 'é', 'İ', 'ẞ', 'ﬃ', 'Σ', '😀', '\u{301}', '\u{200D}',
];

/// Pieces of pasted text: numbers at the edges, names, extensions,
/// line ends and long runs.
const PASTED: [&str; 16] = [
    "1e308", "-1e308", "1e-320", "nan", "-inf", "0", "-0", "12.5", "png", ".svg", "Figure_1", "\n",
    "\t", "é😀", "  ", "..",
];

impl Step {
    fn generate(g: &mut Gen) -> Self {
        let target = if g.chance(0.8) {
            Target::Top
        } else {
            Target::Main
        };
        let at = |g: &mut Gen| g.between(-0.1, 1.1);
        match g.below(40) {
            0..10 => {
                let modifiers = g.pick(&[
                    Modifiers::NONE,
                    Modifiers::NONE,
                    Modifiers::SHIFT,
                    Modifiers::CONTROL,
                    Modifiers::CONTROL | Modifiers::SHIFT,
                    Modifiers::ALT,
                ]);
                let key = if g.chance(0.6) {
                    g.pick(&KEYS)
                } else {
                    Key::Character(g.pick(&CHARS))
                };
                Self::Key(key, modifiers, target)
            }
            10..16 => Self::Type((0..=g.len(12)).map(|_| g.pick(&CHARS)).collect(), target),
            16..19 => Self::Paste(pasted(g), target),
            19 => Self::Answer(g.chance(0.8).then(|| pasted(g)), target),
            20..27 => Self::Click(
                at(g),
                at(g),
                g.pick(&[MouseButton::Left, MouseButton::Left, MouseButton::Right]),
                g.pick(&[1, 1, 2, 3]),
                target,
            ),
            27..30 => Self::Drag([at(g), at(g), at(g), at(g)], target),
            30..32 => Self::Wheel(at(g), at(g), g.pick(&[-3.0, -1.0, 1.0, 2.5]), target),
            32..37 => Self::Window(window_event(g), target),
            _ => Self::Open(g.pick(&[Tool::Save, Tool::Subplots, Tool::Options, Tool::Help])),
        }
    }
}

/// Pasted text: pieces, characters, or a long run. Separators and colons
/// are left out on every system, so that a seed pastes the same text on
/// all of them.
fn pasted(g: &mut Gen) -> String {
    if g.chance(0.05) {
        "9".repeat(g.pick(&[300, 5_000]))
    } else {
        g.soup(&PASTED, 6).replace(['/', '\\', ':'], "")
    }
}

/// A resize, a scale change (any a platform reports: finite and
/// positive), a focus change, a redraw or a close.
fn window_event(g: &mut Gen) -> WindowEvent {
    match g.below(8) {
        0..3 => WindowEvent::Resized(pltrs_platform::PhysicalSize::new(
            g.pick(&[0, 1, 40, 240, 700, 3000]),
            g.pick(&[0, 1, 30, 160, 500, 2000]),
        )),
        3 => WindowEvent::ScaleFactorChanged(g.pick(&[0.5, 1.0, 2.0, 1e-9, 1e-300, 1e9, 1e300])),
        4 => WindowEvent::Focused(g.chance(0.5)),
        5 => WindowEvent::RedrawRequested,
        6 => WindowEvent::CursorLeft {
            position: g
                .chance(0.5)
                .then(|| pltrs_platform::PhysicalPosition::new(g.pick(&[-1.0, 0.0, 5e3]), 12.0)),
        },
        _ => WindowEvent::CloseRequested,
    }
}

/// A session: a figure, the dialog opened first, and the inputs.
#[derive(Debug)]
struct Session {
    figure: FigureKind,
    opened: Option<Tool>,
    steps: Vec<Step>,
}

impl Session {
    fn generate(g: &mut Gen) -> Self {
        let figure = g.pick(&[
            FigureKind::Curve,
            FigureKind::Curve,
            FigureKind::Grid,
            FigureKind::Polar,
            FigureKind::ThreeD,
            FigureKind::Empty,
            FigureKind::TextBox,
        ]);
        let opened = g.chance(0.85).then(|| {
            g.pick(&[
                Tool::Save,
                Tool::Subplots,
                Tool::Options,
                Tool::Options,
                Tool::Help,
            ])
        });
        let mut steps: Vec<Step> = (0..g.len(40)).map(|_| Step::generate(g)).collect();
        steps.push(Step::Key(Key::Enter, Modifiers::NONE, Target::Top));
        Self {
            figure,
            opened,
            steps,
        }
    }
}

/// Runs one loop iteration, feeding the events of `window` and its tool
/// windows and its timers to it; returns whether it is still open.
/// Errors (a file that cannot be written, say) are fine.
fn pump(event_loop: &mut EventLoop, window: &mut FigureWindow) -> bool {
    let mut open = true;
    let _ = event_loop.pump_events(Some(Duration::ZERO), |cx, event| match event {
        Event::Window { window: id, event } if open && window.owns_window(id) => {
            open = !matches!(window.handle(cx, id, &event), Ok(false));
        }
        Event::Timer(timer) if open && window.owns_timer(timer) => {
            let _ = window.tick(cx, timer);
        }
        _ => {}
    });
    open
}

/// The window `target` names.
fn target(handle: &HeadlessHandle, window: &FigureWindow, target: Target) -> WindowId {
    match target {
        Target::Main => window.id(),
        Target::Top => handle
            .stacking_order()
            .into_iter()
            .rev()
            .find(|&id| window.owns_window(id))
            .unwrap_or_else(|| window.id()),
    }
}

/// Presses and releases `key` on `id`; characters typed without Control
/// or Alt type their text.
fn press(handle: &HeadlessHandle, id: WindowId, key: Key, modifiers: Modifiers) {
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
        let _ = handle.inject(id, WindowEvent::Key(event));
    }
}

/// The point at fractions `(fx, fy)` of window `id`.
fn point(handle: &HeadlessHandle, id: WindowId, fx: f64, fy: f64) -> PhysicalPosition {
    let size = handle.window(id).map_or_else(Default::default, |w| w.size);
    PhysicalPosition::new(fx * f64::from(size.width), fy * f64::from(size.height))
}

/// Moves the pointer to `position` and presses or releases `button`.
fn mouse(
    handle: &HeadlessHandle,
    id: WindowId,
    position: PhysicalPosition,
    button: MouseButton,
    state: ButtonState,
    click_count: u32,
) {
    let modifiers = Modifiers::NONE;
    let _ = handle.inject(
        id,
        WindowEvent::CursorMoved {
            position,
            modifiers,
        },
    );
    let click = WindowEvent::MouseInput {
        button,
        state,
        position,
        modifiers,
        click_count,
    };
    let _ = handle.inject(id, click);
}

/// Injects the events of `step`.
fn inject(
    step: &Step,
    handle: &HeadlessHandle,
    window: &mut FigureWindow,
    event_loop: &mut EventLoop,
) {
    match step {
        Step::Key(key, modifiers, to) => {
            press(handle, target(handle, window, *to), *key, *modifiers);
        }
        Step::Type(text, to) => {
            let id = target(handle, window, *to);
            for c in text.chars() {
                press(handle, id, Key::Character(c), Modifiers::NONE);
            }
        }
        Step::Paste(text, to) => {
            handle.set_clipboard_text(text.clone());
            let id = target(handle, window, *to);
            press(handle, id, Key::Character('v'), Modifiers::CONTROL);
        }
        Step::Answer(text, to) => {
            let _ = handle.inject(
                target(handle, window, *to),
                WindowEvent::ClipboardText(text.clone()),
            );
        }
        Step::Click(fx, fy, button, count, to) => {
            let id = target(handle, window, *to);
            let at = point(handle, id, *fx, *fy);
            mouse(handle, id, at, *button, ButtonState::Pressed, *count);
            mouse(handle, id, at, *button, ButtonState::Released, *count);
        }
        Step::Drag([x0, y0, x1, y1], to) => {
            let id = target(handle, window, *to);
            let (from, to) = (point(handle, id, *x0, *y0), point(handle, id, *x1, *y1));
            mouse(handle, id, from, MouseButton::Left, ButtonState::Pressed, 1);
            mouse(handle, id, to, MouseButton::Left, ButtonState::Released, 1);
        }
        Step::Wheel(fx, fy, notches, to) => {
            let id = target(handle, window, *to);
            let position = point(handle, id, *fx, *fy);
            let wheel = WindowEvent::MouseWheel {
                delta: ScrollDelta::Lines {
                    x: 0.0,
                    y: *notches,
                },
                position,
                modifiers: Modifiers::NONE,
            };
            let _ = handle.inject(id, wheel);
        }
        Step::Window(event, to) => {
            let _ = handle.inject(target(handle, window, *to), event.clone());
        }
        Step::Open(tool) => {
            let _ = window.use_tool(&mut event_loop.context(), *tool);
        }
    }
}

/// Runs a session in a fresh headless loop.
fn run(session: &Session) {
    let (figure, _text_box) = figure(session.figure);
    let (mut event_loop, handle) = EventLoop::headless();
    let Ok(mut window) = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1") else {
        return;
    };
    pump(&mut event_loop, &mut window);
    if let Some(tool) = session.opened {
        let _ = window.use_tool(&mut event_loop.context(), tool);
        pump(&mut event_loop, &mut window);
    }
    for step in &session.steps {
        inject(step, &handle, &mut window, &mut event_loop);
        if !pump(&mut event_loop, &mut window) {
            return;
        }
    }
    let _ = window.close(&mut event_loop.context());
}

/// The sandbox the save dialog starts in, removed when dropped.
struct Sandbox {
    root: PathBuf,
    deep: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("pltrs-fuzz-gui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let deep = (0..SANDBOX_DEPTH).fold(root.clone(), |dir, _| dir.join("d"));
        std::fs::create_dir_all(deep.join("sub")).expect("the sandbox is made");
        std::fs::write(deep.join("plot.png"), b"").expect("a file to list");
        Self { root, deep }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn dialogs_survive_random_input() {
    let sandbox = Sandbox::new();
    let directory = sandbox.deep.display().to_string();
    fuzz("gui dialogs", 120, Session::generate, |session| {
        crate::rc::context(|| {
            crate::rc::update(|rc| rc.savefig_directory.clone_from(&directory));
            run(session);
        });
    });
}
