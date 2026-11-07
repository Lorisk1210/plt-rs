//! Artists removed from a 3-D axes and added back to it.
//!
//! matplotlib's 3-D artists keep their 3-D data (`_verts3d`,
//! `_offsets3d`, `_segments3d`, ...) when they are removed, so an artist
//! added back is projected and drawn as before. A removed artist of
//! pltrs keeps its id and its 2-D artist; its 3-D geometry is set aside
//! by the first layout pass after the removal (and forgotten once the
//! artist is in the figure elsewhere), and taken back when the artist is
//! added to the axes again ([`Axes3D::restore_removed`]).

use super::handle::Axes3D;
use super::state::Axes3DState;
use crate::artist::{ArtistHandle, ArtistId, ArtistLookup, ArtistStore, RawHandle};
use crate::figure::FigureState;

impl Axes3DState {
    /// Sets aside the 3-D geometry of the artists removed from the
    /// projection plane, which `apart` names (the figure keeps them apart
    /// from its artists), and forgets that of the artists no longer
    /// removed (added elsewhere).
    pub(crate) fn set_aside_removed(&mut self, store: &ArtistStore, apart: &[ArtistId]) {
        let (kept, removed): (Vec<_>, Vec<_>) = std::mem::take(&mut self.artists)
            .into_iter()
            .partition(|artist| store.contains(artist.id));
        self.artists = kept;
        self.removed.retain(|artist| apart.contains(&artist.id));
        self.removed.extend(
            removed
                .into_iter()
                .filter(|artist| apart.contains(&artist.id)),
        );
    }
}

/// The 3-D artists of the 3-D axes `id`, drawn or set aside, that the
/// figure keeps apart from its artists: removed, but still held (see
/// [`Axes3DState::set_aside_removed`]).
pub(crate) fn removed_artists(figure: &FigureState, id: ArtistId) -> Vec<ArtistId> {
    figure
        .artist_as::<Axes3DState>(id)
        .map(|axes| {
            (axes.artists.iter().chain(&axes.removed))
                .map(|artist| artist.id)
                .filter(|&artist| !figure.is_attached(artist) && figure.artist(artist).is_some())
                .collect()
        })
        .unwrap_or_default()
}

impl Axes3D {
    /// Draws `artist`, an artist removed from these axes and added back
    /// to their projection plane, with the 3-D geometry it had (as
    /// matplotlib's 3-D artists keep their 3-D data), when the axes set
    /// it aside; returns whether they had. An artist added back before a
    /// layout pass set its geometry aside keeps it as it is.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::figure::Figure;
    /// use pltrs::axes::AddAs;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// let line = ax.plot(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// fig.layout();
    /// line.remove();
    /// fig.layout();
    /// ax.projection_plane().adopt_artist(line.raw(), AddAs::Artist)?;
    /// assert!(ax.restore_removed(line.raw()));
    /// assert!(ax.has_data());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn restore_removed(&self, artist: &RawHandle) -> bool {
        let id = artist.id();
        let plane = self.plane().id();
        let on_plane = std::rc::Rc::ptr_eq(self.raw().figure_state(), artist.figure_state())
            && (artist.figure_state().borrow().store.get(id))
                .is_some_and(|added| added.base().parent == Some(plane));
        on_plane
            && self.with_mut(|axes| {
                let Some(index) = axes.removed.iter().position(|kept| kept.id == id) else {
                    return false;
                };
                let kept = axes.removed.remove(index);
                axes.artists.push(kept);
                true
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::axes::AddAs;
    use crate::figure::Figure;
    use crate::mplot3d::Axes3D;

    /// A 3-D axes with a line removed from it after a draw, which set its
    /// geometry aside.
    fn removed_line() -> (Figure, Axes3D, crate::lines::Line2D) {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let line = ax.plot([0.0, 1.0], [0.0, 2.0], [0.0, 3.0]).expect("a line");
        fig.layout();
        line.remove();
        fig.layout();
        (fig, ax, line)
    }

    #[test]
    fn removed_artists_lose_their_geometry_until_added_back() {
        let (fig, ax, line) = removed_line();
        assert!(!ax.has_data());
        assert_eq!(ax.with(|axes| axes.removed.len()), 1);
        ax.projection_plane()
            .adopt_artist(line.raw(), AddAs::Artist)
            .expect("added back");
        assert!(ax.restore_removed(line.raw()));
        assert!(ax.has_data());
        assert!(!ax.restore_removed(line.raw()), "restored once");
        fig.layout();
        assert!(
            line.xdata().iter().any(|x| *x != 0.0),
            "projected again by the draw"
        );
    }

    #[test]
    fn artists_not_added_back_keep_their_geometry_aside() {
        let (_fig, ax, line) = removed_line();
        assert!(!ax.restore_removed(line.raw()), "still removed");
        assert_eq!(ax.with(|axes| axes.removed.len()), 1);
    }

    #[test]
    fn artists_added_elsewhere_are_not_restored() {
        let (fig, ax, line) = removed_line();
        let other = fig.subplot();
        other
            .adopt_artist(line.raw(), AddAs::Artist)
            .expect("added to a 2-D axes");
        assert!(!ax.restore_removed(line.raw()));
        fig.layout();
        assert!(
            ax.with(|axes| axes.removed.is_empty()),
            "forgotten once it is in the figure again"
        );
    }
}
