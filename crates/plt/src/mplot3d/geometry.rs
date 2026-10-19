//! Where the panes, gridlines and axis lines of a 3-D axes go: the
//! projected box and the edges facing the camera (matplotlib's
//! `_get_coord_info`, `_calc_centers_deltas`, `_get_axis_line_edge_points`
//! and `_get_tickdir`).

use super::camera::{Camera, Projection};
use super::coords::{Axis3, roll};
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

    /// The end points of the line of `axis` (matplotlib's
    /// `_get_axis_line_edge_points` at the default position).
    pub(crate) fn axis_line(&self, axis: Axis3, vertical: Axis3) -> [Vec3; 2] {
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
        let mut p0 = start;
        p0[juggled[0]] = end[juggled[0]];
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

/// The order in which the coordinates of an axis line are taken from the
/// near corner (matplotlib's `_AXINFO[...]['juggled']`).
pub(crate) const fn juggled(axis: Axis3) -> [usize; 3] {
    match axis {
        Axis3::X => [1, 0, 2],
        Axis3::Y => [0, 1, 2],
        Axis3::Z => [0, 2, 1],
    }
}

/// The coordinate tick marks of `axis` extend along at the default
/// position (matplotlib's `_get_tickdir('default')`).
pub(crate) fn tick_direction(axis: Axis3, camera: &Camera) -> usize {
    const BASE: [f64; 3] = [1.0, 0.0, 0.0];
    let shift = camera.vertical.index() as isize - 2;
    let rolled = roll(BASE, shift);
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
            let [p0, p1] = geometry.axis_line(axis, camera.vertical);
            let differing: Vec<usize> = (0..3).filter(|&i| (p0[i] - p1[i]).abs() > 0.0).collect();
            assert_eq!(differing, [axis.index()]);
        }
        // matplotlib's x axis line runs along y = 0, z = 0.
        assert_eq!(
            geometry.axis_line(Axis3::X, camera.vertical),
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]
        );
    }

    #[test]
    fn tick_directions_match_matplotlib() {
        let (_, camera) = default_box();
        let dirs: Vec<usize> = Axis3::ALL
            .iter()
            .map(|&a| tick_direction(a, &camera))
            .collect();
        assert_eq!(dirs, [1, 0, 0]);
    }

    #[test]
    fn moving_from_the_centre_skips_the_axis() {
        let (geometry, _) = default_box();
        let moved = geometry.move_from_center([0.0, 1.0, 0.5], [0.1, 0.1, 0.1], Axis3::X);
        assert_eq!(moved, [0.0, 1.1, 0.6]);
    }
}
