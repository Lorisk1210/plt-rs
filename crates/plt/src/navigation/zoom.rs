//! Zooming (matplotlib's zoom-to-rectangle tool and
//! `Axes._set_view_from_bbox`).

use plt_core::{Bbox, Point};

use super::pan::bbox_through;
use super::{DragKey, PointerButton, Rubberband};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxesState, AxisName};
use crate::figure::{Figure, FigureState};
use crate::transforms::{ResolvedTransform, TransformContext};

/// Drags shorter than this many pixels cancel a zoom.
const MIN_DRAG: f64 = 5.0;

impl Axes {
    /// Whether the zoom-to-rectangle tool works on this axes (not on
    /// polar or geographic axes; matplotlib's `can_zoom`).
    pub fn can_zoom(&self) -> bool {
        self.as_polar().is_none() && self.as_geo().is_none()
    }

    /// Whether the pan/zoom tool works on this axes (not on geographic
    /// axes; matplotlib's `can_pan`).
    pub fn can_pan(&self) -> bool {
        self.as_geo().is_none()
    }

    /// Whether the navigation toolbar acts on this axes and reports its
    /// coordinates (matplotlib's `get_navigate`).
    pub fn navigate(&self) -> bool {
        self.raw().with::<AxesState, _>(|state| state.navigate)
    }

    /// Lets the navigation toolbar act on this axes or not (matplotlib's
    /// `set_navigate`; widgets turn it off).
    pub fn set_navigate(&self, navigate: bool) -> &Self {
        self.raw()
            .with_mut::<AxesState, _>(|state| state.navigate = navigate);
        self
    }
}

/// The axes box and data transform of `ax`, brought up to date.
fn geometry(ax: &Axes) -> Option<(Bbox, ResolvedTransform)> {
    let figure = ax.raw().figure_state();
    let mut state = figure.borrow_mut();
    state.prepare();
    let state: &FigureState = &state;
    Some((state.axes_bbox(ax.id())?, state.data_transform(ax.id())?))
}

/// The rectangle to show while dragging from `start` to `at` over `ax`
/// (matplotlib's `drag_zoom`): clipped to the axes, spanning its height
/// with `x` held or its width with `y`.
pub(super) fn rubberband(ax: &Axes, start: Point, at: Point, key: DragKey) -> Option<Rubberband> {
    let (bbox, _) = geometry(ax)?;
    let clip = |p: Point| {
        Point::new(
            p.x.clamp(bbox.xmin(), bbox.xmax()),
            p.y.clamp(bbox.ymin(), bbox.ymax()),
        )
    };
    let (mut a, mut b) = (clip(start), clip(at));
    match key {
        DragKey::X => (a.y, b.y) = bbox.interval_y(),
        DragKey::Y => (a.x, b.x) = bbox.interval_x(),
        _ => {}
    }
    Some(Rubberband {
        bbox: Bbox::from_extents(a.x, a.y, b.x, b.y),
    })
}

/// Which way a zoom box acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    /// The box becomes the view.
    In,
    /// The view shrinks into the box.
    Out,
}

/// Which axes of each zoomed axes change: twins zoomed through an earlier
/// axes and the axis a held key excludes stay put.
#[derive(Clone, Copy, Debug)]
struct Changes {
    x: bool,
    y: bool,
}

/// For each axes, whether its x and y should change, skipping axes
/// sharing x or y with an earlier one (matplotlib's `twinx`/`twiny`).
fn changes(figure: &Figure, axes: &[Axes], key: DragKey) -> Vec<Changes> {
    let state = figure.state();
    axes.iter()
        .enumerate()
        .map(|(i, ax)| {
            let shares = |name: AxisName| {
                axes[..i]
                    .iter()
                    .any(|prev| state.siblings(name, ax.id()).contains(&prev.id()))
            };
            Changes {
                x: !shares(AxisName::X) && key != DragKey::Y,
                y: !shares(AxisName::Y) && key != DragKey::X,
            }
        })
        .collect()
}

