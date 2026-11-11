//! Axes of different figures sharing an axis.
//!
//! matplotlib keeps the axes sharing an axis in one `Grouper` for all
//! figures, so `ax2.sharex(ax1)` joins axes of two figures as it joins
//! those of one: `ax2` takes the tick helpers, the scale and the view of
//! `ax1`, and from then on limits, scales and tick helpers set on one
//! reach the other, each autoscales over the data of both, and each
//! figure is laid out and drawn by itself.
//!
//! A figure's state holds the groups of its own axes
//! ([`ShareGroups`](super::ShareGroups)). What joins them to axes of
//! other figures are *links*, kept here: each of the two axes joined
//! records the other as its [`Peer`], in its own figure. The group of an
//! axes is then its figure's group and, through their links, the groups
//! of its peers in the other figures ([`FigureState::peers`]). The code
//! that keeps a group in step (`axes::view`) reaches the peers after the
//! axes of its own figure: it borrows each other figure for the change,
//! and passes over one that code further up the call stack uses.
//!
//! Links hold their figures weakly and are not archived: a program
//! archiving figures that share axes joins them again
//! ([`Axes::join_across`]), as pltrs (Python) does for the axes of other
//! figures a pickle holds.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use super::axis::AxisState;
use super::state::SharedData;
use super::{Axes, AxesState, AxisName, PlotError, run_limit_callbacks};
use crate::artist::{ArtistHandle, ArtistId, ArtistLookup, RawHandle};
use crate::figure::{FigureCell, FigureState};

pltrs_core::thread_state! {
    /// The figures whose axes got limits from a peer, with callbacks of
    /// theirs waiting: they run after those of the figure that changed
    /// (see [`run_peers_callbacks`]).
    static NOTIFIED: RefCell<Vec<Weak<FigureCell>>> = const { RefCell::new(Vec::new()) };
}

/// An axes of another figure sharing an axis with one of this figure's.
#[derive(Clone, Debug)]
pub(crate) struct Peer {
    figure: Weak<FigureCell>,
    axes: ArtistId,
}

impl Peer {
    /// The axes `axes` of `figure`.
    fn new(figure: &Rc<FigureCell>, axes: ArtistId) -> Self {
        Self {
            figure: Rc::downgrade(figure),
            axes,
        }
    }

    /// Whether this is the axes `axes` of `figure`.
    fn is(&self, figure: &Rc<FigureCell>, axes: ArtistId) -> bool {
        self.axes == axes && std::ptr::eq(self.figure.as_ptr(), Rc::as_ptr(figure))
    }
}

/// One axes of this figure joined to one of another.
#[derive(Clone, Debug)]
struct Link {
    name: AxisName,
    axes: ArtistId,
    peer: Peer,
    /// Whether `axes` took the axis of the peer (matplotlib's `_sharex`
    /// and `_sharey` of `axes` are then the peer).
    follows: bool,
}

/// The links of a figure's axes to axes of other figures.
#[derive(Debug, Default)]
pub(crate) struct SharesAcross {
    /// The figure itself, once it has links: what its peers hold of it.
    me: Weak<FigureCell>,
    links: Vec<Link>,
}

impl SharesAcross {
    /// Whether no axes of the figure shares with one of another.
    fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    /// The peers axis `name` of `axes` is linked to.
    fn peers_of(&self, name: AxisName, axes: ArtistId) -> impl Iterator<Item = &Peer> {
        (self.links.iter())
            .filter(move |link| link.name == name && link.axes == axes)
            .map(|link| &link.peer)
    }

    /// The peer whose axis `name` `axes` took, if it took one.
    fn followed(&self, name: AxisName, axes: ArtistId) -> Option<&Peer> {
        (self.links.iter())
            .find(|link| link.follows && link.name == name && link.axes == axes)
            .map(|link| &link.peer)
    }

