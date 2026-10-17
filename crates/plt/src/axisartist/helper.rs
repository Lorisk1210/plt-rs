//! Where an axis artist runs and where its ticks and label go (matplotlib's
//! `FixedAxisArtistHelperRectilinear` and
//! `FloatingAxisArtistHelperRectilinear`).

use plt_core::{Path, Point};

use crate::artist::ArtistId;
use crate::axes::{AxesState, Tick};
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use crate::transforms::Transform;

/// The relative tolerance of matplotlib's `_interval_contains_close`.
const CLOSE: f64 = 1e-10;

/// The line an axis artist draws along.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AxisHelper {
    /// An edge of the axes.
    Fixed(SpineSide),
    /// Where the data coordinate across `nth` equals `value`: `nth = X`
    /// runs horizontally at `y = value`, `nth = Y` vertically at
    /// `x = value`.
    Floating {
        /// The coordinate varying along the line (and ticked).
        nth: AxisName,
        /// The constant other coordinate, in data units.
        value: f64,
    },
}

/// A tick of an axis artist: its location in the tick coordinates, the
/// angles normal and tangent to the line, and its label.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TickInfo {
    pub(crate) loc: Point,
    pub(crate) angle_normal: f64,
    pub(crate) angle_tangent: f64,
    pub(crate) label: String,
}

impl AxisHelper {
    /// The coordinate varying along the line.
    pub(crate) fn nth(self) -> AxisName {
        match self {
            Self::Fixed(side) => side.axis(),
            Self::Floating { nth, .. } => nth,
        }
    }

    /// `(value, constant)` as a point, `value` along the line.
    fn to_xy(self, value: f64, constant: f64) -> Point {
        match self.nth() {
            AxisName::X => Point::new(value, constant),
            AxisName::Y => Point::new(constant, value),
        }
    }

    /// The constant axes coordinate of the line.
    fn position(self, axes: &AxesState) -> f64 {
        match self {
            Self::Fixed(side) => f64::from(u8::from(side.is_high())),
            Self::Floating { nth, value } => data_to_axes(axes, other(nth), value),
        }
    }

    /// The line in axes coordinates (matplotlib's `get_line`).
    pub(crate) fn line(self, axes: &AxesState) -> Path {
        let constant = self.position(axes);
        Path::polyline(vec![self.to_xy(0.0, constant), self.to_xy(1.0, constant)])
    }

    /// The coordinates of tick locations (matplotlib's
    /// `get_tick_transform`): data along the line and axes across it for
    /// edges, data for floating lines.
    pub(crate) fn tick_transform(self, axes: ArtistId) -> Transform {
        let data = Transform::Data(axes);
        let across = Transform::Axes(axes);
        match self {
            Self::Fixed(side) if side.axis() == AxisName::X => Transform::blended(data, across),
            Self::Fixed(_) => Transform::blended(across, data),
            Self::Floating { .. } => data,
        }
    }

    /// The ticks on the line among `ticks` of the ticked axis (matplotlib's
    /// `get_tick_iterators`).
    pub(crate) fn ticks(self, axes: &AxesState, ticks: &[Tick]) -> Vec<TickInfo> {
        let nth = self.nth();
        let (angle_normal, angle_tangent) = match nth {
            AxisName::X => (90.0, 0.0),
            AxisName::Y => (0.0, 90.0),
        };
        ticks
            .iter()
            .filter_map(|tick| {
                let (loc, shown) = match self {
                    Self::Fixed(side) => {
                        let along = data_to_axes(axes, nth, tick.loc);
                        let constant = f64::from(u8::from(side.is_high()));
                        (self.to_xy(tick.loc, constant), contains_close(along))
                    }
                    Self::Floating { value, .. } => {
                        let loc = self.to_xy(tick.loc, value);
                        let c1 = data_to_axes(axes, AxisName::X, loc.x);
                        let c2 = data_to_axes(axes, AxisName::Y, loc.y);
                        (loc, (0.0..=1.0).contains(&c1) && (0.0..=1.0).contains(&c2))
                    }
                };
                shown.then(|| TickInfo {
                    loc,
                    angle_normal,
                    angle_tangent,
                    label: tick.label.clone(),
                })
            })
            .collect()
    }

