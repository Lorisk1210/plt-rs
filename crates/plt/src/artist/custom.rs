//! Artists of your own (matplotlib's `Artist` subclasses): any type that
//! implements [`Artist`], added to an axes or a figure, and reached
//! through a typed handle.
//!
//! ```
//! use plt::artist::{Artist, ArtistBase, ArtistHandle, DrawContext};
//! use plt::prelude::*;
//! use plt::transforms::{Affine2D, Point};
//! use plt::path::Path;
//!
//! /// A cross at a point, in the coordinates of its transform.
//! #[derive(Debug)]
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
//! cross.with_mut(|cross| cross.at = Point::new(0.25, 0.75));
//! assert_eq!(cross.with(|cross| cross.at.x), 0.25);
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

    /// Reads the artist.
    ///
    /// # Panics
    ///
    /// When `f` changes the figure through another handle: the figure is
    /// borrowed while `f` runs. Reading other artists is fine.
    pub fn with<R>(&self, f: impl FnOnce(&A) -> R) -> R {
        self.raw.with(f)
    }

    /// Changes the artist; the figure is drawn again with it.
    ///
    /// # Panics
    ///
    /// When `f` uses any handle of the same figure, even to read: the
    /// figure is mutably borrowed while `f` runs.
    pub fn with_mut<R>(&self, f: impl FnOnce(&mut A) -> R) -> R {
        self.raw.with_mut(f)
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
    #[derive(Debug, Default)]
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
            probe.with(|p| p.base.transform.clone()),
            Some(Transform::Figure(fig.id())),
            "in figure fractions"
        );
        fig.to_png().unwrap();
        assert_eq!(probe.with(|p| p.draws.get()), 1);
        probe.set_visible(false);
        fig.to_png().unwrap();
        assert_eq!(probe.with(|p| p.draws.get()), 1, "hidden");
        let found = fig.findobj(|handle| handle.downcast::<Custom<Probe>>().is_some());
        assert_eq!(
            found.iter().map(RawHandle::id).collect::<Vec<_>>(),
            [probe.id()]
        );
    }
}
