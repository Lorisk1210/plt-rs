//! Navigating 3-D axes: rotating, panning and zooming with the mouse
//! outside the toolbar modes, panning and zooming to a rectangle in them,
//! and the status-bar text (matplotlib's `Axes3D._button_press`,
//! `_on_move`, `drag_pan`, `_set_view_from_bbox` and `format_coord`).

use plt_core::Point;

use super::readout::{contains, data_transform, display_bbox};
use super::{DragKey, PointerButton};
use crate::artist::ArtistHandle;
use crate::figure::Figure;
use crate::mplot3d::{Axes3D, Axis3};
use crate::mplot3d::{MouseAction, location_coords};

/// The topmost visible 3-D axes under display point `at`.
pub(super) fn axes3d_at(figure: &Figure, at: Point) -> Option<Axes3D> {
    figure
        .axes3d()
        .into_iter()
        .filter(|ax| ax.visible() && contains(&ax.plane(), at))
        .fold(None, |best: Option<Axes3D>, ax| match best {
            Some(best) if best.zorder() >= ax.zorder() => Some(best),
            _ => Some(ax),
        })
}

/// Display point `at` in the projected coordinates of `ax` (matplotlib's
/// `event.xdata` and `event.ydata` over a 3-D axes).
fn projected(ax: &Axes3D, at: Point) -> Option<Point> {
    let inverse = data_transform(&ax.plane())?.inverted()?;
    Some(inverse.transform_point(at))
}

/// A drag on a 3-D axes in progress.
#[derive(Debug)]
pub(super) struct Orbit {
    axes: Axes3D,
    action: MouseAction,
    /// Whether the `x` and `y` keys constrain panning (the toolbar's pan
    /// mode, not the middle button).
    constrained: bool,
    /// The pointer at the last step, in projected coordinates.
    last: Point,
}

impl Orbit {
    /// A drag with `button` from display point `at` outside the toolbar
    /// modes: whatever the axes' mouse buttons say (matplotlib's
    /// `_button_press`).
    pub(super) fn start(figure: &Figure, at: Point, button: PointerButton) -> Option<Self> {
        let axes = axes3d_at(figure, at)?;
        let action = axes.mouse_buttons().action(button)?;
        let last = projected(&axes, at)?;
        Some(Self {
            axes,
            action,
            constrained: false,
            last,
        })
    }

    /// A drag in the toolbar's pan mode, which pans 3-D axes with any
    /// button (matplotlib's `Axes3D.drag_pan`).
    pub(super) fn pan(figure: &Figure, at: Point) -> Option<Self> {
        let axes = axes3d_at(figure, at)?;
        let last = projected(&axes, at)?;
        Some(Self {
            axes,
            action: MouseAction::Pan,
            constrained: true,
            last,
        })
    }

    /// Whether the drag turns the view.
    pub(super) fn is_rotating(&self) -> bool {
        self.action == MouseAction::Rotate
    }

    /// The axes being dragged.
    pub(super) fn axes(&self) -> &Axes3D {
        &self.axes
    }

    /// The pointer moved to display point `at` with `key` held; motion
    /// outside the axes is ignored, as in matplotlib (`_on_move`).
    pub(super) fn drag(&mut self, at: Point, key: DragKey) {
        if !contains(&self.axes.plane(), at) {
            return;
        }
        let Some(point) = projected(&self.axes, at) else {
            return;
        };
        let (du, dv) = (point.x - self.last.x, point.y - self.last.y);
        // Limits only fail to update for degenerate drags, which leave the
        // view as it was.
        let _ = match self.action {
            MouseAction::Rotate => {
                self.axes
                    .drag_rotate(self.last, point, key == DragKey::Control);
                Ok(&self.axes)
            }
            MouseAction::Pan => {
                let (du, dv) = match (self.constrained, key) {
                    (true, DragKey::X) => (du, 0.0),
                    (true, DragKey::Y) => (0.0, dv),
                    _ => (du, dv),
                };
                self.axes.pan_view(du, dv)
            }
            MouseAction::Zoom => self.axes.drag_zoom(self.last.y, point.y),
        };
        self.last = point;
    }
}

