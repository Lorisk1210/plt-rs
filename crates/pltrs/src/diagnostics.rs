//! Counts of what pltrs keeps alive in the whole process, for finding
//! leaks: the figures' states (figures, and the holders of artists made
//! outside any figure) and the removed artists figures keep for their
//! handles.
//!
//! A program that makes, draws and drops figures in a loop must come back
//! to the same counts after each figure is dropped; one that updates a
//! figure in place must not keep more and more removed artists. The counts
//! are process-wide (every thread's figures) and cost one atomic addition
//! where a figure or a removed artist comes or goes.
//!
//! ```
//! use pltrs::diagnostics::live_figure_states;
//! use pltrs::figure::Figure;
//!
//! let before = live_figure_states();
//! let figure = Figure::new();
//! assert!(live_figure_states() > before);
//! drop(figure);
//! ```

use std::sync::atomic::{AtomicUsize, Ordering};

/// The figure states alive.
static FIGURE_STATES: AtomicUsize = AtomicUsize::new(0);

/// The removed artists kept by figures alive.
static REMOVED_ARTISTS: AtomicUsize = AtomicUsize::new(0);

/// How many figure states are alive in the process: one per figure, and
/// one per holder of an artist made outside any figure.
#[must_use]
pub fn live_figure_states() -> usize {
    FIGURE_STATES.load(Ordering::Relaxed)
}

/// How many removed artists the figures alive keep for their handles
/// (see [`ArtistHandle::remove`](crate::artist::ArtistHandle::remove)).
#[must_use]
pub fn kept_removed_artists() -> usize {
    REMOVED_ARTISTS.load(Ordering::Relaxed)
}

/// Counts a figure state made (`made`) or dropped.
pub(crate) fn count_figure_state(made: bool) {
    count(&FIGURE_STATES, made, 1);
}

/// Counts `count` removed artists kept (`kept`) or let go.
pub(crate) fn count_removed_artists(kept: bool, artists: usize) {
    count(&REMOVED_ARTISTS, kept, artists);
}

fn count(counter: &AtomicUsize, up: bool, by: usize) {
    if up {
        counter.fetch_add(by, Ordering::Relaxed);
    } else {
        counter.fetch_sub(by, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    /// The counts are process-wide and the tests run in parallel, so each
    /// test reads its own figure's effect through a lower bound.
    #[test]
    fn a_figure_counts_while_it_lives() {
        let figure = Figure::new();
        assert!(live_figure_states() >= 1);
        drop(figure);
    }

    #[test]
    fn removed_artists_count_while_their_figure_keeps_them() {
        let figure = Figure::new();
        let axes = figure.add_subplot(1, 1, 1).expect("one subplot");
        let line = axes.plot([0.0, 1.0], [1.0, 0.0]).expect("a line");
        line.remove();
        assert!(kept_removed_artists() >= 1);
        drop((line, axes, figure));
    }
}
