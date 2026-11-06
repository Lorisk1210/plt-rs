//! Spines made apart from the ones an axes draws (matplotlib's
//! `Spine(axes, ...)` and its factories `linear_spine`, `arc_spine` and
//! `circular_spine`, which make new spines that `axes.spines` draws once
//! they are put there: in the place of a spine of a side, or beside them).

use std::rc::Rc;

use super::{Spine, SpineArtist, SpineSide};
use crate::artist::{ArtistHandle, RawHandle};
use crate::axes::{Axes, AxesState};

impl Axes {
    /// A new spine of this axes on `side`, styled from the rc parameters
    /// and shown (matplotlib's `Spine(axes, ...)`; `linear_spine` hides
    /// it as `axes.spines.*` say), which the axes does not draw until it
    /// takes the place of one of the spines it draws
    /// ([`Axes::replace_spine`]) or joins them ([`Axes::add_spine`]).
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
        let mut spine = SpineArtist::new(side, self.id(), crate::rc::params());
        spine.base.visible = true;
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
        if !self.made(spine) {
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

    /// Draws `spine`, a spine of this axes it does not draw, after the
    /// ones it draws, as a spine framing no side, on which no axis is
    /// drawn: matplotlib's spine of a type naming no side, which an Axes
    /// class keeps beside the four of the sides (cartopy's `GeoSpine`, the
    /// outline of a map), or `ax.spines[name] = spine` of a new name;
    /// whether it was such a spine.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::spines::{Spine, SpineSide};
    /// let ax = Figure::new().subplot();
    /// let outline = ax.new_spine(SpineSide::Top);
    /// assert!(ax.add_spine(&outline));
    /// assert!(!outline.frames_side());
    /// assert!(!ax.add_spine(&outline), "it is drawn already");
    /// assert!(ax.spines().iter().all(Spine::frames_side));
    /// ```
    pub fn add_spine(&self, spine: &Spine) -> bool {
        if !self.made(spine) {
            return false;
        }
        let id = spine.id();
        let added = self.raw().with_mut::<AxesState, _>(|axes| {
            if axes.spines.contains(&id) {
                return false;
            }
            axes.spines.push(id);
            axes.children.push(id);
            true
        });
        if added {
            spine
                .raw()
                .with_mut::<SpineArtist, _>(|spine| spine.frames_side = false);
            self.raw().figure_state().borrow_mut().stale = true;
        }
        added
    }

    /// Whether `spine` is a spine of this axes.
    fn made(&self, spine: &Spine) -> bool {
        Rc::ptr_eq(self.raw().figure_state(), spine.raw().figure_state())
            && spine.raw().with_base(|base| base.parent) == Some(self.id())
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
    fn added_spines_are_drawn_beside_the_sides() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let outline = ax.new_spine(SpineSide::Top);
        assert!(ax.add_spine(&outline));
        assert!(ax.spines().top.frames_side());
        assert!(!outline.frames_side());
        let other = fig.subplot().new_spine(SpineSide::Top);
        assert!(!ax.add_spine(&other), "a spine of another axes");
        assert!(
            !ax.replace_spine(&ax.spines().top, &outline),
            "drawn already"
        );
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
