//! What every widget shares (matplotlib's `Widget` and `AxesWidget`):
//! whether it reacts to the user, and the figure callbacks a widget
//! connects, which skip events while it is turned off and go when it is
//! dropped.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::{
    Button, CheckButtons, Cursor, EllipseSelector, Lasso, LassoSelector, MultiCursor,
    PolygonSelector, RadioButtons, RangeSlider, RectangleSelector, Slider, SpanSelector, TextBox,
};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::events::{ConnectionId, Event, EventKind};
use crate::figure::Figure;

/// A widget that can be turned off (matplotlib's `Widget`): while it is
/// inactive it ignores the user's input, and it reacts again once turned
/// back on. Every widget of [`widgets`](super) is one, and so is an
/// [`AxesWidget`], the base of widgets of the program's own.
///
/// Check and radio buttons have an inherent `set_active(index)` of their
/// own, which turns one button on as matplotlib's does; call this trait's
/// through it (`Widget::set_active(&buttons, false)`).
///
/// ```
/// use pltrs::figure::Figure;
/// use pltrs::widgets::{Button, ButtonOptions, Widget};
/// let fig = Figure::new();
/// let ok = Button::new(&fig.add_axes([0.7, 0.05, 0.1, 0.075])?, "OK", ButtonOptions::default());
/// let cancel = Button::new(&fig.add_axes([0.81, 0.05, 0.1, 0.075])?, "Cancel", ButtonOptions::default());
/// let buttons: [&dyn Widget; 2] = [&ok, &cancel];
/// for button in buttons {
///     button.set_active(false);
/// }
/// assert!(!ok.active() && cancel.ignores_events());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub trait Widget {
    /// Whether the widget reacts to the user (matplotlib's `active` and
    /// `get_active`).
    fn active(&self) -> bool;

    /// Turns the widget on or off (matplotlib's `set_active`).
    fn set_active(&self, active: bool);

    /// Whether the widget ignores events now, because it is turned off
    /// (matplotlib's `Widget.ignore`).
    fn ignores_events(&self) -> bool {
        !self.active()
    }
}

/// The figure callbacks of a widget, skipped while it is turned off and
/// disconnected when it is dropped.
#[derive(Debug)]
pub(super) struct Connections {
    figure: Figure,
    ids: Vec<ConnectionId>,
    active: Rc<Cell<bool>>,
}

impl Connections {
    /// No callbacks yet, on `figure`, turned on.
    pub(super) fn new(figure: Figure) -> Self {
        Self {
            figure,
            ids: Vec::new(),
            active: Rc::new(Cell::new(true)),
        }
    }

    /// Connects `callback` for events of `kind`, called while the
    /// connections are turned on.
    pub(super) fn connect(
        &mut self,
        kind: EventKind,
        mut callback: impl FnMut(&Event) + 'static,
    ) -> ConnectionId {
        let active = Rc::clone(&self.active);
        let id = self.figure.connect(kind, move |event| {
            if active.get() {
                callback(event);
            }
        });
        self.ids.push(id);
        id
    }

    /// The figure whose events the callbacks take.
    pub(super) fn figure(&self) -> &Figure {
        &self.figure
    }

    /// Whether the callbacks are called.
    pub(super) fn active(&self) -> bool {
        self.active.get()
    }

    /// Turns the callbacks on or off.
    pub(super) fn set_active(&self, active: bool) {
        self.active.set(active);
    }

    /// Disconnects every callback.
    fn disconnect_all(&mut self) {
        for id in self.ids.drain(..) {
            self.figure.disconnect(id);
        }
    }
}

impl Drop for Connections {
    fn drop(&mut self) {
        self.disconnect_all();
    }
}