    /// Links axis `name` of `axes` to `peer`, once.
    fn link(&mut self, name: AxisName, axes: ArtistId, peer: Peer, follows: bool) {
        let same = |link: &Link| {
            link.name == name
                && link.axes == axes
                && link.peer.axes == peer.axes
                && Weak::ptr_eq(&link.peer.figure, &peer.figure)
        };
        match self.links.iter_mut().find(|link| same(link)) {
            Some(link) => link.follows |= follows,
            None => self.links.push(Link {
                name,
                axes,
                peer,
                follows,
            }),
        }
    }

    /// Takes the links of `axes` out.
    fn take(&mut self, axes: ArtistId) -> Vec<Link> {
        let (taken, kept) = std::mem::take(&mut self.links)
            .into_iter()
            .partition(|link| link.axes == axes);
        self.links = kept;
        taken
    }

    /// Forgets the links to `peer`, or hands them to `heir` (another axes
    /// of the peer's figure) when given.
    fn repoint(&mut self, name: AxisName, peer: &Peer, heir: Option<ArtistId>) {
        let to_peer = |link: &Link| {
            link.name == name
                && link.peer.axes == peer.axes
                && Weak::ptr_eq(&link.peer.figure, &peer.figure)
        };
        match heir {
            Some(heir) => {
                for link in self.links.iter_mut().filter(|link| to_peer(link)) {
                    link.peer.axes = heir;
                }
            }
            None => self.links.retain(|link| !to_peer(link)),
        }
    }
}

/// The axes of one other figure in a group.
type PeerAxes = (Rc<FigureCell>, Vec<ArtistId>);

impl FigureState {
    /// The axes of other figures sharing axis `name` with `axes`, by
    /// figure: the peers its group is linked to, with the groups of
    /// their figures and those these are linked to in turn. A figure that
    /// code further up the call stack changes is passed over, as is this
    /// one (its axes in the group are the [`siblings`](Self::siblings)).
    pub(crate) fn peers(&self, name: AxisName, axes: ArtistId) -> Vec<PeerAxes> {
        if self.across.is_empty() {
            return Vec::new();
        }
        let mut found: Vec<PeerAxes> = Vec::new();
        let mut pending: Vec<Peer> = (self.siblings(name, axes).into_iter())
            .flat_map(|sibling| self.across.peers_of(name, sibling))
            .cloned()
            .collect();
        while let Some(peer) = pending.pop() {
            let Some(cell) = peer.figure.upgrade() else {
                continue;
            };
            let known = found
                .iter()
                .position(|(figure, _)| Rc::ptr_eq(figure, &cell));
            if known.is_some_and(|at| found[at].1.contains(&peer.axes)) {
                continue;
            }
            let Some(state) = cell.try_borrow() else {
                continue;
            };
            if std::ptr::eq(&raw const *state, self) {
                continue;
            }
            let group = state.siblings(name, peer.axes);
            pending.extend(
                (group.iter())
                    .flat_map(|&member| state.across.peers_of(name, member))
                    .cloned(),
            );
            drop(state);
            match known {
                Some(at) => {
                    let ids = &mut found[at].1;
                    for id in group {
                        if !ids.contains(&id) {
                            ids.push(id);
                        }
                    }
                }
                None => found.push((cell, group)),
            }
        }
        found
    }

    /// Applies `update` to axis `name` of the axes of other figures
    /// sharing it with `axes` (see
    /// [`update_shared_axis`](Self::update_shared_axis)); their figures
    /// are to be drawn again.
    pub(super) fn update_peers(
        &self,
        axes: ArtistId,
        name: AxisName,
        update: &mut impl FnMut(&mut AxisState),
    ) {
        for (cell, ids) in self.peers(name, axes) {
            let Some(mut state) = cell.try_borrow_mut() else {
                continue;
            };
            for id in ids {
                if let Some(peer) = state.artist_as_mut::<AxesState>(id) {
                    let axis = peer.axis_mut(name);
                    let bound_to = axis.ticker_owner;
                    update(axis);
                    if axis.ticker_owner != bound_to {
                        // Bound to an axes of the figure that set them,
                        // which this one cannot measure: its own counts.
                        axis.ticker_owner = None;
                    }
                }
            }
            state.stale = true;
        }
    }

