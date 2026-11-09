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

impl Ellipses {
    /// The error numpy raises where matplotlib's `_set_transforms` makes
    /// one transform per width: it multiplies the widths, then the
    /// heights, by the sines and cosines of the angles, broadcasting them,
    /// and stores each product into as many rows as there are widths.
    /// `None` when they broadcast.
    pub(crate) fn broadcast_problem(&self) -> Option<String> {
        let (rows, angles) = (self.half_widths.len(), self.angles.len());
        [rows, self.half_heights.len()]
            .into_iter()
            .find_map(|axes| stored_product(axes, angles, rows).err())
    }
}

/// Whether numpy multiplies `a` values by `b` values, broadcasting them,
/// and stores the product into `rows` values; its error otherwise.
fn stored_product(a: usize, b: usize, rows: usize) -> Result<(), String> {
    let product = match (a, b) {
        _ if a == b => a,
        (1, _) => b,
        (_, 1) => a,
        _ => {
            return Err(format!(
                "operands could not be broadcast together with shapes ({a},) ({b},) "
            ));
        }
    };
    if product == rows || product == 1 {
        Ok(())
    } else {
        Err(format!(
            "could not broadcast input array from shape ({product},) into shape ({rows},)"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::{CollectionSpec, EllipseUnits};
    use crate::figure::Figure;

    #[test]
    fn sizes_broadcast_as_numpy_does() {
        let ellipses = |widths: usize, heights: usize, angles: usize| Ellipses {
            half_widths: vec![1.0; widths],
            half_heights: vec![1.0; heights],
            angles: vec![0.0; angles],
            units: EllipseUnits::Xy,
        };
        for fine in [
            (2, 2, 2),
            (2, 1, 1),
            (2, 2, 1),
            (1, 1, 1),
            (0, 1, 1),
            (0, 0, 0),
        ] {
            assert_eq!(ellipses(fine.0, fine.1, fine.2).broadcast_problem(), None);
        }
        assert_eq!(
            ellipses(1, 2, 2).broadcast_problem().as_deref(),
            Some("could not broadcast input array from shape (2,) into shape (1,)")
        );
        assert_eq!(
            ellipses(2, 3, 1).broadcast_problem().as_deref(),
            Some("could not broadcast input array from shape (3,) into shape (2,)")
        );
        assert_eq!(
            ellipses(2, 2, 3).broadcast_problem().as_deref(),
            Some("operands could not be broadcast together with shapes (2,) (3,) ")
        );
        assert_eq!(
            ellipses(2, 0, 1).broadcast_problem().as_deref(),
            Some("could not broadcast input array from shape (0,) into shape (2,)")
        );
    }

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
    fn mismatched_sizes_fail_the_draw_even_hidden() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let spec =
            CollectionSpec::ellipses(&[2.0, 1.0], &[3.0, 1.0], &[0.0, 30.0], EllipseUnits::Xy);
        let ellipses: EllipseCollection = ax.add_collection(spec).raw().downcast().unwrap();
        ellipses.set_widths(&[0.3]);
        for visible in [true, false] {
            ellipses.set_visible(visible);
            let err = fig.to_png().expect_err("the transforms do not broadcast");
            assert!(
                err.to_string()
                    .contains("could not broadcast input array from shape (2,) into shape (1,)"),
                "{err}"
            );
        }
        ellipses.set_widths(&[0.3, 0.4]);
        fig.to_png().expect("sizes that broadcast draw");
    }

    #[test]
    fn other_collections_are_not_ellipses() {
        let ax = Figure::new().subplot();
        let circles = ax.add_collection(CollectionSpec::circles(vec![4.0]));
        assert_eq!(circles.raw().downcast::<EllipseCollection>(), None);
    }
}
