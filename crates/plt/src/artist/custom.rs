//! Artists of your own (matplotlib's `Artist` subclasses): any type that
//! implements [`Artist`], added to an axes or a figure, and reached
//! through a typed handle.
//!
//! The handle reads and changes the artist through copies
//! ([`Custom::get`], [`Custom::update`]), so the figure is never borrowed
//! while your code runs: it may use any handle of the figure meanwhile.
//!
//! ```
//! use plt::artist::{Artist, ArtistBase, ArtistHandle, DrawContext};
//! use plt::prelude::*;
//! use plt::transforms::{Affine2D, Point};
//! use plt::path::Path;
//!
//! /// A cross at a point, in the coordinates of its transform.
//! #[derive(Clone, Debug)]
//! struct Cross {
//!     base: ArtistBase,
//!     at: Point,
//! }
//!
//! impl Artist for Cross {
//!     fn base(&self) -> &ArtistBase {
//!         &self.base
//!     }
//!
//!     fn base_mut(&mut self) -> &mut ArtistBase {
//!         &mut self.base
//!     }
//!
//!     fn draw(&self, ctx: &mut DrawContext<'_>) {
//!         let Some(transform) = self.base.transform.as_ref().and_then(|t| ctx.resolve(t)) else {
//!             return;
//!         };
//!         let centre = transform.transform_point(self.at);
//!         let gc = ctx.base_gc(&self.base);
//!         let (x, y) = (centre.x, centre.y);
//!         for dy in [-5.0, 5.0] {
//!             let stroke = Path::polyline(vec![Point::new(x - 5.0, y - dy), Point::new(x + 5.0, y + dy)]);
//!             ctx.renderer().draw_path(&gc, &stroke, &Affine2D::IDENTITY, None);
//!         }
//!     }
//! }
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let cross = ax.add_custom_artist(Cross { base: ArtistBase::default(), at: Point::new(0.5, 0.5) });
//! cross.update(|cross| cross.at = Point::new(0.25, 0.75));
//! assert_eq!(cross.get().at.x, 0.25);
//! cross.set_zorder(5.0);
//! fig.to_png()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::marker::PhantomData;

use super::{Artist, ArtistHandle, RawHandle, TypedHandle};

/// The handle of an artist of type `A` added with
/// [`Axes::add_custom_artist`](crate::axes::Axes::add_custom_artist) or
/// [`Figure::add_custom_artist`](crate::figure::Figure::add_custom_artist).
#[derive(Debug)]
pub struct Custom<A> {
    raw: RawHandle,
    kind: PhantomData<fn() -> A>,
}

impl<A> Clone for Custom<A> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            kind: PhantomData,
        }
    }
}

impl<A: Artist> Custom<A> {
    /// The handle of the artist `raw` refers to, which is an `A`.
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self {
            raw,
            kind: PhantomData,
        }
    }
}

impl<A: Artist + Clone> Custom<A> {
    /// A copy of the artist. The figure is borrowed only while it is
    /// copied.
    pub fn get(&self) -> A {
        self.raw.with(A::clone)
    }

    /// Changes the artist with `f`; the figure is drawn again with it.
    ///
    /// `f` changes a copy, which replaces the artist once `f` returns, so
    /// the figure is not borrowed while `f` runs and `f` may use any of
    /// its handles. Changes made to this same artist through its handle
    /// while `f` runs are replaced by the copy.
    pub fn update<R>(&self, f: impl FnOnce(&mut A) -> R) -> R {
        let mut artist = self.get();
        let result = f(&mut artist);
        self.raw.with_mut(|stored: &mut A| *stored = artist);
        result
    }
}

impl<A: Artist> ArtistHandle for Custom<A> {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl<A: Artist> TypedHandle for Custom<A> {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<A>().then(|| Self::from_raw(raw.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::{ArtistBase, DrawContext};
    use crate::figure::Figure;
    use crate::transforms::Transform;

    /// Counts how often it is drawn.
    #[derive(Clone, Debug, Default)]
    struct Probe {
        base: ArtistBase,
        draws: std::cell::Cell<usize>,
    }

    impl Artist for Probe {
        fn base(&self) -> &ArtistBase {
            &self.base
        }

        fn base_mut(&mut self) -> &mut ArtistBase {
            &mut self.base
        }

        fn draw(&self, _ctx: &mut DrawContext<'_>) {
            self.draws.set(self.draws.get() + 1);
        }
    }

    #[test]
    fn figure_artists_are_drawn_and_found_by_type() {
        let fig = Figure::new();
        let probe = fig.add_custom_artist(Probe::default());
        assert_eq!(
            probe.get().base.transform,
            Some(Transform::Figure(fig.id())),
            "in figure fractions"
        );
        fig.to_png().unwrap();
        assert_eq!(probe.get().draws.get(), 1);
        probe.set_visible(false);
        fig.to_png().unwrap();
        assert_eq!(probe.get().draws.get(), 1, "hidden");
        let found = fig.findobj(|handle| handle.downcast::<Custom<Probe>>().is_some());
        assert_eq!(
            found.iter().map(RawHandle::id).collect::<Vec<_>>(),
            [probe.id()]
        );
    }

    #[test]
    fn updates_may_use_every_handle_of_the_figure() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        let probe = ax.add_custom_artist(Probe::default());
        let width = probe.update(|p| {
            line.set_linewidth(3.0);
            p.base.zorder = 4.0;
            line.linewidth()
        });
        assert_eq!(width, 3.0);
        assert_eq!(probe.zorder(), 4.0);
        let copy = probe.get();
        probe.set_alpha(Some(0.5));
        assert_eq!(copy.base.alpha, None, "a copy, not a borrow");
        assert_eq!(probe.alpha(), Some(0.5));
    }
}