    /// Whether axis `name` of an axes of another figure sharing it with
    /// `axes` changed since it was last autoscaled.
    pub(super) fn peers_stale(&self, axes: ArtistId, name: AxisName) -> bool {
        self.peers(name, axes).into_iter().any(|(cell, ids)| {
            cell.try_borrow().is_some_and(|state| {
                ids.iter().any(|&id| {
                    (state.store.get_as::<AxesState>(id))
                        .is_some_and(|peer| peer.axis(name).autoscale.stale)
                })
            })
        })
    }

    /// What the axes sharing axis `name` with `axes` contribute to its
    /// autoscaling, those of other figures included.
    pub(super) fn shared_data_across(
        &self,
        axes: ArtistId,
        name: AxisName,
        stickies: bool,
    ) -> SharedData {
        let mut data = self.shared_data(axes, name, stickies);
        for (cell, ids) in self.peers(name, axes) {
            let (Some(state), Some(&first)) = (cell.try_borrow(), ids.first()) else {
                continue;
            };
            let theirs = state.shared_data(first, name, stickies);
            data.interval = match (data.interval, theirs.interval) {
                (Some((low, high)), Some((their_low, their_high))) => {
                    Some((low.min(their_low), high.max(their_high)))
                }
                (ours, theirs) => ours.or(theirs),
            };
            data.minpos = data.minpos.min(theirs.minpos);
            data.stickies.extend(theirs.stickies);
        }
        data.stickies.sort_by(f64::total_cmp);
        data
    }

    /// Gives the axes of other figures sharing axis `name` with `axes`
    /// its new view `limits` and autoscaling `auto`, as
    /// [`set_axis_view`](Self::set_axis_view) gives them to its siblings
    /// (matplotlib's `_set_lim(..., emit=False)` on each, their limit
    /// callbacks, and their figures drawn again).
    pub(super) fn set_peers_view(
        &self,
        axes: ArtistId,
        name: AxisName,
        limits: (f64, f64),
        auto: Option<bool>,
    ) {
        for (cell, ids) in self.peers(name, axes) {
            let Some(mut state) = cell.try_borrow_mut() else {
                continue;
            };
            for id in ids {
                state.set_axis_view(id, name, limits, auto, false);
                state.limit_callbacks.notify(id, name);
                state.sync_parasites(id);
            }
            state.stale = true;
            if state.limit_callbacks.has_pending() {
                NOTIFIED.with(|notified| notified.borrow_mut().push(Rc::downgrade(&cell)));
            }
        }
    }

    /// Takes `axes`, being removed from the figure, out of the groups it
    /// shares with axes of other figures, which stay together
    /// (matplotlib's `Figure._remove_axes` takes the axes out of the
    /// grouper): an axes of this figure still sharing the axis inherits
    /// its links, else its peers are linked to each other. Call it while
    /// the figure's own groups still hold `axes`.
    pub(crate) fn leave_peers(&mut self, axes: ArtistId) {
        if self.across.is_empty() {
            return;
        }
        let left = Peer {
            figure: Weak::clone(&self.across.me),
            axes,
        };
        let mut orphans: Vec<(AxisName, Peer)> = Vec::new();
        for link in self.across.take(axes) {
            let name = link.name;
            let heir = (self.siblings(name, axes).into_iter()).find(|&id| id != axes);
            if let Some(mut state) = (link.peer.figure.upgrade())
                .as_ref()
                .and_then(|cell| cell.try_borrow_mut())
            {
                state.across.repoint(name, &left, heir);
            }
            match heir {
                Some(heir) => self.across.link(name, heir, link.peer, false),
                None => orphans.push((name, link.peer)),
            }
        }
        for pair in orphans.windows(2) {
            let [(name, first), (other, second)] = pair else {
                continue;
            };
            if name == other {
                link_peers(*name, first, second);
            }
        }
    }

