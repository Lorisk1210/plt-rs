//! The artists a change marked stale, reported to a program that asked
//! for them: matplotlib calls an artist's `stale_callback` each time the
//! artist is set stale, and a program that put callbacks of its own on
//! its artists (or a class of its own overriding `stale`) sees each
//! change as it happens. pltrs marks the containers of a changed artist
//! itself (see `stale`), so a figure reports only when asked
//! ([`Figure::report_stale`]), and only the artists changed, not the
//! containers their change reached; a change to the figure's own state
//! that stales it (a scale, the subplot parameters, the size) reports
//! the figure.

use std::cell::Cell;
use std::collections::BTreeSet;

use super::stale::FreshArtists;
use super::{Figure, FigureState};
use crate::artist::{ArtistId, ArtistLookup, RawHandle};

thread_local! {
    /// Whether a figure of the thread was ever asked to report.
    static ASKED: Cell<bool> = const { Cell::new(false) };
    /// Whether the thread's changes go unreported (see [`unreported`]).
    static UNREPORTED: Cell<bool> = const { Cell::new(false) };
}

/// Puts the thread's [`UNREPORTED`] back when dropped, also when the
/// changes panic.
struct Unreported(bool);

impl Drop for Unreported {
    fn drop(&mut self) {
        UNREPORTED.set(self.0);
    }
}

/// Runs `changes`, whose artists marked stale are reported to no one:
/// the program's own `artist.stale = True`, whose callbacks the program
/// runs itself.
///
/// ```
/// use pltrs::prelude::*;
/// let fig = Figure::new();
/// let line = fig.subplot().plot([0.0, 1.0], [1.0, 0.0])?;
/// fig.report_stale(true);
/// pltrs::figure::unreported(|| line.set_stale(true));
/// assert!(fig.take_stale_reports().is_empty() && line.stale());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn unreported<R>(changes: impl FnOnce() -> R) -> R {
    let _restored = Unreported(UNREPORTED.replace(true));
    changes()
}

/// Whether a figure of this thread was ever asked to report
/// ([`Figure::report_stale`]): then [`Figure::take_stale_reports`] of the
/// figures reporting may have something to hand out after each change.
pub fn stale_reports_asked() -> bool {
    ASKED.get()
}

/// What a figure reports: the artists marked stale since they were last
/// taken, and whether the figure was stale when they were.
#[derive(Debug, Default)]
pub(crate) struct StaleReports {
    artists: Vec<ArtistId>,
    figure_stale: bool,
}

impl FreshArtists {
    /// Notes that artist `id` was marked stale, if the figure reports; a
    /// change reported to no one counts as seen, the figure stale with it.
    pub(super) fn report(&mut self, id: ArtistId) {
        if let Some(reports) = &mut self.reports {
            if UNREPORTED.get() {
                reports.figure_stale = true;
            } else {
                reports.artists.push(id);
            }
        }
    }
}

impl FigureState {
    /// The artists reported since they were last taken, each once, in the
    /// order of their first report; those gone from the figure for good
    /// meanwhile are left out, and so are those containing another one
    /// reported: a change pltrs makes to an artist may touch what contains
    /// it, where matplotlib's sets the artist alone stale, whose callback
    /// stales the containers. The figure itself when it went stale with
    /// no artist reported (a change to its own state).
    fn take_reports(&mut self) -> Vec<ArtistId> {
        let stale = self.stale;
        let Some(reports) = &mut self.fresh.reports else {
            return Vec::new();
        };
        let mut taken = std::mem::take(&mut reports.artists);
        let was_stale = std::mem::replace(&mut reports.figure_stale, stale);
        let went_stale = stale && !was_stale;
        let mut seen = BTreeSet::new();
        taken.retain(|&id| seen.insert(id) && self.artist(id).is_some());
        let containers: BTreeSet<ArtistId> =
            (taken.iter()).flat_map(|&id| self.containers(id)).collect();
        taken.retain(|id| !containers.contains(id));
        if taken.is_empty() && went_stale {
            taken.push(self.root);
        }
        taken
    }

    /// The artists containing artist `id`, innermost first.
    fn containers(&self, id: ArtistId) -> impl Iterator<Item = ArtistId> + '_ {
        let parent = |id: ArtistId| self.artist(id).and_then(|artist| artist.base().parent);
        std::iter::successors(parent(id), move |&id| parent(id))
    }
}

impl Figure {
    /// Has the figure note, from now on (or no longer, when `on` is
    /// false), the artists a change marks stale, for the program to take
    /// with [`take_stale_reports`](Self::take_stale_reports): how a
    /// program sees each change to an artist, as matplotlib's
    /// `stale_callback` sees it.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// fig.report_stale(true);
    /// line.set_linewidth(2.0);
    /// line.set_linewidth(3.0);
    /// assert!(pltrs::figure::stale_reports_asked());
    /// let reported = fig.take_stale_reports();
    /// assert_eq!(reported.len(), 1, "once, not its containers");
    /// assert_eq!(reported[0].id(), line.raw().id());
    /// fig.canvas().draw()?;
    /// assert!(fig.take_stale_reports().is_empty());
    /// fig.set_size_inches(4.0, 3.0);
    /// assert_eq!(fig.take_stale_reports()[0].id(), fig.raw().id(), "the figure's own");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn report_stale(&self, on: bool) {
        let mut state = self.state_mut();
        let stale = state.stale;
        let reports = &mut state.fresh.reports;
        match (on, reports.is_some()) {
            (true, false) => {
                *reports = Some(StaleReports {
                    artists: Vec::new(),
                    figure_stale: stale,
                });
                ASKED.set(true);
            }
            (false, true) => *reports = None,
            _ => {}
        }
    }

    /// The artists marked stale since they were last taken, each once, in
    /// the order of their first change (see
    /// [`report_stale`](Self::report_stale)); nothing when the figure does
    /// not report.
    pub fn take_stale_reports(&self) -> Vec<RawHandle> {
        let ids = self.state_mut().take_reports();
        let cell = self.state_cell();
        ids.into_iter()
            .map(|id| RawHandle::new(std::rc::Rc::clone(cell), id))
            .collect()
    }
}
