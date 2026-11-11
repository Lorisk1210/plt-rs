//! Axes drawing from the property cycle of another (matplotlib's
//! `_get_lines` and `_get_patches_for_fill`, the objects an axes takes
//! its lines' and patches' cycled properties from): a parasite draws its
//! lines from its host's cycle, and a program may point any axes at
//! another's cycles (`twin._get_lines = ax._get_lines`, as pandas does for
//! a secondary y axis), both advancing them. Clearing an axes gives it
//! cycles of its own again. Each cycle keeps its cycler and the entry it
//! gives next (matplotlib's `_PropCycle`), which a program may read and
//! move (`_get_lines._prop_cycle._idx`).

use std::rc::Rc;

use super::{Axes, AxesState};
use crate::artist::{ArtistHandle, RawHandle};
use crate::colors::Color;
use crate::rc::{CycleValue, Cycler};

/// One of the two property cycles of an axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropCycle {
    /// The cycle lines take their properties from (matplotlib's
    /// `_get_lines`).
    Lines,
    /// The cycle patches and collections take theirs from (matplotlib's
    /// `_get_patches_for_fill`).
    Patches,
}

/// One property cycle of an axes: its cycler and the entry it gives next
/// (matplotlib's `_PropCycle`).
#[derive(Clone, Debug)]
pub(crate) struct CycleState {
    /// The cycled properties.
    cycler: Cycler,
    /// The entry given next, below the cycler's length.
    next: usize,
}

pltrs_core::archive_struct!(CycleState { cycler, next });

impl CycleState {
    /// The cycle of `cycler`, at its first entry.
    pub(crate) fn new(cycler: Cycler) -> Self {
        Self { cycler, next: 0 }
    }

    /// Moves on to the following entry, back to the first after the last.
    fn advance(&mut self) {
        self.next = (self.next + 1) % self.cycler.len().max(1);
    }

    /// The next entry without the properties `skip` names, the cycle
    /// advancing only when that leaves any (matplotlib's `getdefaults`).
    pub(crate) fn take_entry(&mut self, skip: impl Fn(&str) -> bool) -> Vec<(String, CycleValue)> {
        let entry: Vec<(String, CycleValue)> = (self.cycler.entry(self.next))
            .filter(|(key, _)| !skip(key))
            .map(|(key, value)| (key.to_owned(), value.clone()))
            .collect();
        if !entry.is_empty() {
            self.advance();
        }
        entry
    }

    /// The colour of the next entry, black when the cycle has no colours;
    /// only a cycle that has advances (matplotlib's `get_next_color`).
    pub(crate) fn take_color(&mut self) -> Color {
        let color = self
            .cycler
            .entry(self.next)
            .find_map(|(key, value)| match (key, value) {
                ("color", CycleValue::Color(color)) => Some(color.clone()),
                _ => None,
            });
        if color.is_some() {
            self.advance();
        }
        color.unwrap_or_else(|| Color::from("k"))
    }
}

impl Axes {
    /// The entry the property cycle `cycle` gives next, counted from 0
    /// (matplotlib's `_get_lines._prop_cycle._idx`).
    ///
    /// ```
    /// use pltrs::axes::PropCycle;
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// ax.next_line_color();
    /// assert_eq!(ax.prop_cycle_position(PropCycle::Lines), 1);
    /// assert_eq!(ax.prop_cycle_position(PropCycle::Patches), 0);
    /// ```
    pub fn prop_cycle_position(&self, cycle: PropCycle) -> usize {
        self.with_cycle(cycle, |state| state.next)
    }

    /// Makes the property cycle `cycle` give its entry `position` next
    /// (wrapping around its length; matplotlib's `_prop_cycle._idx = i`).
    pub fn set_prop_cycle_position(&self, cycle: PropCycle, position: usize) -> &Self {
        self.with_cycle(cycle, |state| {
            state.next = position % state.cycler.len().max(1);
        });
        self
    }

    /// Replaces the cycler of the property cycle `cycle` alone and
    /// restarts it; `None` restores `axes.prop_cycle` (matplotlib's
    /// `_get_lines.set_prop_cycle(cycler)`).
    pub fn set_prop_cycle_of(&self, cycle: PropCycle, cycler: Option<Cycler>) -> &Self {
        let cycler = cycler.unwrap_or_else(|| crate::rc::params().axes_prop_cycle.clone());
        self.with_cycle(cycle, |state| *state = CycleState::new(cycler));
        self
    }

    /// Runs `f` on the state of the property cycle `cycle` this axes draws
    /// from (its owner's: see [`share_prop_cycle`](Self::share_prop_cycle)).
    pub(crate) fn with_cycle<R>(
        &self,
        cycle: PropCycle,
        f: impl FnOnce(&mut CycleState) -> R,
    ) -> R {
        self.prop_cycle_owner(cycle)
            .raw()
            .with_mut::<AxesState, _>(|axes| match cycle {
                PropCycle::Lines => f(&mut axes.line_cycle),
                PropCycle::Patches => f(&mut axes.patch_cycle),
            })
    }