    /// The tick helpers, scale, view and autoscaling of axis `name` of
    /// `axes`, brought up to date: what an axes sharing it takes.
    fn lent_axis(&mut self, axes: ArtistId, name: AxisName) -> Option<LentAxis> {
        self.unstale_axes(axes);
        let source = self.artist_as::<AxesState>(axes)?;
        let axis = source.axis(name);
        Some(LentAxis {
            tickers: axis.tickers(),
            scale: axis.scale_rc(),
            view: source.view_interval(name),
            auto: axis.autoscale.enabled,
        })
    }

    /// Has axis `name` of `axes` take what the peer it follows lends it
    /// again, as clearing an axes shares its axes anew (matplotlib's
    /// `clear` calls `sharex(self._sharex)`); whether it follows one whose
    /// figure could be reached.
    pub(super) fn reshare_across(&mut self, axes: ArtistId, name: AxisName) -> bool {
        let Some(peer) = self.across.followed(name, axes).cloned() else {
            return false;
        };
        let lent = (peer.figure.upgrade())
            .and_then(|cell| cell.try_borrow_mut()?.lent_axis(peer.axes, name));
        match lent {
            Some(lent) => {
                self.take_lent_axis(axes, name, lent);
                true
            }
            None => false,
        }
    }

    /// Gives axis `name` of `axes` what an axes of another figure lends
    /// it (see [`share_axis`](Self::share_axis), which does the same
    /// within one figure).
    fn take_lent_axis(&mut self, axes: ArtistId, name: AxisName, lent: LentAxis) {
        if let Some(state) = self.artist_as_mut::<AxesState>(axes) {
            state.axis_mut(name).use_tickers(lent.tickers);
        }
        self.set_axis_limits(axes, name, lent.view, Some(lent.auto), false);
        if let Some(state) = self.artist_as_mut::<AxesState>(axes) {
            state.axis_mut(name).adopt_scale(lent.scale);
        }
        self.stale = true;
    }
}

/// What an axis lends the axes sharing it.
struct LentAxis {
    tickers: super::axis::Tickers,
    scale: Rc<dyn crate::scale::Scale>,
    view: (f64, f64),
    auto: bool,
}

/// Runs the limit callbacks of the figures whose axes got their limits
/// from a peer since this was last called, once the figure that changed
/// ran its own.
pub(crate) fn run_peers_callbacks() {
    loop {
        let notified = NOTIFIED.with(|notified| std::mem::take(&mut *notified.borrow_mut()));
        if notified.is_empty() {
            return;
        }
        for cell in notified.iter().filter_map(Weak::upgrade) {
            run_limit_callbacks(&cell);
        }
    }
}

/// Links axis `name` of `first` and `second`, axes of two figures, both
/// ways, neither following the other; nothing while either figure is in
/// use or gone.
fn link_peers(name: AxisName, first: &Peer, second: &Peer) {
    for (from, to) in [(first, second), (second, first)] {
        let Some(cell) = from.figure.upgrade() else {
            return;
        };
        let Some(mut state) = cell.try_borrow_mut() else {
            return;
        };
        state.across.me = Weak::clone(&from.figure);
        state.across.link(name, from.axes, to.clone(), false);
    }
}

/// Joins, in each figure, the axes that a join made sharers of axis
/// `name` through axes of other figures, so that every figure's own
/// groups hold all of its axes of a group.
fn join_within_figures(figure: &Rc<FigureCell>, name: AxisName, axes: ArtistId) {
    let peers = figure.borrow().peers(name, axes);
    // This figure's axes of the group, as its peers reach them.
    let mut mine = vec![axes];
    for (cell, ids) in &peers {
        let reached = (ids.first()).map_or_else(Vec::new, |&id| cell.borrow().peers(name, id));
        mine.extend(
            (reached.into_iter())
                .filter(|(other, _)| Rc::ptr_eq(other, figure))
                .flat_map(|(_, ids)| ids),
        );
    }
    for (cell, ids) in peers.into_iter().chain([(Rc::clone(figure), mine)]) {
        let mut state = cell.borrow_mut();
        for pair in ids.windows(2) {
            state.shared.join(name, pair[0], pair[1]);
        }
    }
}