    /// Where the axis label goes in axes coordinates and the direction of
    /// the line there (matplotlib's `get_axislabel_pos_angle`); `None` for
    /// a floating line outside the axes.
    pub(crate) fn label_position(self, axes: &AxesState) -> Option<(Point, f64)> {
        match self {
            Self::Fixed(side) => Some(match side {
                SpineSide::Left => (Point::new(0.0, 0.5), 90.0),
                SpineSide::Right => (Point::new(1.0, 0.5), 90.0),
                SpineSide::Bottom => (Point::new(0.5, 0.0), 0.0),
                SpineSide::Top => (Point::new(0.5, 1.0), 0.0),
            }),
            Self::Floating { nth, .. } => {
                let constant = self.position(axes);
                let angle = match nth {
                    AxisName::X => 0.0,
                    AxisName::Y => 90.0,
                };
                (0.0..=1.0)
                    .contains(&constant)
                    .then(|| (self.to_xy(0.5, constant), angle))
            }
        }
    }
}

/// The other coordinate.
fn other(name: AxisName) -> AxisName {
    match name {
        AxisName::X => AxisName::Y,
        AxisName::Y => AxisName::X,
    }
}

/// Data value `v` of axis `name` in axes coordinates, computed like
/// matplotlib's `transData - transAxes` (the scale, then `transLimits`).
pub(crate) fn data_to_axes(axes: &AxesState, name: AxisName, v: f64) -> f64 {
    let axis = axes.axis(name);
    let (v0, v1) = axes.view_interval(name);
    let (low, high) = (axis.forward(v0), axis.forward(v1));
    let scale = 1.0 / (high - low);
    axis.forward(v) * scale + (-low * scale)
}

/// Whether an axes coordinate lies in `[0, 1]` up to a relative tolerance
/// (matplotlib's `_interval_contains_close((0, 1), value)`).
fn contains_close(value: f64) -> bool {
    (-CLOSE..=1.0 + CLOSE).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    fn ticks(locs: &[f64]) -> Vec<Tick> {
        locs.iter()
            .map(|&loc| Tick {
                loc,
                label: format!("{loc}"),
            })
            .collect()
    }

    #[test]
    fn floating_lines_sit_at_their_value() {
        let fig = Figure::new();
        let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
        ax.set_xlim(-2.0, 2.0).set_ylim(-3.0, 3.0);
        let state = fig.raw().figure_state().borrow();
        let axes = state.store.get_as::<AxesState>(ax.id()).unwrap();
        let along_x = AxisHelper::Floating {
            nth: AxisName::X,
            value: 1.5,
        };
        let line = along_x.line(axes);
        assert_eq!(
            line.vertices(),
            &[Point::new(0.0, 0.75), Point::new(1.0, 0.75)]
        );
        let shown = along_x.ticks(axes, &ticks(&[-3.0, -2.0, 0.0, 2.0, 2.5]));
        let locs: Vec<f64> = shown.iter().map(|t| t.loc.x).collect();
        assert_eq!(locs, vec![-2.0, 0.0, 2.0]);
        assert_eq!(shown[0].loc.y, 1.5);
        let outside = AxisHelper::Floating {
            nth: AxisName::Y,
            value: 5.0,
        };
        assert!(outside.label_position(axes).is_none());
        assert_eq!(outside.ticks(axes, &ticks(&[0.0])), Vec::<TickInfo>::new());
    }

    #[test]
    fn fixed_lines_follow_the_edges() {
        let fig = Figure::new();
        let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
        ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
        let state = fig.raw().figure_state().borrow();
        let axes = state.store.get_as::<AxesState>(ax.id()).unwrap();
        let right = AxisHelper::Fixed(SpineSide::Right);
        assert_eq!(right.nth(), AxisName::Y);
        assert_eq!(right.line(axes).vertices()[1], Point::new(1.0, 1.0));
        let shown = right.ticks(axes, &ticks(&[0.0, 1.0 + 1e-12, 1.5]));
        assert_eq!(shown.len(), 2, "within the tolerance of the edge");
        assert_eq!((shown[0].angle_normal, shown[0].angle_tangent), (0.0, 90.0));
    }
}
