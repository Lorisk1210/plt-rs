//! Artists a 3-D axes fails to draw. matplotlib accepts data its 3-D
//! artists cannot project (a line through points of two coordinates,
//! segments of 2-D points, ragged polygons) and 2-D collections and
//! patches lacking the projection its draw asks them for, and fails only
//! when it draws them: the problem is recorded with the artist in advance
//! ([`Axes3D::set_draw_failure`]) and met where matplotlib's draw meets
//! it.
//!
//! matplotlib's `Axes3D.draw` projects its visible collections and
//! patches (`do_3d_projection`) once its background is drawn and before
//! its axes are, and draws its lines and texts among its other artists.
//! A recorded problem is reported at the same place, and its artist is
//! left undrawn.

use std::collections::BTreeMap;
use std::fmt;

use super::error::Axes3DError;
use super::handle::Axes3D;
use super::state::Axes3DState;
use crate::artist::{
    ArtistHandle, ArtistId, ArtistLookup, ArtistStore, DrawContext, DrawErrorKind,
    draws_stop_at_first_problem,
};
use crate::collections::CollectionArtist;

/// A problem a 3-D axes meets when it draws one of its artists, recorded
/// with [`Axes3D::set_draw_failure`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawFailure {
    /// The kind of problem (after the exception matplotlib raises).
    pub kind: DrawErrorKind,
    /// What goes wrong, in matplotlib's words.
    pub message: String,
}

pltrs_core::archive_struct!(DrawFailure { kind, message });

impl fmt::Display for DrawFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DrawFailure {}

/// The problems recorded for the artists of a 3-D axes.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DrawFailures(BTreeMap<ArtistId, DrawFailure>);

pltrs_core::archive_struct!(DrawFailures(v0));

impl DrawFailures {
    /// The problem recorded for artist `id`.
    pub(crate) fn of(&self, id: ArtistId) -> Option<&DrawFailure> {
        self.0.get(&id)
    }

    /// Forgets the problems of the artists `store` no longer holds.
    pub(crate) fn retain_in(&mut self, store: &ArtistStore) {
        self.0.retain(|&id, _| store.contains(id));
    }
}

impl Axes3D {
    /// Records that drawing `artist`, an artist of these axes, meets
    /// `failure` (`None` forgets a problem recorded before): a draw of the
    /// axes reports it where matplotlib's draw meets it (projecting the
    /// visible collections and patches once the background is drawn,
    /// drawing the lines and texts among the other artists), and
    /// [`Axes3D::do_3d_projection`] returns it.
    ///
    /// ```
    /// use pltrs::artist::DrawErrorKind;
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::DrawFailure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// let line = ax.plot(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// let failure = DrawFailure {
    ///     kind: DrawErrorKind::InvalidType,
    ///     message: "too few coordinates".into(),
    /// };
    /// ax.set_draw_failure(&line, Some(failure.clone()))?;
    /// assert_eq!(ax.draw_failure(&line), Some(failure));
    /// let error = fig.canvas().draw().unwrap_err();
    /// assert!(error.to_string().contains("too few coordinates"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `artist` is not drawn on these axes.
    pub fn set_draw_failure(
        &self,
        artist: &impl ArtistHandle,
        failure: Option<DrawFailure>,
    ) -> Result<&Self, Axes3DError> {
        let id = self.plane_child(artist)?;
        self.with_mut(|axes| match failure {
            Some(failure) => axes.failures.0.insert(id, failure),
            None => axes.failures.0.remove(&id),
        });
        self.raw().figure_state().borrow_mut().stale = true;
        Ok(self)
    }

    /// The problem recorded for drawing `artist` (see
    /// [`Axes3D::set_draw_failure`]).
    pub fn draw_failure(&self, artist: &impl ArtistHandle) -> Option<DrawFailure> {
        let id = self.own_id(artist)?;
        self.with(|axes| axes.failures.of(id).cloned())
    }

    /// The id of `artist` when it is drawn on these axes.
    fn plane_child(&self, artist: &impl ArtistHandle) -> Result<ArtistId, Axes3DError> {
        let plane = self.plane().id();
        let id = self.own_id(artist).ok_or(Axes3DError::ForeignArtist)?;
        let on_plane = (self.raw().figure_state().borrow().artist(id))
            .is_some_and(|artist| artist.base().parent == Some(plane));
        on_plane.then_some(id).ok_or(Axes3DError::ForeignArtist)
    }
}

