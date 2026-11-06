//! pltrs' figure windows on real display servers: each test opens a
//! figure window on X11 when `$DISPLAY` is set and on Wayland when
//! `$WAYLAND_DISPLAY` is, drives its tools with synthetic input handed
//! to the windows as the platform hands them input, and checks the
//! figure and what the windows show; without a display server the tests
//! pass trivially. A private Xvfb and the headless weston of
//! `pltrs-platform` serve:
//!
//! ```text
//! Xvfb :117 -screen 0 1280x800x24 -nolisten tcp &
//! crates/pltrs-platform/tools/weston.sh run &
//! DISPLAY=:117 WAYLAND_DISPLAY=wayland-pltrs cargo test -p pltrs --test gui_live
//! ```
//!
//! The figures take matplotlib's look, but for the test of the subplot
//! tool under constrained layout, which takes pltrs' modern look.

#![cfg(target_os = "linux")]

use std::error::Error;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use pltrs::figure::Figure;
use pltrs::gui::FigureWindow;
use pltrs::layout::LayoutEngine;
use pltrs::look::Appearance;
use pltrs_platform::{
    BackendKind, ButtonState, Context, Event, EventLoop, Key, KeyEvent, Modifiers, MouseButton,
    PhysicalPosition, ScrollDelta, WindowEvent, WindowId,
};

/// The tests share the look, the servers' screens and their pointer:
/// they run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

/// How long a window takes to open and draw.
const OPENING: Duration = Duration::from_millis(500);
/// How long a step of input takes to be handled and shown.
const SETTLE: Duration = Duration::from_millis(150);

/// The display servers to test on: X11 with `$DISPLAY`, Wayland with
/// `$WAYLAND_DISPLAY`.
fn servers() -> Vec<BackendKind> {
    let set = |name| std::env::var_os(name).is_some_and(|value| !value.is_empty());
    let mut servers = Vec::new();
    if set("DISPLAY") {
        servers.push(BackendKind::X11);
    }
    if set("WAYLAND_DISPLAY") {
        servers.push(BackendKind::Wayland);
    }
    servers
}

/// An event loop on `kind`'s server. An X server resets when its last
/// client disconnects and drops connections meanwhile: retry a while.
fn connect(kind: BackendKind) -> EventLoop {
    for _ in 0..100 {
        match EventLoop::with_backend(kind) {
            Ok(event_loop) => return event_loop,
            Err(_) => thread::sleep(Duration::from_millis(20)),
        }
    }
    panic!("cannot connect to the {kind:?} server");
}

/// Runs `test` on every display server there is, in `appearance`.
fn on_every_server(appearance: Appearance, test: fn(&mut Shown) -> Result<(), Box<dyn Error>>) {
    let _serial: MutexGuard<'_, ()> = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    pltrs::look::set_appearance(appearance);
    for kind in servers() {
        let mut shown = Shown::open(connect(kind), &grid_figure());
        if let Err(err) = test(&mut shown) {
            panic!("on {kind:?}: {err}");
        }
        shown.close();
    }
}

/// A figure window shown on a display server, driven by synthetic input.
struct Shown {
    event_loop: EventLoop,
    window: Option<FigureWindow>,
    /// Synthetic input waiting to be handed over.
    queued: Vec<(WindowId, WindowEvent)>,
}

impl Shown {
    /// Shows `figure` in a window of `event_loop`'s server.
    fn open(mut event_loop: EventLoop, figure: &Figure) -> Self {
        let window = FigureWindow::open(&mut event_loop.context(), figure, "Figure 1")
            .expect("the window opens");
        let mut shown = Self {
            event_loop,
            window: Some(window),
            queued: Vec::new(),
        };
        shown.run(OPENING);
        shown
    }

    fn window(&self) -> &FigureWindow {
        self.window.as_ref().expect("the window is open")
    }

    fn figure(&self) -> Figure {
        self.window().figure().clone()
    }

    fn main(&self) -> WindowId {
        self.window().id()
    }

