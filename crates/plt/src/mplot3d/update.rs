//! Reading and replacing the 3-D geometry of artists already made, the
//! way animations do (matplotlib's `Line3D.get_data_3d` and
//! `set_data_3d`, `Path3DCollection._offsets3d`, `Text3D.get_position_3d`
//! and `get_depthshade`).
//!
//! Like matplotlib's, the new geometry is shown from the next draw on and
//! leaves the limits alone.

use super::art3d::{Art3D, Artist3D};
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use super::plot::columns;
use crate::artist::ArtistHandle;
use crate::collections::Collection;
use crate::lines::Line2D;
use crate::text::Text;

impl Axes3D {
    /// The 3-D points of a line of these axes, as x, y and z coordinates
    /// (matplotlib's `Line3D.get_data_3d`).
    ///
    /// # Errors
    ///
    /// When `line` is not a 3-D line of these axes.
    pub fn line_data(&self, line: &Line2D) -> Result<[Vec<f64>; 3], Axes3DError> {
        self.read_art(line, |art| match art {
            Art3D::Line(points) => Some(columns(points)),
            _ => None,
        })
    }

    /// Replaces the 3-D points of a line of these axes (matplotlib's
    /// `Line3D.set_data_3d`); the limits stay as they are.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let line = ax.plot(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// ax.set_line_data(&line, &[1.0, 2.0, 3.0], &[0.0, 0.0, 0.0], &[3.0, 2.0, 1.0])?;
    /// let [x, _, z] = ax.line_data(&line)?;
    /// assert_eq!((x, z), (vec![1.0, 2.0, 3.0], vec![3.0, 2.0, 1.0]));
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length, or `line` is not a 3-D line
    /// of these axes.
    pub fn set_line_data(
        &self,
        line: &Line2D,
        x: &[f64],
        y: &[f64],
        z: &[f64],
    ) -> Result<&Self, Axes3DError> {
        let points = points_of(x, y, z)?;
        self.edit_art(line, |art| match art {
            Art3D::Line(old) => {
                *old = points;
                Some(())
            }
            _ => None,
        })?;
        Ok(self)
    }

    /// The 3-D positions of the markers of a scatter plot of these axes,
    /// as x, y and z coordinates (matplotlib's
    /// `Path3DCollection._offsets3d`).
    ///
    /// # Errors
    ///
    /// When `scatter` is not a scatter plot of these axes.
    pub fn scatter_offsets(&self, scatter: &Collection) -> Result<[Vec<f64>; 3], Axes3DError> {
        self.read_art(scatter, |art| match art {
            Art3D::Points(points) => Some(columns(&points.positions)),
            _ => None,
        })
    }

    /// Moves the markers of a scatter plot of these axes, one position
    /// per marker (matplotlib's `Path3DCollection.set_3d_properties` and
    /// `_offsets3d`); their sizes and colours keep their order and the
    /// limits stay as they are.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let points = ax.scatter(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// ax.set_scatter_offsets(&points, &[2.0, 3.0], &[2.0, 3.0], &[2.0, 3.0])?;
    /// assert_eq!(ax.scatter_offsets(&points)?[0], [2.0, 3.0]);
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length, or `scatter` is not a
    /// scatter plot of these axes.
    pub fn set_scatter_offsets(
        &self,
        scatter: &Collection,
        x: &[f64],
        y: &[f64],
        z: &[f64],
    ) -> Result<&Self, Axes3DError> {
        let positions = points_of(x, y, z)?;
        self.edit_art(scatter, |art| match art {
            Art3D::Points(points) => {
                points.positions = positions;
                Some(())
            }
            _ => None,
        })?;
        Ok(self)
    }

    /// Whether the markers of a scatter plot of these axes fade with
    /// depth (matplotlib's `get_depthshade`).
    ///
    /// # Errors
    ///
    /// When `scatter` is not a scatter plot of these axes.
    pub fn depthshade(&self, scatter: &Collection) -> Result<bool, Axes3DError> {
        self.read_art(scatter, |art| match art {
            Art3D::Points(points) => Some(points.depthshade),
            _ => None,
        })
    }