/// Whether a zoom box dragged from `start` to `at` is too small to count:
/// under 5 pixels across a side that `key` does not fix (matplotlib's
/// cancelled `release_zoom`).
pub(super) fn is_click(start: Point, at: Point, key: DragKey) -> bool {
    ((at.x - start.x).abs() < MIN_DRAG && key != DragKey::Y)
        || ((at.y - start.y).abs() < MIN_DRAG && key != DragKey::X)
}

/// Zooms `axes` to the box dragged from `start` to `at` with `button`;
/// returns whether the view changed (matplotlib's `release_zoom`).
pub(super) fn zoom_to_rect(
    figure: &Figure,
    axes: &[Axes],
    start: Point,
    at: Point,
    button: PointerButton,
    key: DragKey,
) -> bool {
    if is_click(start, at, key) {
        return false;
    }
    let direction = if button == PointerButton::Left {
        Direction::In
    } else {
        Direction::Out
    };
    let corners = Bbox::from_extents(start.x, start.y, at.x, at.y);
    for (ax, change) in axes.iter().zip(changes(figure, axes, key)) {
        set_view_from_bbox(ax, &corners, direction, change);
    }
    true
}

/// Zooms `axes` by `scale` (> 1 zooms in) around display point `at`
/// (matplotlib's `_set_view_from_bbox((x, y, scale))`).
pub(super) fn zoom_around(figure: &Figure, axes: &[Axes], at: Point, scale: f64) {
    let (direction, scale) = if scale > 1.0 {
        (Direction::In, scale)
    } else {
        (Direction::Out, 1.0 / scale)
    };
    for (ax, change) in axes.iter().zip(changes(figure, axes, DragKey::None)) {
        let Some((_, trans)) = geometry(ax) else {
            continue;
        };
        let limits = ax.raw().with::<AxesState, _>(|state| state.view_lim);
        let view = bbox_through(&limits, &trans);
        let (center, half) = (view.center(), (view.width() / 2.0, view.height() / 2.0));
        let focus = Point::new(
            (at.x * (scale - 1.0) + center.x) / scale,
            (at.y * (scale - 1.0) + center.y) / scale,
        );
        let corners = Bbox::from_extents(
            focus.x - half.0 / scale,
            focus.y - half.1 / scale,
            focus.x + half.0 / scale,
            focus.y + half.1 / scale,
        );
        set_view_from_bbox(ax, &corners, direction, change);
    }
}

/// matplotlib's `_prepare_view_from_bbox` + `_set_view_from_bbox` for a
/// box given by two display corners.
fn set_view_from_bbox(ax: &Axes, corners: &Bbox, direction: Direction, change: Changes) {
    let Some((_, trans)) = geometry(ax) else {
        return;
    };
    let Some(inverse) = trans.inverted() else {
        return;
    };
    let start = inverse.transform_point(Point::new(corners.x0(), corners.y0()));
    let stop = inverse.transform_point(Point::new(corners.x1(), corners.y1()));
    for (name, changes, (a, b)) in [
        (AxisName::X, change.x, (start.x, stop.x)),
        (AxisName::Y, change.y, (start.y, stop.y)),
    ] {
        if !changes {
            continue;
        }
        let (low0, high0) = bound(ax, name);
        let (low, high) = (a.min(b).clamp(low0, high0), a.max(b).clamp(low0, high0));
        let bounds = match direction {
            Direction::In => (low, high),
            Direction::Out => unzoom(ax, name, (low0, high0), (low, high)),
        };
        set_bound(ax, name, bounds);
    }
}

/// The view shrunk into `(low, high)` from `(low0, high0)`, in the axis'
/// scale space.
fn unzoom(
    ax: &Axes,
    name: AxisName,
    (low0, high0): (f64, f64),
    (low, high): (f64, f64),
) -> (f64, f64) {
    ax.raw().with::<AxesState, _>(|state| {
        let axis = state.axis(name);
        let (s_low0, s_high0) = (axis.forward(low0), axis.forward(high0));
        let (s_low, s_high) = (axis.forward(low), axis.forward(high));
        let factor = (s_high0 - s_low0) / (s_high - s_low);
        (
            axis.inverse(s_low0 - factor * (s_low - s_low0)),
            axis.inverse(s_high0 + factor * (s_high0 - s_high)),
        )
    })
}