    /// Makes this axes draw from the `cycle` that `other` draws from, both
    /// advancing it from then on (matplotlib's
    /// `ax._get_lines = other._get_lines`).
    ///
    /// ```
    /// use pltrs::axes::PropCycle;
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot();
    /// let twin = ax.twinx()?;
    /// twin.share_prop_cycle(PropCycle::Lines, &ax);
    /// let first = ax.next_line_color();
    /// assert_ne!(twin.next_line_color(), first, "the cycle moved on");
    /// # Ok::<(), pltrs::axes::AspectError>(())
    /// ```
    pub fn share_prop_cycle(&self, cycle: PropCycle, other: &Axes) -> &Self {
        let owner = other.prop_cycle_owner(cycle).id();
        let from = (owner != self.id()).then_some(owner);
        self.raw().with_mut::<AxesState, _>(|axes| match cycle {
            PropCycle::Lines => axes.line_cycle_from = from,
            PropCycle::Patches => axes.patch_cycle_from = from,
        });
        self
    }

    /// The axes whose `cycle` this one draws from: the one a program
    /// pointed it at, else for lines the host of a parasite, else itself.
    pub(crate) fn prop_cycle_owner(&self, cycle: PropCycle) -> Self {
        let owner = self.raw().with::<AxesState, _>(|axes| match cycle {
            PropCycle::Lines => axes
                .line_cycle_from
                .or_else(|| axes.parasite.map(|parasite| parasite.host)),
            PropCycle::Patches => axes.patch_cycle_from,
        });
        let state = self.raw().figure_state();
        owner
            .filter(|&id| state.borrow().store.get_as::<AxesState>(id).is_some())
            .map_or_else(
                || self.clone(),
                |id| Self::from_raw(RawHandle::new(Rc::clone(state), id)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn shared_cycles_advance_together_until_cleared() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let twin = ax.twinx().unwrap();
        let alone = fig.subplot();
        twin.share_prop_cycle(PropCycle::Lines, &ax);
        twin.share_prop_cycle(PropCycle::Patches, &ax);
        let colors = [
            ax.next_line_color(),
            twin.next_line_color(),
            ax.next_line_color(),
        ];
        assert_eq!(
            colors,
            [
                alone.next_line_color(),
                alone.next_line_color(),
                alone.next_line_color()
            ]
        );
        assert_eq!(twin.next_patch_color(), alone.next_patch_color());
        assert_eq!(ax.next_patch_color(), alone.next_patch_color());

        twin.clear();
        let fresh = fig.subplot();
        assert_eq!(twin.next_line_color(), fresh.next_line_color());
    }

    #[test]
    fn a_cycle_moved_back_gives_its_colours_again() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let twin = ax.twinx().unwrap();
        twin.share_prop_cycle(PropCycle::Patches, &ax);
        let start = twin.prop_cycle_position(PropCycle::Patches);
        let first = twin.next_patch_color();
        ax.next_patch_color();
        assert_eq!(ax.prop_cycle_position(PropCycle::Patches), start + 2);
        twin.set_prop_cycle_position(PropCycle::Patches, start);
        assert_eq!(ax.next_patch_color(), first, "the owner moved back");
    }

    #[test]
    fn sharing_follows_the_owner_and_never_loops() {
        let fig = Figure::new();
        let (a, b, c) = (fig.subplot(), fig.subplot(), fig.subplot());
        b.share_prop_cycle(PropCycle::Lines, &a);
        c.share_prop_cycle(PropCycle::Lines, &b);
        assert_eq!(c.prop_cycle_owner(PropCycle::Lines).id(), a.id());
        a.share_prop_cycle(PropCycle::Lines, &c);
        assert_eq!(a.prop_cycle_owner(PropCycle::Lines).id(), a.id());
        assert_eq!(c.prop_cycle_owner(PropCycle::Patches).id(), c.id());
    }

    #[test]
    fn a_cycle_s_position_wraps_and_each_cycle_takes_its_own_cycler() {
        let ax = Figure::new().subplot();
        let rgb: Cycler = "cycler(color=['r', 'g', 'b'])".parse().unwrap();
        ax.set_prop_cycle(Some(rgb));
        ax.set_prop_cycle_position(PropCycle::Lines, 5);
        assert_eq!(ax.prop_cycle_position(PropCycle::Lines), 2);
        assert_eq!(ax.next_line_color(), Color::from("b"));
        assert_eq!(
            ax.prop_cycle_position(PropCycle::Lines),
            0,
            "back to the first"
        );

        let markers: Cycler = "cycler(marker=['o', 'x'])".parse().unwrap();
        ax.set_prop_cycle_of(PropCycle::Patches, Some(markers));
        assert_eq!(
            ax.next_patch_color(),
            Color::from("k"),
            "no colours: no advance"
        );
        assert_eq!(ax.prop_cycle_position(PropCycle::Patches), 0);
        assert_eq!(ax.next_line_color(), Color::from("r"), "lines keep theirs");
    }
}
