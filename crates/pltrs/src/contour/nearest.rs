//! The point of a contour plot nearest to a given point (matplotlib's
//! `find_nearest_contour` and `_find_closest_point_on_path`), which
//! manual label placement snaps to.

use pltrs_core::numeric::np_clip;
use pltrs_core::{Path, Point};

use super::{ContourError, ContourSet};
use crate::transforms::ResolvedTransform;

/// Where [`ContourSet::find_nearest_contour`] measures distances.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NearestIn {
    /// Display pixels, through the contour set's transform (matplotlib's
    /// `pixel=True`), as manual labelling does.
    #[default]
    Display,
    /// The coordinates of the contour lines themselves, usually data
    /// coordinates (`pixel=False`).
    Coordinates,
}

/// The point of a contour plot closest to a reference point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NearestContour {
    /// Index of the level the closest line belongs to.
    pub level: usize,
    /// Index of the closest line among the unbroken lines of that level.
    pub line: usize,
    /// Index within that line of the vertex ending the closest segment (0
    /// for a line of a single vertex).
    pub vertex: usize,
    /// The closest point, in the space the distance was measured in.
    pub point: Point,
    /// The squared distance from `point` to the reference point.
    pub distance_squared: f64,
}

/// The closest point on some lines, and the key of the path holding it.
pub(super) struct Closest<K> {
    /// The key the path was given with.
    pub(super) key: K,
    /// Index into the whole path of the vertex ending the closest segment.
    pub(super) vertex: usize,
    /// The point, mapped.
    pub(super) point: Point,
    /// Its squared distance to the target.
    pub(super) distance_squared: f64,
}

impl ContourSet {
    /// The point of the line contours closest to `point` (matplotlib's
    /// `find_nearest_contour`), among the levels at the indices `levels`
    /// (all when `None`). With [`NearestIn::Display`] `point` is in display
    /// pixels and the lines are measured through the set's transform; with
    /// [`NearestIn::Coordinates`] both are in the lines' own coordinates.
    /// `None` when the levels have no lines.
    ///
    /// # Errors
    ///
    /// For filled contours, for a level index out of range, or when the
    /// set's transform cannot be resolved.
    pub fn find_nearest_contour(
        &self,
        point: Point,
        levels: Option<&[usize]>,
        space: NearestIn,
    ) -> Result<Option<NearestContour>, ContourError> {
        if self.filled() {
            return Err(ContourError::FilledUnsupported);
        }
        let paths = self.paths();
        let indices: Vec<usize> = match levels {
            Some(levels) => {
                if let Some(&index) = levels.iter().find(|&&i| i >= paths.len()) {
                    return Err(ContourError::LevelIndex {
                        index,
                        levels: paths.len(),
                    });
                }
                levels.to_vec()
            }
            None => (0..paths.len()).collect(),
        };
        let map = match space {
            NearestIn::Display => self.display_transform()?,
            NearestIn::Coordinates => ResolvedTransform::identity(),
        };
        let levels = indices.iter().map(|&i| (i, &paths[i]));
        let Some(closest) = closest_on_lines(levels, |p| map.transform_point(p), point) else {
            return Ok(None);
        };
        // The line holding the vertex: numpy's `searchsorted(..., "right")`
        // over the cumulative line lengths.
        let ends: Vec<usize> = components(&paths[closest.key])
            .iter()
            .scan(0, |end, line| {
                *end += line.vertices().len();
                Some(*end)
            })
            .collect();
        let line = ends
            .iter()
            .take_while(|&&end| end <= closest.vertex)
            .count();
        let start = line.checked_sub(1).map_or(0, |previous| ends[previous]);
        Ok(Some(NearestContour {
            level: closest.key,
            line,
            vertex: closest.vertex - start,
            point: closest.point,
            distance_squared: closest.distance_squared,
        }))
    }
}

/// The point closest to `target` on the lines of `paths`, given with their
/// keys, with every vertex mapped by `map` first (matplotlib's
/// `_find_nearest_contour`): the first of equally close points, and none
/// that is NaN or infinitely far.
pub(super) fn closest_on_lines<'p, K: Copy>(
    paths: impl IntoIterator<Item = (K, &'p Path)>,
    map: impl Fn(Point) -> Point,
    target: Point,
) -> Option<Closest<K>> {
    let mut best: Option<Closest<K>> = None;
    for (key, path) in paths {
        let mut start = 0;
        for line in components(path) {
            let mapped: Vec<Point> = line.vertices().iter().map(|&p| map(p)).collect();
            if let Some((distance_squared, point, end)) = closest_point(&mapped, target)
                && distance_squared < best.as_ref().map_or(f64::INFINITY, |b| b.distance_squared)
            {
                best = Some(Closest {
                    key,
                    vertex: start + end,
                    point,
                    distance_squared,
                });
            }
            start += mapped.len();
        }
    }
    best
}

/// The connected components of a path (matplotlib's
/// `_iter_connected_components`): a path without codes is one component.
pub(super) fn components(path: &Path) -> Vec<Path> {
    if path.codes().is_none() {
        vec![path.clone()]
    } else {
        path.subpaths()
    }
}