    /// Hands the input queued to the windows, then handles what the
    /// server sends for `time`.
    fn run(&mut self, time: Duration) {
        let window = self.window.as_mut().expect("the window is open");
        let mut handle = |cx: &mut Context<'_>, event: Event| {
            let open = match event {
                Event::Window { window: id, event } if window.owns_window(id) => window
                    .handle(cx, id, &event)
                    .expect("the window handles input"),
                Event::Timer(timer) if window.owns_timer(timer) => {
                    window.tick(cx, timer).expect("the timer runs");
                    true
                }
                _ => true,
            };
            assert!(open, "the window stays open");
        };
        let mut cx = self.event_loop.context();
        for (id, event) in self.queued.drain(..) {
            handle(&mut cx, Event::Window { window: id, event });
        }
        self.event_loop
            .run_until(Instant::now() + time, &mut handle)
            .expect("the server answers");
    }

    /// The windows open besides the figure window.
    fn tool_windows(&mut self) -> Vec<WindowId> {
        let main = self.main();
        let open = self.event_loop.context().windows();
        open.into_iter().filter(|&id| id != main).collect()
    }

    /// The one window open besides the figure window.
    fn tool_window(&mut self) -> WindowId {
        let tools = self.tool_windows();
        assert_eq!(tools.len(), 1, "one tool window: {tools:?}");
        tools[0]
    }

    /// Presses and releases `key` with `modifiers` in `window`, typing
    /// the characters of keys pressed without Control.
    fn press(&mut self, window: WindowId, key: Key, modifiers: Modifiers) {
        let text = match key {
            Key::Character(c) if !modifiers.control() => Some(c.to_string()),
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
            self.queued.push((window, WindowEvent::Key(event)));
        }
    }

    /// Types `text` into `window`.
    fn type_text(&mut self, window: WindowId, text: &str) {
        for c in text.chars() {
            self.press(window, Key::Character(c), Modifiers::NONE);
        }
    }

    /// Opens the subplot tool from the toolbar, from the keyboard:
    /// Ctrl+Tab focuses Home, Tab passes the disabled history buttons
    /// to Pan, Zoom and the tool's button, Space presses it.
    fn open_subplot_tool(&mut self) -> WindowId {
        let main = self.main();
        self.press(main, Key::Tab, Modifiers::CONTROL);
        for _ in 0..3 {
            self.press(main, Key::Tab, Modifiers::NONE);
        }
        self.type_text(main, " ");
        self.run(OPENING);
        self.tool_window()
    }

    /// Moves the pointer over the canvas to `(x, y)` physical pixels, and
    /// presses or releases `button` there.
    fn mouse(&mut self, at: (f64, f64), button: Option<(MouseButton, ButtonState)>) {
        let main = self.main();
        let position = PhysicalPosition::new(at.0, at.1);
        let modifiers = Modifiers::NONE;
        self.queued.push((
            main,
            WindowEvent::CursorMoved {
                position,
                modifiers,
            },
        ));
        if let Some((button, state)) = button {
            let click = WindowEvent::MouseInput {
                button,
                state,
                position,
                modifiers,
                click_count: 1,
            };
            self.queued.push((main, click));
        }
    }

    /// The RGBA pixel at `(x, y)` physical pixels of what `window` shows.
    fn pixel(&mut self, window: WindowId, (x, y): (u32, u32)) -> [u8; 4] {
        let frame = self
            .event_loop
            .context()
            .read_pixels(window)
            .expect("the window reads back");
        let width = frame.size().width;
        let pixels = frame.into_pixels();
        let at = 4 * (y * width + x) as usize;
        pixels[at..at + 4].try_into().expect("four channels")
    }

    /// Closes the figure window and whatever it opened.
    fn close(mut self) {
        if let Some(window) = self.window.take() {
            window
                .close(&mut self.event_loop.context())
                .expect("the window closes");
        }
        assert_eq!(self.event_loop.context().windows(), []);
    }
}

/// A 800 × 600 figure of a 2 × 2 grid of axes painted in a solid colour,
/// so that the windows show where they are.
fn grid_figure() -> Figure {
    let figure = Figure::with_size(8.0, 6.0, 100.0);
    let grid = figure.subplots(2, 2).expect("a 2×2 grid has cells");
    for ax in grid.as_slice() {
        ax.set_facecolor("#1f77b4");
    }
    figure
}