/// The base of a widget of the program's own (matplotlib's
/// `AxesWidget`): the axes it is drawn in and the figure callbacks it
/// connects for it with [`connect_event`](Self::connect_event). While the
/// widget is turned off ([`Widget::set_active`]) the callbacks are not
/// called; when it is dropped they are disconnected.
///
/// ```
/// use std::cell::Cell;
/// use std::rc::Rc;
/// use pltrs::events::{EventKind, EventSource};
/// use pltrs::figure::Figure;
/// use pltrs::navigation::PointerButton;
/// use pltrs::widgets::{AxesWidget, Widget};
///
/// /// Counts the clicks on its axes.
/// struct ClickCounter {
///     base: AxesWidget,
///     clicks: Rc<Cell<u32>>,
/// }
///
/// impl ClickCounter {
///     fn new(fig: &Figure) -> Result<Self, Box<dyn std::error::Error>> {
///         let base = AxesWidget::new(&fig.add_axes([0.1, 0.1, 0.8, 0.8])?);
///         let clicks = Rc::new(Cell::new(0));
///         let (ax, counter) = (base.ax().clone(), Rc::clone(&clicks));
///         base.connect_event(EventKind::ButtonPress, move |event| {
///             if event.location().and_then(|at| at.inaxes.as_ref()) == Some(&ax) {
///                 counter.set(counter.get() + 1);
///             }
///         });
///         Ok(Self { base, clicks })
///     }
/// }
///
/// let fig = Figure::with_size(4.0, 3.0, 100.0);
/// let counter = ClickCounter::new(&fig)?;
/// let mut mouse = EventSource::new();
/// mouse.button_press(&fig, 200.0, 150.0, PointerButton::Left, false);
/// counter.base.set_active(false);
/// mouse.button_press(&fig, 200.0, 150.0, PointerButton::Left, false);
/// assert_eq!(counter.clicks.get(), 1, "the second click came while it was off");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct AxesWidget {
    ax: Axes,
    connections: RefCell<Connections>,
}

impl AxesWidget {
    /// A widget drawn in `ax`, turned on, with no callbacks yet.
    pub fn new(ax: &Axes) -> Self {
        Self {
            ax: ax.clone(),
            connections: RefCell::new(Connections::new(ax.figure())),
        }
    }

    /// The axes the widget is drawn in (matplotlib's `ax`).
    pub fn ax(&self) -> &Axes {
        &self.ax
    }

    /// The figure of the axes, whose events the widget reacts to
    /// (matplotlib's `canvas`: pltrs's canvas is part of the figure).
    pub fn figure(&self) -> Figure {
        self.ax.figure()
    }

    /// Connects `callback` to the figure's events of `kind` (matplotlib's
    /// `connect_event`): it is called for each of them while the widget is
    /// turned on. The id disconnects it alone through
    /// [`Figure::disconnect`].
    pub fn connect_event(
        &self,
        kind: EventKind,
        callback: impl FnMut(&Event) + 'static,
    ) -> ConnectionId {
        self.connections.borrow_mut().connect(kind, callback)
    }

    /// Disconnects every callback the widget connected (matplotlib's
    /// `disconnect_events`).
    pub fn disconnect_events(&self) {
        self.connections.borrow_mut().disconnect_all();
    }
}

impl Widget for AxesWidget {
    fn active(&self) -> bool {
        self.connections.borrow().active()
    }

    fn set_active(&self, active: bool) {
        self.connections.borrow().set_active(active);
    }
}

/// [`Widget`] for widgets whose callbacks are their [`Connections`].
macro_rules! widget_by_connections {
    ($($widget:ty),* $(,)?) => {
        $(
            impl Widget for $widget {
                fn active(&self) -> bool {
                    self.connections.active()
                }

                fn set_active(&self, active: bool) {
                    self.connections.set_active(active);
                }
            }
        )*
    };
}

widget_by_connections!(
    Button,
    CheckButtons,
    RadioButtons,
    RangeSlider,
    Slider,
    TextBox
);

/// [`Widget`] for widgets with `active` and `set_active` of their own.
macro_rules! widget_by_methods {
    ($($widget:ty),* $(,)?) => {
        $(
            impl Widget for $widget {
                fn active(&self) -> bool {
                    <$widget>::active(self)
                }

                fn set_active(&self, active: bool) {
                    <$widget>::set_active(self, active);
                }
            }
        )*
    };
}