impl Axes3DState {
    /// The problem recorded for artist `id` of the plane when the axes
    /// projects it before drawing (a collection or a patch), else `None`.
    fn projection_failure<'a>(
        &'a self,
        store: &ArtistStore,
        id: ArtistId,
    ) -> Option<&'a DrawFailure> {
        let failure = self.failures.of(id)?;
        let artist = store.get(id)?;
        let projected =
            artist.as_patch().is_some() || store.get_as::<CollectionArtist>(id).is_some();
        projected.then_some(failure)
    }

    /// Whether a draw ends where it projects the plane's collections and
    /// patches, before it reaches the lines and texts it projects as it
    /// draws them: draws stop at their first problem, and one of the
    /// visible collections or patches has one recorded.
    pub(super) fn stops_projecting(&self, store: &ArtistStore) -> bool {
        draws_stop_at_first_problem()
            && self.failures.0.keys().any(|&id| {
                store.get(id).is_some_and(|a| a.base().visible)
                    && self.projection_failure(store, id).is_some()
            })
    }

    /// Reports the problems recorded for the visible collections and
    /// patches of the plane, in the order of its children: matplotlib's
    /// draw projects them before it draws the axes. Returns whether there
    /// was one.
    pub(super) fn report_projection_failures(
        &self,
        ctx: &mut DrawContext<'_>,
        children: &[ArtistId],
    ) -> bool {
        let failed: Vec<_> = {
            let layout = ctx.layout();
            let store = layout.store();
            children
                .iter()
                .filter(|&&id| store.get(id).is_some_and(|a| a.base().visible))
                .filter_map(|&id| Some((id, self.projection_failure(store, id)?.clone())))
                .collect()
        };
        let reported = !failed.is_empty();
        for (id, failure) in failed {
            ctx.report_kind(id, failure.kind, failure);
        }
        reported
    }

    /// Draws `id`, an artist of the plane, unless a problem was recorded
    /// for it: one met where it is drawn (a line or a text, which
    /// matplotlib's draw projects even when it is hidden) is reported
    /// there, one met projecting it was reported before.
    pub(super) fn draw_plane_child(&self, ctx: &mut DrawContext<'_>, id: ArtistId) {
        let Some(failure) = self.failures.of(id) else {
            ctx.draw_child(id);
            return;
        };
        let projected = self.projection_failure(ctx.layout().store(), id).is_some();
        if !projected && !ctx.ended() {
            ctx.report_kind(id, failure.kind, failure.clone());
        }
    }

    /// The problem met projecting artist `id` of the plane, which
    /// [`Axes3D::do_3d_projection`] returns.
    pub(super) fn failure_of(&self, id: ArtistId) -> Option<&DrawFailure> {
        self.failures.of(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::CollectionSpec;
    use crate::figure::Figure;
    use crate::mplot3d::Axis3;
    use crate::testing::RecordingRenderer;
    use crate::transforms::Point;

    #[test]
    fn a_failing_collection_ends_the_draw_before_lines_are_projected() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0], [0.0, 1.0]).expect("a line");
        let segment = vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)];
        let collection = ax
            .add_collection3d(CollectionSpec::lines(vec![segment]), &[0.0], Axis3::Z, true)
            .expect("a line collection");
        let failure = DrawFailure {
            kind: DrawErrorKind::OutOfRange,
            message: "index 2 is out of bounds".into(),
        };
        ax.set_draw_failure(&collection, Some(failure.clone()))
            .expect("drawn on the axes");
        assert_eq!(
            ax.do_3d_projection(&collection),
            Err(Axes3DError::Draw(failure))
        );
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        crate::artist::set_draws_stop_at_first_problem(true);
        let errors = fig.draw(&mut renderer).unwrap_err().0;
        crate::artist::set_draws_stop_at_first_problem(false);
        assert_eq!(errors[0].kind, DrawErrorKind::OutOfRange);
        assert_eq!(ax.projected_line_mask(&line), Ok(None), "never projected");
        ax.set_draw_failure(&collection, None)
            .expect("drawn on the axes");
        fig.draw(&mut renderer).expect("nothing fails");
        assert!(matches!(ax.projected_line_mask(&line), Ok(Some(_))));
        let foreign = Figure::new().subplot_3d();
        assert_eq!(
            foreign.set_draw_failure(&collection, None).err(),
            Some(Axes3DError::ForeignArtist)
        );
    }
}
