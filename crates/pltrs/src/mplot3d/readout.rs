//! What the status bar shows over a 3-D axes (matplotlib's
//! `Axes3D.format_coord`): the view angles while rotating, otherwise the
//! point of the nearest pane under the pointer.

use std::any::Any;

use pltrs_core::Point;

use super::camera::{DIST, Projection, norm_angle};
use super::coords::Axis3;
use super::geometry::BoxGeometry;
use super::handle::Axes3D;
use super::math::{Matrix4, Vec3};
use super::state::Axes3DState;
use crate::artist::ArtistHandle;
use crate::ticker::{AxisInfo, ScalarFormatter, TickContext, TickerSettings};
use crate::transforms::TransformContext;

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

    /// The status-bar text for projected point `(xv, yv)` (matplotlib's
    /// `format_coord` outside a rotation): the coordinates of the point
    /// of the nearest pane under it, naming that pane, each formatted as
    /// [`Axes3D::format_data`] does; empty when the projection cannot be
    /// inverted or a formatter fails.
    pub fn format_coord(&self, xv: f64, yv: f64) -> String {
        self.format_pane_point(self.pane_point(xv, yv))
    }

    /// The status-bar text of a point `found` on a pane, as
    /// [`Axes3D::format_coord`] writes it; empty for none or when a
    /// formatter fails.
    pub(super) fn format_pane_point(&self, found: Option<(Vec3, Axis3)>) -> String {
        found
            .and_then(|(point, pane)| {
                let formatted = Axis3::ALL.map(|axis| self.format_data(axis, point[axis.index()]));
                let [Some(x), Some(y), Some(z)] = formatted else {
                    return None;
                };
                Some(location_coords(pane, [x, y, z]))
            })
            .unwrap_or_default()
    }

    /// The point (in data coordinates) of the pane nearest to the camera's
    /// line of sight through projected point `(xv, yv)`, and the axis that
    /// pane is normal to (matplotlib's `_calc_coord`, which works in
    /// transformed coordinates); `None` when the projection cannot be
    /// inverted.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// let (point, pane) = ax.pane_point(0.0, 0.0).expect("an invertible view");
    /// assert_eq!(pane, Axis3::Y);
    /// assert_eq!(point[1], ax.ylim().1);
    /// ```
    pub fn pane_point(&self, xv: f64, yv: f64) -> Option<(Vec3, Axis3)> {
        self.with(|axes| {
            let projection = axes.projection();
            let inverse = projection.matrix.inverse()?;
            pane_point_through(axes, &projection, &inverse, xv, yv)
        })
    }

    /// A coordinate along `axis` as the status bar shows it (matplotlib's
    /// `format_xdata`, `format_ydata` and `format_zdata`): in the short
    /// form of the axis' major formatter, which for a `ScalarFormatter`
    /// has as many digits as a screen pixel resolves. matplotlib's
    /// formatters take every 3-D axis for an x axis, so that pixel lies
    /// along the x axis of the projection plane whatever `axis` is. The
    /// figure is laid out first; `None` when the formatter fails.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// assert_eq!(ax.format_data(Axis3::Z, 0.123456).as_deref(), Some("0.1235"));
    /// ```
    pub fn format_data(&self, axis: Axis3, value: f64) -> Option<String> {
        let plane = self.plane().id();
        let trans = {
            let state = self.raw().figure_state().prepared();
            state.data_transform(plane)?
        };
        let inverse = trans.inverted()?;
        let screen = trans.transform_point(Point::new(value, 0.0));
        let resolution = [-1.0, 1.0]
            .into_iter()
            .map(|dx| {
                let near = inverse.transform_point(Point::new(screen.x + dx, screen.y));
                (near.x - value).abs()
            })
            .fold(0.0, f64::max);
        self.format_coordinate(axis, value, resolution)
    }

    /// `value` along `axis` through the axis' major formatter, a scalar
    /// formatter giving as many digits as `resolution` (the data size of
    /// a screen pixel) needs.
    fn format_coordinate(&self, axis: Axis3, value: f64, resolution: f64) -> Option<String> {
        self.with(|axes| {
            let axis3d = axes.axis(axis);
            let formatter = &axis3d.tickers.major_formatter;
            let settings = TickerSettings::current();
            let any: &dyn Any = formatter.as_ref();
            if any.is::<ScalarFormatter>() {
                return ScalarFormatter::format_data_resolved(value, resolution, &settings).ok();
            }
            let (lo, hi) = axes.limits.view(axis);
            let info = AxisInfo::with_view(lo, hi);
            formatter
                .format_data_short(value, &TickContext::new(&info, &settings))
                .ok()
        })
    }
}

/// The location text for a pane point whose coordinates are already
/// formatted, naming the pane the point lies on (matplotlib's
/// `_location_coords`).
fn location_coords(pane: Axis3, [x, y, z]: [String; 3]) -> String {
    match pane {
        Axis3::X => format!("x pane={x}, y={y}, z={z}"),
        Axis3::Y => format!("x={x}, y pane={y}, z={z}"),
        Axis3::Z => format!("x={x}, y={y}, z pane={z}"),
    }
}

/// The point of the pane of `axes` nearest to the camera's line of sight
/// through projected point `(xv, yv)`, and the axis that pane is normal
/// to, the panes seen through `projection` and the point brought back
/// through `inverse` (matplotlib's `_calc_coord` with `M` and `invM`);
/// `None` when no pane is in sight.
pub(super) fn pane_point_through(
    axes: &Axes3DState,
    projection: &Projection,
    inverse: &Matrix4,
    xv: f64,
    yv: f64,
) -> Option<(Vec3, Axis3)> {
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
    let limits = axes.scaled_limits();
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
    let geometry = BoxGeometry::new(limits, projection);
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
    // The point lies on the pane: rounding must not move it off a pane at
    // zero (which would print as 1e-16). A point under no number stays
    // none (NaN) on every axis, as in matplotlib.
    if !point[nearest].is_nan() {
        point[nearest] = panes[nearest];
    }
    let data = Axis3::ALL.map(|axis| axes.axis(axis).scale.inverse(point[axis.index()]));
    Some((data, Axis3::ALL[nearest]))
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

    #[test]
    fn no_number_lies_on_no_pane() {
        let ax = Figure::new().subplot_3d();
        for (xv, yv) in [(f64::NAN, 0.02), (0.01, f64::INFINITY)] {
            let (point, pane) = ax.pane_point(xv, yv).expect("an invertible view");
            assert_eq!(pane, Axis3::X, "numpy's argmin of NaN scales");
            assert!(point.iter().all(|c| c.is_nan()), "{point:?}");
        }
    }
}
