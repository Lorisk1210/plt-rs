//! Chapter 10: interactive figures.
//!
//! Your program can react to input with callbacks, add controls such as
//! sliders and animate artists. Synthetic events call the same callbacks
//! without a window, for tests.
//!
//! # Windows
//!
//! [`gui::show`](crate::gui::show) shows figures in windows and returns
//! when the user closes all of them. The toolbar and the keyboard have
//! these tools:
//!
//! | tool | key |
//! |------|-----|
//! | home (the original view) | `h` |
//! | back and forward through the views | `left`, `right`, mouse buttons |
//! | pan, zoom to a rectangle | `p`, `o` |
//! | grid, log scale on y and x | `g`, `l`, `k` |
//! | save, copy the figure | `s`, `ctrl+c` |
//! | full screen, close, list of shortcuts | `f`, `q`, `F1` |
//! | subplot spacing, a dialog for titles, limits, scales and line styles | |
//!
//! The mouse wheel zooms around the pointer, and a readout shows the data
//! coordinates. The `keymap.*` settings change the keys.
//!
//! ```no_run
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! fig.subplot().plot([0.0, 1.0, 2.0, 3.0], [1.0, 3.0, 2.0, 4.0])?;
//! pltrs::gui::show(&[fig])?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `gui::show` and [`pyplot::show`] use the current *backend*, which opens
//! windows. If it cannot open a window, it shows a web page for each figure
//! on macOS and Windows, and nothing on Linux. `MPLBACKEND` or [`pyplot::switch_backend`] selects a backend.
//! In interactive mode ([`pyplot::ion`]), `show` returns immediately, and
//! [`pyplot::pause`] updates the windows while a script computes.
//!
//! # Events and callbacks
//!
//! [`Figure::connect`] registers a closure for one kind of event, such as
//! a key press or a draw, and returns an id for [`Figure::disconnect`]. Mouse events have the position in pixels and,
//! over an axes, in data coordinates. The closure must be `'static`, so it
//! owns clones of handles and keeps shared state in an `Rc<RefCell<_>>` or
//! `Rc<Cell<_>>`. It can use any handle of the figure.
//!
//! An [`EventSource`](crate::events::EventSource) sends synthetic input.
//! Here it clicks at the data point (5, 5), which
//! [`Axes::data_to_display`] converts to pixels:
//!
//! ```
//! use std::cell::RefCell;
//! use std::rc::Rc;
//!
//! use pltrs::events::{Event, EventKind, EventSource};
//! use pltrs::navigation::PointerButton;
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 10.0)?;
//!
//! let clicks = Rc::new(RefCell::new(Vec::new()));
//! let seen = Rc::clone(&clicks);
//! let id = fig.connect(EventKind::ButtonPress, move |event| {
//!     if let Event::ButtonPress(mouse) = event
//!         && let (Some(x), Some(y)) = (mouse.location.xdata, mouse.location.ydata)
//!     {
//!         seen.borrow_mut().push((x, y));
//!     }
//! });
//!
//! let at = ax.data_to_display((5.0, 5.0))?;
//! EventSource::new().button_press(&fig, at.x, at.y, PointerButton::Left, false);
//! let (x, y) = clicks.borrow()[0];
//! assert!((x - 5.0).abs() < 0.1 && (y - 5.0).abs() < 0.1, "whole pixels, like a window");
//!
//! assert!(fig.disconnect(id));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Figure::add_timer`] calls a closure at an interval while the figure
//! is shown, until the closure returns `false`. An artist with a picker
//! ([`ArtistHandle::set_picker`](crate::artist::ArtistHandle::set_picker))
//! sends pick events with the artist and the points hit.
//!
//! # Widgets
//!
//! [`widgets`](crate::widgets) are controls in their own axes: sliders,
//! buttons, check buttons, radio buttons, text boxes, and span, rectangle
//! and lasso selectors. A widget calls your closures when its value
//! changes. It stays connected only while it lives, so keep it in a
//! variable.
//!
//! A widget owns its event connections, so you cannot clone it. To use it
//! in the callback of another widget, share it through an `Rc`:
//!
//! ```
//! use std::rc::Rc;
//!
//! use pltrs::events::EventSource;
//! use pltrs::navigation::PointerButton;
//! use pltrs::prelude::*;
//! use pltrs::widgets::{Button, ButtonOptions, Slider, SliderOptions};
//!
//! let fig = Figure::new();
//! let ax = fig.add_axes([0.1, 0.3, 0.8, 0.6])?;
//! let x = linspace(0.0, 1.0, 200);
//! let wave = ax.plot(&x, x.iter().map(|x| (5.0 * x).sin()))?;
//!
//! let frequency = Rc::new(Slider::new(
//!     &fig.add_axes([0.2, 0.1, 0.5, 0.05])?,
//!     "frequency",
//!     1.0,
//!     30.0,
//!     SliderOptions { valinit: 5.0, ..SliderOptions::default() },
//! )?);
//! let line = wave.clone();
//! frequency.on_changed(move |k| {
//!     line.set_ydata(line.xdata().iter().map(|x| (k * x).sin()));
//! });
//!
//! let reset_ax = fig.add_axes([0.8, 0.1, 0.1, 0.05])?;
//! let reset = Button::new(&reset_ax, "Reset", ButtonOptions::default());
//! let slider = Rc::clone(&frequency);
//! reset.on_clicked(move || slider.reset());
//!
//! frequency.set_val(12.0); // what dragging the slider does
//! assert_eq!(wave.ydata()[10], (12.0 * x[10]).sin());
//!
//! // A click in the middle of the button, as the mouse makes one.
//! let middle = reset_ax.data_to_display((0.5, 0.5))?;
//! let mut input = EventSource::new();
//! input.button_press(&fig, middle.x, middle.y, PointerButton::Left, false);
//! input.button_release(&fig, middle.x, middle.y, PointerButton::Left);
//! assert_eq!(frequency.val(), 5.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Gallery: `widgets` in `diagrams.rs`. `cargo run -p pltrs --example
//! widgets` opens a window with live widgets.
//!
//! # The window, off screen
//!
//! [`FigureWindow::screenshot`] draws the window of `gui::show`, with its
//! toolbar, without a display, and returns the pixels. The second argument
//! is the pixel ratio (2 for a high-density display). The slider is where
//! `set_val` moved it. [`image::imsave`](crate::image::imsave) saves the pixels.
//!
//! ```
//! use pltrs::gui::FigureWindow;
//! use pltrs::image::{ImsaveOptions, imsave};
//! use pltrs::prelude::*;
//! # let dir = std::env::temp_dir().join(format!("pltrs-tutorial-window-{}", std::process::id()));
//! # std::fs::create_dir_all(&dir)?;
//! # std::env::set_current_dir(&dir)?;
//!
//! let fig = Figure::with_size(4.0, 3.0, 100.0);
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! let window = FigureWindow::screenshot(&fig, 1.0)?;
//! assert_eq!(window.width(), 400, "as wide as the figure");
//! assert!(window.height() > 300, "and taller by the toolbar");
//! imsave("window.png", window, ImsaveOptions::default())?;
//! # assert!(std::fs::read("window.png")?.starts_with(b"\x89PNG"));
//! # std::env::set_current_dir(std::env::temp_dir())?; // Windows cannot remove it otherwise
//! # std::fs::remove_dir_all(&dir)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `cargo run -p pltrs --example figure_window -- --screenshot window.png`
//! does the same.
//!
//! # Animation
//!
//! [`FuncAnimation`](crate::animation::FuncAnimation) calls a closure for
//! each frame. The closure changes artists and returns them as a handle,
//! an array or `Vec` of handles, or a tuple. With blitting, pltrs draws
//! only those artists again. In a window, the animation plays. You can
//! also save it as an animated PNG, GIF or WebP file, an HTML page or a
//! video through ffmpeg.
//!
//! ```
//! use std::time::Duration;
//!
//! use pltrs::animation::{AnimationOptions, Frames, FuncAnimation};
//! use pltrs::prelude::*;
//!
//! let fig = Figure::with_size(3.0, 2.0, 50.0);
//! let ax = fig.subplot();
//! ax.set_xlim(0.0, 6.3)?.set_ylim(-1.1, 1.1)?;
//! let x = linspace(0.0, 6.3, 100);
//! let wave = ax.plot(&x, x.iter().map(|x| x.sin()))?;
//!
//! let options = AnimationOptions {
//!     interval: Duration::from_millis(40),
//!     ..AnimationOptions::default()
//! };
//! let animation = FuncAnimation::new(&fig, Frames::Count(10), options, move |frame| {
//!     let phase = frame as f64 * 0.6;
//!     wave.set_ydata(x.iter().map(|x| (x + phase).sin()));
//!     wave.clone()
//! });
//! let gif = animation.to_gif()?; // `animation.save("wave.gif")?` writes the file
//! assert!(gif.starts_with(b"GIF89a"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Gallery: `animation` in `diagrams.rs`.
//!
//! # In a web browser
//!
//! [`web::serve`](crate::web::serve) serves figures to a web browser, with
//! the toolbar, events and timers of a window, and prints the address. It
//! also works on a remote machine. The settings `webagg.address` and
//! `webagg.port` select where it listens.
//!
//! ```no_run
//! use pltrs::prelude::*;
//! use pltrs::web::{self, WebOptions};
//!
//! let fig = Figure::new();
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! web::serve(&[fig], WebOptions::from_rc()?)?; // returns when the user closes the page with `q`
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `cargo run -p pltrs --example web_figure` serves an animated figure.
//! The page [`porting::interaction`](crate::porting::interaction) covers
//! events, navigation, blitting and all widgets.
//!
//! Next: [extending pltrs](super::extending).
//!
//! [`pyplot::show`]: crate::pyplot::show
//! [`pyplot::switch_backend`]: crate::pyplot::switch_backend
//! [`pyplot::ion`]: crate::pyplot::ion
//! [`pyplot::pause`]: crate::pyplot::pause
//! [`Figure::connect`]: crate::figure::Figure::connect
//! [`Figure::disconnect`]: crate::figure::Figure::disconnect
//! [`Figure::add_timer`]: crate::figure::Figure::add_timer
//! [`Axes::data_to_display`]: crate::axes::Axes::data_to_display
//! [`FigureWindow::screenshot`]: crate::gui::FigureWindow::screenshot
