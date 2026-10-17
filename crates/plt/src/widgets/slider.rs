//! Sliders (matplotlib's `widgets.SliderBase` and `widgets.Slider`).

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use plt_core::Point;
use plt_text::{HAlign, VAlign};

use super::{Connections, Observers};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::colors::Color;
use crate::events::{Event, EventKind, MouseEvent};
use crate::lines::{Line2D, LineProps, LineStyle};
use crate::markers::{Marker, MarkerSymbol};
use crate::navigation::PointerButton;
use crate::patches::{Patch, Rectangle};
use crate::rc::ColorOrAuto;
use crate::text::Text;
use crate::ticker::pyfmt::PrintfFormat;
use crate::ticker::{
    AxisInfo, Formatter, OffsetMode, ScalarFormatter, TickContext, TickerSettings,
};

/// The direction a slider moves in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// Left to right.
    #[default]
    Horizontal,
    /// Bottom to top.
    Vertical,
}

/// Values a slider snaps to (matplotlib's `valstep`).
#[derive(Clone, Debug, PartialEq)]
pub enum ValStep {
    /// Multiples of this step from the minimum.
    Step(f64),
    /// The closest of these values.
    Values(Vec<f64>),
}

/// Options of a [`Slider`] (and, apart from `valinit` and `initcolor`,
/// of a [`RangeSlider`](super::RangeSlider)).
#[derive(Clone, Debug, PartialEq)]
pub struct SliderOptions {
    /// The initial value.
    pub valinit: f64,
    /// A printf-style format of the value (`%1.2f`); by default the
    /// axis' scalar formatter.
    pub valfmt: Option<String>,
    /// Whether the minimum can be reached.
    pub closedmin: bool,
    /// Whether the maximum can be reached.
    pub closedmax: bool,
    /// Whether the value follows the pointer while dragging (otherwise it
    /// only jumps on clicks).
    pub dragging: bool,
    /// Values to snap to.
    pub valstep: Option<ValStep>,
    /// The direction.
    pub orientation: Orientation,
    /// Colour of the line marking the initial value (`None` hides it).
    pub initcolor: Option<Color>,
    /// Colour of the track.
    pub track_color: Color,
    /// Colour of the selected part of the track (the patch colour by
    /// default).
    pub facecolor: Option<Color>,
}

impl Default for SliderOptions {
    /// matplotlib's defaults: from 0.5, a red initial-value line on a
    /// light grey track.
    fn default() -> Self {
        Self {
            valinit: 0.5,
            valfmt: None,
            closedmin: true,
            closedmax: true,
            dragging: true,
            valstep: None,
            orientation: Orientation::Horizontal,
            initcolor: Some(Color::from("r")),
            track_color: Color::from("lightgrey"),
            facecolor: None,
        }
    }
}

/// What every slider shares: its range, snapping and formatting
/// (matplotlib's `SliderBase`).
#[derive(Debug)]
pub(super) struct SliderBase {
    pub(super) ax: Axes,
    pub(super) orientation: Orientation,
    pub(super) valmin: f64,
    pub(super) valmax: f64,
    pub(super) closedmin: Cell<bool>,
    pub(super) closedmax: Cell<bool>,
    valstep: Option<ValStep>,
    valfmt: Option<PrintfFormat>,
    formatter: ScalarFormatter,
    settings: TickerSettings,
    pub(super) dragging: bool,
    pub(super) drag_active: Cell<bool>,
}

impl SliderBase {
    /// Sets up `ax` for a slider over `valmin..valmax`: limits along the
    /// slider, no decorations, no navigation.
    pub(super) fn new(ax: &Axes, valmin: f64, valmax: f64, options: &SliderOptions) -> Self {
        let axis = match options.orientation {
            Orientation::Vertical => {
                ax.set_ylim(valmin, valmax);
                ax.yaxis()
            }
            Orientation::Horizontal => {
                ax.set_xlim(valmin, valmax);
                ax.xaxis()
            }
        };
        let settings = TickerSettings::from(&*crate::rc::params());
        // matplotlib changes the axis' own formatter, which the value text
        // then shares.
        let formatter = ScalarFormatter::from_settings(&settings)
            .with_use_offset(OffsetMode::Off)
            .with_use_mathtext(true);
        axis.set_major_formatter(formatter.clone());
        ax.set_axis_off();
        ax.set_navigate(false);
        Self {
            ax: ax.clone(),
            orientation: options.orientation,
            valmin,
            valmax,
            closedmin: Cell::new(options.closedmin),
            closedmax: Cell::new(options.closedmax),
            valstep: options.valstep.clone(),
            valfmt: options
                .valfmt
                .as_deref()
                .and_then(|format| PrintfFormat::parse(format).ok()),
            formatter,
            settings,
            dragging: options.dragging,
            drag_active: Cell::new(false),
        }
    }