/// The point of the polyline `line` closest to `p`: its squared distance,
/// the point, and the vertex index ending the segment it lies on (0 for a
/// single vertex).
fn closest_point(line: &[Point], p: Point) -> Option<(f64, Point, usize)> {
    match line {
        [] => None,
        [only] => Some(((p.x - only.x).powi(2) + (p.y - only.y).powi(2), *only, 0)),
        _ => {
            let candidates = line.windows(2).map(|w| {
                let (a, b) = (w[0], w[1]);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let mut norm = dx.powi(2) + dy.powi(2);
                if norm == 0.0 {
                    norm = 1.0;
                }
                let rel = np_clip(((p.x - a.x) * dx + (p.y - a.y) * dy) / norm, 0.0, 1.0);
                let proj = Point::new(a.x + rel * dx, a.y + rel * dy);
                ((proj.x - p.x).powi(2) + (proj.y - p.y).powi(2), proj)
            });
            // numpy's argmin: the first minimum, or the first NaN.
            let mut best: Option<(usize, f64, Point)> = None;
            for (i, (d2, proj)) in candidates.enumerate() {
                let better = match best {
                    None => true,
                    Some((_, current, _)) => !current.is_nan() && (d2.is_nan() || d2 < current),
                };
                if better {
                    best = Some((i, d2, proj));
                }
            }
            best.map(|(i, d2, proj)| (d2, proj, i + 1))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contour::{ContourOptions, Levels, QuadCoords, QuadField};
    use crate::figure::Figure;
    use crate::numeric::Grid;
    use crate::testing::points;

    #[test]
    fn closest_point_projects_onto_segments() {
        let path = points(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
        let (d2, proj, index) = closest_point(&path, Point::new(4.0, 2.0)).unwrap();
        assert_eq!((d2, proj, index), (4.0, Point::new(4.0, 0.0), 1));
        let (_, proj, index) = closest_point(&path, Point::new(12.0, 7.0)).unwrap();
        assert_eq!((proj, index), (Point::new(10.0, 7.0), 2));
        assert_eq!(closest_point(&[], Point::ORIGIN), None);
        assert_eq!(
            closest_point(&path[..1], Point::new(3.0, 4.0)),
            Some((25.0, Point::ORIGIN, 0))
        );
    }

    /// Circles of radius 1 and 2 around the origin, on a grid over
    /// [-3, 3]², as line or filled contours.
    fn circles(filled: bool) -> ContourSet {
        let z = Grid::from_fn(25, 25, |i, j| {
            let (x, y) = (-3.0 + 0.25 * j as f64, -3.0 + 0.25 * i as f64);
            x.hypot(y)
        });
        let field = QuadField::new(z).with_coords(QuadCoords::Implicit {
            origin: None,
            extent: Some([-3.0, 3.0, -3.0, 3.0]),
        });
        let options = ContourOptions {
            levels: Levels::Values(vec![1.0, 2.0, 2.5]),
            ..ContourOptions::default()
        };
        let ax = Figure::new().subplot();
        if filled {
            ax.contourf(&field, options).unwrap()
        } else {
            ax.contour(&field, options).unwrap()
        }
    }

    #[test]
    fn nearest_in_coordinates_snaps_to_the_closest_level() {
        let cs = circles(false);
        let near = cs
            .find_nearest_contour(Point::new(1.2, 0.0), None, NearestIn::Coordinates)
            .unwrap()
            .unwrap();
        assert_eq!((near.level, near.line), (0, 0));
        assert!((near.point.x - 1.0).abs() < 0.02 && near.point.y.abs() < 1e-12);
        assert!((near.distance_squared - (1.2 - near.point.x).powi(2)).abs() < 1e-12);
        let paths = cs.paths();
        let vertices = paths[0].vertices();
        let segment = (vertices[near.vertex - 1], vertices[near.vertex]);
        assert!(segment.0.y.min(segment.1.y) <= 0.0 && segment.0.y.max(segment.1.y) >= 0.0);

        let outer = cs
            .find_nearest_contour(Point::new(1.2, 0.0), Some(&[1, 2]), NearestIn::Coordinates)
            .unwrap()
            .unwrap();
        assert_eq!(outer.level, 1, "only the listed levels count");
    }

    #[test]
    fn nearest_in_display_measures_pixels() {
        let cs = circles(false);
        let to_display = cs.display_transform().unwrap();
        let target = to_display.transform_point(Point::new(0.0, 2.2));
        let near = cs
            .find_nearest_contour(target, None, NearestIn::Display)
            .unwrap()
            .unwrap();
        assert_eq!(near.level, 1);
        let on_line = to_display.transform_point(Point::new(0.0, 2.0));
        assert!((near.point.y - on_line.y).abs() < 1.0);
    }

    #[test]
    fn nearest_rejects_fills_and_unknown_levels() {
        assert!(matches!(
            circles(true).find_nearest_contour(Point::ORIGIN, None, NearestIn::Display),
            Err(ContourError::FilledUnsupported)
        ));
        assert!(matches!(
            circles(false).find_nearest_contour(Point::ORIGIN, Some(&[3]), NearestIn::Display),
            Err(ContourError::LevelIndex {
                index: 3,
                levels: 3
            })
        ));
        assert_eq!(
            circles(false)
                .find_nearest_contour(Point::ORIGIN, Some(&[]), NearestIn::Display)
                .unwrap(),
            None
        );
    }
}
