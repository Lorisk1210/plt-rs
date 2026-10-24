//! What every slider shares: its range, snapping, value formatting, drag
//! handling and decorations (matplotlib's `SliderBase`).

use std::cell::Cell;

use plt_core::Point;
use plt_text::{HAlign, VAlign};

use super::{HandleStyle, Orientation, SliderOptions, ValStep};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, PlotError};
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

/// What every slider shares: its range, snapping and formatting
/// (matplotlib's `SliderBase`).
#[derive(Debug)]
pub(in crate::widgets) struct SliderBase {
    pub(in crate::widgets) ax: Axes,
    pub(in crate::widgets) orientation: Orientation,
    pub(in crate::widgets) valmin: f64,
    pub(in crate::widgets) valmax: f64,
    pub(in crate::widgets) closedmin: Cell<bool>,
    pub(in crate::widgets) closedmax: Cell<bool>,
    valstep: Option<ValStep>,
    valfmt: Option<PrintfFormat>,
    formatter: ScalarFormatter,
    settings: TickerSettings,
    pub(in crate::widgets) dragging: bool,
    pub(in crate::widgets) drag_active: Cell<bool>,
    handle_style: HandleStyle,
}

impl SliderBase {
    /// Sets up `ax` for a slider over `valmin..valmax`: limits along the
    /// slider, no decorations, no navigation.
    ///
    /// # Errors
    ///
    /// When `valmin` or `valmax` is NaN or infinite.
    pub(in crate::widgets) fn new(
        ax: &Axes,
        valmin: f64,
        valmax: f64,
        options: &SliderOptions,
    ) -> Result<Self, PlotError> {
        let axis = match options.orientation {
            Orientation::Vertical => ax.set_ylim(valmin, valmax)?.yaxis(),
            Orientation::Horizontal => ax.set_xlim(valmin, valmax)?.xaxis(),
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
        Ok(Self {
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
            handle_style: options.handle_style.clone(),
        })
    }

    /// `val` snapped to the step grid (matplotlib's `_stepped_value`).
    pub(in crate::widgets) fn stepped(&self, val: f64) -> f64 {
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
    pub(in crate::widgets) fn format(&self, values: &[f64]) -> Vec<String> {
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
    pub(in crate::widgets) fn in_range(&self, val: f64) -> Option<f64> {
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
    pub(in crate::widgets) fn event_value(&self, mouse: &MouseEvent) -> Option<f64> {
        match self.orientation {
            Orientation::Horizontal => mouse.location.xdata,
            Orientation::Vertical => mouse.location.ydata,
        }
    }

    /// matplotlib's `_update` up to the value: starts a drag on a press in
    /// the axes, ends it on a release or a press elsewhere; returns the
    /// mouse event to take a value from while dragging.
    pub(in crate::widgets) fn drag<'e>(&self, event: &'e Event) -> Option<&'e MouseEvent> {
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
    pub(in crate::widgets) fn event_kinds(&self) -> Vec<EventKind> {
        let mut kinds = vec![EventKind::ButtonPress, EventKind::ButtonRelease];
        if self.dragging {
            kinds.push(EventKind::Motion);
        }
        kinds
    }

    /// The light grey track the selection slides along, as wide as half
    /// the axes (`height` is matplotlib's track length: 2 for vertical
    /// range sliders).
    pub(in crate::widgets) fn track(&self, color: &Color, length: f64) -> Patch<Rectangle> {
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
    pub(in crate::widgets) fn handle(&self, value: f64) -> Line2D {
        let (x, y) = match self.orientation {
            Orientation::Vertical => (0.5, value),
            Orientation::Horizontal => (value, 0.5),
        };
        let style = &self.handle_style;
        let props = LineProps::default()
            .linestyle(LineStyle::None)
            .marker(Marker::Symbol(MarkerSymbol::Circle))
            .markerfacecolor(ColorOrAuto::Color(style.facecolor.clone()))
            .markeredgecolor(ColorOrAuto::Color(style.edgecolor.clone()))
            .markersize(style.size);
        let handle = self
            .ax
            .plot_with(vec![x], vec![y], props)
            .expect("one point makes a valid line");
        handle.set_clip_on(false);
        handle
    }

    /// Moves `handle` to `value` along the slider.
    pub(in crate::widgets) fn move_handle(&self, handle: &Line2D, value: f64) {
        match self.orientation {
            Orientation::Vertical => handle.set_ydata(vec![value]),
            Orientation::Horizontal => handle.set_xdata(vec![value]),
        };
    }

    /// The label beside (horizontal) or above (vertical) the slider and
    /// the value text on the other side.
    pub(in crate::widgets) fn texts(&self, label: &str, value: &str) -> (Text, Text) {
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