    /// The 3-D anchor of a text of these axes (matplotlib's
    /// `Text3D.get_position_3d`).
    ///
    /// ```
    /// use plt::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let label = ax.text(1.0, 2.0, 3.0, "here", None);
    /// assert_eq!(ax.text_position(&label)?, [1.0, 2.0, 3.0]);
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `text` is not a 3-D text of these axes.
    pub fn text_position(&self, text: &Text) -> Result<Vec3, Axes3DError> {
        self.read_art(text, |art| match art {
            Art3D::Text { position, .. } => Some(*position),
            _ => None,
        })
    }

    /// Reads the 3-D geometry of `artist` with `read`.
    fn read_art<R>(
        &self,
        artist: &impl ArtistHandle,
        read: impl FnOnce(&Art3D) -> Option<R>,
    ) -> Result<R, Axes3DError> {
        let id = self.own_id(artist);
        self.with(|axes| {
            let entry: &Artist3D = axes.artists.iter().find(|a| Some(a.id) == id)?;
            read(&entry.art)
        })
        .ok_or(Axes3DError::Not3DArtist)
    }

    /// Changes the 3-D geometry of `artist` with `edit`.
    fn edit_art(
        &self,
        artist: &impl ArtistHandle,
        edit: impl FnOnce(&mut Art3D) -> Option<()>,
    ) -> Result<(), Axes3DError> {
        let id = self.own_id(artist);
        self.with_mut(|axes| {
            let entry = axes.artists.iter_mut().find(|a| Some(a.id) == id)?;
            edit(&mut entry.art)
        })
        .ok_or(Axes3DError::Not3DArtist)
    }
}

/// The points `(x, y, z)`, which must be as many in each coordinate.
fn points_of(x: &[f64], y: &[f64], z: &[f64]) -> Result<Vec<Vec3>, Axes3DError> {
    check_len("y", x.len(), y.len())?;
    check_len("z", x.len(), z.len())?;
    Ok(x.iter()
        .zip(y)
        .zip(z)
        .map(|((&x, &y), &z)| [x, y, z])
        .collect())
}

#[cfg(test)]
mod tests {
    use crate::figure::Figure;
    use crate::mplot3d::Axes3DError;

    #[test]
    fn replacing_points_keeps_the_limits_and_checks_lengths() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let line = ax
            .plot(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .expect("a line");
        fig.layout();
        let before = ax.xlim();
        ax.set_line_data(&line, &[10.0, 20.0], &[0.0, 0.0], &[0.0, 0.0])
            .expect("same lengths");
        assert_eq!(ax.xlim(), before);
        assert!(matches!(
            ax.set_line_data(&line, &[1.0, 2.0], &[0.0], &[0.0, 0.0]),
            Err(Axes3DError::Shape { .. })
        ));
    }

    #[test]
    fn artists_of_other_kinds_or_axes_are_refused() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let other = fig.subplot_3d();
        let line = other.plot(&[0.0], &[0.0], &[0.0]).expect("a line");
        assert_eq!(ax.line_data(&line), Err(Axes3DError::Not3DArtist));
        let points = ax.scatter(&[0.0], &[0.0], &[0.0]).expect("markers");
        ax.set_depthshade(&points, false).expect("a scatter");
        assert_eq!(ax.depthshade(&points), Ok(false));
        let label = ax.text(1.0, 2.0, 3.0, "t", None);
        assert_eq!(
            ax.scatter_offsets(&points),
            Ok([vec![0.0], vec![0.0], vec![0.0]])
        );
        assert_eq!(
            ax.depthshade(&wireframe(&ax)),
            Err(Axes3DError::Not3DArtist)
        );
        assert_eq!(ax.text_position(&label), Ok([1.0, 2.0, 3.0]));
    }

    /// A collection of `ax` that is not a scatter plot.
    fn wireframe(ax: &crate::mplot3d::Axes3D) -> crate::collections::Collection {
        ax.plot_wireframe(
            &crate::numeric::Grid::filled(2, 2, 0.0),
            &crate::numeric::Grid::filled(2, 2, 0.0),
            &crate::numeric::Grid::filled(2, 2, 0.0),
            crate::mplot3d::WireframeOptions::default(),
        )
        .expect("a wireframe")
    }
}