/// The view interval of one axis, increasing (matplotlib's `get_xbound`).
fn bound(ax: &Axes, name: AxisName) -> (f64, f64) {
    let (a, b) = match name {
        AxisName::X => ax.xlim(),
        AxisName::Y => ax.ylim(),
    };
    (a.min(b), a.max(b))
}

/// Sets the view interval keeping the axis direction and turns its
/// autoscaling off (matplotlib's `set_xbound` + `set_autoscalex_on`).
fn set_bound(ax: &Axes, name: AxisName, (low, high): (f64, f64)) {
    let inverted = ax.raw().with::<AxesState, _>(|state| {
        let (a, b) = state.view_interval(name);
        a > b
    });
    let (start, end) = if inverted { (high, low) } else { (low, high) };
    ax.set_limits(name, start, end);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(ax: &Axes, x: f64, y: f64) -> Point {
        geometry(ax).unwrap().1.transform_point(Point::new(x, y))
    }

    #[test]
    fn dragging_a_box_zooms_into_it() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0).set_ylim(10.0, 0.0);
        let (a, b) = (display(&ax, 2.0, 3.0), display(&ax, 4.0, 7.0));
        assert!(zoom_to_rect(
            &fig,
            std::slice::from_ref(&ax),
            a,
            b,
            PointerButton::Left,
            DragKey::None
        ));
        let (x0, x1) = ax.xlim();
        assert!((x0 - 2.0).abs() < 1e-9 && (x1 - 4.0).abs() < 1e-9);
        let (y0, y1) = ax.ylim();
        assert!(
            (y0 - 7.0).abs() < 1e-9 && (y1 - 3.0).abs() < 1e-9,
            "inversion kept"
        );
    }

    #[test]
    fn tiny_drags_cancel_and_keys_fix_an_axis() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
        let a = display(&ax, 2.0, 2.0);
        let near = Point::new(a.x + 2.0, a.y + 50.0);
        assert!(!zoom_to_rect(
            &fig,
            std::slice::from_ref(&ax),
            a,
            near,
            PointerButton::Left,
            DragKey::None
        ));
        let b = display(&ax, 5.0, 6.0);
        zoom_to_rect(
            &fig,
            std::slice::from_ref(&ax),
            a,
            b,
            PointerButton::Left,
            DragKey::X,
        );
        let (x0, x1) = ax.xlim();
        assert!((x0 - 2.0).abs() < 1e-9 && (x1 - 5.0).abs() < 1e-9);
        assert_eq!(ax.ylim(), (0.0, 10.0));
    }

    #[test]
    fn zooming_out_and_around_a_point() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
        let (a, b) = (display(&ax, 0.0, 0.0), display(&ax, 5.0, 5.0));
        zoom_to_rect(
            &fig,
            std::slice::from_ref(&ax),
            a,
            b,
            PointerButton::Right,
            DragKey::None,
        );
        let (x0, x1) = ax.xlim();
        assert!((x0 - 0.0).abs() < 1e-9 && (x1 - 20.0).abs() < 1e-9);
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
        zoom_around(&fig, std::slice::from_ref(&ax), display(&ax, 0.0, 0.0), 2.0);
        let (x0, x1) = ax.xlim();
        assert!(
            x0.abs() < 1e-9 && (x1 - 5.0).abs() < 1e-9,
            "the focus stays put"
        );
    }

    #[test]
    fn rubberband_spans_the_axes_with_a_key() {
        let ax = Figure::new().subplot();
        let (bbox, _) = geometry(&ax).unwrap();
        let band = rubberband(&ax, bbox.center(), Point::new(-50.0, 0.0), DragKey::X).unwrap();
        assert_eq!(band.bbox.x1(), bbox.x0(), "clipped to the axes");
        assert_eq!(band.bbox.interval_y(), bbox.interval_y());
    }
}
