//! Where the panes, gridlines and axis lines of a 3-D axes go: the
//! projected box and the edges facing the camera (matplotlib's
//! `_get_coord_info`, `_calc_centers_deltas`, `_get_axis_line_edge_points`,
//! `_get_tickdir` and `get_axis_position`).

use super::axis_handle::Axis3D;
use super::camera::{Camera, Projection};
use super::coords::{Axis3, roll};
use super::handle::Axes3D;
use super::math::Vec3;

/// The box corners making up the two planes normal to x, y and z (low
/// then high side), as indices into [`BoxGeometry::corners`].
pub(crate) const PLANES: [[usize; 4]; 6] = [
    [0, 3, 7, 4],
    [1, 2, 6, 5],
    [0, 1, 5, 4],
    [3, 2, 6, 7],
    [0, 1, 2, 3],
    [4, 5, 6, 7],
];

/// Fraction of the box size between an axis line and its tick labels'
/// reference (matplotlib 3.9's `1/12 * 24/25`).
const DELTA_SCALE: f64 = 0.08;

/// Which of the box edges along an axis carry its line, tick marks, tick
/// labels and offset text, or its label (matplotlib's
/// `Axis.set_ticks_position` and `set_label_position` for 3-D axes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AxisPosition {
    /// The edge matplotlib picks for the view.
    #[default]
    Default,
    /// The lower of the two edges along the axis next to the panes.
    Lower,
    /// The upper of those edges.
    Upper,
    /// Both of them.
    Both,
    /// No edge: those parts are not drawn.
    None,
}

pltrs_core::archive_enum!(AxisPosition {
    Default,
    Lower,
    Upper,
    Both,
    None,
});

impl AxisPosition {
    /// The edges drawn, each named by its own position (matplotlib's
    /// `_get_all_axis_line_edge_points`).
    pub(crate) const fn edges(self) -> &'static [Self] {
        match self {
            Self::Default => &[Self::Default],
            Self::Lower => &[Self::Lower],
            Self::Upper => &[Self::Upper],
            Self::Both => &[Self::Lower, Self::Upper],
            Self::None => &[],
        }
    }
}

/// The projected data box of one draw, in transformed coordinates (the
/// axis scales applied).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BoxGeometry {
    /// The low corner of the box.
    pub(crate) mins: Vec3,
    /// The high corner of the box.
    pub(crate) maxs: Vec3,
    /// The eight corners, projected.
    pub(crate) corners: [Vec3; 8],
    /// For each axis, whether its high plane is the far one.
    pub(crate) highs: [bool; 3],
    /// The centre of the box.
    pub(crate) centers: Vec3,
    /// Offsets used to move ticks and labels away from the box.
    pub(crate) deltas: Vec3,
}

impl BoxGeometry {
    /// The box of the limits `[xmin, xmax, ymin, ymax, zmin, zmax]` in
    /// transformed coordinates (possibly inverted) seen through
    /// `projection` (matplotlib's `_get_coord_info`).
    pub(crate) fn new(scaled: [f64; 6], projection: &Projection) -> Self {
        let bounds = [0, 1, 2].map(|i| {
            let (a, b) = (scaled[2 * i], scaled[2 * i + 1]);
            (a.min(b), a.max(b))
        });
        let mins = bounds.map(|b| b.0);
        let maxs = bounds.map(|b| b.1);
        let [(x0, x1), (y0, y1), (z0, z1)] = bounds;
        // Projected together, as matplotlib's `_transformed_cube`.
        let projected = projection.project_transformed_all(&[
            [x0, y0, z0],
            [x1, y0, z0],
            [x1, y1, z0],
            [x0, y1, z0],
            [x0, y0, z1],
            [x1, y0, z1],
            [x1, y1, z1],
            [x0, y1, z1],
        ]);
        let corners: [Vec3; 8] = std::array::from_fn(|k| projected[k]);
        let mean_depth = |plane: usize| {
            PLANES[plane]
                .iter()
                .fold(0.0, |sum, &k| sum + corners[k][2])
                / 4.0
        };
        let low = [0, 1, 2].map(|i| mean_depth(2 * i));
        let high = [0, 1, 2].map(|i| mean_depth(2 * i + 1));
        let mut highs = [0, 1, 2].map(|i| low[i] < high[i]);
        // Edge-on views: keep the panes matplotlib keeps.
        let equal = [0, 1, 2].map(|i| (low[i] - high[i]).abs() <= f64::EPSILON);
        if equal.iter().filter(|&&e| e).count() == 2 {
            match equal.iter().position(|e| !e) {
                Some(2) => highs = [true, true, highs[2]],
                Some(1) => highs = [true, highs[1], false],
                Some(0) => highs = [highs[0], false, false],
                _ => {}
            }
        }
        let centers = [0, 1, 2].map(|i| f64::midpoint(maxs[i], mins[i]));
        let deltas = [0, 1, 2].map(|i| (maxs[i] - mins[i]) * DELTA_SCALE);
        Self {
            mins,
            maxs,
            corners,
            highs,
            centers,
            deltas,
        }
    }

