//! What a 3-D axes keeps of its last draw: the projection matrix it drew
//! with and its inverse (matplotlib's `Axes3D.M` and `invM`, `None` until
//! the axes is first drawn), and which of its artists a draw projected.
//!
//! matplotlib reads the side each axis is drawn on (`get_axis_position`),
//! the panes (`Axis.active_pane`), the point under the pointer
//! (`_calc_coord`, `format_coord`) and the projections a program asks of
//! its artists (`do_3d_projection`) through `M`: they answer for the view
//! of the last draw, not for the view as it is now, and fail before the
//! first draw. Measuring the axes for a tight box sets `M` when there is
//! none (matplotlib's `Axis.get_tightbbox`), and so may a program.

use super::axis_handle::Axis3D;
use super::camera::Projection;
use super::coords::Axis3;
use super::error::Axes3DError;
use super::geometry::{active_pane_through, axis_highs_through};
use super::handle::Axes3D;
use super::math::{Matrix4, Vec3};
use super::readout::pane_point_through;
use super::state::Axes3DState;
use crate::artist::{Artist, ArtistHandle, ArtistId};
use crate::figure::{FigureState, Painting};
use crate::lines::{Line2D, LineArtist};

/// The projection matrix of a 3-D axes' last draw and its inverse
/// (matplotlib's `M` and `invM`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DrawnMatrices {
    /// The projection matrix (`M`).
    matrix: Option<Matrix4>,
    /// Its inverse (`invM`): a draw sets both, a program either alone.
    inverse: Option<Matrix4>,
}

pltrs_core::archive_struct!(DrawnMatrices { matrix, inverse });

impl Axes3DState {
    /// Keeps the projection of the view as it is now as the one drawn
    /// with (matplotlib's `self.M = self.get_proj()` and `self.invM =
    /// np.linalg.inv(self.M)`).
    fn keep_projection(&mut self) {
        let matrix = self.projection().matrix;
        self.drawn = DrawnMatrices {
            matrix: Some(matrix),
            inverse: numpy_inverse(&matrix),
        };
    }

    /// The projection of the last draw: the current one through `M`.
    fn drawn_projection(&self) -> Option<Projection> {
        Some(self.projection().with_matrix(self.drawn.matrix?))
    }
}

/// The inverse numpy's `linalg.inv` gives `matrix`: NaN throughout for a
/// matrix with non-finite entries (a projection through limits a scale
/// cannot map), which LAPACK's elimination spreads without failing; none
/// for a singular one, where numpy raises.
fn numpy_inverse(matrix: &Matrix4) -> Option<Matrix4> {
    if matrix.rows().iter().flatten().any(|v| !v.is_finite()) {
        return Some(Matrix4::from_rows([[f64::NAN; 4]; 4]));
    }
    matrix.inverse()
}

/// Records a draw of the 3-D axes `id`, just laid out for it (matplotlib's
/// `Axes3D.draw`, which a figure's draws leave out for an animated or an
/// invisible axes): its projection becomes `M`, and the artists the
/// layout projected for the draw hold a draw's projection (with
/// `painting` stopped, or a projection of the collections and patches
/// failing, not those projected as they are drawn; never those the draw
/// fails on).
pub(crate) fn note_draw(figure: &mut FigureState, id: ArtistId, painting: Painting) {
    let saving = figure.saving;
    figure.with_others::<Axes3DState, _>(id, |axes, store| {
        if axes.base().animated || !axes.base().visible {
            return;
        }
        axes.keep_projection();
        let stops = axes.stops_projecting(store);
        let failures = &axes.failures;
        for artist in &mut axes.artists {
            let animated = store.get(artist.id).is_some_and(|a| a.base().animated);
            let as_drawn = artist.art.projected_when_drawn();
            let reached = !(as_drawn && (painting == Painting::Stopped || stops));
            if (saving || !(animated && as_drawn)) && reached && failures.of(artist.id).is_none() {
                artist.drawn = true;
            }
        }
    });
}

/// Gives the 3-D axes `id` an `M` when it has none, as measuring a visible
/// axes showing a visible axis for a tight box does in matplotlib.
pub(crate) fn note_measure(figure: &mut FigureState, id: ArtistId) {
    if let Some(axes) = figure.store.get_as_mut::<Axes3DState>(id)
        && axes.drawn.matrix.is_none()
        && axes.base().visible
        && axes.axis_on
        && Axis3::ALL.iter().any(|&dir| axes.axis(dir).visible)
    {
        axes.keep_projection();
    }
}