/// Zooms `ax` into (or, with `out`, out of) the display rectangle from
/// `start` to `end`: the rectangle's centre moves to the axes' centre and
/// the limits scale by the larger of its relative sides (matplotlib's
/// `_set_view_from_bbox`); `x` and `y` keep the full height or width.
pub(super) fn zoom_to_rect(ax: &Axes3D, start: Point, end: Point, out: bool, key: DragKey) {
    let Some(bbox) = display_bbox(&ax.plane()) else {
        return;
    };
    let (mut x, mut y) = ((start.x, end.x), (start.y, end.y));
    match key {
        DragKey::X => y = (bbox.y0(), bbox.y1()),
        DragKey::Y => x = (bbox.x0(), bbox.x1()),
        _ => {}
    }
    let clip =
        |(a, b): (f64, f64), lo: f64, hi: f64| (a.min(b).clamp(lo, hi), a.max(b).clamp(lo, hi));
    let (x0, x1) = clip(x, bbox.x0(), bbox.x1());
    let (y0, y1) = clip(y, bbox.y0(), bbox.y1());
    let zoom_center = Point::new(f64::midpoint(x0, x1), f64::midpoint(y0, y1));
    let ax_center = Point::new(
        f64::midpoint(bbox.x1(), bbox.x0()),
        f64::midpoint(bbox.y1(), bbox.y0()),
    );
    let (Some(from), Some(to)) = (projected(ax, zoom_center), projected(ax, ax_center)) else {
        return;
    };
    // Limits only fail to update for degenerate rectangles, which leave
    // the view as it was.
    let _ = ax.pan_view(to.x - from.x, to.y - from.y);
    let scale_u = (x0 - x1).abs() / (bbox.x1() - bbox.x0());
    let scale_v = (y0 - y1).abs() / (bbox.y1() - bbox.y0());
    let scale = scale_u.max(scale_v);
    let scale = if out { 1.0 / scale } else { scale };
    let _ = ax.zoom_data_limits(scale, scale, scale);
}

/// The status-bar text over `ax` at display point `at`: the view angles
/// while rotating, otherwise the point of the nearest pane under the
/// pointer (matplotlib's `Axes3D.format_coord`).
pub(super) fn readout(ax: &Axes3D, at: Point, rotating: bool) -> String {
    if rotating {
        return ax.rotation_coords();
    }
    pane_readout(ax, at).unwrap_or_default()
}

fn pane_readout(ax: &Axes3D, at: Point) -> Option<String> {
    let trans = data_transform(&ax.plane())?;
    let inverse = trans.inverted()?;
    let projected = inverse.transform_point(at);
    let (point, pane) = ax.pane_point(projected.x, projected.y)?;
    // Every 3-D axis takes the precision of a screen pixel along the
    // plane's x axis, as matplotlib's formatter sees them all as x axes.
    let resolution = |value: f64| {
        let screen = trans.transform_point(Point::new(value, 0.0));
        [-1.0, 1.0]
            .into_iter()
            .map(|dx| {
                (inverse
                    .transform_point(Point::new(screen.x + dx, screen.y))
                    .x
                    - value)
                    .abs()
            })
            .fold(0.0, f64::max)
    };
    let formatted = Axis3::ALL.map(|axis| {
        let value = point[axis.index()];
        ax.format_coordinate(axis, value, resolution(value))
    });
    let [Some(x), Some(y), Some(z)] = formatted else {
        return None;
    };
    Some(location_coords(pane, [x, y, z]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A figure with a 3-D axes, laid out, and the display centre of the
    /// axes.
    fn scene() -> (Figure, Axes3D, Point) {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.subplot_3d();
        ax.set_xlim(0.0, 10.0).unwrap();
        fig.layout();
        let bbox = display_bbox(&ax.plane()).unwrap();
        let center = Point::new(
            f64::midpoint(bbox.x0(), bbox.x1()),
            f64::midpoint(bbox.y0(), bbox.y1()),
        );
        (fig, ax, center)
    }

    #[test]
    fn dragging_the_left_button_rotates() {
        let (fig, ax, center) = scene();
        let mut orbit = Orbit::start(&fig, center, PointerButton::Left).unwrap();
        assert!(orbit.is_rotating());
        orbit.drag(Point::new(center.x + 20.0, center.y), DragKey::None);
        assert_ne!(ax.view().azim, -60.0);
        assert!(readout(&ax, center, true).starts_with("elevation="));
        ax.disable_mouse_rotation();
        assert!(Orbit::start(&fig, center, PointerButton::Left).is_none());
    }

    #[test]
    fn the_right_button_zooms_and_the_middle_pans() {
        let (fig, ax, center) = scene();
        let mut zoom = Orbit::start(&fig, center, PointerButton::Right).unwrap();
        zoom.drag(Point::new(center.x, center.y - 30.0), DragKey::None);
        let (lo, hi) = ax.xlim();
        assert!(hi - lo < 10.0, "dragging down zooms in: {lo}..{hi}");
        let before = ax.ylim();
        let mut pan = Orbit::start(&fig, center, PointerButton::Middle).unwrap();
        pan.drag(Point::new(center.x + 15.0, center.y), DragKey::None);
        assert_ne!(ax.ylim(), before);
    }

    #[test]
    fn rectangles_zoom_and_the_readout_names_a_pane() {
        let (_fig, ax, center) = scene();
        let start = Point::new(center.x - 20.0, center.y - 20.0);
        zoom_to_rect(
            &ax,
            start,
            Point::new(center.x + 20.0, center.y + 20.0),
            false,
            DragKey::None,
        );
        let (lo, hi) = ax.xlim();
        assert!(hi - lo < 5.0, "zoomed in: {lo}..{hi}");
        assert!(readout(&ax, center, false).contains("pane="));
    }
}
