//! Lines and texts in 3-D (matplotlib's `Axes3D.plot`, `text` and
//! `text2D`).

use super::art3d::Art3D;
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use super::units::Sources3D;
use crate::artist::ArtistHandle;
use crate::lines::{Line2D, LineProps};
use crate::text::Text;
use crate::units::{AxisValue, AxisValues};

impl Axes3D {
    /// Plots a line through `(x, y, z)`; `z` holds one value per point or
    /// one for all (matplotlib's `ax.plot(xs, ys, zs)`). The coordinates
    /// are numbers, dates, category names or user values, converted on
    /// their axes as 2-D plotting values are ([`Axis3D::convert_units`]).
    ///
    /// ```
    /// use pltrs::dates::DateTime;
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let day = |d| DateTime::new(2020, 1, d, 0, 0, 0, 0);
    /// ax.plot(["a", "b", "c"], [1.0, 2.0, 3.0], vec![day(1)?, day(2)?, day(3)?])?;
    /// assert_eq!(ax.xaxis().ticklabels(false)?, ["a", "b", "c"]);
    /// assert_eq!(ax.zaxis().ticklabels(false)?[0], "01-01 00");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length.
    ///
    /// [`Axis3D::convert_units`]: crate::mplot3d::Axis3D::convert_units
    pub fn plot(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        z: impl Into<AxisValues>,
    ) -> Result<Line2D, Axes3DError> {
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
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        z: impl Into<AxisValues>,
        format: &str,
    ) -> Result<Line2D, Axes3DError> {
        let props = LineProps::fmt(format).map_err(crate::axes::PlotError::from)?;
        self.plot_with(x, y, z, Axis3::Z, props)
    }

