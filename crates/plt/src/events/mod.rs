//! Figure events and callbacks (matplotlib's `FigureCanvasBase.mpl_connect`
//! and the `Event` classes of `backend_bases`).
//!
//! Callbacks are connected to a [`Figure`](crate::figure::Figure) for one
//! [`EventKind`] and
//! receive every [`Event`] of that kind:
//!
//! ```
//! use std::cell::Cell;
//! use std::rc::Rc;
//! use plt::events::{Event, EventKind, EventSource};
//! use plt::figure::Figure;
//! use plt::navigation::PointerButton;
//!
//! let fig = Figure::new();
//! fig.subplot();
//! let clicks = Rc::new(Cell::new(0));
//! let counter = Rc::clone(&clicks);
//! fig.connect(EventKind::ButtonPress, move |event| {
//!     if let Event::ButtonPress(mouse) = event {
//!         assert!(mouse.location.inaxes.is_some());
//!         counter.set(counter.get() + 1);
//!     }
//! });
//! // A window system feeds input through an event source.
//! let mut source = EventSource::new();
//! source.button_press(&fig, 320.0, 240.0, PointerButton::Left, false);
//! assert_eq!(clicks.get(), 1);
//! ```
//!
//! [`Figure::add_timer`](crate::figure::Figure::add_timer) calls back
//! periodically while the figure is shown
//! (the basis of [`crate::animation`]). [`GinputOptions`] configures
//! pyplot's `ginput`, which waits for clicks in a shown figure.
//!
//! Interactive tools share the pointer through the figure's widget lock
//! ([`Figure::lock_widgets`](crate::figure::Figure::lock_widgets)) and ask
//! the window for cursor shapes
//! ([`Figure::set_cursor`](crate::figure::Figure::set_cursor)).
//!
//! [`EventSource`] turns raw input into events the way matplotlib's
//! canvas does: it finds the axes under the pointer and the data
//! coordinates there, remembers the held button and key, and emits axes
//! enter/leave events as the pointer moves.

mod drag;
mod lock;
mod pick;
mod registry;
mod source;
mod timers;
pub(crate) mod wait;

use crate::axes::Axes;
use crate::navigation::PointerButton;
use crate::transforms::TransformError;

pub(crate) use drag::{DragTarget, Dragger};
pub use lock::{CursorShape, LockError, LockOwner};
pub use pick::PickEvent;
pub(crate) use registry::CallbackRegistry;
pub use registry::ConnectionId;
pub use source::EventSource;
pub use timers::TimerKey;
pub(crate) use timers::TimerRegistry;
pub use wait::GinputOptions;

/// What an event is about (matplotlib's event names).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// A mouse button went down (`button_press_event`).
    ButtonPress,
    /// A mouse button went up (`button_release_event`).
    ButtonRelease,
    /// The pointer moved (`motion_notify_event`).
    Motion,
    /// The wheel turned (`scroll_event`).
    Scroll,
    /// A key went down (`key_press_event`).
    KeyPress,
    /// A key went up (`key_release_event`).
    KeyRelease,
    /// The pointer moved onto an axes (`axes_enter_event`).
    AxesEnter,
    /// The pointer moved off an axes (`axes_leave_event`).
    AxesLeave,
    /// The pointer entered the figure (`figure_enter_event`).
    FigureEnter,
    /// The pointer left the figure (`figure_leave_event`).
    FigureLeave,
    /// The figure was resized (`resize_event`).
    Resize,
    /// The figure's window is closing (`close_event`).
    Close,
    /// A press or scroll picked an artist (`pick_event`).
    Pick,
    /// The figure was drawn (`draw_event`).
    Draw,
}

/// Where the pointer is (matplotlib's `LocationEvent` attributes); an
/// [`EventSource`] reports whole pixels, as matplotlib does.
#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    /// Pixels from the left of the figure.
    pub x: f64,
    /// Pixels from the bottom of the figure.
    pub y: f64,
    /// The axes under the pointer, if any.
    pub inaxes: Option<Axes>,
    /// The data x coordinate in [`Location::inaxes`].
    pub xdata: Option<f64>,
    /// The data y coordinate in [`Location::inaxes`].
    pub ydata: Option<f64>,
}