    /// The box corner the axis lines start from: on each axis the side
    /// of the far plane (matplotlib's `minmax`).
    pub(crate) fn far_corner(&self) -> Vec3 {
        [0, 1, 2].map(|i| {
            if self.highs[i] {
                self.maxs[i]
            } else {
                self.mins[i]
            }
        })
    }

    /// The opposite corner, nearest to the camera (matplotlib's `maxmin`).
    pub(crate) fn near_corner(&self) -> Vec3 {
        [0, 1, 2].map(|i| {
            if self.highs[i] {
                self.mins[i]
            } else {
                self.maxs[i]
            }
        })
    }

    /// The projected corners of the pane of `axis`: its far plane.
    pub(crate) fn pane(&self, axis: Axis3) -> [Vec3; 4] {
        let i = axis.index();
        let plane = if self.highs[i] { 2 * i + 1 } else { 2 * i };
        PLANES[plane].map(|k| self.corners[k])
    }

    /// The end points of the line of `axis` along the edge `position`
    /// ([`AxisPosition::Default`], [`AxisPosition::Lower`] or
    /// [`AxisPosition::Upper`]; matplotlib's `_get_axis_line_edge_points`).
    pub(crate) fn axis_line(
        &self,
        axis: Axis3,
        vertical: Axis3,
        position: AxisPosition,
    ) -> [Vec3; 2] {
        let (minmax, maxmin) = (self.far_corner(), self.near_corner());
        let forward = [minmax, maxmin];
        let backward = [maxmin, minmax];
        let table = [
            [forward, backward, backward],
            [backward, backward, forward],
            [forward, forward, forward],
        ];
        let [start, end] = table[vertical.index()][axis.index()];
        let juggled = juggled(axis);
        let last = juggled[2];
        let across_last = match position {
            AxisPosition::Lower => end[last] < start[last],
            AxisPosition::Upper => end[last] > start[last],
            _ => false,
        };
        let mut p0 = start;
        if across_last {
            p0[last] = end[last];
        } else {
            p0[juggled[0]] = end[juggled[0]];
        }
        let mut p1 = p0;
        p1[juggled[1]] = end[juggled[1]];
        [p0, p1]
    }

    /// `p` moved away from the centre of the box by `deltas` along every
    /// axis but `axis` (matplotlib's `_move_from_center` with `_axmask`).
    pub(crate) fn move_from_center(&self, p: Vec3, deltas: Vec3, axis: Axis3) -> Vec3 {
        [0, 1, 2].map(|i| {
            if i == axis.index() {
                p[i]
            } else {
                p[i] + 1.0_f64.copysign(p[i] - self.centers[i]) * deltas[i]
            }
        })
    }
}

impl Axes3D {
    /// matplotlib's `get_axis_position` in the current view: with `(x0,
    /// x1)`, `(y0, y1)` and `(z0, z1)` the view limits (scaled, in their
    /// order) and the depth of a corner its projected z (larger is
    /// farther from the camera), whether the corners `(x1, y0, z0)`, `(x0,
    /// y1, z0)` and `(x0, y0, z0)` each lie deeper than `(x1, y1, z0)`.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// assert_eq!(ax.axis_highs(), [false, true, false]);
    /// ax.view_init(-30.0, 120.0, 0.0);
    /// assert_eq!(ax.axis_highs(), [true, false, true]);
    /// ```
    pub fn axis_highs(&self) -> [bool; 3] {
        self.with(|axes| {
            let [x0, x1, y0, y1, z0, _] = axes.scaled_limits();
            let corners = axes.projection().project_transformed_all(&[
                [x0, y0, z0],
                [x1, y0, z0],
                [x1, y1, z0],
                [x0, y1, z0],
            ]);
            let deeper = |k: usize| corners[k][2] > corners[2][2];
            [deeper(1), deeper(3), deeper(0)]
        })
    }
}

