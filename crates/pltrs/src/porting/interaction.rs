//! Interaction: events, widgets, navigation, toolbar tools, animation
//! and blitting.
//!
//! **Events.** Callbacks connect to the figure itself, not to a canvas:
//! [`Figure::connect`] takes an [`EventKind`] (matplotlib's event name)
//! and a closure receiving an [`Event`], whose variants carry the
//! matplotlib event objects ([`MouseEvent`], [`KeyEvent`],
//! [`PickEvent`]). Windows ([`crate::gui`]) and the browser backend
//! ([`crate::web`]) feed real input; an [`EventSource`] feeds synthetic
//! input the same way, which makes interactive code testable without a
//! display. Timers are [`Figure::add_timer`]; the closure returns whether
//! the timer keeps running.
//!
//! **Widgets.** Each widget takes an axes and an options struct, and
//! registers callbacks with `on_*` methods. A widget stays connected as
//! long as its value lives, so keep it while the figure is shown.
//!
//! **Navigation.** The toolbar's pan, zoom and view history live in a
//! [`Navigation`] value that works on any figure, shown or not. Toolbar
//! tools of your own are [`Figure::add_tool`] (matplotlib's tool
//! manager).
//!
//! **Animation.** [`FuncAnimation`] covers both of matplotlib's
//! animation classes: [`FuncAnimation::new`] calls a closure per frame,
//! [`FuncAnimation::from_artists`] is `ArtistAnimation`. Saving does not
//! need writers: animated PNG, GIF and WebP and the HTML player are
//! encoded by pltrs; videos go through an installed ffmpeg.
//!
//! **Blitting.** The [`FigureCanvas`] from [`Figure::canvas`] keeps the
//! last rendering, with matplotlib's `copy_from_bbox`, `restore_region`
//! and `blit`, and [`Axes::draw_artist`] draws one artist onto it.
//!
//! # Equivalences
//!
//! | matplotlib                                      | pltrs                                          |
//! |-------------------------------------------------|------------------------------------------------|
//! | `fig.canvas.mpl_connect("button_press_event", f)` | [`Figure::connect`]`(EventKind::ButtonPress, f)` |
//! | `"motion_notify_event"`, `"scroll_event"`, `"key_press_event"` | [`EventKind::Motion`], [`EventKind::Scroll`], [`EventKind::KeyPress`] |
//! | `"pick_event"`, `"draw_event"`, `"resize_event"`, `"close_event"` | [`EventKind::Pick`], [`EventKind::Draw`], [`EventKind::Resize`], [`EventKind::Close`] |
//! | `fig.canvas.mpl_disconnect(cid)`                | [`Figure::disconnect`]                         |
//! | `with fig.canvas.callbacks.blocked(signal=...):` | [`Figure::with_callbacks_blocked`]`(Some(kind), f)`, or the guard of [`Figure::block_callbacks`] |
//! | `ax.callbacks.connect("xlim_changed", f)`      | [`Axes::on_limits_changed`]`(AxisName::X, f)`  |
//! | `ax.callbacks.disconnect(cid)`                  | [`Axes::disconnect_limits_changed`]            |
//! | `event.x`, `event.xdata`, `event.inaxes`        | [`Location::x`], [`Location::xdata`], [`Location::inaxes`] |
//! | `event.button`, `event.key`, `event.dblclick`   | [`MouseEvent::button`], [`MouseEvent::key`], [`MouseEvent::dblclick`] |
//! | `MouseButton.BACK`, `MouseButton.FORWARD`, `.value` | [`PointerButton::Back`], [`PointerButton::Forward`], [`PointerButton::number`] |
//! | `event.artist`, `event.ind` (pick)              | [`PickEvent::artist`], [`PickEvent::ind`]      |
//! | `artist.set_picker(5)`                          | [`ArtistHandle::set_picker`]`(Some(Picker::Radius(5.0)))` |
//! | `fig.canvas.button_press_event(...)` in tests   | [`EventSource::button_press`] and friends      |
//! | `ax.transData.transform((x, y))` for a click    | [`Axes::data_to_display`]`((x, y))?`           |
//! | `fig.canvas.new_timer(interval)`, `timer.start()` | [`Figure::add_timer`], [`Figure::remove_timer`] |
//! | `fig.canvas.set_cursor(...)`                    | [`Figure::set_cursor`]                         |
//! | `fig.canvas.widgetlock`                         | [`Figure::lock_widgets`], [`Figure::release_widgets`] |
//! | `plt.ginput(n)`, `plt.waitforbuttonpress()`     | [`crate::pyplot::ginput`], [`crate::pyplot::waitforbuttonpress`] |
//! | `Slider(ax, label, valmin, valmax, valinit=)`   | [`Slider::new`] with [`SliderOptions`]         |
//! | `slider.on_changed(f)`, `slider.set_val(v)`     | [`Slider::on_changed`], [`Slider::set_val`]    |
//! | `cid = button.on_clicked(f)`, `button.disconnect(cid)` | [`Button::on_clicked`] returns a [`CallbackId`] for [`Button::disconnect`]; every widget has both |
//! | `RangeSlider`, `Button`, `CheckButtons`, `RadioButtons`, `TextBox` | [`RangeSlider`], [`Button`], [`CheckButtons`], [`RadioButtons`], [`TextBox`] |
//! | `Cursor(ax, useblit=True)`, `MultiCursor(axes)` | [`Cursor`], [`MultiCursor`] with [`CursorOptions::useblit`] |
//! | `SpanSelector`, `RectangleSelector`, `EllipseSelector` | [`SpanSelector`], [`RectangleSelector`], [`EllipseSelector`] |
//! | `span.new_axes(ax)`                             | [`SpanSelector::new_axes`]                     |
//! | `LassoSelector`, `PolygonSelector`, `Lasso`     | [`LassoSelector`], [`PolygonSelector`], [`Lasso`] |
//! | `plt.subplot_tool()`, `SubplotTool`             | [`crate::pyplot::subplot_tool`], [`SubplotTool`] |
//! | `toolbar.pan()`, `toolbar.zoom()`               | [`Navigation::toggle`]`(&fig, NavMode::Pan)`, `NavMode::Zoom` |
//! | `toolbar.home()`, `back()`, `forward()`, `push_current()` | [`Navigation::home`], [`Navigation::back`], [`Navigation::forward`], [`Navigation::push_current`] |
//! | `ax.format_coord(x, y)`, the status bar         | [`Axes::format_coord`], [`Navigation::readout`] |
//! | `ax.set_navigate(False)`                        | [`Axes::set_navigate`]                         |
//! | `toolmanager.add_tool(name, ToolBase)`          | [`Figure::add_tool`] with a [`ToolSpec`]       |
//! | `toolmanager.trigger_tool(name)`                | [`Figure::trigger_tool`]                       |
//! | `ToolToggleBase.radio_group`, `cursor`, `default_toggled` | [`ToolSpec::radio_group`], [`ToolSpec::cursor`], [`ToolSpec::default_toggled`]; pan and zoom are in [`NAVIGATION_GROUP`] |
//! | `FuncAnimation(fig, update, frames=n, interval=, blit=)` | [`FuncAnimation::new`] with [`Frames`] and [`AnimationOptions`] |
//! | `FuncAnimation(..., init_func=init)`            | [`FuncAnimation::with_init`]                   |
//! | `ArtistAnimation(fig, frames)`                  | [`FuncAnimation::from_artists`]                |
//! | `anim.pause()`, `anim.resume()`                 | [`FuncAnimation::pause`], [`FuncAnimation::resume`] |
//! | `anim.save("a.gif")`, `"a.png"`, `"a.mp4"`      | [`FuncAnimation::save`] (by extension)         |
//! | `anim.to_jshtml()`, `anim.to_html5_video()`     | [`FuncAnimation::to_jshtml`], [`FuncAnimation::to_html5_video`] |
//! | `PillowWriter`, `FFMpegWriter(fps=, codec=)`    | [`FuncAnimation::to_gif`], [`FuncAnimation::save_movie`] with [`MovieOptions`] |
//! | `bg = fig.canvas.copy_from_bbox(ax.bbox)`       | [`FigureCanvas::copy_from_bbox`]`(&ax.bbox())` |
//! | `fig.canvas.restore_region(bg)`                 | [`FigureCanvas::restore_region`]               |
//! | `ax.draw_artist(line)`, `fig.canvas.blit(ax.bbox)` | [`Axes::draw_artist`], [`FigureCanvas::blit`] |
//! | `fig.canvas.draw()`, `fig.canvas.draw_idle()`   | [`FigureCanvas::draw`], [`FigureCanvas::draw_idle`] |
//!
//! # Events
//!
//! ```python
//! clicks = []
//! def on_click(event):
//!     if event.inaxes is not None:
//!         clicks.append((event.xdata, event.ydata))
//! cid = fig.canvas.mpl_connect("button_press_event", on_click)
//! # ... the user clicks in the axes ...
//! fig.canvas.mpl_disconnect(cid)
//! ```
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
//! let clicks = Rc::new(RefCell::new(Vec::new()));
//! let seen = Rc::clone(&clicks);
//! let cid = fig.connect(EventKind::ButtonPress, move |event| {
//!     if let Event::ButtonPress(mouse) = event
//!         && mouse.location.inaxes.is_some()
//!     {
//!         seen.borrow_mut().push((mouse.location.xdata, mouse.location.ydata));
//!     }
//! });
//!
//! // A click at data (5, 5): where a window would report it, in pixels.
//! let at = ax.data_to_display((5.0, 5.0))?;
//! let mut input = EventSource::new();
//! input.button_press(&fig, at.x, at.y, PointerButton::Left, false);
//! assert_eq!(clicks.borrow().len(), 1);
//! // Like a window, the event source reports whole pixels.
//! let (x, y) = clicks.borrow()[0];
//! assert!((x.ok_or("in the axes")? - 5.0).abs() < 0.1);
//! assert!((y.ok_or("in the axes")? - 5.0).abs() < 0.1);
//!
//! assert!(fig.disconnect(cid));
//! input.button_press(&fig, at.x, at.y, PointerButton::Left, false);
//! assert_eq!(clicks.borrow().len(), 1);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Picking and timers
//!
//! ```python
//! line, = ax.plot([0, 1, 2], [0, 1, 0], picker=5)
//! fig.canvas.mpl_connect("pick_event", lambda event: print(event.ind))
//! timer = fig.canvas.new_timer(interval=100)
//! timer.add_callback(lambda: ticks.append(1))
//! timer.start()
//! ```
//!
//! ```
//! use std::cell::{Cell, RefCell};
//! use std::rc::Rc;
//! use std::time::Duration;
//!
//! use pltrs::artist::Picker;
//! use pltrs::events::{Event, EventKind, EventSource};
//! use pltrs::navigation::PointerButton;
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let line = ax.plot([0.0, 1.0, 2.0], [0.0, 1.0, 0.0])?;
//! line.set_picker(Some(Picker::Radius(5.0)));
//! let picked = Rc::new(RefCell::new(Vec::new()));
//! let seen = Rc::clone(&picked);
//! fig.connect(EventKind::Pick, move |event| {
//!     if let Event::Pick(pick) = event {
//!         seen.borrow_mut().extend(pick.ind.iter().copied());
//!     }
//! });
//! let at = ax.data_to_display((1.0, 1.0))?;
//! EventSource::new().button_press(&fig, at.x, at.y, PointerButton::Left, false);
//! assert!(picked.borrow().contains(&1), "the vertex at (1, 1)");
//!
//! let ticks = Rc::new(Cell::new(0));
//! let counter = Rc::clone(&ticks);
//! let timer = fig.add_timer(Duration::from_millis(100), move || {
//!     counter.set(counter.get() + 1);
//!     true // keep running
//! });
//! fig.fire_timer(timer); // a window fires it every 100 ms
//! assert_eq!(ticks.get(), 1);
//! assert!(fig.remove_timer(timer));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Widgets
//!
//! ```python
//! freq = Slider(fig.add_axes([0.2, 0.1, 0.6, 0.05]), "Freq", 0.1, 30, valinit=5)
//! freq.on_changed(lambda value: line.set_ydata(np.sin(value * x)))
//! freq.set_val(7.5)
//! reset = Button(fig.add_axes([0.8, 0.02, 0.1, 0.05]), "Reset")
//! cid = reset.on_clicked(lambda event: freq.reset())
//! reset.disconnect(cid)
//! ```
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::widgets::{Button, ButtonOptions, Slider, SliderOptions};
//!
//! let fig = Figure::new();
//! let ax = fig.add_axes([0.1, 0.3, 0.8, 0.6])?;
//! let x = linspace(0.0, 1.0, 50);
//! let line = ax.plot(&x, x.iter().map(|v| (5.0 * v).sin()))?;
//!
//! let options = SliderOptions { valinit: 5.0, ..SliderOptions::default() };
//! let freq = Slider::new(&fig.add_axes([0.2, 0.1, 0.6, 0.05])?, "Freq", 0.1, 30.0, options)?;
//! let wave = line.clone();
//! freq.on_changed(move |value| {
//!     wave.set_ydata(wave.xdata().iter().map(|v| (value * v).sin()));
//! });
//! freq.set_val(7.5);
//! assert_eq!(freq.val(), 7.5);
//! assert_eq!(line.ydata()[1], (7.5 * x[1]).sin());
//!
//! let reset = Button::new(&fig.add_axes([0.8, 0.02, 0.1, 0.05])?, "Reset", ButtonOptions::default());
//! let cid = reset.on_clicked(|| {});
//! assert_eq!(reset.label().text(), "Reset");
//! assert!(reset.disconnect(cid));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Navigation and toolbar tools
//!
//! ```python
//! toolbar.zoom()                        # then the user drags a rectangle
//! toolbar.home()
//! fig.canvas.manager.toolmanager.add_tool("grid", GridTool)
//! ```
//!
//! ```
//! use pltrs::gui::{Icon, ToolKind, ToolSpec};
//! use pltrs::navigation::{DragKey, NavMode, Navigation, PointerButton};
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
//! let (from, to) = (ax.data_to_display((0.25, 0.25))?, ax.data_to_display((0.75, 0.75))?);
//!
//! let mut nav = Navigation::new();
//! nav.toggle(&fig, NavMode::Zoom)?;
//! nav.press(&fig, from, PointerButton::Left);
//! nav.drag(to, DragKey::None);
//! nav.release(&fig, to, DragKey::None);
//! let (low, high) = ax.xlim();
//! assert!((low - 0.25).abs() < 1e-9 && (high - 0.75).abs() < 1e-9);
//! nav.home(&fig);
//! assert_eq!(ax.xlim(), (0.0, 1.0));
//!
//! let spec = ToolSpec {
//!     name: "grid".into(),
//!     description: "Show the grid".into(),
//!     icon: Icon::new(24.0),
//!     keys: vec!["ctrl+g".into()],
//!     kind: ToolKind::Toggle,
//!     ..ToolSpec::default()
//! };
//! fig.add_tool(spec, |event| {
//!     for ax in event.figure.axes() {
//!         ax.grid(event.toggled);
//!     }
//! })?;
//! fig.trigger_tool("grid");
//! assert_eq!(fig.tool_toggled("grid"), Some(true));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Animation
//!
//! ```python
//! line, = ax.plot([0, 1], [0, 0])
//! def update(i):
//!     line.set_data([0, 1], [0, i / 4])
//!     return line,
//! anim = FuncAnimation(fig, update, frames=5, interval=50, blit=True)
//! anim.save("rise.gif")
//! html = anim.to_jshtml()
//! ```
//!
//! ```
//! use std::time::Duration;
//!
//! use pltrs::animation::{AnimationOptions, Frames, FuncAnimation, HtmlOptions};
//! use pltrs::prelude::*;
//!
//! let fig = Figure::with_size(2.0, 2.0, 50.0);
//! let ax = fig.subplot();
//! let line = ax.plot([0.0, 1.0], [0.0, 0.0])?;
//! line.set_animated(true);
//! let options = AnimationOptions {
//!     interval: Duration::from_millis(50),
//!     blit: true,
//!     ..AnimationOptions::default()
//! };
//! let rising = line.clone();
//! let animation = FuncAnimation::new(&fig, Frames::Count(5), options, move |i| {
//!     rising.set_data([0.0, 1.0], [0.0, i as f64 / 4.0]);
//!     rising.clone()
//! });
//! assert!(animation.to_gif()?.starts_with(b"GIF89a")); // `animation.save("rise.gif")?`
//! assert!(animation.to_jshtml(HtmlOptions::default())?.contains("<script"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Blitting
//!
//! ```python
//! line, = ax.plot([0, 1], [0, 1], animated=True)
//! fig.canvas.draw()
//! background = fig.canvas.copy_from_bbox(ax.bbox)
//! for y in (0.2, 0.4):
//!     fig.canvas.restore_region(background)
//!     line.set_ydata([0, y])
//!     ax.draw_artist(line)
//!     fig.canvas.blit(ax.bbox)
//! ```
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let fig = Figure::with_size(3.0, 2.0, 50.0);
//! let ax = fig.subplot();
//! let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
//! line.set_animated(true);
//! let canvas = fig.canvas();
//! canvas.draw()?;
//! let background = canvas.copy_from_bbox(&ax.bbox())?;
//! for y in [0.2, 0.4] {
//!     canvas.restore_region(&background)?;
//!     line.set_ydata(vec![0.0, y]);
//!     ax.draw_artist(&line)?;
//!     canvas.blit(Some(&ax.bbox()))?;
//! }
//! assert!(!fig.is_stale(), "blitted, not redrawn");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Figure::connect`]: crate::figure::Figure::connect
//! [`Axes::data_to_display`]: crate::axes::Axes::data_to_display
//! [`Figure::disconnect`]: crate::figure::Figure::disconnect
//! [`Figure::add_timer`]: crate::figure::Figure::add_timer
//! [`Figure::remove_timer`]: crate::figure::Figure::remove_timer
//! [`Figure::set_cursor`]: crate::figure::Figure::set_cursor
//! [`Figure::lock_widgets`]: crate::figure::Figure::lock_widgets
//! [`Figure::release_widgets`]: crate::figure::Figure::release_widgets
//! [`Figure::add_tool`]: crate::figure::Figure::add_tool
//! [`Figure::trigger_tool`]: crate::figure::Figure::trigger_tool
//! [`Figure::canvas`]: crate::figure::Figure::canvas
//! [`FigureCanvas`]: crate::figure::FigureCanvas
//! [`FigureCanvas::copy_from_bbox`]: crate::figure::FigureCanvas::copy_from_bbox
//! [`FigureCanvas::restore_region`]: crate::figure::FigureCanvas::restore_region
//! [`FigureCanvas::blit`]: crate::figure::FigureCanvas::blit
//! [`FigureCanvas::draw`]: crate::figure::FigureCanvas::draw
//! [`FigureCanvas::draw_idle`]: crate::figure::FigureCanvas::draw_idle
//! [`Axes::draw_artist`]: crate::axes::Axes::draw_artist
//! [`Axes::format_coord`]: crate::axes::Axes::format_coord
//! [`Axes::set_navigate`]: crate::axes::Axes::set_navigate
//! [`ArtistHandle::set_picker`]: crate::artist::ArtistHandle::set_picker
//! [`EventKind`]: crate::events::EventKind
//! [`EventKind::Motion`]: crate::events::EventKind::Motion
//! [`EventKind::Scroll`]: crate::events::EventKind::Scroll
//! [`EventKind::KeyPress`]: crate::events::EventKind::KeyPress
//! [`EventKind::Pick`]: crate::events::EventKind::Pick
//! [`EventKind::Draw`]: crate::events::EventKind::Draw
//! [`EventKind::Resize`]: crate::events::EventKind::Resize
//! [`EventKind::Close`]: crate::events::EventKind::Close
//! [`Event`]: crate::events::Event
//! [`MouseEvent`]: crate::events::MouseEvent
//! [`MouseEvent::button`]: crate::events::MouseEvent::button
//! [`MouseEvent::key`]: crate::events::MouseEvent::key
//! [`MouseEvent::dblclick`]: crate::events::MouseEvent::dblclick
//! [`KeyEvent`]: crate::events::KeyEvent
//! [`PickEvent`]: crate::events::PickEvent
//! [`PickEvent::artist`]: crate::events::PickEvent::artist
//! [`PickEvent::ind`]: crate::events::PickEvent::ind
//! [`Location::x`]: crate::events::Location::x
//! [`Location::xdata`]: crate::events::Location::xdata
//! [`Location::inaxes`]: crate::events::Location::inaxes
//! [`EventSource`]: crate::events::EventSource
//! [`EventSource::button_press`]: crate::events::EventSource::button_press
//! [`Slider::new`]: crate::widgets::Slider::new
//! [`Button::on_clicked`]: crate::widgets::Button::on_clicked
//! [`Button::disconnect`]: crate::widgets::Button::disconnect
//! [`CallbackId`]: crate::widgets::CallbackId
//! [`Slider::on_changed`]: crate::widgets::Slider::on_changed
//! [`Slider::set_val`]: crate::widgets::Slider::set_val
//! [`SliderOptions`]: crate::widgets::SliderOptions
//! [`RangeSlider`]: crate::widgets::RangeSlider
//! [`Button`]: crate::widgets::Button
//! [`CheckButtons`]: crate::widgets::CheckButtons
//! [`RadioButtons`]: crate::widgets::RadioButtons
//! [`TextBox`]: crate::widgets::TextBox
//! [`Cursor`]: crate::widgets::Cursor
//! [`MultiCursor`]: crate::widgets::MultiCursor
//! [`CursorOptions::useblit`]: crate::widgets::CursorOptions::useblit
//! [`SpanSelector`]: crate::widgets::SpanSelector
//! [`RectangleSelector`]: crate::widgets::RectangleSelector
//! [`EllipseSelector`]: crate::widgets::EllipseSelector
//! [`LassoSelector`]: crate::widgets::LassoSelector
//! [`PolygonSelector`]: crate::widgets::PolygonSelector
//! [`Lasso`]: crate::widgets::Lasso
//! [`SubplotTool`]: crate::widgets::SubplotTool
//! [`Navigation`]: crate::navigation::Navigation
//! [`Navigation::toggle`]: crate::navigation::Navigation::toggle
//! [`Navigation::home`]: crate::navigation::Navigation::home
//! [`Navigation::back`]: crate::navigation::Navigation::back
//! [`Navigation::forward`]: crate::navigation::Navigation::forward
//! [`Navigation::push_current`]: crate::navigation::Navigation::push_current
//! [`Navigation::readout`]: crate::navigation::Navigation::readout
//! [`ToolSpec`]: crate::gui::ToolSpec
//! [`ToolSpec::radio_group`]: crate::gui::ToolSpec::radio_group
//! [`ToolSpec::cursor`]: crate::gui::ToolSpec::cursor
//! [`ToolSpec::default_toggled`]: crate::gui::ToolSpec::default_toggled
//! [`NAVIGATION_GROUP`]: crate::gui::NAVIGATION_GROUP
//! [`SpanSelector::new_axes`]: crate::widgets::SpanSelector::new_axes
//! [`FuncAnimation::pause`]: crate::animation::FuncAnimation::pause
//! [`FuncAnimation::resume`]: crate::animation::FuncAnimation::resume
//! [`Figure::with_callbacks_blocked`]: crate::figure::Figure::with_callbacks_blocked
//! [`Figure::block_callbacks`]: crate::figure::Figure::block_callbacks
//! [`PointerButton::Back`]: crate::navigation::PointerButton::Back
//! [`PointerButton::Forward`]: crate::navigation::PointerButton::Forward
//! [`PointerButton::number`]: crate::navigation::PointerButton::number
//! [`FuncAnimation`]: crate::animation::FuncAnimation
//! [`FuncAnimation::new`]: crate::animation::FuncAnimation::new
//! [`FuncAnimation::with_init`]: crate::animation::FuncAnimation::with_init
//! [`FuncAnimation::from_artists`]: crate::animation::FuncAnimation::from_artists
//! [`FuncAnimation::save`]: crate::animation::FuncAnimation::save
//! [`FuncAnimation::to_jshtml`]: crate::animation::FuncAnimation::to_jshtml
//! [`FuncAnimation::to_html5_video`]: crate::animation::FuncAnimation::to_html5_video
//! [`FuncAnimation::to_gif`]: crate::animation::FuncAnimation::to_gif
//! [`FuncAnimation::save_movie`]: crate::animation::FuncAnimation::save_movie
//! [`Frames`]: crate::animation::Frames
//! [`AnimationOptions`]: crate::animation::AnimationOptions
//! [`MovieOptions`]: crate::animation::MovieOptions
//! [`Axes::on_limits_changed`]: crate::axes::Axes::on_limits_changed
//! [`Axes::disconnect_limits_changed`]: crate::axes::Axes::disconnect_limits_changed
