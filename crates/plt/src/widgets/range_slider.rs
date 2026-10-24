//! Range sliders (matplotlib's `widgets.RangeSlider`): two handles
//! choosing an interval.

use std::cell::Cell;
use std::rc::Rc;

use plt_core::Point;

use super::slider::{Orientation, SliderBase, SliderOptions};
use super::{CallbackId, Connections, Observers};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::lines::Line2D;
use crate::patches::{Patch, Polygon};
use crate::text::Text;

/// The parts of a range slider its event handlers change.
#[derive(Debug)]
struct RangeParts {
    base: SliderBase,
    poly: Patch<Polygon>,
    handles: [Line2D; 2],
    valtext: Text,
    val: Cell<(f64, f64)>,
    changed: Observers<(f64, f64)>,
}

impl RangeParts {
    /// The lower end held between the minimum and the upper end
    /// (matplotlib's `_min_in_bounds`).
    fn min_in_bounds(&self, low: f64) -> f64 {
        let base = &self.base;
        let low = if low <= base.valmin {
            if !base.closedmin.get() {
                return self.val.get().0;
            }
            base.valmin
        } else {
            low
        };
        base.stepped(low.min(self.val.get().1))
    }

    /// The upper end held between the lower end and the maximum
    /// (matplotlib's `_max_in_bounds`).
    fn max_in_bounds(&self, high: f64) -> f64 {
        let base = &self.base;
        let high = if high >= base.valmax {
            if !base.closedmax.get() {
                return self.val.get().1;
            }
            base.valmax
        } else {
            high
        };
        base.stepped(high.max(self.val.get().0))
    }

    /// The selection outline: the rectangle between the ends, half as
    /// wide as the axes.
    fn selection(&self, (low, high): (f64, f64)) -> Polygon {
        let corners = match self.base.orientation {
            Orientation::Vertical => [(0.25, low), (0.25, high), (0.75, high), (0.75, low)],
            Orientation::Horizontal => [(low, 0.25), (low, 0.75), (high, 0.75), (high, 0.25)],
        };
        Polygon::new(
            corners
                .iter()
                .map(|&(x, y)| Point::new(x, y))
                .collect::<Vec<_>>(),
            true,
        )
    }

    /// matplotlib's `set_val`: sorts, bounds and shows the interval.
    fn set_val(&self, (a, b): (f64, f64)) {
        let (a, b) = (a.min(b), a.max(b));
        let base = &self.base;
        self.val.set((base.valmin, base.valmax));
        let low = self.min_in_bounds(a);
        let high = self.max_in_bounds(b);
        self.poly.set_shape(self.selection((low, high)));
        base.move_handle(&self.handles[0], low);
        base.move_handle(&self.handles[1], high);
        let texts = base.format(&[low, high]);
        self.valtext
            .set_text(format!("({}, {})", texts[0], texts[1]));
        self.val.set((low, high));
        self.changed.process(&(low, high));
    }

    /// Moves the end closest to `pos` there (matplotlib's
    /// `_update_val_from_pos`).
    fn update_from(&self, pos: f64) {
        let (low, high) = self.val.get();
        if (low - pos).abs() <= (high - pos).abs() {
            self.set_val((self.min_in_bounds(pos), high));
        } else {
            self.set_val((low, self.max_in_bounds(pos)));
        }
    }
}

/// A slider choosing an interval in a range (matplotlib's `RangeSlider`).
#[derive(Debug)]
pub struct RangeSlider {
    parts: Rc<RangeParts>,
    label: Text,
    valinit: (f64, f64),
    _connections: Connections,
}

