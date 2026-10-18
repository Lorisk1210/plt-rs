//! What the status bar shows over a 3-D axes (matplotlib's
//! `Axes3D.format_coord`): the view angles while rotating, otherwise the
//! point of the nearest pane under the pointer.

use std::any::Any;

use super::camera::{DIST, norm_angle};
use super::coords::Axis3;
use super::geometry::BoxGeometry;
use super::handle::Axes3D;
use super::math::Vec3;
use crate::ticker::{AxisInfo, ScalarFormatter, TickContext};

/// The focal length standing in for an orthographic camera's infinite one
/// when placing the camera (matplotlib's `_get_camera_loc`).
const ORTHO_FOCAL_LENGTH: f64 = 1e9;

impl Axes3D {
    /// The view angles as shown while rotating, each normalised to
    /// `(-180, 180]` and rounded to a degree with a true minus sign
    /// (matplotlib's `_rotation_coords`).
    pub fn rotation_coords(&self) -> String {
        let view = self.view();
        let angle = |a: f64| format!("{:.0}", norm_angle(a)).replace('-', "\u{2212}");
        format!(
            "elevation={}°, azimuth={}°, roll={}°",
            angle(view.elev),
            angle(view.azim),
            angle(view.roll)
        )
    }

    /// The point of the pane nearest to the camera's line of sight
    /// through projected point `(xv, yv)`, and the axis that pane is normal
    /// to (matplotlib's `_calc_coord`); `None` when the projection cannot
    /// be inverted.
    pub(crate) fn pane_point(&self, xv: f64, yv: f64) -> Option<(Vec3, Axis3)> {
        self.with(|axes| {
            let projection = axes.projection();
            let inverse = projection.matrix.inverse()?;
            let focal_length = axes.camera.focal_length();
            let zv = if focal_length.is_infinite() {
                1.0
            } else {
                -1.0 / focal_length
            };
            let [x, y, z, w] = inverse.transform([xv, yv, zv, 1.0]);
            let p1 = if w == 0.0 {
                [x, y, z]
            } else {
                [x / w, y / w, z / w]
            };
            let limits = axes.limits.scaled();
            let aspect = axes.camera.stored_box_aspect();
            let view_w = axes.camera.view_frame()[2];
            let focal = if focal_length.is_infinite() {
                ORTHO_FOCAL_LENGTH
            } else {
                focal_length
            };
            let eye = [0, 1, 2].map(|i| {
                let (lo, hi) = (limits[2 * i], limits[2 * i + 1]);
                f64::midpoint(hi, lo) + view_w[i] * DIST * (hi - lo) / aspect[i] * focal
            });
            let vec = [0, 1, 2].map(|i| eye[i] - p1[i]);
            let geometry = BoxGeometry::new(&axes.limits, &projection);
            let panes = [0, 1, 2].map(|i| {
                if geometry.highs[i] {
                    geometry.maxs[i]
                } else {
                    geometry.mins[i]
                }
            });
            let scales = [0, 1, 2].map(|i| {
                if vec[i] == 0.0 {
                    f64::INFINITY
                } else {
                    (panes[i] - p1[i]) / vec[i]
                }
            });
            let nearest = (0..3).min_by(|&a, &b| scales[a].abs().total_cmp(&scales[b].abs()))?;
            let scale = scales[nearest];
            let mut point = [0, 1, 2].map(|i| p1[i] + scale * vec[i]);
            // The point lies on the pane: rounding must not move it off a
            // pane at zero (which would print as 1e-16).
            point[nearest] = panes[nearest];
            Some((point, Axis3::ALL[nearest]))
        })
    }

    /// A coordinate along `axis` as the status bar shows it: through the
    /// axis' major formatter, a scalar formatter giving as many digits as
    /// `resolution` (the data size of a screen pixel) needs (matplotlib's
    /// `format_xdata`, `format_ydata` and `format_zdata`).
    pub(crate) fn format_coordinate(
        &self,
        axis: Axis3,
        value: f64,
        resolution: f64,
    ) -> Option<String> {
        self.with(|axes| {
            let axis3d = axes.axis(axis);
            let formatter: &dyn Any = axis3d.major_formatter.as_ref();
            if formatter.is::<ScalarFormatter>() {
                return ScalarFormatter::format_data_resolved(value, resolution, &axis3d.settings)
                    .ok();
            }
            let (lo, hi) = axes.limits.view(axis);
            let info = AxisInfo::with_view(lo, hi);
            axis3d
                .major_formatter
                .format_data_short(value, &TickContext::new(&info, &axis3d.settings))
                .ok()
        })
    }
}

/// The location text for a pane point whose coordinates are already
/// formatted, naming the pane the point lies on (matplotlib's
/// `_location_coords`).
pub(crate) fn location_coords(pane: Axis3, [x, y, z]: [String; 3]) -> String {
    match pane {
        Axis3::X => format!("x pane={x}, y={y}, z={z}"),
        Axis3::Y => format!("x={x}, y pane={y}, z={z}"),
        Axis3::Z => format!("x={x}, y={y}, z pane={z}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn rotation_angles_use_true_minus_signs() {
        let ax = Figure::new().subplot_3d();
        assert_eq!(
            ax.rotation_coords(),
            "elevation=30°, azimuth=\u{2212}60°, roll=0°"
        );
        ax.view_init(200.0, 10.4, -0.4);
        assert_eq!(
            ax.rotation_coords(),
            "elevation=\u{2212}160°, azimuth=10°, roll=\u{2212}0°"
        );
    }

    #[test]
    fn the_point_lies_on_a_pane() {
        let ax = Figure::new().subplot_3d();
        ax.set_xlim(0.0, 10.0).unwrap();
        let (point, pane) = ax.pane_point(0.01, -0.02).unwrap();
        let (lo, hi) = ax.limits(pane);
        let on_pane = [lo, hi]
            .iter()
            .any(|&l| (point[pane.index()] - l).abs() < 1e-9);
        assert!(on_pane, "{point:?} on the {pane:?} pane");
        assert_eq!(
            location_coords(Axis3::Y, ["1".into(), "2".into(), "3".into()]),
            "x=1, y pane=2, z=3"
        );
    }
}
