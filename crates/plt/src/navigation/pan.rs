//! Panning (matplotlib's `Axes.start_pan`/`drag_pan` and the polar
//! overrides).

use plt_core::{Bbox, Point};

use super::{DragKey, PointerButton};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxesState, AxisName};
use crate::colorbar::Colorbar;
use crate::figure::FigureState;
use crate::polar::PolarAxes;
use crate::rc::Aspect;
use crate::transforms::{ResolvedTransform, TransformContext};

/// The state of an axes when a pan started (matplotlib's `_pan_start`).
#[derive(Debug)]
pub(super) enum PanStart {
    /// Rectilinear axes: the view moves with the pointer (left button) or
    /// scales around the press point (right button).
    Cartesian {
        /// The view limits.
        lim: Bbox,
        /// Data → display.
        trans: ResolvedTransform,
        /// Display → data.
        inverse: ResolvedTransform,
        /// The axes box in display coordinates.
        bbox: Bbox,
        /// Where the button went down.
        start: Point,
    },
    /// Polar axes: dragging near the radial labels turns them, the right
    /// button scales the radius.
    Polar {
        /// What the drag does.
        mode: PolarPan,
        /// The outer radius.
        rmax: f64,
        /// The radial label angle in degrees.
        label_angle: f64,
        /// Display → data.
        inverse: ResolvedTransform,
        /// Where the button went down.
        start: Point,
    },
}

/// What dragging polar axes does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PolarPan {
    /// Nothing (left button away from the labels).
    Idle,
    /// Turn the radial labels.
    DragLabels,
    /// Scale the outer radius.
    Zoom,
}

/// The display box and data transforms of `ax`, brought up to date.
fn geometry(ax: &Axes) -> Option<(Bbox, ResolvedTransform, ResolvedTransform)> {
    let figure = ax.raw().figure_state();
    let mut state = figure.borrow_mut();
    state.prepare();
    let state: &FigureState = &state;
    let bbox = state.axes_bbox(ax.id())?;
    let trans = state.data_transform(ax.id())?;
    let inverse = trans.inverted()?;
    Some((bbox, trans, inverse))
}

/// `b` mapped through `t` the way matplotlib's `Bbox.transformed` does:
/// from its lower-left, upper-left and lower-right corners.
pub(super) fn bbox_through(b: &Bbox, t: &ResolvedTransform) -> Bbox {
    let low_left = t.transform_point(Point::new(b.x0(), b.y0()));
    let up_left = t.transform_point(Point::new(b.x0(), b.y1()));
    let low_right = t.transform_point(Point::new(b.x1(), b.y0()));
    Bbox::from_extents(low_left.x, low_left.y, low_right.x, up_left.y)
}

/// matplotlib's `format_deltas`: the pointer motion under a held key.
fn constrain(key: DragKey, dx: f64, dy: f64) -> (f64, f64) {
    match key {
        DragKey::Control if dx.abs() > dy.abs() => (dx, dx),
        DragKey::Control => (dy, dy),
        DragKey::X => (dx, 0.0),
        DragKey::Y => (0.0, dy),
        DragKey::Shift if 2.0 * dx.abs() < dy.abs() => (0.0, dy),
        DragKey::Shift if 2.0 * dy.abs() < dx.abs() => (dx, 0.0),
        DragKey::Shift if dx.abs() > dy.abs() => (dx, dy / dy.abs() * dx.abs()),
        DragKey::Shift => (dx / dx.abs() * dy.abs(), dy),
        DragKey::None => (dx, dy),
    }
}