impl Axes {
    /// Shares axis `name` with `other`, an axes of another figure (see
    /// the module).
    pub(super) fn share_across(&self, name: AxisName, other: &Self) -> Result<(), PlotError> {
        let (figure, theirs) = (self.raw().figure_state(), other.raw().figure_state());
        let taken = {
            let state = figure.borrow();
            let within = state
                .artist_as::<AxesState>(self.id())
                .and_then(|axes| match name {
                    AxisName::X => axes.shares.0,
                    AxisName::Y => axes.shares.1,
                });
            let across = state.across.followed(name, self.id());
            within.is_some() || across.is_some_and(|peer| !peer.is(theirs, other.id()))
        };
        if taken {
            return Err(PlotError::AlreadyShared(name));
        }
        let Some(lent) = theirs.borrow_mut().lent_axis(other.id(), name) else {
            return Ok(());
        };
        self.link_across(name, other, true);
        figure.borrow_mut().take_lent_axis(self.id(), name, lent);
        Ok(())
    }

    /// Links axis `name` of this axes and of `other`, an axes of another
    /// figure, both ways.
    fn link_across(&self, name: AxisName, other: &Self, follows: bool) {
        let (figure, theirs) = (self.raw().figure_state(), other.raw().figure_state());
        for (cell, axes, peer, follows) in [
            (figure, self.id(), Peer::new(theirs, other.id()), follows),
            (theirs, other.id(), Peer::new(figure, self.id()), false),
        ] {
            let mut state = cell.borrow_mut();
            state.across.me = Rc::downgrade(cell);
            state.across.link(name, axes, peer, follows);
            state.stale = true;
        }
        join_within_figures(figure, name, self.id());
    }

