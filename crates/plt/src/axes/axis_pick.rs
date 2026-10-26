//! Picking an axis (matplotlib's `Axis.contains`, `Axis.pickradius` and
//! `Axis.set_picker`): a press within a band along the outside of an axes
//! picks its x or y axis.

use plt_core::{Bbox, Point};

use super::{AxesState, Axis, PlotError};
use crate::artist::{ArtistHandle, LayoutContext, Picker};
use crate::ticker::AxisName;

/// The pick radius of a new axis, in pixels (matplotlib's `pickradius`
/// default).
const DEFAULT_PICKRADIUS: f64 = 15.0;

/// How an axis is picked.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AxisPicking {
    /// Whether, and how, a press picks the axis.
    pub(crate) picker: Option<Picker>,
    /// How far outside the axes box, in pixels, a press still hits the
    /// axis.
    pub(crate) radius: f64,
}

impl Default for AxisPicking {
    fn default() -> Self {
        Self {
            picker: None,
            radius: DEFAULT_PICKRADIUS,
        }
    }
}

impl AxisPicking {
    /// Whether display point `at` hits axis `name` of an axes whose box in
    /// display pixels is `bbox` (matplotlib's `XAxis.contains` and
    /// `YAxis.contains`): across the axes span, and outside its box by
    /// less than the pick radius, on either side.
    pub(crate) fn hits(&self, bbox: &Bbox, name: AxisName, at: Point) -> bool {
        let (along, across, (start, end), (low, high)) = match name {
            AxisName::X => (at.x, at.y, bbox.interval_x(), bbox.interval_y()),
            AxisName::Y => (at.y, at.x, bbox.interval_y(), bbox.interval_x()),
        };
        let fraction = (along - start) / (end - start);
        (0.0..=1.0).contains(&fraction)
            && ((low - self.radius < across && across < low)
                || (high < across && across < high + self.radius))
    }
}

impl Axis {
    /// How far outside the axes box, in pixels, a press still picks the
    /// axis (matplotlib's `get_pickradius`); 15 unless set.
    pub fn pickradius(&self) -> f64 {
        self.read(|axis| axis.picking.radius)
    }

    /// Sets how far outside the axes box, in pixels, a press still picks
    /// the axis (matplotlib's `set_pickradius`).
    ///
    /// # Errors
    ///
    /// [`PlotError::InvalidPickRadius`] when `pickradius` is negative or
    /// NaN, where matplotlib raises for a negative one.
    pub fn set_pickradius(&self, pickradius: f64) -> Result<&Self, PlotError> {
        if pickradius.is_nan() || pickradius < 0.0 {
            return Err(PlotError::InvalidPickRadius(pickradius));
        }
        self.update(|axis| axis.picking.radius = pickradius);
        Ok(self)
    }