impl PanStart {
    /// The state of `ax` for a pan starting at display point `at`
    /// (`None` if the axes is no longer laid out).
    pub(super) fn new(ax: &Axes, at: Point, button: PointerButton) -> Option<Self> {
        let (bbox, trans, inverse) = geometry(ax)?;
        if let Some(polar) = ax.as_polar() {
            let label_angle = polar.rlabel_position();
            let angle = label_angle.to_radians();
            let theta = inverse.transform_point(at).x;
            let epsilon = std::f64::consts::PI / 45.0;
            let mode = match button {
                PointerButton::Left if (angle - epsilon..=angle + epsilon).contains(&theta) => {
                    PolarPan::DragLabels
                }
                PointerButton::Right => PolarPan::Zoom,
                _ => PolarPan::Idle,
            };
            return Some(Self::Polar {
                mode,
                rmax: polar.rmax(),
                label_angle,
                inverse,
                start: at,
            });
        }
        let lim = ax.raw().with::<AxesState, _>(|state| state.view_lim);
        Some(Self::Cartesian {
            lim,
            trans,
            inverse,
            bbox,
            start: at,
        })
    }

    /// The pointer moved to `at` with `key` held while `button` is down.
    /// A colorbar takes the limits along its colour scale as its norm's
    /// (matplotlib's `Colorbar.drag_pan`).
    pub(super) fn drag(&self, ax: &Axes, button: PointerButton, key: DragKey, at: Point) {
        match self {
            Self::Cartesian { .. } => {
                let Some((x, y)) = self.cartesian_points(ax, button, key, at) else {
                    return;
                };
                if let Some(colorbar) = Colorbar::in_axes(ax) {
                    let along = match colorbar.long_axis().name() {
                        AxisName::X => x,
                        AxisName::Y => y,
                    };
                    if let (Some(low), Some(high)) = along {
                        colorbar.set_view((low, high));
                    }
                } else {
                    set_limits(ax, AxisName::X, x);
                    set_limits(ax, AxisName::Y, y);
                }
            }
            Self::Polar {
                mode,
                rmax,
                label_angle,
                inverse,
                start,
            } => {
                let Some(polar) = ax.as_polar() else {
                    return;
                };
                let from = inverse.transform_point(*start);
                let to = inverse.transform_point(at);
                drag_polar(&polar, *mode, (from, to), *rmax, *label_angle);
            }
        }
    }

    /// matplotlib's `_get_pan_points`: the new x and y limits, each end
    /// `None` when it would not be finite.
    #[expect(
        clippy::type_complexity,
        reason = "two (low, high) pairs of optional limits, as matplotlib returns them"
    )]
    fn cartesian_points(
        &self,
        ax: &Axes,
        button: PointerButton,
        key: DragKey,
        at: Point,
    ) -> Option<((Option<f64>, Option<f64>), (Option<f64>, Option<f64>))> {
        let Self::Cartesian {
            lim,
            trans,
            inverse,
            bbox,
            start,
        } = self
        else {
            return None;
        };
        let (dx, dy) = (at.x - start.x, at.y - start.y);
        if dx == 0.0 && dy == 0.0 {
            return None;
        }
        let result = match button {
            PointerButton::Left => {
                let (dx, dy) = constrain(key, dx, dy);
                bbox_through(&bbox.translated(-dx, -dy), inverse)
            }
            PointerButton::Right => {
                let current = geometry(ax).map_or(*bbox, |(bbox, _, _)| bbox);
                let (mut dx, mut dy) =
                    constrain(key, -dx / current.width(), -dy / current.height());
                if ax.aspect() != Aspect::Auto {
                    dx = f64::midpoint(dx, dy);
                    dy = dx;
                }
                let (ax_scale, ay_scale) = (10f64.powf(dx), 10f64.powf(dy));
                let old = bbox_through(lim, trans);
                let scale = |v: f64, origin: f64, alpha: f64| origin + alpha * (v - origin);
                let scaled = Bbox::from_extents(
                    scale(old.x0(), start.x, ax_scale),
                    scale(old.y0(), start.y, ay_scale),
                    scale(old.x1(), start.x, ax_scale),
                    scale(old.y1(), start.y, ay_scale),
                );
                bbox_through(&scaled, inverse)
            }
            PointerButton::Middle => return None,
        };
        let check = bbox_through(&result, trans);
        let keep = |value: f64, mapped: f64| mapped.is_finite().then_some(value);
        Some((
            (keep(result.x0(), check.x0()), keep(result.x1(), check.x1())),
            (keep(result.y0(), check.y0()), keep(result.y1(), check.y1())),
        ))
    }
}