impl RangeSlider {
    /// Turns `ax` into a range slider over `valmin..valmax` labelled
    /// `label`, starting at `valinit` (by default the middle half of the
    /// range). `options.valinit` and `options.initcolor` are not used.
    pub fn new(
        ax: &Axes,
        label: &str,
        valmin: f64,
        valmax: f64,
        valinit: Option<(f64, f64)>,
        options: SliderOptions,
    ) -> Self {
        let base = SliderBase::new(ax, valmin, valmax, &options);
        let length = match options.orientation {
            Orientation::Vertical => 2.0,
            Orientation::Horizontal => 1.0,
        };
        base.track(&options.track_color, length);
        let transform = match options.orientation {
            Orientation::Vertical => ax.trans_yaxis(),
            Orientation::Horizontal => ax.trans_xaxis(),
        };
        let empty = Polygon::new(vec![Point::new(0.0, 0.0); 5], true);
        let poly = ax.add_patch_in(empty, transform, (false, false));
        poly.update_props(options.poly.clone());
        let extent = valmax - valmin;
        let start = valinit.unwrap_or((valmin + extent * 0.25, valmin + extent * 0.75));
        let handles = [base.handle(start.0), base.handle(start.1)];
        let (label, valtext) = base.texts(label, "");
        let kinds = base.event_kinds();
        let parts = Rc::new(RangeParts {
            base,
            poly,
            handles,
            valtext,
            val: Cell::new((valmin, valmax)),
            changed: Observers::default(),
        });
        let valinit = match valinit {
            Some((a, b)) => (parts.min_in_bounds(a), parts.max_in_bounds(b)),
            None => start,
        };
        parts.set_val(valinit);
        let mut connections = Connections::new(ax.figure());
        for kind in kinds {
            let parts = Rc::clone(&parts);
            connections.connect(kind, move |event| {
                let pos = parts
                    .base
                    .drag(event)
                    .and_then(|mouse| parts.base.event_value(mouse));
                if let Some(pos) = pos {
                    parts.update_from(pos);
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

    /// The current interval.
    pub fn val(&self) -> (f64, f64) {
        self.parts.val.get()
    }

    /// Sets the interval (its ends are sorted and bounded) and runs the
    /// callbacks.
    pub fn set_val(&self, val: (f64, f64)) {
        self.parts.set_val(val);
    }

    /// Returns to the initial interval.
    pub fn reset(&self) {
        if self.val() != self.valinit {
            self.set_val(self.valinit);
        }
    }

    /// The patch between the ends (matplotlib's `poly`).
    pub fn poly(&self) -> &Patch<Polygon> {
        &self.parts.poly
    }

    /// The handles at the lower and the upper end (matplotlib's
    /// `_handles`).
    pub fn handles(&self) -> &[Line2D; 2] {
        &self.parts.handles
    }

    /// The label.
    pub fn label(&self) -> &Text {
        &self.label
    }

    /// The text showing the interval.
    pub fn valtext(&self) -> &Text {
        &self.parts.valtext
    }

    /// Runs `callback` with each new interval (matplotlib's
    /// `on_changed`); returns its id for [`RangeSlider::disconnect`].
    pub fn on_changed(&self, mut callback: impl FnMut((f64, f64)) + 'static) -> CallbackId {
        self.parts.changed.connect(move |&value| callback(value))
    }

    /// Disconnects the callback with id `id` (matplotlib's `disconnect`);
    /// returns whether it was connected.
    pub fn disconnect(&self, id: CallbackId) -> bool {
        self.parts.changed.disconnect(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn intervals_are_sorted_and_bounded() {
        let fig = Figure::new();
        let ax = fig.add_axes([0.2, 0.6, 0.6, 0.08]);
        let slider = RangeSlider::new(
            &ax,
            "Range",
            0.0,
            1.0,
            Some((0.2, 0.7)),
            SliderOptions::default(),
        );
        assert_eq!(slider.val(), (0.2, 0.7));
        assert_eq!(
            slider.valtext().text(),
            r"($\mathdefault{0.2}$, $\mathdefault{0.7}$)"
        );
        slider.set_val((1.5, -1.0));
        assert_eq!(slider.val(), (0.0, 1.0));
        let ends: Vec<f64> = slider.handles().iter().map(|h| h.xdata()[0]).collect();
        assert_eq!(ends, [0.0, 1.0]);
        let corners = slider.poly().shape().vertices().to_vec();
        assert_eq!((corners[0].x, corners[2].x), (0.0, 1.0));
        slider.reset();
        assert_eq!(slider.val(), (0.2, 0.7));
    }
}
