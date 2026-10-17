//! Lines and texts in 3-D (matplotlib's `Axes3D.plot`, `text` and
//! `text2D`).

use super::art3d::Art3D;
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use crate::artist::ArtistHandle;
use crate::lines::{Line2D, LineProps};
use crate::text::Text;

impl Axes3D {
    /// Plots a line through `(x, y, z)`; `z` holds one value per point or
    /// one for all (matplotlib's `ax.plot(xs, ys, zs)`).
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length.
    pub fn plot(&self, x: &[f64], y: &[f64], z: &[f64]) -> Result<Line2D, Axes3DError> {
        self.plot_with(x, y, z, Axis3::Z, LineProps::default())
    }

    /// Plots a line styled by a format string such as `"r--o"`.
    ///
    /// # Errors
    ///
    /// When the format string is invalid or the coordinates differ in
    /// length.
    pub fn plot_fmt(
        &self,
        x: &[f64],
        y: &[f64],
        z: &[f64],
        format: &str,
    ) -> Result<Line2D, Axes3DError> {
        let props = LineProps::fmt(format).map_err(crate::axes::PlotError::from)?;
        self.plot_with(x, y, z, Axis3::Z, props)
    }

    /// Plots a line with explicit properties; the 2-D points `(x, y)` lie
    /// in the plane normal to `zdir`, at `z` along it (matplotlib's
    /// `ax.plot(xs, ys, zs, zdir=..., **kwargs)`).
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length.
    pub fn plot_with(
        &self,
        x: &[f64],
        y: &[f64],
        z: &[f64],
        zdir: Axis3,
        props: LineProps,
    ) -> Result<Line2D, Axes3DError> {
        check_len("y", x.len(), y.len())?;
        let z = broadcast("z", z, x.len())?;
        let had_data = self.has_data();
        let line = self.plane().cycled_line(x.to_vec(), y.to_vec(), props);
        let points = juggled(x, y, &z, zdir);
        let raw = self.add_artist(line, Art3D::Line(points.clone()));
        let [xs, ys, zs] = columns(&points);
        self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)?;
        Ok(Line2D::from_raw(raw))
    }

    /// Adds text at `(x, y, z)` in data coordinates; with a `zdir` the
    /// baseline follows that axis as seen on screen (matplotlib's
    /// `ax.text(x, y, z, s, zdir=...)`).
    pub fn text(
        &self,
        x: f64,
        y: f64,
        z: f64,
        text: impl Into<String>,
        zdir: Option<Axis3>,
    ) -> Text {
        let handle = self.plane().text(x, y, text);
        self.register(
            handle.id(),
            Art3D::Text {
                position: [x, y, z],
                direction: zdir.map_or([0.0; 3], Axis3::unit),
            },
        );
        handle
    }

    /// Adds text at `(x, y)` in axes coordinates, over the 3-D scene
    /// (matplotlib's `ax.text2D(x, y, s, transform=ax.transAxes)`).
    pub fn text2d(&self, x: f64, y: f64, text: impl Into<String>) -> Text {
        let handle = self.plane().text(x, y, text);
        handle.set_transform(self.trans_axes());
        handle
    }
}

/// `values` with one entry per point: as given, or a single value
/// repeated (numpy broadcasting).
pub(crate) fn broadcast(
    argument: &'static str,
    values: &[f64],
    len: usize,
) -> Result<Vec<f64>, Axes3DError> {
    if let [single] = values {
        return Ok(vec![*single; len]);
    }
    check_len(argument, len, values.len())?;
    Ok(values.to_vec())
}

/// The points `(x, y)` placed in the plane normal to `zdir` at `z`.
pub(crate) fn juggled(x: &[f64], y: &[f64], z: &[f64], zdir: Axis3) -> Vec<Vec3> {
    x.iter()
        .zip(y)
        .zip(z)
        .map(|((&x, &y), &z)| zdir.juggle([x, y, z]))
        .collect()
}

/// The x, y and z coordinates of `points`.
pub(crate) fn columns(points: &[Vec3]) -> [Vec<f64>; 3] {
    [0, 1, 2].map(|i| points.iter().map(|p| p[i]).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn broadcasting_repeats_single_values() {
        assert_eq!(broadcast("z", &[2.0], 3).expect("valid"), [2.0, 2.0, 2.0]);
        assert!(broadcast("z", &[1.0, 2.0], 3).is_err());
        assert_eq!(broadcast("z", &[], 0).expect("valid"), Vec::<f64>::new());
    }

    #[test]
    fn plots_autoscale_like_matplotlib() {
        // The helix of the mplot3d spec cases.
        let n = 200;
        let t: Vec<f64> = (0..n)
            .map(|i| 4.0 * std::f64::consts::PI * f64::from(i) / f64::from(n - 1))
            .collect();
        let x: Vec<f64> = t.iter().map(|t| t.cos()).collect();
        let y: Vec<f64> = t.iter().map(|t| t.sin()).collect();
        let z: Vec<f64> = t.iter().map(|t| t / (4.0 * std::f64::consts::PI)).collect();
        let ax = Figure::new().subplot_3d();
        ax.plot(&x, &y, &z).expect("valid line");
        let close = |a: f64, b: f64| (a - b).abs() < 1e-6;
        let (y0, y1) = ax.ylim();
        assert!(close(y0, -1.145_798) && close(y1, 1.145_798));
        assert!(close(ax.zlim().0, -0.020_833));
    }

    #[test]
    fn zdir_places_the_line_in_another_plane() {
        let ax = Figure::new().subplot_3d();
        ax.plot_with(
            &[0.0, 1.0],
            &[0.0, 2.0],
            &[5.0],
            Axis3::Y,
            LineProps::default(),
        )
        .expect("valid line");
        let (y0, y1) = ax.ylim();
        assert!(y0 < 5.0 && y1 > 5.0, "the line lies at y = 5");
        assert!(ax.plot(&[1.0], &[1.0, 2.0], &[0.0]).is_err());
    }
}
