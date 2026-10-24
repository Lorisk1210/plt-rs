//! Sliders (matplotlib's `widgets.SliderBase` and `widgets.Slider`); the
//! part every slider shares is in `base`.

mod base;
mod options;

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use super::{CallbackId, Connections, Observers};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, PlotError};
use crate::lines::{Line2D, LineProps};
use crate::patches::{Patch, Rectangle};
use crate::text::Text;

pub(super) use base::SliderBase;
pub use options::{HandleStyle, Orientation, SliderOptions, ValStep};

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
    ///
    /// # Errors
    ///
    /// When `valmin` or `valmax` is NaN or infinite; then `ax` is
    /// unchanged.
    pub fn new(
        ax: &Axes,
        label: &str,
        valmin: f64,
        valmax: f64,
        options: SliderOptions,
    ) -> Result<Self, PlotError> {
        let base = SliderBase::new(ax, valmin, valmax, &options)?;
        let valinit = base.in_range(options.valinit).unwrap_or(valmin);
        let track = base.track(&options.track_color, 1.0);
        let poly = match options.orientation {
            Orientation::Vertical => ax.axhspan_between(valmin, valinit, 0.25, 0.75),
            Orientation::Horizontal => ax.axvspan_between(valmin, valinit, 0.25, 0.75),
        };
        poly.update_props(options.poly.clone());
        let props = LineProps::default()
            .color(options.initcolor.clone())
            .linewidth(1.0);
        let line = match options.orientation {
            Orientation::Vertical => ax.axhline(valinit, props),
            Orientation::Horizontal => ax.axvline(valinit, props),
        };
        // matplotlib gives the line its clip path before adding it to the
        // axes, which then leaves it without a clip box.
        line.raw().with_base_mut(|b| {
            b.clip_path = Some(crate::artist::ClipPathSpec::Artist(track.id()));
            b.clip_box = None;
        });
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
        Ok(Self {
            parts,
            label,
            valinit,
            _connections: connections,
        })
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

    /// The slider made with `other` as its `slidermin` (matplotlib's
    /// keyword argument): the initial value is held above `other`'s too.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::widgets::{Slider, SliderOptions};
    /// let ax = Figure::new().subplot();
    /// let at = |valinit| SliderOptions { valinit, ..SliderOptions::default() };
    /// let low = Slider::new(&ax, "low", 0.0, 10.0, at(5.0))?;
    /// let high = Slider::new(&ax, "high", 0.0, 10.0, at(2.0))?.with_slidermin(&low);
    /// assert_eq!(high.val(), 5.0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn with_slidermin(self, other: &Self) -> Self {
        self.set_slidermin(other);
        self.rebound_valinit()
    }

    /// The slider made with `other` as its `slidermax` (matplotlib's
    /// keyword argument): the initial value is held below `other`'s too.
    #[must_use]
    pub fn with_slidermax(self, other: &Self) -> Self {
        self.set_slidermax(other);
        self.rebound_valinit()
    }

    /// The initial value bounded again after a link was added, as
    /// matplotlib bounds `valinit` (the minimum where an open end would be
    /// reached), and shown.
    fn rebound_valinit(mut self) -> Self {
        let base = &self.parts.base;
        self.valinit = self.parts.in_bounds(self.valinit).unwrap_or(base.valmin);
        self.parts.show(self.valinit);
        self
    }

    /// The patch filling the slider up to the value (matplotlib's
    /// `poly`).
    pub fn poly(&self) -> &Patch<Rectangle> {
        &self.parts.poly
    }

    /// The label.
    pub fn label(&self) -> &Text {
        &self.label
    }

    /// The text showing the value.
    pub fn valtext(&self) -> &Text {
        &self.parts.valtext
    }

    /// Runs `callback` with each new value (matplotlib's `on_changed`);
    /// returns its id for [`Slider::disconnect`].
    pub fn on_changed(&self, mut callback: impl FnMut(f64) + 'static) -> CallbackId {
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
    use std::cell::RefCell;

    use super::*;
    use crate::colors::Color;
    use crate::events::EventSource;
    use crate::figure::Figure;
    use crate::navigation::PointerButton;

    #[test]
    fn values_snap_format_and_follow_drags() {
        let fig = Figure::with_size(5.0, 3.0, 100.0);
        let ax = fig
            .add_axes([0.2, 0.8, 0.6, 0.08])
            .expect("a valid rectangle");
        let slider = Slider::new(
            &ax,
            "Freq",
            0.1,
            30.0,
            SliderOptions {
                valinit: 5.0,
                ..SliderOptions::default()
            },
        )
        .expect("a finite range");
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
        let ax = fig
            .add_axes([0.05, 0.1, 0.05, 0.6])
            .expect("a valid rectangle");
        let slider = Slider::new(
            &ax,
            "Amp",
            0.0,
            10.0,
            SliderOptions {
                valinit: 7.2,
                valstep: Some(ValStep::Step(1.0)),
                orientation: Orientation::Vertical,
                ..SliderOptions::default()
            },
        )
        .expect("a finite range");
        assert_eq!(slider.val(), 7.0);
        assert_eq!(slider.valtext().text(), r"$\mathdefault{7}$");
    }

    #[test]
    fn handles_and_selection_take_their_styles() {
        use crate::patches::PatchProps;

        let fig = Figure::new();
        let ax = fig
            .add_axes([0.2, 0.5, 0.6, 0.1])
            .expect("a valid rectangle");
        let options = SliderOptions {
            handle_style: HandleStyle {
                size: 14.0,
                facecolor: Color::from("tab:orange"),
                ..HandleStyle::default()
            },
            poly: PatchProps::default().facecolor("tab:green").alpha(0.6),
            ..SliderOptions::default()
        };
        let slider = Slider::new(&ax, "s", 0.0, 1.0, options).expect("a finite range");
        assert_eq!(slider.parts.handle.markersize(), 14.0);
        assert_eq!(slider.parts.poly.alpha(), Some(0.6));
    }

    #[test]
    fn bounding_sliders_hold_the_initial_value() {
        let ax = Figure::new().subplot();
        let at = |valinit| SliderOptions {
            valinit,
            ..SliderOptions::default()
        };
        let high = Slider::new(&ax, "high", 0.0, 24.0, at(5.0)).expect("a finite range");
        let low = Slider::new(&ax, "low", 0.0, 24.0, at(10.0))
            .expect("a finite range")
            .with_slidermax(&high);
        assert_eq!(low.val(), 5.0);
        low.set_val(1.0);
        low.reset();
        assert_eq!(low.val(), 5.0, "the bounded value is the initial one");
        assert_eq!(low.poly().shape().width, 5.0);
    }
}
