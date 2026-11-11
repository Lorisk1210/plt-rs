//! The geometry of path collections as matplotlib's `_path` combines it:
//! extents (`get_path_collection_extents`) and picking
//! (`point_in_path_collection`).

use std::fmt;

use super::Path;
use super::extents::DataLimits;
use crate::{Affine2D, Point};

/// Why collection extents could not be computed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionError {
    /// The collection has no paths.
    NoPaths,
    /// The collection has no offsets.
    NoOffsets,
}

impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoPaths => "no paths provided",
            Self::NoOffsets => "no offsets provided",
        })
    }
}

impl std::error::Error for CollectionError {}

/// Paths, per-item transforms and offsets, combined like a matplotlib
/// collection: item `i` uses `paths[i % len]`, `transforms[i % len]` and
/// `offsets[i % len]`, for `max(paths, offsets)` items.
#[derive(Clone, Copy, Debug)]
pub struct CollectionGeometry<'a> {
    /// The collection's transform (data to display for the paths).
    pub master_transform: Affine2D,
    /// The item shapes.
    pub paths: &'a [Path],
    /// Per-item transforms; when non-empty they replace the master
    /// transform for extents and precede it for picking, as in matplotlib.
    pub transforms: &'a [Affine2D],
    /// Per-item offsets, mapped by `offset_transform`.
    pub offsets: &'a [Point],
    /// The transform applied to the offsets.
    pub offset_transform: Affine2D,
}

impl CollectionGeometry<'_> {
    fn item_count(&self) -> usize {
        self.paths.len().max(self.offsets.len())
    }

    /// Item `i`'s path and the transform `base` followed by its offset.
    fn item(&self, i: usize, base: Affine2D) -> (&Path, Affine2D) {
        let path = &self.paths[i % self.paths.len()];
        let transform = if self.offsets.is_empty() {
            base
        } else {
            let offset = self
                .offset_transform
                .agg_transform_point(self.offsets[i % self.offsets.len()]);
            base.agg_then(Affine2D::translation(offset.x, offset.y))
        };
        (path, transform)
    }

    /// The per-item transform, or `None` without per-item transforms.
    fn item_transform(&self, i: usize) -> Option<Affine2D> {
        let count = self.transforms.len().min(self.item_count());
        (count > 0).then(|| self.transforms[i % count])
    }

    /// matplotlib's `get_path_collection_extents`: the limits (with
    /// `minpos`) of every transformed, offset item path, curve control
    /// points included.
    pub fn extents(&self) -> Result<DataLimits, CollectionError> {
        if self.paths.is_empty() {
            return Err(CollectionError::NoPaths);
        }
        if self.offsets.is_empty() {
            return Err(CollectionError::NoOffsets);
        }
        let mut limits = DataLimits::NULL;
        for i in 0..self.item_count() {
            let base = self.item_transform(i).unwrap_or(self.master_transform);
            let (path, transform) = self.item(i, base);
            limits.update_from_transformed_path(path, &transform);
        }
        Ok(limits)
    }

    /// matplotlib's `point_in_path_collection`: the indices of the items
    /// hit by `point` (in display coordinates). Filled items are tested
    /// with [`Path::contains_point`] grown by `radius`; unfilled ones with
    /// [`Path::stroke_contains_point`] (a stroke `2 * radius` wide).
    pub fn pick(&self, point: Point, radius: f64, filled: bool) -> Vec<usize> {
        if self.paths.is_empty() {
            return Vec::new();
        }
        (0..self.item_count())
            .filter(|&i| {
                let base = self
                    .item_transform(i)
                    .map_or(self.master_transform, |t| t.agg_then(self.master_transform));
                let (path, transform) = self.item(i, base);
                if filled {
                    path.contains_point(point, Some(&transform), radius)
                } else {
                    path.stroke_contains_point(point, &transform, radius)
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Bbox;

    #[test]
    fn per_item_transforms_replace_or_precede_the_master() {
        let paths = [Path::unit_rectangle()];
        let scale = [Affine2D::scaling(2.0, 2.0)];
        let geometry = CollectionGeometry {
            master_transform: Affine2D::translation(10.0, 0.0),
            paths: &paths,
            transforms: &scale,
            offsets: &[Point::ORIGIN],
            offset_transform: Affine2D::IDENTITY,
        };
        let limits = geometry.extents().expect("non-empty");
        assert_eq!(limits.bbox, Bbox::from_extents(0.0, 0.0, 2.0, 2.0));
        assert_eq!(geometry.pick(Point::new(11.5, 0.5), 0.0, true), vec![0]);
        assert_eq!(
            geometry.pick(Point::new(1.5, 0.5), 0.0, true),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn offsets_repeat_paths() {
        let paths = [Path::unit_rectangle()];
        let offsets = [Point::new(0.0, 0.0), Point::new(5.0, 0.0)];
        let geometry = CollectionGeometry {
            master_transform: Affine2D::IDENTITY,
            paths: &paths,
            transforms: &[],
            offsets: &offsets,
            offset_transform: Affine2D::IDENTITY,
        };
        assert_eq!(geometry.pick(Point::new(5.5, 1.2), 0.3, false), vec![1]);
        assert_eq!(
            geometry.extents().map(|l| l.bbox),
            Ok(Bbox::from_extents(0.0, 0.0, 6.0, 1.0))
        );
        let empty = CollectionGeometry {
            paths: &[],
            ..geometry
        };
        assert_eq!(empty.extents(), Err(CollectionError::NoPaths));
    }
}
