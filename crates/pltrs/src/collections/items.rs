//! Where a collection's items go: offsets that may come from a masked
//! array, and the shape transform of each item (matplotlib's
//! `set_offsets` given a masked array, and `get_transforms`).

use pltrs_core::{Affine2D, Point};

use super::{Collection, CollectionArtist};
use crate::artist::ArtistHandle;

impl Collection {
    /// Moves the items to `offsets` as [`Collection::set_offsets`] does,
    /// recording whether they are a masked array (NaN where masked), as
    /// matplotlib keeps the array it is given: a masked array loses the
    /// values a non-linear scale cannot map instead of clipping them, as
    /// scatter's offsets do.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let dots = ax.scatter(vec![1.0, 2.0], vec![3.0, 4.0])?;
    /// assert!(dots.offsets_masked(), "scatter's offsets are masked");
    /// dots.set_offsets_masked(vec![Point::new(1.0, 1.0)], false);
    /// assert!(!dots.offsets_masked());
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn set_offsets_masked(&self, offsets: Vec<Point>, masked: bool) -> &Self {
        self.set_offsets(offsets);
        self.raw()
            .with_mut::<CollectionArtist, _>(|c| c.offsets_masked = masked);
        self
    }

    /// Whether the offsets are a masked array (see
    /// [`Collection::set_offsets_masked`]).
    pub fn offsets_masked(&self) -> bool {
        self.raw().with::<CollectionArtist, _>(|c| c.offsets_masked)
    }

    /// The shape transform of each item drawn at `dpi` dots per inch
    /// (matplotlib's `get_transforms` once the collection is drawn at that
    /// resolution): markers scaled to their areas, else the transforms the
    /// collection was made with. Ellipses, which take their transforms
    /// from the axes as they are drawn, have none.
    pub fn item_transforms(&self, dpi: f64) -> Vec<Affine2D> {
        self.raw().with::<CollectionArtist, _>(|c| {
            if c.ellipses.is_some() {
                Vec::new()
            } else {
                c.item_transforms(c.drawn_sizes(), dpi)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::collections::{CollectionSpec, EllipseUnits};
    use crate::figure::Figure;
    use pltrs_core::{Affine2D, Path};

    #[test]
    fn markers_scale_with_their_areas_and_the_resolution() {
        let ax = Figure::new().subplot();
        let dots = ax.scatter(vec![1.0, 2.0], vec![3.0, 4.0]).unwrap();
        dots.set_sizes(vec![4.0, 16.0]);
        assert_eq!(
            dots.item_transforms(72.0),
            [Affine2D::scaling(2.0, 2.0), Affine2D::scaling(4.0, 4.0)]
        );
        assert_eq!(dots.item_transforms(144.0)[0], Affine2D::scaling(4.0, 4.0));
    }

    #[test]
    fn collections_without_sizes_keep_their_own_transforms() {
        let ax = Figure::new().subplot();
        let paths = ax.add_collection(CollectionSpec::paths(vec![Path::unit_rectangle()]));
        assert_eq!(paths.item_transforms(72.0), []);
        let ellipses = ax.add_collection(CollectionSpec::ellipses(
            &[1.0],
            &[2.0],
            &[0.0],
            EllipseUnits::Points,
        ));
        assert_eq!(ellipses.item_transforms(72.0), []);
    }
}