impl Axis3D {
    /// The end points of the axis line in projected coordinates, as their
    /// x and y coordinates, along the last edge the tick position names in
    /// the current view (matplotlib's `axis.line.get_data()` once drawn);
    /// `None` when the tick position is `none`.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.view_init(0.0, 0.0, 0.0);
    /// let (xs, ys) = ax.zaxis().line_data().expect("a line");
    /// // matplotlib: ([-0.06329114, -0.06329114], [-0.04746835, 0.04746835]).
    /// assert!((xs[0] + 0.063_291_14).abs() < 1e-8 && (xs[1] - xs[0]).abs() < 1e-12);
    /// assert!((ys[0] + 0.047_468_35).abs() < 1e-8 && (ys[1] - 0.047_468_35).abs() < 1e-8);
    /// ```
    pub fn line_data(&self) -> Option<([f64; 2], [f64; 2])> {
        self.axes.with(|axes| {
            let position = *axes.axis(self.dir).tick_position.edges().last()?;
            let projection = axes.projection();
            let geometry = BoxGeometry::new(axes.scaled_limits(), &projection);
            let ends = geometry.axis_line(self.dir, axes.camera.vertical, position);
            let q = projection.project_transformed_all(&ends);
            Some(([q[0][0], q[1][0]], [q[0][1], q[1][1]]))
        })
    }

    /// The pane of the axis the camera sees, the side of the box farthest
    /// from it along the axis: its four corners projected (x and y in the
    /// projection plane, then depth), and where it lies along the axis in
    /// scaled coordinates (matplotlib's `active_pane`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let (corners, at) = ax.zaxis().active_pane();
    /// assert_eq!(corners.len(), 4);
    /// assert_eq!(at, ax.zlim().0);
    /// ```
    pub fn active_pane(&self) -> ([Vec3; 4], f64) {
        self.axes.with(|axes| {
            let projection = axes.projection();
            let geometry = BoxGeometry::new(axes.scaled_limits(), &projection);
            let i = self.dir.index();
            let corners = geometry.pane(self.dir);
            let at = if geometry.highs[i] {
                geometry.maxs[i]
            } else {
                geometry.mins[i]
            };
            (corners, at)
        })
    }

    /// The coordinate (0 for x, 1 for y, 2 for z) the tick marks extend
    /// along on the edge `position` in the current view (matplotlib's
    /// `_get_tickdir`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::AxisPosition;
    /// let ax = Figure::new().subplot_3d();
    /// assert_eq!(ax.xaxis().tick_direction(AxisPosition::Default), 1);
    /// ```
    pub fn tick_direction(&self, position: AxisPosition) -> usize {
        self.axes
            .with(|axes| tick_direction(self.dir, &axes.camera, position))
    }
}

/// The order in which the coordinates of an axis line are taken from the
/// near corner (matplotlib's `_AXINFO[...]['juggled']`).
pub(crate) const fn juggled(axis: Axis3) -> [usize; 3] {
    match axis {
        Axis3::X => [1, 0, 2],
        Axis3::Y => [0, 1, 2],
        Axis3::Z => [0, 2, 1],
    }
}

/// The coordinate the tick marks of `axis` extend along on the edge
/// `position` ([`AxisPosition::Default`], [`AxisPosition::Lower`] or
/// [`AxisPosition::Upper`]; matplotlib's `_get_tickdir`).
pub(crate) fn tick_direction(axis: Axis3, camera: &Camera, position: AxisPosition) -> usize {
    let looking_down = (camera.angles.elev + 180.0).rem_euclid(360.0) - 180.0 >= 0.0;
    let azim_below_180 = camera.angles.azim.rem_euclid(360.0) < 180.0;
    let base = match position {
        AxisPosition::Upper => {
            let [x, y, _] = if looking_down {
                [2.0, 2.0, 0.0]
            } else {
                [1.0, 0.0, 0.0]
            };
            [x, y, if azim_below_180 { 1.0 } else { 0.0 }]
        }
        AxisPosition::Lower => {
            let [x, y, z] = if looking_down {
                [1.0, 0.0, 1.0]
            } else {
                [2.0, 2.0, 1.0]
            };
            [x, y, if azim_below_180 { 0.0 } else { z }]
        }
        _ => [1.0, 0.0, 0.0],
    };
    let shift = camera.vertical.index() as isize - 2;
    let rolled = roll(base, shift);
    let base = rolled[axis.index()] as isize;
    (base + shift).rem_euclid(3) as usize
}

