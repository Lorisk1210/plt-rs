//! The contour set as one artist: removing it with its labels and clipping
//! all its contours at once (matplotlib's `ContourSet.remove` and the
//! `set_clip_path`/`get_clip_path` it inherits from `Collection`).

use super::{ContourSet, ContourWarning};
use crate::artist::{ArtistHandle, ClipPathSpec};

impl ContourSet {
    /// Removes the contours and their labels from the axes (matplotlib's
    /// `remove`). Labels removed one by one beforehand are skipped, and
    /// reported as matplotlib warns about them; removing a set twice does
    /// nothing. Like the handles of every removed artist, the set stays
    /// usable.
    ///
    /// ```
    /// use pltrs::contour::{ClabelOptions, ContourOptions, QuadField};
    /// use pltrs::numeric::Grid;
    /// use pltrs::prelude::*;
    ///
    /// let ax = Figure::new().subplot();
    /// let z = Grid::from_fn(4, 4, |i, j| (4 * i + j) as f64);
    /// let cs = ax.contour(&QuadField::new(z), ContourOptions::default())?;
    /// cs.clabel(ClabelOptions::default())?;
    /// assert_eq!(cs.remove(), None);
    /// assert!(ax.collections().is_empty() && ax.texts().is_empty());
    /// # Ok::<(), pltrs::contour::ContourError>(())
    /// ```
    pub fn remove(&self) -> Option<ContourWarning> {
        if !self.is_attached() {
            return None;
        }
        let mut manually_removed = false;
        for label in self.labels() {
            if label.artist.raw().is_attached() {
                label.artist.remove();
            } else {
                manually_removed = true;
            }
        }
        for collection in self.collections() {
            if collection.raw().is_attached() {
                collection.remove();
            }
        }
        self.remove_state();
        manually_removed.then_some(ContourWarning::LabelsRemoved)
    }

    /// Clips every contour to `clip` besides the axes box, or removes the
    /// clip region with `None` (matplotlib's `set_clip_path`, also the
    /// `clip_path` keyword of `contour`).
    pub fn set_clip_path(&self, clip: Option<ClipPathSpec>) -> &Self {
        for collection in self.collections() {
            collection.set_clip_path(clip.clone());
        }
        self
    }

    /// The region the contours are clipped to besides the axes box, if any
    /// (matplotlib's `get_clip_path`).
    pub fn clip_path(&self) -> Option<ClipPathSpec> {
        self.collections().first().and_then(ArtistHandle::clip_path)
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Path;

    use super::*;
    use crate::contour::{ClabelOptions, ContourOptions, QuadField};
    use crate::figure::Figure;
    use crate::numeric::Grid;

    fn ramp(ax: &crate::axes::Axes) -> ContourSet {
        let z = Grid::from_fn(4, 4, |i, j| (4 * i + j) as f64);
        ax.contour(&QuadField::new(z), ContourOptions::default())
            .unwrap()
    }

    #[test]
    fn removing_restores_the_axes() {
        let ax = Figure::new().subplot();
        let before = (ax.collections(), ax.texts(), ax.artists());
        let cs = ramp(&ax);
        cs.clabel(ClabelOptions::default()).unwrap();
        assert_ne!((ax.collections(), ax.texts(), ax.artists()), before);
        assert_eq!(cs.remove(), None);
        assert_eq!((ax.collections(), ax.texts(), ax.artists()), before);
        assert_eq!(cs.remove(), None, "a second removal does nothing");
    }

    #[test]
    fn labels_removed_by_hand_are_reported() {
        let ax = Figure::new().subplot();
        let cs = ramp(&ax);
        let labels = cs.clabel(ClabelOptions::default()).unwrap();
        labels[0].remove();
        assert_eq!(cs.remove(), Some(ContourWarning::LabelsRemoved));
        assert_eq!(ax.texts().len(), 0, "the other labels are gone too");
    }

    #[test]
    fn clip_paths_apply_to_every_contour() {
        let ax = Figure::new().subplot();
        let cs = ramp(&ax);
        assert_eq!(cs.clip_path(), None);
        let clip = ClipPathSpec::Path {
            path: Path::unit_circle(),
            transform: ax.trans_axes(),
        };
        cs.set_clip_path(Some(clip.clone()));
        assert_eq!(cs.clip_path(), Some(clip));
        cs.set_clip_path(None);
        assert_eq!(cs.clip_path(), None);
    }
}
