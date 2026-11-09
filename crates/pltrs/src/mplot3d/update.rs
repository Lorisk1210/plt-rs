//! Reading and replacing the 3-D geometry of artists already made, the
//! way animations do (matplotlib's `Line3D.get_data_3d` and
//! `set_data_3d`, the offsets of `Path3DCollection` and
//! `Patch3DCollection`, `Text3D.get_position_3d` and `get_depthshade`).
//!
//! Like matplotlib's, the new geometry is shown from the next draw on and
//! leaves the limits alone.

use super::art3d::{Art3D, Artist3D};
use super::camera::Projection;
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use super::plot::columns;
use super::state::Axes3DState;
use super::units::Sources3D;
use crate::artist::ArtistHandle;
use crate::collections::{Collection, CollectionArtist, DepthSorted};
use crate::lines::Line2D;
use crate::text::Text;
use crate::units::AxisValues;

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
    /// `Line3D.set_data_3d`); the limits stay as they are. The
    /// coordinates convert on their axes as those of [`Axes3D::plot`]
    /// do.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let line = ax.plot(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// ax.set_line_data(&line, &[1.0, 2.0, 3.0], &[0.0, 0.0, 0.0], &[3.0, 2.0, 1.0])?;
    /// let [x, _, z] = ax.line_data(&line)?;
    /// assert_eq!((x, z), (vec![1.0, 2.0, 3.0], vec![3.0, 2.0, 1.0]));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length, or `line` is not a 3-D line
    /// of these axes.
    pub fn set_line_data(
        &self,
        line: &Line2D,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        z: impl Into<AxisValues>,
    ) -> Result<&Self, Axes3DError> {
        let values = [x.into(), y.into(), z.into()];
        check_lengths(&values)?;
        self.read_art(line, |art| matches!(art, Art3D::Line(_)).then_some(()))?;
        let sources = Sources3D::of(values.each_ref());
        let points = points_of(self.convert_xyz(values));
        self.edit_art(line, |art| match art {
            Art3D::Line(old) => {
                *old = points;
                Some(())
            }
            _ => None,
        })?;
        self.keep_unit_sources(line.id(), sources);
        Ok(self)
    }

    /// The 3-D positions of the markers of a scatter plot of these axes,
    /// or of the offsets of a patch collection added to them, as x, y and
    /// z coordinates (matplotlib's `_offsets3d` of `Path3DCollection` and
    /// `Patch3DCollection`).
    ///
    /// # Errors
    ///
    /// When `scatter` is neither a scatter plot nor a patch collection of
    /// these axes.
    pub fn scatter_offsets(&self, scatter: &Collection) -> Result<[Vec<f64>; 3], Axes3DError> {
        self.read_art(scatter, |art| match art {
            Art3D::Points(points) => Some(columns(&points.positions)),
            _ => None,
        })
    }

    /// Moves the markers of a scatter plot of these axes, one position
    /// per marker, or the offsets of a patch collection added to them
    /// (matplotlib's `set_3d_properties` and `_offsets3d` of
    /// `Path3DCollection` and `Patch3DCollection`); their sizes and
    /// colours keep their order and the limits stay as they are. The
    /// coordinates convert on their axes as those of [`Axes3D::scatter`]
    /// do.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let points = ax.scatter(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// ax.set_scatter_offsets(&points, &[2.0, 3.0], &[2.0, 3.0], &[2.0, 3.0])?;
    /// assert_eq!(ax.scatter_offsets(&points)?[0], [2.0, 3.0]);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length, or `scatter` is neither a
    /// scatter plot nor a patch collection of these axes.
    pub fn set_scatter_offsets(
        &self,
        scatter: &Collection,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        z: impl Into<AxisValues>,
    ) -> Result<&Self, Axes3DError> {
        let values = [x.into(), y.into(), z.into()];
        check_lengths(&values)?;
        self.read_art(scatter, |art| matches!(art, Art3D::Points(_)).then_some(()))?;
        let positions = points_of(self.convert_xyz(values));
        self.edit_art(scatter, |art| match art {
            Art3D::Points(points) => {
                points.positions = positions;
                Some(())
            }
            _ => None,
        })?;
        // matplotlib's `set_3d_properties` leaves the colours unsorted until
        // the next projection.
        scatter
            .raw()
            .with_mut::<CollectionArtist, _>(|c| c.depth_sorted = Some(DepthSorted::unprojected()));
        Ok(self)
    }

    /// Whether the markers of a scatter plot of these axes fade with
    /// depth, or the offsets of a patch collection would (matplotlib's
    /// `get_depthshade` of `Path3DCollection` and `Patch3DCollection`).
    ///
    /// # Errors
    ///
    /// When `scatter` is neither a scatter plot nor a patch collection of
    /// these axes.
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
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let label = ax.text(1.0, 2.0, 3.0, "here", None);
    /// assert_eq!(ax.text_position(&label)?, [1.0, 2.0, 3.0]);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
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

    /// Projects a 3-D artist of these axes onto its 2-D artist in the
    /// current view and returns its depth, the projected z of its nearest
    /// point (NaN without points, and for lines and texts, which keep
    /// their own z-order; matplotlib's `do_3d_projection`).
    ///
    /// ```
    /// use pltrs::collections::CollectionProps;
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let empty = ax.add_poly_collection3d(Vec::new(), CollectionProps::default(), false)?;
    /// assert!(ax.do_3d_projection(&empty)?.is_nan());
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `artist` is not a 3-D artist of these axes, or the problem
    /// recorded for drawing it (see [`Axes3D::set_draw_failure`]).
    pub fn do_3d_projection(&self, artist: &impl ArtistHandle) -> Result<f64, Axes3DError> {
        self.project_artist(artist, |projection| projection)
    }

    /// Projects a 3-D artist of these axes as [`Axes3D::do_3d_projection`]
    /// does, through the current projection changed by `adjust`.
    pub(super) fn project_artist(
        &self,
        artist: &impl ArtistHandle,
        adjust: impl FnOnce(Projection) -> Projection,
    ) -> Result<f64, Axes3DError> {
        let id = self.own_id(artist).ok_or(Axes3DError::Not3DArtist)?;
        if let Some(failure) = self.with(|axes| axes.failure_of(id).cloned()) {
            return Err(Axes3DError::Draw(failure));
        }
        let depth = self.raw().with_others::<Axes3DState, _>(|axes, store| {
            let entry = axes.artists.iter().find(|a| a.id == id)?;
            let projection = adjust(axes.projection());
            let projection = if entry.axlim_clip {
                projection.hiding_outside(Axis3::ALL.map(|axis| axes.limits.bound(axis)))
            } else {
                projection
            };
            entry.art.warn_projecting(&projection);
            entry.art.project(id, store, &projection)
        });
        depth.ok_or(Axes3DError::Not3DArtist)
    }

    /// Reads the 3-D geometry of `artist` with `read`.
    pub(super) fn read_art<R>(
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
    pub(super) fn edit_art(
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

/// Checks that the coordinates `[x, y, z]` are as many in each.
fn check_lengths([x, y, z]: &[AxisValues; 3]) -> Result<(), Axes3DError> {
    check_len("y", x.len(), y.len())?;
    check_len("z", x.len(), z.len())
}

/// The points `(x, y, z)` of coordinates as many in each.
fn points_of([x, y, z]: [Vec<f64>; 3]) -> Vec<Vec3> {
    x.into_iter()
        .zip(y)
        .zip(z)
        .map(|((x, y), z)| [x, y, z])
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::figure::Figure;
    use crate::mplot3d::Axes3DError;

    #[test]
    fn replacing_points_keeps_the_limits_and_checks_lengths() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0], [0.0, 1.0]).expect("a line");
        fig.layout();
        let before = ax.xlim();
        ax.set_line_data(&line, [10.0, 20.0], [0.0, 0.0], [0.0, 0.0])
            .expect("same lengths");
        assert_eq!(ax.xlim(), before);
        assert!(matches!(
            ax.set_line_data(&line, [1.0, 2.0], [0.0], [0.0, 0.0]),
            Err(Axes3DError::Shape { .. })
        ));
    }

    #[test]
    fn artists_of_other_kinds_or_axes_are_refused() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let other = fig.subplot_3d();
        let line = other.plot([0.0], [0.0], [0.0]).expect("a line");
        assert_eq!(ax.line_data(&line), Err(Axes3DError::Not3DArtist));
        let points = ax.scatter([0.0], [0.0], [0.0]).expect("markers");
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
