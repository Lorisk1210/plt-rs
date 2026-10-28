//! Chapter 10: interactive figures.
//!
//! A figure shown in a window is live. The user pans, zooms and saves it
//! from a toolbar; your program reacts to clicks, keys and timers through
//! callbacks, adds controls such as sliders and buttons, and animates
//! artists. All of this also works without a window: synthetic events
//! drive the same callbacks, which makes interactive code testable.
//!
//! # Windows
//!
//! [`gui::show`](crate::gui::show) shows figures in windows (on X11) and
//! returns when the user has closed them all. Each window has a toolbar:
//! home (the original view), back and forward through the views, pan,
//! zoom to a rectangle, the spacing of the subplots, a dialog editing
//! titles, limits, scales and line styles, and save. The mouse wheel
//! zooms around the pointer, and a readout shows the data coordinates
//! under it.
//!
//! The keyboard has matplotlib's shortcuts: `h` home, `left` and `right`
//! back and forward (as do the mouse's back and forward buttons), `p`
//! pan, `o` zoom, `g` grid, `l` and `k` log scales on y and x, `s` save,
//! `f` full screen, `ctrl+c` copy the figure, `q` close, `F1` the list of
//! shortcuts. The `keymap.*` settings rebind them.
//!
//! ```no_run
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! fig.subplot().plot([0.0, 1.0, 2.0, 3.0], [1.0, 3.0, 2.0, 4.0])?;
//! plt::gui::show(&[fig])?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `gui::show` and [`pyplot::show`], which shows pyplot's figures, use
//! the current *backend*: windows where there is X11, a web page each
//! where there is none (macOS, Windows), nothing on a machine without a
//! display. The `MPLBACKEND` environment variable or
//! [`pyplot::switch_backend`] choose explicitly. [`pyplot::ion`] turns on interactive mode, where
//! `show` returns at once and [`pyplot::pause`] lets the windows update
//! while a script computes.
//!
//! # Events and callbacks
//!
//! [`Figure::connect`] registers a closure for one kind of event (a mouse
//! button pressed, the pointer moved, a key pressed, an artist picked,
//! the figure drawn or resized, ...) and returns an id for
//! [`Figure::disconnect`]. Mouse events carry the position in pixels and,
//! over an axes, in data coordinates. The closure must be `'static`, so
//! it owns what it uses: clones of handles, and shared state in an
//! `Rc<RefCell<_>>` or `Rc<Cell<_>>`. It may use any handle of the
//! figure, as every closure plt-rs runs between draws may.
//!
//! Windows and the browser backend deliver real input. An
//! [`EventSource`](crate::events::EventSource) delivers synthetic input
//! in the same way, here a click at the data point (5, 5), which
//! [`Axes::data_to_display`] places in pixels:
//!
//! ```
//! use std::cell::RefCell;
//! use std::rc::Rc;
//!
//! use plt::events::{Event, EventKind, EventSource};
//! use plt::navigation::PointerButton;
//! use plt::prelude::*;
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
//! is shown; the closure returns whether to keep going. Artists answer
//! clicks of their own once given a picker
//! ([`ArtistHandle::set_picker`](crate::artist::ArtistHandle::set_picker)),
//! which sends pick events naming the artist and the points hit.
//!
//! # Widgets
//!
//! [`widgets`](crate::widgets) are controls drawn in axes of their own:
//! sliders, buttons, check and radio buttons, text boxes, and selectors
//! for spans, rectangles and lassos. Each takes the axes it lives in and
//! its options, and calls your closures when its value changes. A widget
//! stays connected while its value lives, so keep it in a variable for
//! as long as the figure is shown.
//!
//! A widget is a value, not a handle: it owns its connections to the
//! figure's events, so it cannot be cloned. To use one in another
//! widget's callback, share it through an `Rc`, as the reset button
//! below shares the slider:
//!
//! ```
//! use std::rc::Rc;
//!
//! use plt::events::EventSource;
//! use plt::navigation::PointerButton;
//! use plt::prelude::*;
//! use plt::widgets::{Button, ButtonOptions, Slider, SliderOptions};
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
//! Gallery: `widgets` in `diagrams.rs`; `cargo run -p plt --example
//! widgets` opens a window with live widgets.
//!
//! # The window, off screen
//!
//! [`FigureWindow::screenshot`] draws the window `gui::show` would open,
//! toolbar and all, without a display, and returns its pixels: for a
//! picture of the window in documentation, a test of what users see, or
//! a server with no screen. The second argument is the pixel ratio of
//! the display; 2 draws the window as a high-density display shows it.
//! The figure is drawn as it is now, so a screenshot taken after the
//! widget code above shows the slider where `set_val` moved it.
//! [`image::imsave`](crate::image::imsave) saves the pixels.
//!
//! ```
//! use plt::gui::FigureWindow;
//! use plt::image::{ImsaveOptions, imsave};
//! use plt::prelude::*;
//! # let dir = std::env::temp_dir().join(format!("plt-tutorial-window-{}", std::process::id()));
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
//! # std::fs::remove_dir_all(&dir)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `cargo run -p plt --example figure_window -- --screenshot window.png`
//! saves the window of that example the same way.
//!
//! # Animation
//!
//! [`FuncAnimation`](crate::animation::FuncAnimation) calls a closure for
//! each frame. The closure updates artists and returns the ones it
//! changed (a handle, an array or a `Vec` of handles, or a tuple of
//! handles of different kinds), so that with blitting only those are
//! redrawn. Shown in a window, the
//! animation plays; saved, it becomes an animated PNG, GIF or WebP file,
//! an HTML page with a player, or a video through an installed ffmpeg.
//!
//! ```
//! use std::time::Duration;
//!
//! use plt::animation::{AnimationOptions, Frames, FuncAnimation};
//! use plt::prelude::*;
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
//! [`web::serve`](crate::web::serve) serves figures to a web browser,
//! with the same toolbar, events and timers as a window, and prints the
//! address. It works on any system, a remote machine included: the
//! `webagg.address` and `webagg.port` settings choose where it listens.
//!
//! ```no_run
//! use plt::prelude::*;
//! use plt::web::{self, WebOptions};
//!
//! let fig = Figure::new();
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! web::serve(&[fig], WebOptions::from_rc()?)?; // until the page is closed with `q`
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `cargo run -p plt --example web_figure` serves an animated figure. The
//! [`porting::interaction`](crate::porting::interaction) page covers
//! events, navigation, blitting and every widget.
//!
//! Next: [extending plt-rs](super::extending).
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