/// Where the tool windows cannot cover the figure window: its
/// bottom-right part (windows open in the top-left corner of a server
/// without a window manager), inside the bottom-right axes as the
/// default subplot parameters place it, below its top edge once the
/// bottom border is at 0.3.
const BOTTOM_RIGHT: (u32, u32) = (650, 500);
/// The colour of the axes.
const AXES: [u8; 4] = [0x1f, 0x77, 0xb4, 0xff];
/// The figure's background.
const WHITE: [u8; 4] = [0xff; 4];

#[test]
fn the_subplot_tool_adjusts_the_figure_as_matplotlibs_qt_tool() {
    on_every_server(Appearance::MATPLOTLIB, |shown| {
        let main = shown.main();
        assert_eq!(shown.pixel(main, BOTTOM_RIGHT), AXES);
        let tool = shown.open_subplot_tool();
        // From the top border's slider, which has the focus, to the
        // bottom border's field: 0.3 there, applied by Enter.
        for _ in 0..3 {
            shown.press(tool, Key::Tab, Modifiers::NONE);
        }
        shown.press(tool, Key::Character('a'), Modifiers::CONTROL);
        shown.type_text(tool, "0.3");
        shown.press(tool, Key::Enter, Modifiers::NONE);
        shown.run(SETTLE);
        let params = shown.figure().subplot_params();
        assert_eq!(params.bottom, 0.3, "the field applied");
        assert_eq!(shown.pixel(main, BOTTOM_RIGHT), WHITE, "the axes moved up");
        assert_eq!(shown.tool_windows(), [tool], "Enter left the tool open");
        // Back to the top border's slider; Right steps it by 0.005.
        for _ in 0..3 {
            shown.press(tool, Key::Tab, Modifiers::SHIFT);
        }
        shown.press(tool, Key::Right, Modifiers::NONE);
        shown.run(SETTLE);
        assert_eq!(shown.figure().subplot_params().top, 0.885);
        // Reset: Shift+Tab from the slider wraps to Close, then Reset.
        shown.press(tool, Key::Tab, Modifiers::SHIFT);
        shown.press(tool, Key::Tab, Modifiers::SHIFT);
        shown.type_text(tool, " ");
        shown.run(SETTLE);
        let params = shown.figure().subplot_params();
        assert_eq!((params.bottom, params.top), (0.11, 0.88));
        assert_eq!(shown.pixel(main, BOTTOM_RIGHT), AXES, "back in place");
        // Escape closes the tool, and the figure window stays.
        shown.press(tool, Key::Escape, Modifiers::NONE);
        shown.run(SETTLE);
        assert_eq!(shown.tool_windows(), []);
        Ok(())
    });
}

#[test]
fn the_subplot_tool_adjusts_constrained_layout_and_gives_way() {
    on_every_server(Appearance::MODERN, |shown| {
        let figure = shown.figure();
        figure.set_layout_engine(LayoutEngine::constrained())?;
        let main = shown.main();
        shown.run(SETTLE);
        // 20 pixels above the bottom edge of the bottom-right axes, in the
        // middle: out of the tool's way, outside the axes once the
        // padding is half an inch, and inside again in a tight layout.
        let (x, y, width, _) = figure.axes()[3].position().bounds();
        let inside = (
            ((x + width / 2.0) * 800.0) as u32,
            ((1.0 - y) * 600.0) as u32 - 20,
        );
        assert_eq!(shown.pixel(main, inside), AXES);
        let tool = shown.open_subplot_tool();
        // The switch has the focus; Tab leads to the slider of h_pad,
        // whose End gives the widest padding.
        shown.press(tool, Key::Tab, Modifiers::NONE);
        shown.press(tool, Key::End, Modifiers::NONE);
        shown.run(SETTLE);
        let Some(LayoutEngine::Constrained(layout)) = figure.layout_engine() else {
            return Err("constrained layout went off".into());
        };
        assert_eq!(layout.h_pad, 0.5);
        assert_eq!(shown.pixel(main, inside), WHITE, "the padding grew");
        // Back to the switch: Right hands the subplots to the subplot
        // parameters, from a tight layout.
        shown.press(tool, Key::Tab, Modifiers::SHIFT);
        shown.press(tool, Key::Right, Modifiers::NONE);
        shown.run(SETTLE);
        assert!(matches!(
            figure.layout_engine(),
            Some(LayoutEngine::PlaceHolder(_))
        ));
        let params = figure.subplot_params();
        assert!(params.left < 0.125 && params.top > 0.88, "{params:?}");
        assert_eq!(shown.pixel(main, inside), AXES, "the axes grew back");
        Ok(())
    });
}

