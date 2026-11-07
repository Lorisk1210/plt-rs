//! 2-D patches and collections of the program's classes that project
//! themselves onto a 3-D axes (matplotlib's `do_3d_projection` defined by
//! a subclass, such as the `Arrow3D` of many programs: a
//! `FancyArrowPatch` placing its ends where its 3-D points project).
//!
//! matplotlib's `Axes3D.draw` asks every visible collection and patch for
//! its depth (`do_3d_projection`) and, with computed z-order, numbers them
//! from the deepest. pltrs (Rust) projects its own 3-D artists in the
//! layout pass; such an artist is projected by the program once the
//! figure is laid out, before it is drawn, and the depth it returns
//! ([`Axes3D::set_program_depth`]) orders it among the others.

use super::art3d::{Art3D, SortKind};
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::state::Axes3DState;
use crate::artist::{ArtistHandle, ArtistId, ArtistStore};
use crate::collections::CollectionArtist;

impl Axes3D {
    /// Has `artist`, a 2-D patch or collection drawn on these axes whose
    /// class projects it, ordered by the depth its projection returns
    /// ([`Axes3D::set_program_depth`]) among the collections or patches
    /// of the axes when their z-order is computed.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::patches::Rectangle;
    /// let ax = Figure::new().subplot_3d();
    /// let patch = ax.projection_plane().add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0));
    /// ax.project_by_program(&patch)?;
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `artist` is not drawn on these axes, or is a 3-D artist of
    /// pltrs (Rust)'s.
    pub fn project_by_program(&self, artist: &impl ArtistHandle) -> Result<(), Axes3DError> {
        let id = self.plane_child(artist)?;
        if self.with(|axes| axes.artists.iter().any(|a| a.id == id)) {
            return Err(Axes3DError::ForeignArtist);
        }
        let kind = if artist.raw().is::<CollectionArtist>() {
            SortKind::Collection
        } else {
            SortKind::Patch
        };
        let art = Art3D::Program {
            kind,
            depth: f64::NAN,
        };
        self.register(id, art, false);
        Ok(())
    }

    /// Takes `depth`, what the projection of `artist` (see
    /// [`Axes3D::project_by_program`]) returned in the draw coming, and
    /// orders the collections and patches of the axes again with it.
    ///
    /// # Errors
    ///
    /// When `artist` is not projected by the program on these axes.
    pub fn set_program_depth(
        &self,
        artist: &impl ArtistHandle,
        depth: f64,
    ) -> Result<(), Axes3DError> {
        let id = self.own_id(artist).ok_or(Axes3DError::Not3DArtist)?;
        let taken = self
            .raw()
            .with_others::<Axes3DState, _>(|axes, store| axes.take_program_depth(id, depth, store));
        taken.then_some(()).ok_or(Axes3DError::Not3DArtist)
    }
}

impl Axes3DState {
    /// [`Axes3D::set_program_depth`] of artist `id`: whether it is
    /// projected by the program.
    fn take_program_depth(&mut self, id: ArtistId, depth: f64, store: &mut ArtistStore) -> bool {
        let Some(Art3D::Program { depth: kept, .. }) = (self.artists.iter_mut())
            .find(|artist| artist.id == id)
            .map(|artist| &mut artist.art)
        else {
            return false;
        };
        *kept = depth;
        if let Some(entry) = self.depth_order.iter_mut().find(|entry| entry.0 == id) {
            entry.2 = depth;
            self.order_by_depth(store);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::mplot3d::Axes3DError;
    use crate::patches::Rectangle;
    use crate::testing::RecordingRenderer;

    #[test]
    fn a_patch_projected_by_the_program_is_ordered_by_its_depth() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let plane = ax.plane();
        let near = plane.add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0));
        let far = plane.add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0));
        ax.project_by_program(&near).expect("on the axes");
        ax.project_by_program(&far).expect("on the axes");
        assert_eq!(
            ax.project_by_program(&far),
            Err(Axes3DError::ForeignArtist),
            "once"
        );
        fig.draw(&mut RecordingRenderer::new(640.0, 480.0, 100.0))
            .expect("drawn");
        ax.set_program_depth(&near, -1.0).expect("projected");
        ax.set_program_depth(&far, 1.0).expect("projected");
        assert!(far.zorder() < near.zorder(), "the deepest is drawn first");
        ax.set_program_depth(&near, 2.0).expect("projected");
        assert!(near.zorder() < far.zorder());
        let line = ax.plot([0.0, 1.0], [0.0, 1.0], [0.0, 1.0]).expect("a line");
        assert_eq!(
            ax.set_program_depth(&line, 0.0),
            Err(Axes3DError::Not3DArtist)
        );
        let foreign = Figure::new().subplot_3d();
        assert_eq!(
            foreign.project_by_program(&near),
            Err(Axes3DError::ForeignArtist)
        );
    }
}
