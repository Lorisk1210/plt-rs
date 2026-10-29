//! The [`EllipseCollection`] handle: the axes and angles of an ellipse
//! collection, read and changed after it was made (matplotlib's
//! `EllipseCollection.get_widths` and friends).

use std::ops::Deref;

use super::{Collection, CollectionArtist, Ellipses};
use crate::artist::{ArtistHandle, RawHandle, TypedHandle};

/// A handle to an ellipse collection, what
/// [`CollectionSpec::ellipses`](super::CollectionSpec::ellipses) makes
/// (matplotlib's `EllipseCollection`). It dereferences to the
/// [`Collection`] it is.
///
/// ```
/// use pltrs::collections::{CollectionSpec, EllipseCollection, EllipseUnits};
/// use pltrs::prelude::*;
///
/// let ax = Figure::new().subplot();
/// let spec = CollectionSpec::ellipses(&[2.0], &[3.0], &[45.0], EllipseUnits::X);
/// let ellipses: EllipseCollection = ax.add_collection(spec).raw().downcast().expect("ellipses");
/// assert_eq!((ellipses.widths(), ellipses.heights()), (vec![2.0], vec![3.0]));
/// ellipses.set_angles(&[10.0, 20.0]);
/// assert_eq!(ellipses.angles().len(), 2);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EllipseCollection {
    collection: Collection,
}

impl Deref for EllipseCollection {
    type Target = Collection;

    fn deref(&self) -> &Collection {
        &self.collection
    }
}

impl ArtistHandle for EllipseCollection {
    fn raw(&self) -> &RawHandle {
        self.collection.raw()
    }
}

impl TypedHandle for EllipseCollection {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        let is_ellipses = raw.is::<CollectionArtist>()
            && raw.with::<CollectionArtist, _>(|c| c.ellipses.is_some());
        is_ellipses.then(|| Self {
            collection: Collection::from_raw(raw.clone()),
        })
    }
}

impl EllipseCollection {
    /// Reads the ellipses.
    fn read<R>(&self, f: impl FnOnce(&Ellipses) -> R) -> R {
        self.raw().with::<CollectionArtist, _>(|c| {
            f(c.ellipses
                .as_ref()
                .expect("an ellipse collection keeps its ellipses"))
        })
    }

    /// Changes the ellipses.
    fn edit(&self, f: impl FnOnce(&mut Ellipses)) -> &Self {
        self.raw().with_mut::<CollectionArtist, _>(|c| {
            f(c.ellipses
                .as_mut()
                .expect("an ellipse collection keeps its ellipses"));
        });
        self
    }

    /// The full widths of the ellipses in their units, cycled over the
    /// offsets (matplotlib's `get_widths`).
    pub fn widths(&self) -> Vec<f64> {
        self.read(|e| e.half_widths.iter().map(|w| 2.0 * w).collect())
    }

    /// Sets the full widths (matplotlib's `set_widths`).
    pub fn set_widths(&self, widths: &[f64]) -> &Self {
        self.edit(|e| e.half_widths = widths.iter().map(|w| 0.5 * w).collect())
    }

    /// The full heights of the ellipses in their units (matplotlib's
    /// `get_heights`).
    pub fn heights(&self) -> Vec<f64> {
        self.read(|e| e.half_heights.iter().map(|h| 2.0 * h).collect())
    }

    /// Sets the full heights (matplotlib's `set_heights`).
    pub fn set_heights(&self, heights: &[f64]) -> &Self {
        self.edit(|e| e.half_heights = heights.iter().map(|h| 0.5 * h).collect())
    }

    /// The counter-clockwise rotations of the ellipses in degrees
    /// (matplotlib's `get_angles`).
    pub fn angles(&self) -> Vec<f64> {
        self.read(|e| e.angles.iter().map(|a| a.to_degrees()).collect())
    }

    /// Sets the rotations in degrees (matplotlib's `set_angles`).
    pub fn set_angles(&self, angles: &[f64]) -> &Self {
        self.edit(|e| e.angles = angles.iter().map(|a| a.to_radians()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::{CollectionSpec, EllipseUnits};
    use crate::figure::Figure;

    #[test]
    fn axes_and_angles_read_back_as_set() {
        let ax = Figure::new().subplot();
        let spec = CollectionSpec::ellipses(&[2.0], &[3.0], &[45.0], EllipseUnits::X);
        let ellipses: EllipseCollection = ax.add_collection(spec).raw().downcast().unwrap();
        assert_eq!(ellipses.read(|e| e.half_widths.clone()), [1.0]);
        assert!((ellipses.angles()[0] - 45.0).abs() < 1e-12);
        ellipses
            .set_widths(&[1.0, 4.0])
            .set_heights(&[6.0])
            .set_angles(&[90.0]);
        assert_eq!(ellipses.widths(), [1.0, 4.0]);
        assert_eq!(ellipses.heights(), [6.0]);
        assert!((ellipses.read(|e| e.angles[0]) - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
    }

    #[test]
    fn other_collections_are_not_ellipses() {
        let ax = Figure::new().subplot();
        let circles = ax.add_collection(CollectionSpec::circles(vec![4.0]));
        assert_eq!(circles.raw().downcast::<EllipseCollection>(), None);
    }
}