#[test]
fn panning_and_the_wheel_move_the_view_as_in_matplotlib() {
    on_every_server(Appearance::MATPLOTLIB, |shown| {
        let figure = shown.figure();
        let ax = figure.axes()[3].clone();
        ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 10.0)?;
        let main = shown.main();
        shown.run(SETTLE);
        // Pan mode ('p'), then drag in the bottom-right axes.
        shown.type_text(main, "p");
        let (from, to) = ((600.0, 450.0), (500.0, 450.0));
        shown.mouse(from, Some((MouseButton::Left, ButtonState::Pressed)));
        shown.mouse(to, None);
        shown.mouse(to, Some((MouseButton::Left, ButtonState::Released)));
        shown.run(SETTLE);
        let (x0, x1) = ax.xlim();
        assert!(x0 > 0.0 && (x1 - x0 - 10.0).abs() < 1e-9, "{x0}, {x1}");
        assert_eq!(ax.ylim(), (0.0, 10.0));
        // A bare wheel turn zooms nothing; with Control it zooms in.
        let wheel = |modifiers| WindowEvent::MouseWheel {
            delta: ScrollDelta::Lines { x: 0.0, y: 1.0 },
            position: PhysicalPosition::new(600.0, 450.0),
            modifiers,
        };
        shown.queued.push((main, wheel(Modifiers::NONE)));
        shown.run(SETTLE);
        assert_eq!(ax.xlim(), (x0, x1));
        shown.queued.push((main, wheel(Modifiers::CONTROL)));
        shown.run(SETTLE);
        let (z0, z1) = ax.xlim();
        assert!((z1 - z0 - 8.5).abs() < 1e-9, "0.85 per step: {z0}, {z1}");
        // Home ('h') puts the first view back.
        shown.type_text(main, "h");
        shown.run(SETTLE);
        assert_eq!(ax.xlim(), (0.0, 10.0));
        Ok(())
    });
}

#[test]
fn the_save_dialog_help_and_resizing_work_on_the_display() {
    on_every_server(Appearance::MATPLOTLIB, |shown| {
        let directory = std::env::temp_dir().join(format!(
            "pltrs-gui-live-{}-{:?}",
            std::process::id(),
            thread::current().id()
        ));
        std::fs::create_dir_all(&directory)?;
        let saved = pltrs::rc::context(|| -> Result<bool, Box<dyn Error>> {
            pltrs::rc::update(|rc| rc.savefig_directory = directory.display().to_string());
            let main = shown.main();
            shown.type_text(main, "s");
            shown.run(OPENING);
            let dialog = shown.tool_window();
            // The file name has the focus: a name in another format,
            // which Enter saves.
            shown.press(dialog, Key::Character('a'), Modifiers::CONTROL);
            shown.type_text(dialog, "live.svgz");
            shown.press(dialog, Key::Enter, Modifiers::NONE);
            shown.run(SETTLE);
            Ok(directory.join("live.svgz").is_file())
        })?;
        std::fs::remove_dir_all(&directory)?;
        assert!(saved, "the dialog saved the figure as SVGZ");
        assert_eq!(shown.tool_windows(), []);

        let main = shown.main();
        shown.press(main, Key::F(1), Modifiers::NONE);
        shown.run(OPENING);
        let help = shown.tool_window();
        shown.press(help, Key::Escape, Modifiers::NONE);
        shown.run(SETTLE);
        assert_eq!(shown.tool_windows(), []);

        // The figure follows the window's size.
        let window = shown.window.as_mut().expect("the window is open");
        window.resize(&mut shown.event_loop.context(), 700, 500)?;
        shown.run(OPENING);
        let (width, _) = shown.figure().size_inches();
        if (width - 7.0).abs() > 1e-9 {
            return Err(format!("the figure is {width} inches wide").into());
        }
        Ok(())
    });
}