impl Axes3D {
    /// The projection matrix the axes was last drawn with (matplotlib's
    /// `M`), `None` before its first draw.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// assert_eq!(ax.drawn_proj(), None);
    /// fig.canvas().draw()?;
    /// assert_eq!(ax.drawn_proj(), Some(ax.proj()));
    /// ax.view_init(10.0, 20.0, 0.0);
    /// assert_ne!(ax.drawn_proj(), Some(ax.proj()), "the view of the draw");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn drawn_proj(&self) -> Option<Matrix4> {
        self.with(|axes| axes.drawn.matrix)
    }

    /// Replaces the projection matrix of the last draw (matplotlib's
    /// `ax.M = ...`), leaving its inverse as it is.
    pub fn set_drawn_proj(&self, matrix: Option<Matrix4>) -> &Self {
        self.with_mut(|axes| axes.drawn.matrix = matrix);
        self
    }

    /// The inverse of the projection matrix of the last draw (matplotlib's
    /// `invM`), `None` before the first draw or for a singular one.
    pub fn drawn_proj_inverse(&self) -> Option<Matrix4> {
        self.with(|axes| axes.drawn.inverse)
    }

    /// Replaces the inverse of the projection matrix of the last draw
    /// (matplotlib's `ax.invM = ...`).
    pub fn set_drawn_proj_inverse(&self, inverse: Option<Matrix4>) -> &Self {
        self.with_mut(|axes| axes.drawn.inverse = inverse);
        self
    }

    /// Whether the x, y and z axes are drawn on their high side in the
    /// view of the last draw (matplotlib's `get_axis_position`, which
    /// reads `M`); `None` before the first draw.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// assert_eq!(ax.drawn_axis_position(), None);
    /// fig.canvas().draw()?;
    /// ax.view_init(-30.0, 120.0, 0.0);
    /// assert_eq!(ax.drawn_axis_position(), Some([false, true, false]));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn drawn_axis_position(&self) -> Option<[bool; 3]> {
        self.with(|axes| Some(axis_highs_through(axes, &axes.drawn_projection()?)))
    }

    /// The point (in data coordinates) of the pane nearest to the camera's
    /// line of sight through projected point `(xv, yv)`, and the axis that
    /// pane is normal to, through the projection of the last draw and its
    /// inverse (matplotlib's `_calc_coord`, which reads `M` and `invM`);
    /// `None` without either.
    pub fn drawn_pane_point(&self, xv: f64, yv: f64) -> Option<(Vec3, Axis3)> {
        self.with(|axes| {
            let projection = axes.drawn_projection()?;
            pane_point_through(axes, &projection, &axes.drawn.inverse?, xv, yv)
        })
    }

    /// The status-bar text for projected point `(xv, yv)` outside a
    /// rotation (matplotlib's `format_coord`): the point of the pane under
    /// it, as [`Axes3D::drawn_pane_point`] finds it and
    /// [`Axes3D::format_coord`] writes it; empty before the first draw.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// assert_eq!(ax.drawn_format_coord(0.01, 0.01), "");
    /// fig.canvas().draw()?;
    /// assert_eq!(ax.drawn_format_coord(0.01, 0.01), ax.format_coord(0.01, 0.01));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn drawn_format_coord(&self, xv: f64, yv: f64) -> String {
        if self.drawn_proj().is_none() {
            return String::new();
        }
        self.format_pane_point(self.drawn_pane_point(xv, yv))
    }

    /// Projects a 3-D artist of these axes as [`Axes3D::do_3d_projection`]
    /// does, through the projection matrix of the last draw (matplotlib's
    /// `do_3d_projection`, which reads `M`); `Ok(None)` before the first
    /// draw for an artist with points, leaving it as it is (numpy then
    /// fails to multiply them by matplotlib's `None`, where it multiplies
    /// no points at all).
    ///
    /// ```
    /// use pltrs::collections::CollectionProps;
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let empty = ax.add_poly_collection3d(Vec::new(), CollectionProps::default(), false)?;
    /// assert!(ax.do_3d_projection_as_drawn(&empty)?.is_some_and(f64::is_nan));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `artist` is not a 3-D artist of these axes.
    pub fn do_3d_projection_as_drawn(
        &self,
        artist: &impl ArtistHandle,
    ) -> Result<Option<f64>, Axes3DError> {
        if let Some(failure) = self.draw_failure(artist) {
            return Err(Axes3DError::Draw(failure));
        }
        if let Some(matrix) = self.drawn_proj() {
            return self
                .project_artist(artist, |projection| projection.with_matrix(matrix))
                .map(Some);
        }
        if self.read_art(artist, |art| Some(art.points().is_empty()))? {
            return self.do_3d_projection(artist).map(Some);
        }
        Ok(None)
    }

    /// Which points of the 3-D line `line` its last draw masked, as
    /// matplotlib's `Line3D.draw` masks the projected data it gives the
    /// line (those a perspective camera clips, its NaN points among
    /// them); `None` until a draw of the axes projected the line, when
    /// its 2-D data are those it was made with.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// let line = ax.plot(&[0.0, 1.0, f64::NAN], &[0.0, 1.0, 1.0], &[0.0, 1.0, 1.0])?;
    /// assert_eq!(ax.projected_line_mask(&line)?, None);
    /// fig.canvas().draw()?;
    /// assert_eq!(ax.projected_line_mask(&line)?, Some(vec![false, false, true]));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `line` is not a 3-D line of these axes.
    pub fn projected_line_mask(&self, line: &Line2D) -> Result<Option<Vec<bool>>, Axes3DError> {
        let id = self.own_id(line).ok_or(Axes3DError::Not3DArtist)?;
        let (drawn, ortho) = self
            .with(|axes| {
                let entry = axes.artists.iter().find(|artist| artist.id == id)?;
                Some((entry.drawn, axes.camera.is_ortho()))
            })
            .ok_or(Axes3DError::Not3DArtist)?;
        if !drawn {
            return Ok(None);
        }
        let mask = line.raw().with::<LineArtist, _>(|line| {
            (line.xdata.iter().zip(&line.ydata))
                .map(|(x, y)| !ortho && (x.is_nan() || y.is_nan()))
                .collect()
        });
        Ok(Some(mask))
    }
}