    /// `val` snapped to the step grid (matplotlib's `_stepped_value`).
    pub(super) fn stepped(&self, val: f64) -> f64 {
        match &self.valstep {
            None => val,
            Some(ValStep::Step(step)) => {
                self.valmin + ((val - self.valmin) / step).round_ties_even() * step
            }
            Some(ValStep::Values(values)) => values
                .iter()
                .copied()
                .fold(None, |best: Option<f64>, v| match best {
                    Some(b) if (b - val).abs() <= (v - val).abs() => Some(b),
                    _ => Some(v),
                })
                .unwrap_or(val),
        }
    }

    /// The values formatted together (so they share a precision), between
    /// the limits (matplotlib's `_format`).
    pub(super) fn format(&self, values: &[f64]) -> Vec<String> {
        if let Some(format) = &self.valfmt {
            return values
                .iter()
                .map(|&v| format.format(v).unwrap_or_default())
                .collect();
        }
        let mut locs = vec![self.valmin];
        locs.extend_from_slice(values);
        locs.push(self.valmax);
        let axis = AxisInfo::with_view(self.valmin, self.valmax);
        let ctx = TickContext::new(&axis, &self.settings);
        self.formatter.format_ticks(&locs, &ctx).map_or_else(
            |_| vec![String::new(); values.len()],
            |labels| {
                labels.labels[1..=values.len()]
                    .iter()
                    .map(|label| format!("{label}{}", labels.offset))
                    .collect()
            },
        )
    }

    /// `val` snapped and held in the range (`None` when an open end
    /// would be reached), the first part of matplotlib's
    /// `Slider._value_in_bounds`.
    pub(super) fn in_range(&self, val: f64) -> Option<f64> {
        let val = self.stepped(val);
        if val <= self.valmin {
            self.closedmin.get().then_some(self.valmin)
        } else if val >= self.valmax {
            self.closedmax.get().then_some(self.valmax)
        } else {
            Some(val)
        }
    }

    /// The value under a mouse event, along the slider.
    pub(super) fn event_value(&self, mouse: &MouseEvent) -> Option<f64> {
        match self.orientation {
            Orientation::Horizontal => mouse.location.xdata,
            Orientation::Vertical => mouse.location.ydata,
        }
    }

    /// matplotlib's `_update` up to the value: starts a drag on a press in
    /// the axes, ends it on a release or a press elsewhere; returns the
    /// mouse event to take a value from while dragging.
    pub(super) fn drag<'e>(&self, event: &'e Event) -> Option<&'e MouseEvent> {
        let (mouse, pressed) = match event {
            Event::ButtonPress(mouse) => (mouse, Some(true)),
            Event::ButtonRelease(mouse) => (mouse, Some(false)),
            Event::Motion(mouse) => (mouse, None),
            _ => return None,
        };
        if mouse.button != Some(PointerButton::Left) {
            return None;
        }
        let figure = self.ax.figure();
        let inside = self
            .ax
            .contains_point(Point::new(mouse.location.x, mouse.location.y));
        if pressed == Some(true) && inside {
            self.drag_active.set(true);
            figure.grab_mouse(&self.ax);
        }
        if !self.drag_active.get() {
            return None;
        }
        if pressed == Some(false) || (pressed == Some(true) && !inside) {
            self.drag_active.set(false);
            figure.release_mouse(&self.ax);
            return None;
        }
        Some(mouse)
    }

    /// The event kinds a slider listens to.
    pub(super) fn event_kinds(&self) -> Vec<EventKind> {
        let mut kinds = vec![EventKind::ButtonPress, EventKind::ButtonRelease];
        if self.dragging {
            kinds.push(EventKind::Motion);
        }
        kinds
    }

    /// The light grey track the selection slides along, as wide as half
    /// the axes (`height` is matplotlib's track length: 2 for vertical
    /// range sliders).
    pub(super) fn track(&self, color: &Color, length: f64) -> Patch<Rectangle> {
        let shape = match self.orientation {
            Orientation::Vertical => Rectangle::new(Point::new(0.25, 0.0), 0.5, length),
            Orientation::Horizontal => Rectangle::new(Point::new(0.0, 0.25), length, 0.5),
        };
        let track = self
            .ax
            .add_patch_in(shape, self.ax.trans_axes(), (false, false));
        track.set_facecolor(color.clone());
        track
    }

    /// A round handle at `value` along the slider.
    pub(super) fn handle(&self, value: f64) -> Line2D {
        let (x, y) = match self.orientation {
            Orientation::Vertical => (0.5, value),
            Orientation::Horizontal => (value, 0.5),
        };
        let props = LineProps::default()
            .linestyle(LineStyle::None)
            .marker(Marker::Symbol(MarkerSymbol::Circle))
            .markerfacecolor(ColorOrAuto::Color(Color::from("white")))
            .markeredgecolor(ColorOrAuto::Color(Color::from(".75")))
            .markersize(10.0);
        let handle = self
            .ax
            .plot_with(vec![x], vec![y], props)
            .expect("one point makes a valid line");
        handle.set_clip_on(false);
        handle
    }

    /// Moves `handle` to `value` along the slider.
    pub(super) fn move_handle(&self, handle: &Line2D, value: f64) {
        match self.orientation {
            Orientation::Vertical => handle.set_ydata(vec![value]),
            Orientation::Horizontal => handle.set_xdata(vec![value]),
        };
    }

    /// The label beside (horizontal) or above (vertical) the slider and
    /// the value text on the other side.
    pub(super) fn texts(&self, label: &str, value: &str) -> (Text, Text) {
        let place = |x: f64, y: f64, text: &str, halign, valign| {
            let placed = self.ax.text(x, y, text);
            placed.set_transform(self.ax.trans_axes());
            placed.set_halign(halign).set_valign(valign);
            placed
        };
        match self.orientation {
            Orientation::Vertical => (
                place(0.5, 1.02, label, HAlign::Center, VAlign::Bottom),
                place(0.5, -0.02, value, HAlign::Center, VAlign::Top),
            ),
            Orientation::Horizontal => (
                place(-0.02, 0.5, label, HAlign::Right, VAlign::Center),
                place(1.02, 0.5, value, HAlign::Left, VAlign::Center),
            ),
        }
    }
}