    /// Plots a line with explicit properties; the 2-D points `(x, y)` lie
    /// in the plane normal to `zdir`, at `z` along it (matplotlib's
    /// `ax.plot(xs, ys, zs, zdir=..., **kwargs)`).
    /// [`Axes3D::set_axlim_clip`] on the line hides its points outside the
    /// view limits (matplotlib's `axlim_clip`). The values convert on the
    /// axes they lie along once placed, and the user values among them
    /// convert again when their axis changes unit.
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length.
    pub fn plot_with(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        z: impl Into<AxisValues>,
        zdir: Axis3,
        props: LineProps,
    ) -> Result<Line2D, Axes3DError> {
        let values = [x.into(), y.into(), z.into()];
        let [x, y, z] = values.each_ref().map(AxisValues::len);
        check_len("y", x, y)?;
        if z != 1 {
            check_len("z", x, z)?;
        }
        let sources = Sources3D::of(zdir.juggle(values.each_ref()));
        let had_data = self.has_data();
        let [x, y, z] = self.convert_juggled(values, zdir);
        let z = spread(z, x.len());
        let line = self.plane().cycled_line(x.clone(), y.clone(), props);
        let points = juggled(&x, &y, &z, zdir);
        let raw = self.add_artist(line, Art3D::Line(points.clone()), false);
        self.keep_unit_sources(raw.id(), sources);
        let [xs, ys, zs] = columns(&points);
        self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)?;
        Ok(Line2D::from_raw(raw))
    }

    /// Places a legend of the labelled artists in the axes (matplotlib's
    /// `ax.legend()` on 3-D axes).
    ///
    /// # Errors
    ///
    /// As [`Axes::legend`](crate::axes::Axes::legend).
    pub fn legend(
        &self,
        options: crate::legend::LegendOptions,
    ) -> Result<crate::legend::Legend, crate::legend::LegendError> {
        self.plane().legend(options)
    }

    /// The legend placed with [`Axes3D::legend`], if any (matplotlib's
    /// `get_legend`).
    pub fn current_legend(&self) -> Option<crate::legend::Legend> {
        self.plane().current_legend()
    }

    /// Adds text at `(x, y, z)` in data coordinates; with a `zdir` the
    /// baseline follows that axis as seen on screen (matplotlib's
    /// `ax.text(x, y, z, s, zdir=...)`). [`Axes3D::set_axlim_clip`] on
    /// the text hides it while its anchor is outside the view limits
    /// (matplotlib's `axlim_clip`). The coordinates may be dates,
    /// category names or user values, converted without changing their
    /// axes as 2-D texts convert theirs (a name needs an axis with
    /// categories); user values convert again when their axis changes
    /// unit.
    pub fn text(
        &self,
        x: impl Into<AxisValue>,
        y: impl Into<AxisValue>,
        z: impl Into<AxisValue>,
        text: impl Into<String>,
        zdir: Option<Axis3>,
    ) -> Text {
        self.text_along(x, y, z, text, zdir.map_or([0.0; 3], Axis3::unit))
    }

    /// Adds text at `(x, y, z)` whose baseline follows `direction` as
    /// seen on screen; a zero direction keeps it horizontal (matplotlib's
    /// `ax.text(x, y, z, s, zdir=(dx, dy, dz))`).
    pub fn text_along(
        &self,
        x: impl Into<AxisValue>,
        y: impl Into<AxisValue>,
        z: impl Into<AxisValue>,
        text: impl Into<String>,
        direction: Vec3,
    ) -> Text {
        let values = [x.into(), y.into(), z.into()];
        let sources = Sources3D::of_position(&values);
        let position = self.convert_position(values);
        let handle = self.plane().text(position[0], position[1], text);
        self.register(
            handle.id(),
            Art3D::Text {
                position,
                direction,
            },
            false,
        );
        self.keep_unit_sources(handle.id(), sources);
        handle
    }

    /// Moves a text placed with [`Axes3D::text`] or [`Axes3D::text_along`]
    /// to `position`, in axis numbers, its baseline following `direction`
    /// when given and keeping its direction otherwise (matplotlib's
    /// `Text3D.set_position_3d`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let label = ax.text(0.0, 0.0, 0.0, "moved", None);
    /// ax.set_text_position(&label, [1.0, 2.0, 3.0], Some([1.0, 1.0, 0.0]))?;
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `text` is not a 3-D text of this axes.
    pub fn set_text_position(
        &self,
        text: &Text,
        position: Vec3,
        direction: Option<Vec3>,
    ) -> Result<&Self, Axes3DError> {
        let id = self.own_id(text);
        let found = self.with_mut(|axes| {
            let entry = axes.artists.iter_mut().find(|a| Some(a.id) == id)?;
            let Art3D::Text {
                position: anchor,
                direction: baseline,
            } = &mut entry.art
            else {
                return None;
            };
            // Numbers replace the values the anchor came from.
            entry.sources = Sources3D::default();
            *anchor = position;
            if let Some(direction) = direction {
                *baseline = direction;
            }
            Some(())
        });
        found.map(|()| self).ok_or(Axes3DError::Not3DArtist)
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

/// `values` with `len` entries when it holds one value, repeated, and as
/// it is otherwise (numpy broadcasting of lengths checked before).
pub(crate) fn spread(values: Vec<f64>, len: usize) -> Vec<f64> {
    match values[..] {
        [single] => vec![single; len],
        _ => values,
    }
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
            [0.0, 1.0],
            [0.0, 2.0],
            [5.0],
            Axis3::Y,
            LineProps::default(),
        )
        .expect("valid line");
        let (y0, y1) = ax.ylim();
        assert!(y0 < 5.0 && y1 > 5.0, "the line lies at y = 5");
        assert!(ax.plot([1.0], [1.0, 2.0], [0.0]).is_err());
    }

    #[test]
    fn moved_texts_land_where_placed_texts_do() {
        // matplotlib's test_text3d_modification.
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let placed = ax.text_along(1.0, 2.0, 3.0, "placed", [1.0, 1.0, 0.0]);
        let moved = ax.text(0.0, 0.0, 0.0, "moved", None);
        ax.set_text_position(&moved, [1.0, 2.0, 3.0], Some([1.0, 1.0, 0.0]))
            .unwrap();
        let kept = ax.text(0.0, 0.0, 0.0, "kept", Some(Axis3::Y));
        ax.set_text_position(&kept, [1.0, 2.0, 3.0], None).unwrap();
        let along_y = ax.text(1.0, 2.0, 3.0, "along y", Some(Axis3::Y));
        fig.layout();
        assert_eq!(moved.position(), placed.position());
        assert_eq!(moved.rotation(), placed.rotation());
        assert_eq!(kept.rotation(), along_y.rotation());
        assert_ne!(kept.rotation(), placed.rotation());
        let flat = ax.text2d(0.5, 0.5, "2-D");
        assert_eq!(
            ax.set_text_position(&flat, [0.0; 3], None).err(),
            Some(Axes3DError::Not3DArtist)
        );
    }
}
