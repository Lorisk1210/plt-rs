//! Figure events and callbacks (matplotlib's `FigureCanvasBase.mpl_connect`
//! and the `Event` classes of `backend_bases`).
//!
//! Callbacks are connected to a [`Figure`] for one [`EventKind`] and
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
//! [`Figure::add_timer`] calls back periodically while the figure is shown
//! (the basis of [`crate::animation`]).
//!
//! [`EventSource`] turns raw input into events the way matplotlib's
//! canvas does: it finds the axes under the pointer and the data
//! coordinates there, remembers the held button and key, and emits axes
//! enter/leave events as the pointer moves.

mod registry;
mod source;
mod timers;

use crate::axes::Axes;
use crate::navigation::PointerButton;

pub(crate) use registry::CallbackRegistry;
pub use registry::ConnectionId;
pub use source::EventSource;
pub use timers::TimerKey;
pub(crate) use timers::TimerRegistry;

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
}

/// Where the pointer is (matplotlib's `LocationEvent` attributes).
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
        }
    }

    /// Where the pointer is, for events that have a location.
    pub fn location(&self) -> Option<&Location> {
        match self {
            Self::ButtonPress(mouse)
            | Self::ButtonRelease(mouse)
            | Self::Motion(mouse)
            | Self::Scroll(mouse) => Some(&mouse.location),
            Self::KeyPress(key) | Self::KeyRelease(key) => Some(&key.location),
            Self::AxesEnter(location) | Self::AxesLeave(location) | Self::FigureEnter(location) => {
                Some(location)
            }
            Self::FigureLeave | Self::Resize { .. } | Self::Close => None,
        }
    }
}
