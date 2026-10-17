//! Interactive widgets drawn in axes (matplotlib's `widgets`): buttons,
//! sliders, check and radio buttons and text boxes.
//!
//! Each widget takes over an axes: it draws itself with ordinary artists
//! and reacts to the figure's [events](crate::events) when the figure is
//! shown in a [window](crate::gui). Callbacks registered with `on_*`
//! methods run when the user acts. A widget stays connected for as long as
//! it is alive; keep it around while the figure is shown.
//!
//! ```
//! use plt::figure::Figure;
//! use plt::widgets::{Slider, SliderOptions};
//! let fig = Figure::new();
//! let ax = fig.add_axes([0.2, 0.1, 0.6, 0.05]);
//! let options = SliderOptions { valinit: 5.0, ..SliderOptions::default() };
//! let freq = Slider::new(&ax, "Freq", 0.1, 30.0, &options);
//! freq.on_changed(|value| println!("now {value}"));
//! freq.set_val(7.5);
//! assert_eq!(freq.val(), 7.5);
//! ```

mod button;
mod range_slider;
mod slider;
mod textbox;
mod toggles;

use std::cell::RefCell;
use std::rc::Rc;

pub use button::{Button, ButtonOptions};
pub use range_slider::RangeSlider;
pub use slider::{Orientation, Slider, SliderOptions, ValStep};
pub use textbox::{TextBox, TextBoxOptions};
pub use toggles::{CheckButtons, RadioButtons};

use crate::events::{ConnectionId, Event, EventKind};
use crate::figure::Figure;

/// The figure callbacks of a widget, disconnected when it is dropped.
#[derive(Debug)]
struct Connections {
    figure: Figure,
    ids: Vec<ConnectionId>,
}

impl Connections {
    fn new(figure: Figure) -> Self {
        Self {
            figure,
            ids: Vec::new(),
        }
    }

    /// Connects `callback` for events of `kind`.
    fn connect(&mut self, kind: EventKind, callback: impl FnMut(&Event) + 'static) {
        self.ids.push(self.figure.connect(kind, callback));
    }
}

impl Drop for Connections {
    fn drop(&mut self) {
        for &id in &self.ids {
            self.figure.disconnect(id);
        }
    }
}

/// The callbacks of one widget signal.
type CallbackList<T> = Vec<Box<dyn FnMut(&T)>>;

/// Callbacks taking a value of type `T` (matplotlib's widget
/// `_observers`), shared between a widget and its event handlers.
struct Observers<T> {
    callbacks: Rc<RefCell<CallbackList<T>>>,
}

impl<T> Clone for Observers<T> {
    fn clone(&self) -> Self {
        Self {
            callbacks: Rc::clone(&self.callbacks),
        }
    }
}

impl<T> Default for Observers<T> {
    fn default() -> Self {
        Self {
            callbacks: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl<T> std::fmt::Debug for Observers<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observers")
            .field("connected", &self.callbacks.borrow().len())
            .finish()
    }
}

impl<T> Observers<T> {
    fn connect(&self, callback: impl FnMut(&T) + 'static) {
        self.callbacks.borrow_mut().push(Box::new(callback));
    }

    /// Calls every callback with `value`; callbacks connected while this
    /// runs are kept for the next call.
    fn process(&self, value: &T) {
        let mut running = std::mem::take(&mut *self.callbacks.borrow_mut());
        for callback in &mut running {
            callback(value);
        }
        let mut callbacks = self.callbacks.borrow_mut();
        running.append(&mut callbacks);
        *callbacks = running;
    }
}

/// The pointer position of a mouse event and whether `left` is the
/// button involved.
fn left_click(event: &Event) -> Option<plt_core::Point> {
    match event {
        Event::ButtonPress(mouse) | Event::ButtonRelease(mouse)
            if mouse.button == Some(crate::navigation::PointerButton::Left) =>
        {
            Some(plt_core::Point::new(mouse.location.x, mouse.location.y))
        }
        _ => None,
    }
}