    /// Joins the groups of the axes sharing axis `name` with this axes
    /// and with `other`, an axes of this or of another figure, as they
    /// are: neither takes the limits, the scale or the tick helpers of the
    /// other, unlike [`share_x`](Self::share_x). For figures read back
    /// from archives, which do not hold the links between figures: the
    /// axes come back with all they shared, and are joined again.
    /// `follows` records `other` as the axes whose axis this one took
    /// ([`shared_from`](Self::shared_from)).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::ticker::AxisName;
    /// let (first, second) = (Figure::new().subplot(), Figure::new().subplot());
    /// second.join_across(AxisName::X, &first, false);
    /// first.set_xlim(2.0, 3.0)?;
    /// assert_eq!(second.xlim(), (2.0, 3.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn join_across(&self, name: AxisName, other: &Self, follows: bool) {
        let figure = self.raw().figure_state();
        if Rc::ptr_eq(figure, other.raw().figure_state()) {
            figure.borrow_mut().shared.join(name, self.id(), other.id());
        } else {
            self.link_across(name, other, follows);
        }
    }

    /// The axes of other figures sharing axis `name` with this one
    /// (matplotlib's `get_shared_x_axes().get_siblings(ax)` lists them
    /// with those of its own figure, [`Axes::shared_axes`]).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::ticker::AxisName;
    /// let (first, second) = (Figure::new().subplot(), Figure::new().subplot());
    /// second.share_x(&first)?;
    /// assert_eq!(first.shared_across(AxisName::X), [second.clone()]);
    /// assert_eq!(second.shared_across(AxisName::Y), []);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn shared_across(&self, name: AxisName) -> Vec<Self> {
        let peers = self.raw().figure_state().borrow().peers(name, self.id());
        peers
            .into_iter()
            .flat_map(|(cell, ids)| {
                ids.into_iter()
                    .map(move |id| Self::from_raw(RawHandle::new(Rc::clone(&cell), id)))
            })
            .collect()
    }

    /// The axes of another figure whose axis `name` this one took with
    /// [`share_x`](Self::share_x) or [`share_y`](Self::share_y)
    /// (matplotlib's `_sharex` and `_sharey`; [`Axes::shared_from`] gives
    /// one of its own figure).
    pub fn shared_from_across(&self, name: AxisName) -> Option<Self> {
        let state = self.raw().figure_state().borrow();
        let peer = state.across.followed(name, self.id())?;
        let cell = peer.figure.upgrade()?;
        Some(Self::from_raw(RawHandle::new(cell, peer.axes)))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::axes::{AxisName, PlotError};
    use crate::figure::Figure;
    use crate::prelude::*;
    use crate::scale::ScaleName;
    use crate::ticker::MultipleLocator;

    /// Two figures, the axes of the second sharing x with the first's.
    fn joined() -> (Figure, Axes, Figure, Axes) {
        let (fig1, fig2) = (Figure::new(), Figure::new());
        let (ax1, ax2) = (fig1.subplot(), fig2.subplot());
        ax2.share_x(&ax1).unwrap();
        (fig1, ax1, fig2, ax2)
    }

    #[test]
    fn sharing_takes_the_view_and_follows_it_both_ways() {
        let (fig1, fig2) = (Figure::new(), Figure::new());
        let (ax1, ax2) = (fig1.subplot(), fig2.subplot());
        ax1.set_xlim(2.0, 5.0).unwrap();
        ax2.share_x(&ax1).unwrap();
        assert_eq!(ax2.xlim(), (2.0, 5.0));
        assert!(
            !ax2.autoscale_on(Some(AxisName::X)),
            "taken with the limits"
        );
        ax2.set_xlim(0.0, 1.0).unwrap();
        assert_eq!(ax1.xlim(), (0.0, 1.0));
        ax1.set_xlim(3.0, 4.0).unwrap();
        assert_eq!(ax2.xlim(), (3.0, 4.0));
        assert_eq!(ax2.ylim(), (0.0, 1.0), "y is not shared");
        assert_eq!(ax1.shared_across(AxisName::X), std::slice::from_ref(&ax2));
        assert_eq!(ax2.shared_from_across(AxisName::X), Some(ax1.clone()));
        assert_eq!(ax1.shared_from_across(AxisName::X), None);
        assert_eq!(ax2.shared_axes(AxisName::X), [ax2], "its own figure's");
    }

    #[test]
    fn each_autoscales_over_the_data_of_both() {
        let (fig1, ax1, fig2, ax2) = joined();
        ax1.plot([1.0, 2.0], [0.0, 1.0]).unwrap();
        ax2.plot([10.0, 20.0], [0.0, 1.0]).unwrap();
        fig2.layout();
        let (low, high) = ax2.xlim();
        assert!((low - 0.05).abs() < 1e-12 && (high - 20.95).abs() < 1e-12);
        assert_eq!(ax1.xlim(), (low, high));
        // Data added in one figure waits for either to be laid out.
        ax1.plot([-9.0], [0.0]).unwrap();
        fig1.layout();
        assert_eq!(ax2.xlim(), ax1.xlim());
        assert!(ax2.xlim().0 < -9.0);
    }

    #[test]
    fn scales_and_tick_helpers_reach_the_other_figure() {
        let (_fig1, ax1, _fig2, ax2) = joined();
        ax1.set_xscale("log").unwrap();
        assert_eq!(ax2.xscale(), ScaleName::Log);
        ax2.set_xscale("linear").unwrap();
        ax2.xaxis()
            .set_major_locator(MultipleLocator::new(0.25).unwrap());
        ax1.set_xlim(0.0, 1.0).unwrap();
        let locs = ax1.xaxis().ticklocs(false).unwrap();
        assert!(locs.contains(&0.25) && locs.contains(&0.75), "{locs:?}");
    }

    #[test]
    fn limit_callbacks_of_the_other_figure_run_after_those_of_the_changed() {
        let (_fig1, ax1, _fig2, ax2) = joined();
        let calls = Rc::new(RefCell::new(Vec::new()));
        for (ax, tag) in [(&ax1, "first"), (&ax2, "second")] {
            let calls = Rc::clone(&calls);
            ax.on_limits_changed(AxisName::X, move |_| calls.borrow_mut().push(tag));
        }
        ax2.set_xlim(0.0, 5.0).unwrap();
        assert_eq!(*calls.borrow(), ["second", "first"]);
    }

    #[test]
    fn groups_span_figures_and_join_their_own_axes() {
        let (fig1, ax1, _fig2, ax2) = joined();
        let fig3 = Figure::new();
        let ax3 = fig3.subplot();
        ax3.share_x(&ax2).unwrap();
        assert_eq!(ax1.shared_across(AxisName::X).len(), 2);
        ax3.set_xlim(7.0, 8.0).unwrap();
        assert_eq!(ax1.xlim(), (7.0, 8.0));
        // A second axes of the first figure joined through the third.
        let other = fig1.add_axes([0.1, 0.1, 0.2, 0.2]).unwrap();
        other.share_x(&ax3).unwrap();
        assert!(ax1.shared_axes(AxisName::X).contains(&other));
        assert_eq!(
            ax1.share_x(&ax3).err(),
            None,
            "an axes lending its axis may still take one"
        );
        assert_eq!(
            ax2.share_x(&ax3).err(),
            Some(PlotError::AlreadyShared(AxisName::X))
        );
    }

    #[test]
    fn a_removed_axes_leaves_the_others_together() {
        let (_fig1, ax1, fig2, ax2) = joined();
        let fig3 = Figure::new();
        let ax3 = fig3.subplot();
        ax3.share_x(&ax2).unwrap();
        ax2.remove();
        assert_eq!(ax2.shared_across(AxisName::X), []);
        assert_eq!(ax1.shared_across(AxisName::X), std::slice::from_ref(&ax3));
        ax1.set_xlim(1.0, 2.0).unwrap();
        assert_eq!(ax3.xlim(), (1.0, 2.0));
        assert_eq!(ax2.xlim(), (0.0, 1.0), "out of the group");
        // Clearing a figure takes its axes out too.
        fig3.clear();
        assert_eq!(ax1.shared_across(AxisName::X), []);
        drop(fig2);
    }

    #[test]
    fn a_cleared_axes_takes_its_peers_axis_again() {
        let (_fig1, ax1, _fig2, ax2) = joined();
        ax1.set_xlim(4.0, 6.0).unwrap();
        ax2.clear();
        assert_eq!(ax2.xlim(), (4.0, 6.0));
        assert_eq!(ax1.xlim(), (4.0, 6.0));
        // The one lending its axis starts over, and the group with it.
        ax1.clear();
        assert_eq!((ax1.xlim(), ax2.xlim()), ((0.0, 1.0), (0.0, 1.0)));
    }

    #[test]
    fn joining_as_they_are_takes_nothing() {
        let (first, second) = (Figure::new().subplot(), Figure::new().subplot());
        first.set_xlim(2.0, 3.0).unwrap();
        second.join_across(AxisName::X, &first, true);
        assert_eq!(second.xlim(), (0.0, 1.0));
        assert_eq!(second.shared_from_across(AxisName::X), Some(first.clone()));
        second.set_xlim(5.0, 6.0).unwrap();
        assert_eq!(first.xlim(), (5.0, 6.0));
        let figure = Figure::new();
        let (left, right) = (figure.subplot(), figure.subplot());
        left.join_across(AxisName::Y, &right, false);
        assert_eq!(left.shared_axes(AxisName::Y).len(), 2);
    }

    #[test]
    fn a_dropped_figure_leaves_its_peers_usable() {
        let (fig1, ax1, _fig2, ax2) = joined();
        drop((fig1, ax1));
        ax2.set_xlim(1.0, 2.0).unwrap();
        assert_eq!(ax2.shared_across(AxisName::X), []);
    }
}