    /// Makes presses pick the axis (matplotlib's `axis.set_picker`), or
    /// not with `None`: a press across the axes span and outside the axes
    /// box by less than the [pick radius](Self::pickradius), on either
    /// side, fires a [`PickEvent`](crate::events::PickEvent) whose
    /// [`axis`](crate::events::PickEvent::axis) names this one. As in
    /// matplotlib, a [`Picker::Radius`] tolerance does not apply: the pick
    /// radius does. A [`Picker::Custom`] test is given the axes.
    ///
    /// ```
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    /// use plt::artist::Picker;
    /// use plt::events::{Event, EventKind, EventSource};
    /// use plt::figure::Figure;
    /// use plt::navigation::PointerButton;
    /// use plt::ticker::AxisName;
    ///
    /// let fig = Figure::with_size(4.0, 3.0, 100.0);
    /// let ax = fig.add_axes([0.25, 0.25, 0.5, 0.5])?;
    /// ax.xaxis().set_picker(Some(Picker::Default));
    /// let picked = Rc::new(RefCell::new(Vec::new()));
    /// let log = Rc::clone(&picked);
    /// fig.connect(EventKind::Pick, move |event| {
    ///     if let Event::Pick(pick) = event {
    ///         log.borrow_mut().push(pick.axis);
    ///     }
    /// });
    /// // The axes spans 100 to 300 pixels across, 75 to 225 up.
    /// let mut source = EventSource::new();
    /// source.button_press(&fig, 200.0, 65.0, PointerButton::Left, false);
    /// source.button_press(&fig, 200.0, 50.0, PointerButton::Left, false);
    /// assert_eq!(*picked.borrow(), [Some(AxisName::X)], "within 15 pixels");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_picker(&self, picker: Option<Picker>) -> &Self {
        self.update(|axis| axis.picking.picker = picker);
        self
    }

    /// How a press picks the axis, if it does (matplotlib's
    /// `axis.get_picker`).
    pub fn picker(&self) -> Option<Picker> {
        self.read(|axis| axis.picking.picker.clone())
    }

    /// Whether presses pick the axis (matplotlib's `axis.pickable`).
    pub fn pickable(&self) -> bool {
        self.read(|axis| axis.picking.picker.is_some())
    }

    /// Whether a press at display point `at` (pixels from the figure's
    /// bottom-left corner) hits the axis in the figure's current layout:
    /// across the axes span and outside the axes box by less than the pick
    /// radius (matplotlib's `Axis.contains`).
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::transforms::Point;
    ///
    /// let fig = Figure::with_size(4.0, 3.0, 100.0);
    /// let ax = fig.add_axes([0.25, 0.25, 0.5, 0.5])?;
    /// // Left of the axes box (100 pixels from the left), within 15.
    /// assert!(ax.yaxis().contains(Point::new(90.0, 150.0)));
    /// assert!(!ax.yaxis().contains(Point::new(150.0, 150.0)), "inside");
    /// ax.yaxis().set_pickradius(5.0)?;
    /// assert!(!ax.yaxis().contains(Point::new(90.0, 150.0)));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn contains(&self, at: Point) -> bool {
        let figure = self.axes.raw().figure_state();
        let mut state = figure.borrow_mut();
        state.prepare();
        let layout = LayoutContext::new(&state);
        let id = self.axes.id();
        let picking = layout
            .store()
            .get_as::<AxesState>(id)
            .map(|axes| axes.axis(self.name).picking.clone());
        picking
            .zip(layout.axes_extent(id))
            .is_some_and(|(picking, bbox)| picking.hits(&bbox, self.name, at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_lie_in_bands_outside_the_box() {
        let bbox = Bbox::from_extents(100.0, 50.0, 300.0, 250.0);
        let picking = AxisPicking::default();
        let x = |px, py| picking.hits(&bbox, AxisName::X, Point::new(px, py));
        assert!(x(200.0, 40.0) && x(200.0, 260.0), "below and above");
        assert!(x(100.0, 49.0) && x(300.0, 251.0), "the span's ends");
        assert!(!x(200.0, 50.0) && !x(200.0, 35.0), "open bands");
        assert!(!x(200.0, 150.0), "inside");
        assert!(!x(99.0, 40.0) && !x(301.0, 40.0), "beyond the span");
        let y = |px, py| picking.hits(&bbox, AxisName::Y, Point::new(px, py));
        assert!(y(90.0, 100.0) && y(310.0, 100.0));
        assert!(!y(90.0, 260.0) && !y(150.0, 100.0));
        let none = AxisPicking {
            radius: 0.0,
            ..AxisPicking::default()
        };
        assert!(!none.hits(&bbox, AxisName::X, Point::new(200.0, 49.9)));
        let flat = Bbox::from_extents(100.0, 50.0, 100.0, 250.0);
        assert!(!picking.hits(&flat, AxisName::X, Point::new(100.0, 40.0)));
        assert!(!picking.hits(&bbox, AxisName::X, Point::new(f64::NAN, 40.0)));
    }

    #[test]
    fn pick_radii_are_distances() {
        let ax = crate::figure::Figure::new().subplot();
        let axis = ax.xaxis();
        assert_eq!(axis.pickradius(), 15.0);
        assert!(!axis.pickable() && axis.picker().is_none());
        assert_eq!(axis.set_pickradius(2.5).map(Axis::pickradius), Ok(2.5));
        assert_eq!(
            axis.set_pickradius(-1.0).err(),
            Some(PlotError::InvalidPickRadius(-1.0))
        );
        assert!(axis.set_pickradius(f64::NAN).is_err());
        assert_eq!(axis.pickradius(), 2.5);
        axis.set_picker(Some(Picker::Default));
        assert!(axis.pickable());
        assert!(!ax.yaxis().pickable(), "one axis at a time");
        ax.clear();
        assert_eq!((axis.pickradius(), axis.pickable()), (2.5, true), "kept");
    }
}
