//! Interactive widgets drawn in axes (matplotlib's `widgets`): buttons,
//! sliders, check and radio buttons, text boxes, the subplot tool,
//! crosshair cursors and selectors of ranges, regions and shapes.
//!
//! Each widget takes over an axes: it draws itself with ordinary artists
//! and reacts to the figure's [events](crate::events) when the figure is
//! shown in a [window](crate::gui). Callbacks registered with `on_*`
//! methods run when the user acts; each returns a [`CallbackId`] that the
//! widget's `disconnect` takes to remove the callback again (matplotlib's
//! `cid = widget.on_changed(...)` and `widget.disconnect(cid)`). A widget
//! stays connected for as long as it is alive; keep it around while the
//! figure is shown.
//!
//! ```
//! use plt::figure::Figure;
//! use plt::widgets::{Slider, SliderOptions};
//! let fig = Figure::new();
//! let ax = fig.add_axes([0.2, 0.1, 0.6, 0.05]);
//! let options = SliderOptions { valinit: 5.0, ..SliderOptions::default() };
//! let freq = Slider::new(&ax, "Freq", 0.1, 30.0, options);
//! let cid = freq.on_changed(|value| println!("now {value}"));
//! freq.set_val(7.5);
//! assert_eq!(freq.val(), 7.5);
//! assert!(freq.disconnect(cid));
//! ```

mod button;
mod cursor;
mod observers;
mod range_slider;
mod selectors;
mod slider;
mod subplot_tool;
mod textbox;
mod toggles;

pub use button::{Button, ButtonOptions};
pub use cursor::{Cursor, CursorOptions, MultiCursor};
pub use observers::CallbackId;
pub use range_slider::RangeSlider;
pub use selectors::{
    EllipseSelector, Lasso, LassoSelector, ModifierKeys, PolygonSelector, PolygonSelectorOptions,
    RectangleSelector, RectangleSelectorOptions, SelectorError, SelectorEvent, SelectorState,
    SelectorUnits, SpanSelector, SpanSelectorOptions, UnknownState,
};
pub use slider::{HandleStyle, Orientation, Slider, SliderOptions, ValStep};
pub use subplot_tool::SubplotTool;
pub use textbox::{TextBox, TextBoxOptions};
pub use toggles::{
    ButtonLayout, ButtonMarkerProps, ButtonsError, CheckButtons, CheckButtonsOptions, RadioButtons,
    RadioButtonsOptions,
};

use observers::Observers;

use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::events::{ConnectionId, Event, EventKind, Location};
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

/// The data coordinates of `location` in `ax`: those of the event when it
/// happened over `ax`, else the pointer mapped through the data transform
/// of `ax` (matplotlib's `_call_with_reparented_event`).
fn data_in(ax: &Axes, location: &Location) -> Option<plt_core::Point> {
    match (&location.inaxes, location.xdata, location.ydata) {
        (Some(inaxes), Some(x), Some(y)) if inaxes == ax => Some(plt_core::Point::new(x, y)),
        _ => {
            let trans = ax.figure().resolve_transform(&ax.trans_data()).ok()?;
            let p = trans
                .inverted()?
                .transform_point(plt_core::Point::new(location.x, location.y));
            (p.x.is_finite() && p.y.is_finite()).then_some(p)
        }
    }
}

/// `(a, b)` in increasing order (matplotlib's `sorted` of an interval).
fn ordered((a, b): (f64, f64)) -> (f64, f64) {
    if a <= b { (a, b) } else { (b, a) }
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
