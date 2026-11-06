//! Spines made apart from the ones an axes draws (matplotlib's
//! `Spine(axes, ...)` and its factories `linear_spine`, `arc_spine` and
//! `circular_spine`, which make new spines that `axes.spines` draws once
//! they are put there).

use std::rc::Rc;

use super::{Spine, SpineArtist, SpineSide};
use crate::artist::{ArtistHandle, RawHandle};
use crate::axes::{Axes, AxesState};

impl Axes {
    /// A new spine of this axes on `side`, styled from the rc parameters,
    /// which the axes does not draw (matplotlib's `Spine(axes, ...)`) until
    /// it takes the place of one of the spines it draws
    /// ([`Axes::replace_spine`]).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// let spine = ax.new_spine(SpineSide::Left);
    /// assert_ne!(spine, ax.spines().left);
    /// assert_eq!(spine.axes(), Some(ax.clone()));
    /// assert!(ax.replace_spine(&ax.spines().left, &spine));
    /// assert_eq!(ax.spines().left, spine);
    /// ```
    pub fn new_spine(&self, side: SpineSide) -> Spine {
        let figure = Rc::clone(self.raw().figure_state());
        let spine = SpineArtist::new(side, self.id(), crate::rc::params());
        let id = {
            let mut state = figure.borrow_mut();
            let id = state.store.insert(Box::new(spine));
            if state.detached.contains(self.id()) {
                // The spine of a removed axes is kept apart with it.
                let kept = state.store.take(id).expect("the spine was just inserted");
                state.detached.insert(id, kept);
            }
            id
        };
        Spine::from_raw(RawHandle::new(figure, id))
    }

    /// Draws `spine`, a spine of this axes it does not draw, in the place
    /// of `old`, one it draws, which it then no longer draws (matplotlib's
    /// `ax.spines[name] = spine`); whether they were such spines.
    pub fn replace_spine(&self, old: &Spine, spine: &Spine) -> bool {
        let same_figure = Rc::ptr_eq(self.raw().figure_state(), spine.raw().figure_state());
        if !same_figure || spine.raw().with_base(|base| base.parent) != Some(self.id()) {
            return false;
        }
        let (old, new) = (old.id(), spine.id());
        let replaced = self.raw().with_mut::<AxesState, _>(|axes| {
            let Some(at) = axes.spines.iter().position(|&id| id == old) else {
                return false;
            };
            if axes.spines.contains(&new) {
                return false;
            }
            axes.spines[at] = new;
            for child in axes.children.iter_mut().filter(|child| **child == old) {
                *child = new;
            }
            true
        });
        if replaced {
            self.raw().figure_state().borrow_mut().stale = true;
        }
        replaced
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::testing::RecordingRenderer;

    #[test]
    fn new_spines_are_drawn_once_they_replace_one() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let old = ax.spines().top.clone();
        let spine = ax.new_spine(SpineSide::Top);
        spine.set_linewidth(7.0);
        assert!(
            !ax.replace_spine(&spine, &old),
            "the new spine is not drawn"
        );
        assert!(ax.replace_spine(&old, &spine));
        assert!(!ax.replace_spine(&old, &spine), "the old one no longer is");
        assert_eq!(ax.spines().top, spine);
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
    }

    #[test]
    fn spines_of_other_axes_replace_none() {
        let fig = Figure::new();
        let (ax, other) = (fig.subplot(), fig.subplot());
        let spine = other.new_spine(SpineSide::Left);
        assert!(!ax.replace_spine(&ax.spines().left, &spine));
    }
}
