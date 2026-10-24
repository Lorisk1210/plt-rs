//! What the navigation tools hand a projection that pans or zooms its
//! axes itself (the arguments of matplotlib's `start_pan`, `drag_pan` and
//! `_set_view_from_bbox`), and the view limits it answers with.

use plt_core::{Bbox, Point};

use crate::axes::Axes;
use crate::navigation::{PointerButton, ZoomDirection};
use crate::ticker::AxisName;
use crate::transforms::ResolvedTransform;

/// The view limits a projection panning or zooming its axes sets
/// (matplotlib's `set_xlim` and `set_ylim` in `drag_pan` and
/// `_set_view_from_bbox`): an axis left `None` keeps its limits. Limits
/// are set as given, `v0 > v1` inverting the axis; limits that are not
/// finite leave the axis as it is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewLimits {
    /// The new x limits `(v0, v1)`.
    pub x: Option<(f64, f64)>,
    /// The new y limits `(v0, v1)`.
    pub y: Option<(f64, f64)>,
}

impl ViewLimits {
    /// Sets these limits on `ax`.
    pub(crate) fn apply(self, ax: &Axes) {
        for (name, limits) in [(AxisName::X, self.x), (AxisName::Y, self.y)] {
            if let Some((v0, v1)) = limits {
                // matplotlib raises for limits that are not finite; the
                // navigation tools leave the axis as it is.
                let _ = ax.set_limits(name, v0, v1);
            }
        }
    }
}

/// An axes as a navigation found it: its view limits, its box on the
/// canvas and its data transforms (matplotlib's frozen `viewLim`, `bbox`
/// and `transData`).
#[derive(Clone, Debug)]
pub struct FrozenView {
    limits: Bbox,
    bbox: Bbox,
    to_display: ResolvedTransform,
    to_data: ResolvedTransform,
}

impl FrozenView {
    /// The view of an axes with view limits `limits` placed at `bbox`
    /// (display coordinates), whose data transform is `to_display` with
    /// the inverse `to_data`.
    pub(crate) fn new(
        limits: Bbox,
        bbox: Bbox,
        to_display: ResolvedTransform,
        to_data: ResolvedTransform,
    ) -> Self {
        Self {
            limits,
            bbox,
            to_display,
            to_data,
        }
    }

    /// The view limits in data coordinates (matplotlib's `viewLim`); an
    /// inverted axis has its limits the other way round.
    pub fn limits(&self) -> Bbox {
        self.limits
    }

    /// The box of the axes in display coordinates (matplotlib's `bbox`).
    pub fn bbox(&self) -> Bbox {
        self.bbox
    }

    /// Data coordinates → display coordinates (matplotlib's `transData`).
    pub fn to_display(&self) -> &ResolvedTransform {
        &self.to_display
    }

    /// Display coordinates → data coordinates (matplotlib's
    /// `transData.inverted()`).
    pub fn to_data(&self) -> &ResolvedTransform {
        &self.to_data
    }
}

/// Where a drag of the pan/zoom tool started on an axes, and the axes
/// then (the arguments of matplotlib's `start_pan` and the `_pan_start`
/// it keeps).
#[derive(Clone, Debug)]
pub struct PanStart {
    at: Point,
    button: PointerButton,
    view: FrozenView,
}

impl PanStart {
    /// A pan started at display point `at` with `button` on an axes with
    /// `view`.
    pub(crate) fn new(at: Point, button: PointerButton, view: FrozenView) -> Self {
        Self { at, button, view }
    }

    /// The display point the button went down at (matplotlib's `x` and
    /// `y`).
    pub fn at(&self) -> Point {
        self.at
    }

    /// The button held down.
    pub fn button(&self) -> PointerButton {
        self.button
    }

    /// The axes when the button went down.
    pub fn view(&self) -> &FrozenView {
        &self.view
    }
}

/// A zoom of the zoom-to-rectangle tool or of the wheel on an axes (the
/// arguments of matplotlib's `_set_view_from_bbox`).
#[derive(Clone, Debug)]
pub struct ZoomBox {
    corners: Bbox,
    wheel: Option<(Point, f64)>,
    direction: ZoomDirection,
    changes_x: bool,
    changes_y: bool,
    view: FrozenView,
}

impl ZoomBox {
    /// A zoom to `corners` (display coordinates) by the wheel at the
    /// point and factor `wheel`, if it is one, in `direction`, changing
    /// the x and the y limits as the flags say, of an axes with `view`.
    pub(crate) fn new(
        corners: Bbox,
        wheel: Option<(Point, f64)>,
        direction: ZoomDirection,
        (changes_x, changes_y): (bool, bool),
        view: FrozenView,
    ) -> Self {
        Self {
            corners,
            wheel,
            direction,
            changes_x,
            changes_y,
            view,
        }
    }

    /// The box in display coordinates, from where the drag started to
    /// where it ended (matplotlib's 4-tuple `bbox`); for the wheel, the
    /// box around the pointer that matplotlib's `_prepare_view_from_bbox`
    /// makes of it.
    pub fn corners(&self) -> Bbox {
        self.corners
    }

    /// The display point and the factor of a zoom by the wheel, a factor
    /// above 1 zooming in (matplotlib's 3-tuple `bbox`); `None` for a
    /// dragged box.
    pub fn wheel(&self) -> Option<(Point, f64)> {
        self.wheel
    }

    /// Whether the view zooms into the box or out of it (the left or the
    /// right button; matplotlib's `direction`).
    pub fn direction(&self) -> ZoomDirection {
        self.direction
    }

    /// Whether the zoom may change the limits of `axis`: not when a key
    /// holds the other axis alone (matplotlib's `mode`), nor when a twin
    /// zoomed already changed them (`twinx` and `twiny`).
    pub fn changes(&self, axis: AxisName) -> bool {
        match axis {
            AxisName::X => self.changes_x,
            AxisName::Y => self.changes_y,
        }
    }

    /// The axes before the zoom.
    pub fn view(&self) -> &FrozenView {
        &self.view
    }
}
