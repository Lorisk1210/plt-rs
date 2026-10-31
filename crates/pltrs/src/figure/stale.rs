//! Whether artists changed since they were last drawn (matplotlib's
//! `Artist.stale`, and its propagation to the figure through
//! `stale_callback`).
//!
//! A figure keeps the artists drawn since they last changed, its *fresh*
//! artists: drawing an artist makes it fresh, and a change to it makes it
//! stale again, together with the artists containing it and the figure,
//! unless it is animated (figures leave animated artists out of their
//! draws) or removed. An artist never drawn, one added since the last
//! draw included, is stale.

use std::cell::RefCell;
use std::collections::BTreeSet;

use super::FigureState;
use crate::artist::{ArtistId, ArtistLookup};

/// The artists of a figure drawn since they last changed.
#[derive(Debug, Default)]
pub(crate) struct FreshArtists(RefCell<BTreeSet<ArtistId>>);

impl FigureState {
    /// Records that artist `id` was drawn (matplotlib's `self.stale =
    /// False` at the end of `draw`).
    pub(crate) fn mark_drawn(&self, id: ArtistId) {
        self.fresh.0.borrow_mut().insert(id);
    }

    /// Whether artist `id` changed since it was last drawn; for the root
    /// figure, whether anything in the figure did.
    pub(crate) fn is_stale(&self, id: ArtistId) -> bool {
        if id == self.root {
            self.stale
        } else {
            !self.fresh.0.borrow().contains(&id)
        }
    }

    /// Marks artist `id` as changed since it was last drawn and, unless
    /// it is animated or removed, the artists containing it and the
    /// figure (matplotlib's `artist.stale = True`).
    pub(crate) fn mark_stale(&mut self, id: ArtistId) {
        let fresh = self.fresh.0.get_mut();
        fresh.remove(&id);
        let Some(artist) = self.store.get(id) else {
            return;
        };
        if artist.base().animated {
            return;
        }
        let mut container = artist.base().parent;
        while let Some(id) = container {
            fresh.remove(&id);
            container = self.store.get(id).and_then(|artist| artist.base().parent);
        }
        self.stale = true;
    }

    /// Marks artist `id` as drawn as it is, without drawing it
    /// (matplotlib's `artist.stale = False`); the root figure counts as
    /// drawn as a whole.
    pub(crate) fn mark_fresh(&mut self, id: ArtistId) {
        if id == self.root {
            self.stale = false;
        }
        if self.artist(id).is_some() {
            self.fresh.0.get_mut().insert(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    /// Draws `fig` onto its raster canvas.
    fn draw(fig: &Figure) {
        fig.canvas().draw().expect("the figure draws");
    }

    #[test]
    fn drawing_freshens_and_changes_spread_to_the_figure() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("plotted");
        assert!(fig.stale() && ax.stale() && line.stale(), "never drawn");
        draw(&fig);
        assert!(!fig.stale() && !ax.stale() && !line.stale());
        line.set_linewidth(3.0);
        assert!(fig.stale() && ax.stale() && line.stale());
        draw(&fig);
        let text = ax.text(0.5, 0.5, "new");
        assert!(
            text.stale() && ax.stale() && fig.stale(),
            "adding stales the axes"
        );
    }

    #[test]
    fn animated_and_removed_artists_keep_the_figure_as_drawn() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("plotted");
        let other = ax.plot([0.0, 1.0], [0.0, 1.0]).expect("plotted");
        line.set_animated(true);
        draw(&fig);
        line.set_linewidth(3.0);
        assert!(line.stale() && !ax.stale() && !fig.stale());
        other.remove();
        assert!(ax.stale() && fig.stale(), "removing stales the container");
        draw(&fig);
        other.set_linewidth(2.0);
        assert!(other.stale() && !fig.stale());
    }

    #[test]
    fn staleness_can_be_set() {
        let fig = Figure::new();
        let ax = fig.subplot();
        draw(&fig);
        ax.set_stale(true);
        assert!(ax.stale() && fig.stale());
        fig.set_stale(false);
        ax.set_stale(false);
        assert!(!ax.stale() && !fig.stale());
    }
}