/// `set_xlim`/`set_ylim` with `None` keeping the current end.
fn set_limits(ax: &Axes, name: AxisName, (low, high): (Option<f64>, Option<f64>)) {
    let (current_low, current_high) = match name {
        AxisName::X => ax.xlim(),
        AxisName::Y => ax.ylim(),
    };
    // An end that is not finite (a function scale can map one to a
    // finite position) leaves the axis as it is, where matplotlib's
    // event handler raises.
    let _ = ax.set_limits(
        name,
        low.unwrap_or(current_low),
        high.unwrap_or(current_high),
    );
}

/// matplotlib's `PolarAxes.drag_pan` from data point `from` to `to`.
fn drag_polar(
    polar: &PolarAxes,
    mode: PolarPan,
    (from, to): (Point, Point),
    rmax: f64,
    angle: f64,
) {
    match mode {
        PolarPan::DragLabels => {
            let turn = (from.x - to.x).to_degrees();
            polar.set_rlabel_position(angle - turn);
        }
        PolarPan::Zoom => {
            let scale = to.y / from.y;
            polar.set_rmax(rmax / scale);
        }
        PolarPan::Idle => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    fn display(ax: &Axes, x: f64, y: f64) -> Point {
        let (_, trans, _) = geometry(ax).unwrap();
        trans.transform_point(Point::new(x, y))
    }

    #[test]
    fn left_drag_moves_the_view_with_the_pointer() {
        let ax = Figure::new().subplot();
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let start = display(&ax, 5.0, 5.0);
        let pan = PanStart::new(&ax, start, PointerButton::Left).unwrap();
        let end = display(&ax, 6.0, 7.0);
        pan.drag(&ax, PointerButton::Left, DragKey::None, end);
        let (x0, x1) = ax.xlim();
        let (y0, y1) = ax.ylim();
        assert!((x0 + 1.0).abs() < 1e-9 && (x1 - 9.0).abs() < 1e-9);
        assert!((y0 + 2.0).abs() < 1e-9 && (y1 - 8.0).abs() < 1e-9);
    }

    #[test]
    fn keys_constrain_the_motion() {
        assert_eq!(constrain(DragKey::X, 3.0, 4.0), (3.0, 0.0));
        assert_eq!(constrain(DragKey::Control, 3.0, -4.0), (-4.0, -4.0));
        assert_eq!(constrain(DragKey::Shift, 1.0, 5.0), (0.0, 5.0));
        assert_eq!(constrain(DragKey::Shift, 3.0, -4.0), (4.0, -4.0));
    }

    #[test]
    fn right_drag_scales_around_the_press() {
        let ax = Figure::new().subplot();
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let start = display(&ax, 5.0, 5.0);
        let pan = PanStart::new(&ax, start, PointerButton::Right).unwrap();
        // Dragging right by a whole axes width zooms x in tenfold.
        let width = geometry(&ax).unwrap().0.width();
        pan.drag(
            &ax,
            PointerButton::Right,
            DragKey::None,
            Point::new(start.x + width, start.y),
        );
        let (x0, x1) = ax.xlim();
        assert!((x0 - 4.5).abs() < 1e-9 && (x1 - 5.5).abs() < 1e-9);
        let (y0, y1) = ax.ylim();
        assert!(
            y0.abs() < 1e-9 && (y1 - 10.0).abs() < 1e-9,
            "y is unchanged"
        );
    }

    #[test]
    fn polar_right_drag_scales_the_radius() {
        let fig = Figure::new();
        let polar = fig.polar_subplot();
        polar.set_rlim(0.0, 2.0).expect("finite limits");
        let start = display(&polar, 0.0, 1.0);
        let pan = PanStart::new(&polar, start, PointerButton::Right).unwrap();
        pan.drag(
            &polar,
            PointerButton::Right,
            DragKey::None,
            display(&polar, 0.0, 2.0),
        );
        assert!((polar.rmax() - 1.0).abs() < 1e-9);
    }
}
