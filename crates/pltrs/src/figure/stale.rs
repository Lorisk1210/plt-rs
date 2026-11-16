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
//!
//! The figure also counts the draws of each artist, which tells what
//! matplotlib's `draw` methods compute as they draw (an annotation's
//! transform, a spine's path) from what they hold before a first draw,
//! and keeps the tick its last draw ended at (see `drawn_changes`).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

use pltrs_core::archive::{Archive, ArchiveError, Decoder, Encoder};

use super::FigureState;
use super::drawn_changes::{drawn_after_change, next_draw_tick};
use crate::artist::{ArtistId, ArtistLookup};

thread_local! {
    /// Whether the thread's changes to artists go unseen (see [`unseen`]).
    static UNSEEN: Cell<bool> = const { Cell::new(false) };
}

/// Puts the thread's [`UNSEEN`] back when dropped, also when the changes
/// panic.
struct Unseen(bool);

impl Drop for Unseen {
    fn drop(&mut self) {
        UNSEEN.set(self.0);
    }
}

/// Runs `changes`, whose changes to artists make nothing stale: they hand
/// a figure state its program already counts as the artists' (matplotlib
/// keeps an artist's state in arrays and lists the program changes in
/// place, which makes no artist stale; pltrs is handed such a change when
/// it next reads the artist).
///
/// ```
/// use pltrs::prelude::*;
/// let fig = Figure::new();
/// let line = fig.subplot().plot([0.0, 1.0], [1.0, 0.0])?;
/// fig.canvas().draw()?;
/// pltrs::figure::unseen(|| line.set_data(vec![0.0, 1.0], vec![2.0, 3.0]));
/// assert!(!line.stale() && !fig.is_stale());
/// line.set_data(vec![0.0, 1.0], vec![1.0, 0.0]);
/// assert!(line.stale() && fig.is_stale());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn unseen<R>(changes: impl FnOnce() -> R) -> R {
    let _restored = Unseen(UNSEEN.replace(true));
    changes()
}

/// The artists of a figure drawn since they last changed, and the draws
/// of each artist.
#[derive(Debug, Default)]
pub(crate) struct FreshArtists {
    fresh: RefCell<BTreeSet<ArtistId>>,
    draws: RefCell<BTreeMap<ArtistId, Draws>>,
}

/// The draws of an artist: how many, and the tick the last ended at.
#[derive(Clone, Copy, Debug, Default)]
struct Draws {
    count: u64,
    last_tick: u64,
}

impl Archive for FreshArtists {
    /// The fresh artists and how often each artist was drawn, as a pickle
    /// keeps the `stale` flags of matplotlib's artists and what their draws
    /// computed; the tick a draw ended at belongs to the process that drew,
    /// and is not kept.
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        self.fresh.borrow().encode(encoder)?;
        let counts: BTreeMap<ArtistId, u64> = (self.draws.borrow().iter())
            .map(|(&id, draws)| (id, draws.count))
            .collect();
        counts.encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        let fresh = BTreeSet::decode(decoder)?;
        let counts = BTreeMap::<ArtistId, u64>::decode(decoder)?;
        let draws = (counts.into_iter())
            .map(|(id, count)| {
                let draws = Draws {
                    count,
                    last_tick: 0,
                };
                (id, draws)
            })
            .collect();
        Ok(Self {
            fresh: RefCell::new(fresh),
            draws: RefCell::new(draws),
        })
    }
}

impl FreshArtists {
    /// How many times artist `id` was drawn, by the figure or alone.
    pub(crate) fn draw_count(&self, id: ArtistId) -> u64 {
        (self.draws.borrow().get(&id)).map_or(0, |draws| draws.count)
    }

    /// Forgets the artists `ids`, gone from the figure for good.
    pub(crate) fn forget(&mut self, ids: impl IntoIterator<Item = ArtistId>) {
        let (fresh, draws) = (self.fresh.get_mut(), self.draws.get_mut());
        for id in ids {
            fresh.remove(&id);
            draws.remove(&id);
        }
    }
}

impl FigureState {
    /// Records that artist `id` was drawn: it counts one draw more and,
    /// when `fresh`, is fresh (matplotlib's `self.stale = False` at the end
    /// of `draw`, which some `draw` methods skip).
    pub(crate) fn mark_drawn(&self, id: ArtistId, fresh: bool) {
        if fresh {
            self.fresh.fresh.borrow_mut().insert(id);
        }
        let mut draws = self.fresh.draws.borrow_mut();
        let draws = draws.entry(id).or_default();
        draws.count += 1;
        draws.last_tick = next_draw_tick();
    }

    /// How many times artist `id` was drawn, by the figure or alone.
    pub(crate) fn draw_count(&self, id: ArtistId) -> u64 {
        self.fresh.draw_count(id)
    }

    /// Whether artist `id` changed since it was last drawn; for the root
    /// figure, whether anything in the figure did.
    pub(crate) fn is_stale(&self, id: ArtistId) -> bool {
        if id == self.root {
            self.stale
        } else {
            !self.fresh.fresh.borrow().contains(&id)
        }
    }

    /// Marks artist `id` as changed since it was last drawn and, unless
    /// it is animated or removed, the artists containing it and the
    /// figure (matplotlib's `artist.stale = True`). A change the program's
    /// code asked for while a figure was drawn leaves the artists whose
    /// draw ended after it fresh (see `drawn_changes`), and one made
    /// [`unseen`] marks nothing.
    pub(crate) fn mark_stale(&mut self, id: ArtistId) {
        if UNSEEN.get() {
            return;
        }
        let FreshArtists { fresh, draws } = &mut self.fresh;
        let (fresh, draws) = (fresh.get_mut(), draws.get_mut());
        let mut changed = |id: ArtistId| {
            let kept = draws
                .get(&id)
                .is_some_and(|draws| drawn_after_change(draws.last_tick));
            if !kept {
                fresh.remove(&id);
            }
        };
        changed(id);
        let Some(artist) = self.store.get(id) else {
            return;
        };
        if artist.base().animated {
            return;
        }
        let mut container = artist.base().parent;
        while let Some(id) = container {
            changed(id);
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
            self.fresh.fresh.get_mut().insert(id);
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
    fn marking_artists_animated_stales_nothing() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("plotted");
        draw(&fig);
        line.set_animated(true);
        assert!(!line.stale() && !ax.stale() && !fig.stale());
        line.set_animated(false);
        assert!(
            !line.stale() && !ax.stale() && !fig.stale(),
            "matplotlib's set_animated only calls pchanged"
        );
        line.set_linewidth(2.0);
        assert!(line.stale() && fig.stale(), "other changes still do");
    }

    #[test]
    fn draws_are_counted_for_the_artists_drawn() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("plotted");
        let hidden = ax.plot([0.0, 1.0], [0.0, 1.0]).expect("plotted");
        hidden.set_visible(false);
        assert_eq!(line.raw().draw_count(), 0, "never drawn");
        draw(&fig);
        line.set_linewidth(3.0);
        draw(&fig);
        assert_eq!(line.raw().draw_count(), 2, "changes keep the count");
        assert_eq!(
            hidden.raw().draw_count(),
            0,
            "a hidden artist draws nothing"
        );
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