widget_by_methods!(
    Cursor,
    EllipseSelector,
    Lasso,
    LassoSelector,
    MultiCursor,
    PolygonSelector,
    RectangleSelector,
    SpanSelector,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventSource;
    use crate::navigation::PointerButton;
    use crate::widgets::{ButtonOptions, SliderOptions, TextBoxOptions};

    fn click(fig: &Figure, at: (f64, f64)) {
        let mut mouse = EventSource::new();
        mouse.button_press(fig, at.0, at.1, PointerButton::Left, false);
        mouse.button_release(fig, at.0, at.1, PointerButton::Left);
    }

    #[test]
    fn inactive_buttons_ignore_clicks() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.add_axes([0.5, 0.5, 0.4, 0.2]).expect("a rectangle");
        let button = Button::new(&ax, "Go", ButtonOptions::default());
        let clicks = Rc::new(Cell::new(0));
        let counter = Rc::clone(&clicks);
        button.on_clicked(move || counter.set(counter.get() + 1));
        assert!(button.active() && !button.ignores_events());
        Widget::set_active(&button, false);
        click(&fig, (280.0, 180.0));
        assert_eq!(clicks.get(), 0);
        Widget::set_active(&button, true);
        click(&fig, (280.0, 180.0));
        assert_eq!(clicks.get(), 1);
    }

    #[test]
    fn inactive_sliders_and_text_boxes_keep_their_values() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.add_axes([0.2, 0.1, 0.6, 0.1]).expect("a rectangle");
        let slider = Slider::new(&ax, "x", 0.0, 10.0, SliderOptions::default()).expect("a slider");
        Widget::set_active(&slider, false);
        click(&fig, (320.0, 45.0));
        assert_eq!(slider.val(), SliderOptions::default().valinit);
        Widget::set_active(&slider, true);
        click(&fig, (320.0, 45.0));
        assert!(slider.val() > 9.0, "{}", slider.val());

        let box_ax = fig.add_axes([0.2, 0.7, 0.6, 0.1]).expect("a rectangle");
        let text = TextBox::new(&box_ax, "name", "", TextBoxOptions::default());
        assert!(text.active());
        text.set_active(false);
        assert!(text.ignores_events());
    }

    #[test]
    fn own_widgets_connect_switch_and_disconnect() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let base = AxesWidget::new(&fig.subplot());
        assert_eq!(base.figure().id(), fig.id());
        let presses = Rc::new(Cell::new(0));
        let counter = Rc::clone(&presses);
        base.connect_event(EventKind::ButtonPress, move |_| {
            counter.set(counter.get() + 1);
        });
        click(&fig, (200.0, 150.0));
        base.set_active(false);
        click(&fig, (200.0, 150.0));
        base.set_active(true);
        click(&fig, (200.0, 150.0));
        assert_eq!(presses.get(), 2);
        base.disconnect_events();
        click(&fig, (200.0, 150.0));
        assert_eq!(presses.get(), 2, "disconnected");
        let counter = Rc::clone(&presses);
        base.connect_event(EventKind::ButtonPress, move |_| {
            counter.set(counter.get() + 1);
        });
        drop(base);
        click(&fig, (200.0, 150.0));
        assert_eq!(presses.get(), 2, "dropped");
    }

    #[test]
    fn selectors_and_cursors_switch_through_the_trait() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let selector = SpanSelector::new(
            &ax,
            crate::orientation::Orientation::Horizontal,
            crate::widgets::SpanSelectorOptions::default(),
        );
        let widget: &dyn Widget = &selector;
        widget.set_active(false);
        assert!(!selector.active() && widget.ignores_events());
        let cursor = Cursor::new(&ax, crate::widgets::CursorOptions::default());
        Widget::set_active(&cursor, false);
        assert!(!cursor.active());
    }
}
