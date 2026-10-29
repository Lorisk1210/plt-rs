//! Hiding the data outside the view limits (matplotlib 3.10's
//! `axlim_clip`).
//!
//! The plotting methods taking options have an `axlim_clip` field; this
//! module lets any 3-D artist be clipped to the view limits after the
//! fact, for the methods that take no options (`plot`, `text`, `bar` and
//! `add_collection3d`) as for the others.

use std::rc::Rc;

use super::error::Axes3DError;
use super::handle::Axes3D;
use crate::artist::{ArtistHandle, ArtistId};

impl Axes3D {
    /// Hides the parts of a 3-D artist of this axes that lie outside the
    /// view limits, or shows them again (matplotlib's `axlim_clip`): line
    /// points, markers and texts outside the limits at draw time
    /// disappear, and so do the segments and polygon edges reaching them.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let line = ax.plot(&[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0])?;
    /// ax.set_axlim_clip(&line, true)?.set_xlim(0.0, 1.0)?;
    /// assert_eq!(ax.axlim_clip(&line), Some(true));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `artist` is not a 3-D artist of this axes (a text placed with
    /// [`Axes3D::text2d`], a contour set rather than its collections, an
    /// artist of another axes, ...).
    pub fn set_axlim_clip(
        &self,
        artist: &impl ArtistHandle,
        axlim_clip: bool,
    ) -> Result<&Self, Axes3DError> {
        let id = self.own_id(artist);
        let found = self.with_mut(|axes| {
            let entry = axes.artists.iter_mut().find(|a| Some(a.id) == id)?;
            entry.axlim_clip = axlim_clip;
            Some(())
        });
        found.map(|()| self).ok_or(Axes3DError::Not3DArtist)
    }

    /// Whether a 3-D artist of this axes hides its parts outside the view
    /// limits; `None` when `artist` is not one.
    pub fn axlim_clip(&self, artist: &impl ArtistHandle) -> Option<bool> {
        let id = self.own_id(artist)?;
        self.with(|axes| {
            axes.artists
                .iter()
                .find(|a| a.id == id)
                .map(|a| a.axlim_clip)
        })
    }

    /// The id of `artist` if it belongs to the figure of this axes.
    pub(super) fn own_id(&self, artist: &impl ArtistHandle) -> Option<ArtistId> {
        Rc::ptr_eq(self.raw().figure_state(), artist.raw().figure_state()).then(|| artist.id())
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::collections::CollectionArtist;
    use crate::figure::Figure;
    use crate::lines::LineArtist;
    use crate::mplot3d::{Contour3DOptions, Errorbar3DOptions, Scatter3DOptions, Stem3DOptions};
    use crate::numeric::Grid;

    /// The projected x data of a line after a draw.
    fn projected_x(line: &crate::lines::Line2D) -> Vec<f64> {
        line.raw().with::<LineArtist, _>(|l| l.xdata.clone())
    }

    #[test]
    fn lines_hide_their_points_outside_the_view_limits() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let values = [0.0, 1.0, 2.0];
        let line = ax.plot(values, values, values).expect("valid line");
        ax.set_xlim(-0.5, 1.5).expect("valid limits");
        fig.to_png().expect("drawable");
        assert!(projected_x(&line).iter().all(|x| x.is_finite()));
        assert_eq!(ax.axlim_clip(&line), Some(false));
        ax.set_axlim_clip(&line, true).expect("a 3-D line");
        fig.to_png().expect("drawable");
        let x = projected_x(&line);
        assert!(x[0].is_finite() && x[1].is_finite());
        assert!(x[2].is_nan(), "the point beyond x = 1.5 is hidden");
    }