/// The parts of a slider its event handlers change.
#[derive(Debug)]
struct SliderParts {
    base: SliderBase,
    poly: Patch<Rectangle>,
    handle: Line2D,
    valtext: Text,
    val: Cell<f64>,
    /// Sliders whose values bound this one from below and above
    /// (matplotlib's `slidermin` and `slidermax`).
    links: RefCell<(Weak<SliderParts>, Weak<SliderParts>)>,
    changed: Observers<f64>,
}

impl SliderParts {
    /// matplotlib's `Slider._value_in_bounds`: snapped, held in the range
    /// and between the linked sliders (`None` when an open end would be
    /// reached).
    fn in_bounds(&self, val: f64) -> Option<f64> {
        let val = self.base.in_range(val)?;
        let (below, above) = &*self.links.borrow();
        if let Some(below) = below.upgrade()
            && val <= below.val.get()
        {
            return self.base.closedmin.get().then(|| below.val.get());
        }
        if let Some(above) = above.upgrade()
            && val >= above.val.get()
        {
            return self.base.closedmax.get().then(|| above.val.get());
        }
        Some(val)
    }

    /// Shows `val` without running the callbacks.
    fn show(&self, val: f64) {
        match self.base.orientation {
            Orientation::Vertical => self.poly.update_shape(|r| r.height = val - r.xy.y),
            Orientation::Horizontal => self.poly.update_shape(|r| r.width = val - r.xy.x),
        };
        self.base.move_handle(&self.handle, val);
        let text = self.base.format(&[val]).remove(0);
        self.valtext.set_text(text);
        self.val.set(val);
    }

    fn set_val(&self, val: f64) {
        self.show(val);
        self.changed.process(&val);
    }
}

/// A slider choosing one value in a range (matplotlib's `Slider`).
#[derive(Debug)]
pub struct Slider {
    parts: Rc<SliderParts>,
    label: Text,
    valinit: f64,
    _connections: Connections,
}

