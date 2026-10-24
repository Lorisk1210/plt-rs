//! A projection panning and zooming its axes itself through the
//! navigation tools.

use plt_core::Point;

use super::display;
use crate::axes::{Axes, AxisName};
use crate::figure::Figure;
use crate::navigation::{DragKey, NavMode, Navigation, PointerButton, ZoomDirection};
use crate::projections::{PanStart, Projection, ProjectionView, ViewLimits, ZoomBox, register};
use crate::transforms::ResolvedTransform;

/// Rectilinear axes that pan with the right button by sliding x alone, a
/// data unit per 10 pixels, and zoom into boxes along x alone; they keep
/// what they were given.
#[derive(Debug, Default)]
struct Sliding {
    /// The press point and the x limits when a pan of its own started.
    start: Option<(Point, (f64, f64))>,
    /// How many pans of its own ended.
    ended: usize,
    /// For every zoom: whether it came from the wheel, and whether it may
    /// change y.
    zooms: Vec<(bool, bool)>,
}

impl Projection for Sliding {
    fn name(&self) -> &'static str {
        "sliding"
    }

    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        view.rectilinear()
    }

    fn start_pan(&mut self, start: &PanStart) -> bool {
        let own = start.button() == PointerButton::Right;
        self.start = own.then(|| (start.at(), start.view().limits().interval_x()));
        own
    }

    fn drag_pan(&mut self, start: &PanStart, _: DragKey, at: Point) -> ViewLimits {
        let (_, (x0, x1)) = self.start.expect("drags follow the start of a pan");
        let shift = (at.x - start.at().x) / 10.0;
        ViewLimits {
            x: Some((x0 - shift, x1 - shift)),
            y: None,
        }
    }

    fn end_pan(&mut self) {
        self.start = None;
        self.ended += 1;
    }

    fn zoom_to_box(&mut self, zoom: &ZoomBox) -> Option<ViewLimits> {
        self.zooms
            .push((zoom.wheel().is_some(), zoom.changes(AxisName::Y)));
        (zoom.direction() == ZoomDirection::In).then(|| {
            let (corners, to_data) = (zoom.corners(), zoom.view().to_data());
            let a = to_data.transform_point(Point::new(corners.x0(), corners.y0()));
            let b = to_data.transform_point(Point::new(corners.x1(), corners.y1()));
            ViewLimits {
                x: Some((a.x.min(b.x), a.x.max(b.x))),
                y: None,
            }
        })
    }
}

/// A figure with sliding axes showing `[0, 10]` on both axes.
fn sliding() -> (Figure, Axes) {
    let fig = Figure::new();
    let ax = fig
        .subplot_with_projection(register(Sliding::default).unwrap())
        .unwrap();
    ax.set_xlim(0.0, 10.0).unwrap().set_ylim(0.0, 10.0).unwrap();
    (fig, ax)
}

/// Drags over `ax` from data point `from` to `to` in `mode` with `button`
/// and `key` held, then leaves the mode.
fn drag(
    fig: &Figure,
    ax: &Axes,
    mode: NavMode,
    button: PointerButton,
    key: DragKey,
    (from, to): (Point, Point),
) {
    let (start, end) = (
        display(fig, &ax.trans_data(), from),
        display(fig, &ax.trans_data(), to),
    );
    let mut navigation = Navigation::new();
    navigation.toggle(fig, mode).unwrap();
    navigation.press(fig, start, button);
    navigation.drag(end, key);
    navigation.release(fig, end, key);
    navigation.toggle(fig, mode).unwrap();
}

/// `(x0, y0)` to `(x1, y1)`.
fn path(x0: f64, y0: f64, x1: f64, y1: f64) -> (Point, Point) {
    (Point::new(x0, y0), Point::new(x1, y1))
}

/// The record `ax` keeps of what it was given.
fn record<R>(ax: &Axes, read: impl FnOnce(&Sliding) -> R) -> R {
    ax.with_projection(read).unwrap()
}

#[test]
fn projections_pan_their_axes_themselves() {
    let (fig, ax) = sliding();
    let width = display(&fig, &ax.trans_data(), Point::new(1.0, 0.0)).x
        - display(&fig, &ax.trans_data(), Point::new(0.0, 0.0)).x;
    drag(
        &fig,
        &ax,
        NavMode::Pan,
        PointerButton::Right,
        DragKey::None,
        path(5.0, 5.0, 6.0, 9.0),
    );
    let (x0, x1) = ax.xlim();
    let shift = width / 10.0;
    assert!(
        (x0 + shift).abs() < 1e-9 && (x1 + shift - 10.0).abs() < 1e-9,
        "{x0} {x1}"
    );
    assert_eq!(ax.ylim(), (0.0, 10.0), "y is left alone");
    assert_eq!(record(&ax, |s| (s.start, s.ended)), (None, 1));

    drag(
        &fig,
        &ax,
        NavMode::Pan,
        PointerButton::Left,
        DragKey::None,
        path(5.0, 5.0, 6.0, 7.0),
    );
    let (y0, y1) = ax.ylim();
    assert!(
        (y0 + 2.0).abs() < 1e-9 && (y1 - 8.0).abs() < 1e-9,
        "the left button pans as usual"
    );
    assert_eq!(
        record(&ax, |s| s.ended),
        1,
        "only pans of its own end there"
    );
}

#[test]
fn projections_zoom_their_axes_themselves() {
    let (fig, ax) = sliding();
    drag(
        &fig,
        &ax,
        NavMode::Zoom,
        PointerButton::Left,
        DragKey::None,
        path(6.0, 2.0, 2.0, 8.0),
    );
    let (x0, x1) = ax.xlim();
    assert!((x0 - 2.0).abs() < 1e-9 && (x1 - 6.0).abs() < 1e-9);
    assert_eq!(ax.ylim(), (0.0, 10.0));

    drag(
        &fig,
        &ax,
        NavMode::Zoom,
        PointerButton::Right,
        DragKey::None,
        path(3.0, 3.0, 5.0, 6.0),
    );
    let (y0, y1) = ax.ylim();
    assert!(y0 < 0.0 && y1 > 10.0, "zooming out goes as usual");
    drag(
        &fig,
        &ax,
        NavMode::Zoom,
        PointerButton::Left,
        DragKey::X,
        path(3.0, 4.0, 4.0, 8.0),
    );
    let (x0, x1) = ax.xlim();
    assert!((x0 - 3.0).abs() < 1e-9 && (x1 - 4.0).abs() < 1e-9);
    assert_eq!(ax.ylim(), (y0, y1));

    let mut navigation = Navigation::new();
    navigation.wheel(
        &fig,
        display(&fig, &ax.trans_data(), Point::new(3.5, 4.0)),
        1.0,
    );
    let zooms = record(&ax, |s| s.zooms.clone());
    assert_eq!(
        zooms,
        [(false, true), (false, true), (false, false), (true, true)]
    );
}