    #[test]
    fn only_3d_artists_of_the_axes_can_be_clipped() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let label = ax.text2d(0.1, 0.1, "2-D");
        assert_eq!(
            ax.set_axlim_clip(&label, true).err(),
            Some(Axes3DError::Not3DArtist)
        );
        assert_eq!(ax.axlim_clip(&label), None);
        let other = Figure::new().subplot_3d();
        let line = other.plot([0.0], [0.0], [0.0]).expect("valid line");
        assert!(ax.set_axlim_clip(&line, true).is_err());
        let text = ax.text(0.0, 0.0, 0.0, "3-D", None);
        assert!(ax.set_axlim_clip(&text, true).is_ok());
        assert_eq!(ax.axlim_clip(&text), Some(true));
    }

    #[test]
    fn hidden_markers_drop_out_of_the_depth_shading() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let options = Scatter3DOptions {
            axlim_clip: true,
            ..Scatter3DOptions::default()
        };
        let points = ax
            .scatter_with([0.0, 0.5, 9.0], [0.0, 0.5, 0.0], [0.0, 0.5, 0.0], options)
            .expect("valid scatter");
        ax.set_xlim(-1.0, 1.0).expect("valid limits");
        fig.to_png().expect("drawable");
        let offsets = points
            .raw()
            .with::<CollectionArtist, _>(|c| c.offsets.clone());
        // Hidden markers come first, like matplotlib's masked depths.
        assert!(offsets[0].x.is_nan());
        assert!(offsets[1..].iter().all(|o| o.is_finite()));
    }

    #[test]
    fn errorbars_and_stems_clip_what_matplotlib_clips() {
        let ax = Figure::new().subplot_3d();
        let options = Errorbar3DOptions {
            errors: [None, None, Some(0.5.into())],
            lolims: [Vec::new(), Vec::new(), vec![true, false]],
            axlim_clip: true,
            ..Errorbar3DOptions::default()
        };
        let bars = ax
            .errorbar([0.0, 1.0], [0.0, 1.0], [0.0, 1.0], options)
            .expect("valid error bars");
        let data_line = bars.data_line.expect("a data line");
        assert_eq!(ax.axlim_clip(&data_line), Some(true));
        assert!(bars.errlines.iter().all(|c| ax.axlim_clip(c) == Some(true)));
        assert!(bars.caplines.iter().all(|c| ax.axlim_clip(c) == Some(true)));
        let arrows: Vec<Option<bool>> =
            bars.limit_arrows.iter().map(|c| ax.axlim_clip(c)).collect();
        assert_eq!(arrows, [Some(false)]);
        let options = Stem3DOptions {
            axlim_clip: true,
            ..Stem3DOptions::default()
        };
        let stem = ax
            .stem([0.0, 1.0], [0.0, 1.0], [1.0, 2.0], options)
            .expect("valid stems");
        assert_eq!(ax.axlim_clip(&stem.stemlines), Some(true));
        assert_eq!(ax.axlim_clip(&stem.markerline), Some(false));
        assert_eq!(ax.axlim_clip(&stem.baseline), Some(false));
    }

    #[test]
    fn contour_ribbons_are_never_clipped() {
        let ax = Figure::new().subplot_3d();
        let x = Grid::from_fn(4, 4, |_, j| j as f64);
        let y = Grid::from_fn(4, 4, |i, _| i as f64);
        let z = Grid::from_fn(4, 4, |i, j| (i * j) as f64);
        let lines = Contour3DOptions {
            axlim_clip: true,
            ..Contour3DOptions::default()
        };
        ax.contour(&x, &y, &z, lines).expect("valid contours");
        assert!(ax.with(|axes| axes.artists.iter().all(|a| a.axlim_clip)));
        let ribbons = Contour3DOptions {
            extend3d: NonZeroUsize::new(2),
            axlim_clip: true,
            ..Contour3DOptions::default()
        };
        let before = ax.with(|axes| axes.artists.len());
        ax.contour(&x, &y, &z, ribbons).expect("valid ribbons");
        let after = ax.with(|axes| axes.artists[before..].iter().all(|a| !a.axlim_clip));
        assert!(after);
    }
}