impl Slider {
    /// Turns `ax` into a slider over `valmin..valmax` labelled `label`.
    pub fn new(ax: &Axes, label: &str, valmin: f64, valmax: f64, options: &SliderOptions) -> Self {
        let base = SliderBase::new(ax, valmin, valmax, options);
        let valinit = base.in_range(options.valinit).unwrap_or(valmin);
        let track = base.track(&options.track_color, 1.0);
        let poly = match options.orientation {
            Orientation::Vertical => ax.axhspan_between(valmin, valinit, 0.25, 0.75),
            Orientation::Horizontal => ax.axvspan_between(valmin, valinit, 0.25, 0.75),
        };
        if let Some(facecolor) = &options.facecolor {
            poly.set_facecolor(facecolor.clone());
        }
        if let Some(initcolor) = &options.initcolor {
            let props = LineProps::default().color(initcolor.clone()).linewidth(1.0);
            let line = match options.orientation {
                Orientation::Vertical => ax.axhline(valinit, props),
                Orientation::Horizontal => ax.axvline(valinit, props),
            };
            line.set_clip_path(Some(crate::artist::ClipPathSpec::Artist(track.id())));
        }
        let handle = base.handle(valinit);
        let (label, valtext) = base.texts(label, &base.format(&[valinit]).remove(0));
        let kinds = base.event_kinds();
        let parts = Rc::new(SliderParts {
            base,
            poly,
            handle,
            valtext,
            val: Cell::new(valinit),
            links: RefCell::new((Weak::new(), Weak::new())),
            changed: Observers::default(),
        });
        parts.set_val(valinit);
        let mut connections = Connections::new(ax.figure());
        for kind in kinds {
            let parts = Rc::clone(&parts);
            connections.connect(kind, move |event| {
                let Some(mouse) = parts.base.drag(event) else {
                    return;
                };
                let value = parts
                    .base
                    .event_value(mouse)
                    .and_then(|v| parts.in_bounds(v));
                if let Some(value) = value.filter(|&v| v.total_cmp(&parts.val.get()).is_ne()) {
                    parts.set_val(value);
                }
            });
        }
        Self {
            parts,
            label,
            valinit,
            _connections: connections,
        }
    }

    /// The current value.
    pub fn val(&self) -> f64 {
        self.parts.val.get()
    }

    /// Sets the value, moving the slider and running the callbacks.
    pub fn set_val(&self, val: f64) {
        self.parts.set_val(val);
    }

    /// Returns to the initial value.
    pub fn reset(&self) {
        if self.val().total_cmp(&self.valinit).is_ne() {
            self.set_val(self.valinit);
        }
    }

    /// Returns to the initial value without running the callbacks.
    pub(super) fn reset_quietly(&self) {
        self.parts.show(self.valinit);
    }

    /// Whether the minimum and the maximum can be reached.
    pub fn set_closed(&self, min: bool, max: bool) {
        self.parts.base.closedmin.set(min);
        self.parts.base.closedmax.set(max);
    }

    /// Keeps the value above `other`'s (matplotlib's `slidermin`).
    pub fn set_slidermin(&self, other: &Self) {
        self.parts.links.borrow_mut().0 = Rc::downgrade(&other.parts);
    }

    /// Keeps the value below `other`'s (matplotlib's `slidermax`).
    pub fn set_slidermax(&self, other: &Self) {
        self.parts.links.borrow_mut().1 = Rc::downgrade(&other.parts);
    }

    /// The label.
    pub fn label(&self) -> &Text {
        &self.label
    }

    /// The text showing the value.
    pub fn valtext(&self) -> &Text {
        &self.parts.valtext
    }

    /// Runs `callback` with each new value (matplotlib's `on_changed`).
    pub fn on_changed(&self, mut callback: impl FnMut(f64) + 'static) {
        self.parts.changed.connect(move |&value| callback(value));
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::events::EventSource;
    use crate::figure::Figure;

    #[test]
    fn values_snap_format_and_follow_drags() {
        let fig = Figure::with_size(5.0, 3.0, 100.0);
        let ax = fig.add_axes([0.2, 0.8, 0.6, 0.08]);
        let slider = Slider::new(
            &ax,
            "Freq",
            0.1,
            30.0,
            &SliderOptions {
                valinit: 5.0,
                ..SliderOptions::default()
            },
        );
        assert_eq!(slider.val(), 5.0);
        assert_eq!(slider.valtext().text(), r"$\mathdefault{5.0}$");
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&seen);
        slider.on_changed(move |v| log.borrow_mut().push(v));
        let mut source = EventSource::new();
        let y = 0.84 * 300.0;
        source.button_press(&fig, 0.2 * 500.0 + 150.0, y, PointerButton::Left, false);
        source.motion(&fig, 2000.0, y);
        source.button_release(&fig, 2000.0, y, PointerButton::Left);
        let seen = seen.borrow();
        assert_eq!(seen.len(), 2, "{seen:?}");
        assert_eq!(*seen.last().expect("values"), 30.0, "held at the maximum");
        drop(seen);
        slider.reset();
        assert_eq!(slider.val(), 5.0);
    }

    #[test]
    fn steps_snap_to_the_grid() {
        let fig = Figure::new();
        let ax = fig.add_axes([0.05, 0.1, 0.05, 0.6]);
        let slider = Slider::new(
            &ax,
            "Amp",
            0.0,
            10.0,
            &SliderOptions {
                valinit: 7.2,
                valstep: Some(ValStep::Step(1.0)),
                orientation: Orientation::Vertical,
                ..SliderOptions::default()
            },
        );
        assert_eq!(slider.val(), 7.0);
        assert_eq!(slider.valtext().text(), r"$\mathdefault{7}$");
    }
}