/// The coordinate offset texts of `axis` are aligned against
/// (matplotlib's `_AXINFO[...]['tickdir']`).
pub(crate) const fn static_tick_direction(axis: Axis3) -> usize {
    match axis {
        Axis3::X => 1,
        Axis3::Y | Axis3::Z => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mplot3d::camera::ProjType;

    fn default_box() -> (BoxGeometry, Camera) {
        let camera = Camera::new(ProjType::default()).expect("valid camera");
        let limits = [0.0, 1.0, 0.0, 1.0, 0.0, 1.0];
        let projection = camera.projection(limits);
        (BoxGeometry::new(limits, &projection), camera)
    }

    #[test]
    fn default_view_shows_the_back_panes() {
        let (geometry, _) = default_box();
        // matplotlib: highs = [False, True, False] for elev=30, azim=-60.
        assert_eq!(geometry.highs, [false, true, false]);
        assert_eq!(geometry.far_corner(), [0.0, 1.0, 0.0]);
        assert_eq!(geometry.near_corner(), [1.0, 0.0, 1.0]);
    }

    #[test]
    fn axis_lines_run_along_their_axis() {
        let (geometry, camera) = default_box();
        for axis in Axis3::ALL {
            let [p0, p1] = geometry.axis_line(axis, camera.vertical, AxisPosition::Default);
            let differing: Vec<usize> = (0..3).filter(|&i| (p0[i] - p1[i]).abs() > 0.0).collect();
            assert_eq!(differing, [axis.index()]);
        }
        // matplotlib's x axis line runs along y = 0, z = 0.
        assert_eq!(
            geometry.axis_line(Axis3::X, camera.vertical, AxisPosition::Default),
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]
        );
    }

    #[test]
    fn tick_directions_match_matplotlib() {
        let (_, camera) = default_box();
        let dirs: Vec<usize> = Axis3::ALL
            .iter()
            .map(|&a| tick_direction(a, &camera, AxisPosition::Default))
            .collect();
        assert_eq!(dirs, [1, 0, 0]);
    }

    #[test]
    fn moving_from_the_centre_skips_the_axis() {
        let (geometry, _) = default_box();
        let moved = geometry.move_from_center([0.0, 1.0, 0.5], [0.1, 0.1, 0.1], Axis3::X);
        assert_eq!(moved, [0.0, 1.1, 0.6]);
    }

    #[test]
    fn axis_positions_pick_matplotlibs_edges() {
        // matplotlib's _get_axis_line_edge_points and _get_tickdir in the
        // default view.
        let (geometry, camera) = default_box();
        let cases = [
            (
                Axis3::X,
                AxisPosition::Lower,
                [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                1,
            ),
            (
                Axis3::X,
                AxisPosition::Upper,
                [[0.0, 1.0, 1.0], [1.0, 1.0, 1.0]],
                2,
            ),
            (
                Axis3::Y,
                AxisPosition::Upper,
                [[0.0, 1.0, 1.0], [0.0, 0.0, 1.0]],
                2,
            ),
            (
                Axis3::Z,
                AxisPosition::Lower,
                [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
                1,
            ),
            (
                Axis3::Z,
                AxisPosition::Upper,
                [[1.0, 1.0, 0.0], [1.0, 1.0, 1.0]],
                0,
            ),
        ];
        for (axis, position, edge, tickdir) in cases {
            let line = geometry.axis_line(axis, camera.vertical, position);
            assert_eq!(line, edge, "{axis:?} {position:?}");
            let direction = tick_direction(axis, &camera, position);
            assert_eq!(direction, tickdir, "{axis:?} {position:?}");
        }
        assert_eq!(AxisPosition::None.edges(), []);
        assert_eq!(
            AxisPosition::Both.edges(),
            [AxisPosition::Lower, AxisPosition::Upper]
        );
    }
}