impl Axis3D {
    /// The pane of the axis seen in the view of the last draw, as
    /// [`Axis3D::active_pane`] gives it (matplotlib's `active_pane`, which
    /// reads `M`); `None` before the first draw.
    pub fn drawn_active_pane(&self) -> Option<([Vec3; 4], f64)> {
        self.axes.with(|axes| {
            Some(active_pane_through(
                axes,
                self.dir,
                &axes.drawn_projection()?,
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::mplot3d::Axis3;

    #[test]
    fn a_draw_keeps_its_projection_and_measuring_gives_one() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        assert!(ax.drawn_pane_point(0.0, 0.0).is_none());
        assert!(ax.zaxis().drawn_active_pane().is_none());
        ax.tight_bbox(false);
        assert_eq!(ax.drawn_proj(), Some(ax.proj()), "measured");
        let inverse = ax.drawn_proj_inverse().expect("an invertible view");
        assert_eq!(Some(inverse), ax.proj().inverse());
        ax.view_init(0.0, 0.0, 0.0);
        ax.tight_bbox(false);
        assert_ne!(ax.drawn_proj(), Some(ax.proj()), "kept while there is one");
        assert_ne!(
            ax.zaxis().drawn_active_pane(),
            Some(ax.zaxis().active_pane())
        );
        fig.canvas().draw().expect("drawn");
        assert_eq!(ax.drawn_proj(), Some(ax.proj()));
        assert_eq!(ax.drawn_pane_point(0.01, -0.02), ax.pane_point(0.01, -0.02));
    }

    #[test]
    fn hidden_and_animated_axes_are_not_drawn() {
        let fig = Figure::new();
        let hidden = fig.subplot_3d();
        hidden.set_visible(false);
        let animated = fig.subplot_3d();
        animated.set_animated(true);
        fig.canvas().draw().expect("drawn");
        assert_eq!(hidden.drawn_proj(), None);
        assert_eq!(animated.drawn_proj(), None);
    }

    #[test]
    fn programs_may_set_either_matrix() {
        let ax = Figure::new().subplot_3d();
        ax.set_drawn_proj(Some(ax.proj()));
        assert_eq!(ax.drawn_proj_inverse(), None);
        assert!(ax.drawn_pane_point(0.0, 0.0).is_none(), "no inverse");
        assert!(ax.drawn_axis_position().is_some());
        ax.set_drawn_proj_inverse(ax.proj().inverse());
        assert_eq!(ax.drawn_pane_point(0.0, 0.0), ax.pane_point(0.0, 0.0));
        assert_eq!(ax.drawn_pane_point(0.0, 0.0).map(|p| p.1), Some(Axis3::Y));
        ax.set_drawn_proj(None);
        assert_eq!(ax.drawn_format_coord(0.0, 0.0), "");
    }

    #[test]
    fn explicit_projections_wait_for_a_draw() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0], [0.0, 1.0]).unwrap();
        assert_eq!(ax.do_3d_projection_as_drawn(&line).unwrap(), None);
        fig.canvas().draw().unwrap();
        let depth = ax.do_3d_projection_as_drawn(&line).unwrap();
        assert!(depth.is_some_and(f64::is_nan), "lines keep their z-order");
    }
}