impl Location {
    /// The location of data point `(x, y)` of `ax` as matplotlib's
    /// synthetic events place it (`LocationEvent._from_ax_coords`, which
    /// widget tests use): over `ax`, at exactly those data coordinates, and
    /// at the whole display pixels the point maps to (matplotlib's events
    /// truncate them).
    ///
    /// ```
    /// use plt::events::Location;
    /// use plt::figure::Figure;
    /// let fig = Figure::with_size(4.0, 4.0, 100.0);
    /// let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    /// ax.set_xlim(0.0, 3.0)?.set_ylim(0.0, 3.0)?;
    /// let at = Location::from_ax_coords(&ax, (1.0, 2.0))?;
    /// assert_eq!((at.x, at.y), (133.0, 266.0));
    /// assert_eq!((at.xdata, at.ydata), (Some(1.0), Some(2.0)));
    /// assert_eq!(at.inaxes, Some(ax));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the data transform of `ax` cannot be resolved.
    pub fn from_ax_coords(ax: &Axes, (x, y): (f64, f64)) -> Result<Self, TransformError> {
        let display = ax.data_to_display((x, y))?;
        Ok(Self {
            x: display.x.trunc(),
            y: display.y.trunc(),
            inaxes: Some(ax.clone()),
            xdata: Some(x),
            ydata: Some(y),
        })
    }
}

/// A mouse event (matplotlib's `MouseEvent`).
#[derive(Clone, Debug, PartialEq)]
pub struct MouseEvent {
    /// Where the pointer is.
    pub location: Location,
    /// The button pressed, released or held during motion.
    pub button: Option<PointerButton>,
    /// The key held, in matplotlib's naming (e.g. `"shift"`, `"x"`).
    pub key: Option<String>,
    /// Wheel steps for scroll events: positive up, negative down.
    pub step: f64,
    /// Whether a press is the second click of a double click.
    pub dblclick: bool,
}

/// A key event (matplotlib's `KeyEvent`).
#[derive(Clone, Debug, PartialEq)]
pub struct KeyEvent {
    /// Where the pointer is.
    pub location: Location,
    /// The key in matplotlib's naming (e.g. `"a"`, `"ctrl+s"`, `"left"`).
    pub key: String,
}

/// An event delivered to figure callbacks.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// A mouse button went down.
    ButtonPress(MouseEvent),
    /// A mouse button went up.
    ButtonRelease(MouseEvent),
    /// The pointer moved.
    Motion(MouseEvent),
    /// The wheel turned.
    Scroll(MouseEvent),
    /// A key went down.
    KeyPress(KeyEvent),
    /// A key went up.
    KeyRelease(KeyEvent),
    /// The pointer moved onto an axes.
    AxesEnter(Location),
    /// The pointer moved off an axes (the location names that axes).
    AxesLeave(Location),
    /// The pointer entered the figure.
    FigureEnter(Location),
    /// The pointer left the figure.
    FigureLeave,
    /// The figure was resized to `width × height` pixels.
    Resize {
        /// The new width in pixels.
        width: f64,
        /// The new height in pixels.
        height: f64,
    },
    /// The figure's window is closing.
    Close,
    /// A press or scroll picked an artist that has a picker.
    Pick(PickEvent),
    /// [`Figure::draw`](crate::figure::Figure::draw) finished drawing the
    /// figure: on screen, into a file or for the layout alone.
    Draw,
}

impl Event {
    /// The kind of event.
    pub fn kind(&self) -> EventKind {
        match self {
            Self::ButtonPress(_) => EventKind::ButtonPress,
            Self::ButtonRelease(_) => EventKind::ButtonRelease,
            Self::Motion(_) => EventKind::Motion,
            Self::Scroll(_) => EventKind::Scroll,
            Self::KeyPress(_) => EventKind::KeyPress,
            Self::KeyRelease(_) => EventKind::KeyRelease,
            Self::AxesEnter(_) => EventKind::AxesEnter,
            Self::AxesLeave(_) => EventKind::AxesLeave,
            Self::FigureEnter(_) => EventKind::FigureEnter,
            Self::FigureLeave => EventKind::FigureLeave,
            Self::Resize { .. } => EventKind::Resize,
            Self::Close => EventKind::Close,
            Self::Pick(_) => EventKind::Pick,
            Self::Draw => EventKind::Draw,
        }
    }

    /// Where the pointer is, for events that have a location.
    pub fn location(&self) -> Option<&Location> {
        match self {
            Self::ButtonPress(mouse)
            | Self::ButtonRelease(mouse)
            | Self::Motion(mouse)
            | Self::Scroll(mouse)
            | Self::Pick(PickEvent { mouse, .. }) => Some(&mouse.location),
            Self::KeyPress(key) | Self::KeyRelease(key) => Some(&key.location),
            Self::AxesEnter(location) | Self::AxesLeave(location) | Self::FigureEnter(location) => {
                Some(location)
            }
            Self::FigureLeave | Self::Resize { .. } | Self::Close | Self::Draw => None,
        }
    }
}
