//! Axes drawing from the property cycle of another (matplotlib's
//! `_get_lines` and `_get_patches_for_fill`, the objects an axes takes
//! its lines' and patches' cycled properties from): a parasite draws its
//! lines from its host's cycle, and a program may point any axes at
//! another's cycles (`twin._get_lines = ax._get_lines`, as pandas does for
//! a secondary y axis), both advancing them. Clearing an axes gives it
//! cycles of its own again.

use std::rc::Rc;

use super::{Axes, AxesState};
use crate::artist::{ArtistHandle, RawHandle};

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

impl Axes {
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

    /// How far `cycle` has advanced (the entry it gives next), on the axes
    /// this one draws it from.
    pub fn prop_cycle_position(&self, cycle: PropCycle) -> usize {
        self.prop_cycle_owner(cycle)
            .raw()
            .with::<AxesState, _>(|axes| match cycle {
                PropCycle::Lines => axes.line_cycle,
                PropCycle::Patches => axes.patch_cycle,
            })
    }

    /// Moves `cycle` back (or on) to `position`, as
    /// [`prop_cycle_position`](Self::prop_cycle_position) gave it: the
    /// colours an attempt took that matplotlib's code then takes again.
    pub fn set_prop_cycle_position(&self, cycle: PropCycle, position: usize) -> &Self {
        self.prop_cycle_owner(cycle)
            .raw()
            .with_mut::<AxesState, _>(|axes| match cycle {
                PropCycle::Lines => axes.line_cycle = position,
                PropCycle::Patches => axes.patch_cycle = position,
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
}
