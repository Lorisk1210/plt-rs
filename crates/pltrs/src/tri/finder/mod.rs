//! Point location in a triangulation: matplotlib's
//! `TrapezoidMapTriFinder` (`_tri.cpp`), its trapezoid map built as
//! matplotlib builds it, edge order included, so that every point,
//! including those on edges and vertices and the non-finite ones, finds
//! the triangle matplotlib's finder does.

mod map;
mod shuffle;

use pltrs_core::Point;
use pltrs_core::numeric::{CONTRACTION, Contraction};

use self::map::{Edge, TrapezoidMap, is_right_of};
use self::shuffle::{Mt19937, shuffle};
use super::{TriError, Triangulation};

/// Finds the unmasked triangle containing a point, with matplotlib's
/// trapezoid map (de Berg et al., "Computational Geometry", 2nd edition).
///
/// A point strictly inside a triangle gets that triangle and a point
/// outside every triangle `None`. Points on edges and vertices, and
/// non-finite ones, get whatever the map's search ends on, as in
/// matplotlib: the tree of the map depends on the order its edges were
/// inserted in, which pltrs takes from matplotlib (see `shuffle`).
#[derive(Clone, Debug)]
pub struct TriFinder<'a> {
    tri: &'a Triangulation,
    map: TrapezoidMap,
}

impl<'a> TriFinder<'a> {
    /// Builds the trapezoid map of the unmasked triangles of `tri`.
    ///
    /// Fails with [`TriError::InvalidTriangulation`] where matplotlib's
    /// does ("Triangulation is invalid"): an edge the map cannot place,
    /// as those of duplicate or overlapping triangles.
    pub fn new(tri: &'a Triangulation) -> Result<Self, TriError> {
        let (points, corners) = map_points(tri);
        let mut point_triangles = vec![None; points.len()];
        let mut edges = vec![
            boundary_edge(corners, corners + 1),
            boundary_edge(corners + 2, corners + 3),
        ];
        let triangles = tri.triangles();
        for t in tri.unmasked() {
            let v = triangles[t];
            for e in 0..3 {
                let (start, end, other) = (v[e], v[(e + 1) % 3], v[(e + 2) % 3]);
                let neighbor = tri.neighbors()[t][e];
                if is_right_of(points[end], points[start]) {
                    edges.push(Edge {
                        left: start,
                        right: end,
                        triangle_below: neighbor,
                        triangle_above: Some(t),
                        point_below: neighbor.map(|n| opposite_point(triangles[n], end)),
                        point_above: Some(other),
                    });
                } else if neighbor.is_none() {
                    edges.push(Edge {
                        left: end,
                        right: start,
                        triangle_below: Some(t),
                        triangle_above: None,
                        point_below: Some(other),
                        point_above: None,
                    });
                }
                point_triangles[start].get_or_insert(t);
            }
        }
        shuffle(&mut edges[2..], &mut Mt19937::new(1234));
        let map = TrapezoidMap::new(points, point_triangles, edges, corners)
            .map_err(|_| TriError::InvalidTriangulation)?;
        Ok(Self { tri, map })
    }

    /// The triangulation being searched.
    pub fn triangulation(&self) -> &'a Triangulation {
        self.tri
    }

    /// The index of the triangle containing `(x, y)`, if any.
    pub fn find(&self, x: f64, y: f64) -> Option<usize> {
        self.map.find(Point::new(x, y))
    }
}

/// The box `lower`..`upper` grown by a tenth of its size on every side,
/// matplotlib's `BoundingBox::expand((upper - lower) * 0.1)`: clang rounds
/// the products (they pass through `XY::operator*`), GCC inlines the
/// operators and fuses them into `lower -= delta` and `upper += delta`
/// (its `widening_mul` GIMPLE).
fn expanded(contraction: Contraction, lower: Point, upper: Point) -> (Point, Point) {
    let size = upper - lower;
    match contraction {
        Contraction::Gcc => (
            Point::new(
                contraction.fused(-size.x, 0.1, lower.x),
                contraction.fused(-size.y, 0.1, lower.y),
            ),
            Point::new(
                contraction.fused(size.x, 0.1, upper.x),
                contraction.fused(size.y, 0.1, upper.y),
            ),
        ),
        Contraction::Unfused | Contraction::Clang => {
            let (dx, dy) = (size.x * 0.1, size.y * 0.1);
            (
                Point::new(lower.x - dx, lower.y - dy),
                Point::new(upper.x + dx, upper.y + dy),
            )
        }
    }
}

/// The triangulation's points, negative zeros made positive, followed by
/// the corners of a rectangle around them (south-west, south-east,
/// north-west, north-east), and the index of the first corner.
fn map_points(tri: &Triangulation) -> (Vec<Point>, usize) {
    // `-0.0 + 0.0` is `0.0`: matplotlib replaces negative zeros.
    let mut points: Vec<Point> = (0..tri.npoints())
        .map(|i| {
            let p = tri.point(i);
            Point::new(p.x + 0.0, p.y + 0.0)
        })
        .collect();
    let (lower, upper) = match bounds(&points) {
        None => (Point::new(0.0, 0.0), Point::new(1.0, 1.0)),
        Some((lower, upper)) => expanded(CONTRACTION, lower, upper),
    };
    let corners = points.len();
    points.extend([
        lower,
        Point::new(upper.x, lower.y),
        Point::new(lower.x, upper.y),
        upper,
    ]);
    (points, corners)
}

/// The bounds of `points` as matplotlib's `BoundingBox::add` grows them:
/// from the first point, a coordinate below the lower bound lowers it,
/// else one above the upper bound raises it (NaN does neither).
fn bounds(points: &[Point]) -> Option<(Point, Point)> {
    let (first, rest) = points.split_first()?;
    let (mut lower, mut upper) = (*first, *first);
    for p in rest {
        if p.x < lower.x {
            lower.x = p.x;
        } else if p.x > upper.x {
            upper.x = p.x;
        }
        if p.y < lower.y {
            lower.y = p.y;
        } else if p.y > upper.y {
            upper.y = p.y;
        }
    }
    Some((lower, upper))
}

/// An edge of the enclosing rectangle, with no triangle on either side.
fn boundary_edge(left: usize, right: usize) -> Edge {
    Edge {
        left,
        right,
        triangle_below: None,
        triangle_above: None,
        point_below: None,
        point_above: None,
    }
}

/// The point of the neighbour triangle `triangle` opposite the edge it
/// shares, found as matplotlib does: two places after the first place of
/// `end` (the place after 1 where `end` is missing).
fn opposite_point(triangle: [usize; 3], end: usize) -> usize {
    let place = triangle
        .iter()
        .position(|&v| v == end)
        .map_or(1, |e| (e + 2) % 3);
    triangle[place]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_enclosing_box_grows_as_each_compiler_builds_matplotlib() {
        let (lower, upper) = (Point::new(0.1, 0.1), Point::new(0.4, 0.4));
        // The lower corner's `(upper - lower) * 0.1` is fused under GCC only.
        assert_eq!(
            expanded(Contraction::Gcc, lower, upper).0.x,
            0.069_999_999_999_999_99
        );
        assert_eq!(expanded(Contraction::Clang, lower, upper).0.x, 0.07);
        assert_eq!(expanded(Contraction::Unfused, lower, upper).0.x, 0.07);
    }

    fn grid3(mask: Option<Vec<bool>>) -> Triangulation {
        let x = [0.0, 1.0, 2.0, 0.0, 1.0, 2.0, 0.0, 1.0, 2.0];
        let y = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0];
        let t = vec![
            [0, 1, 4],
            [0, 4, 3],
            [1, 2, 5],
            [1, 5, 4],
            [3, 4, 7],
            [3, 7, 6],
            [4, 5, 8],
            [4, 8, 7],
        ];
        Triangulation::new(&x, &y, t)
            .unwrap()
            .with_mask(mask)
            .unwrap()
    }

    /// `finder(xs, ys)` of matplotlib's `TrapezoidMapTriFinder`.
    fn found(finder: &TriFinder<'_>, points: &[(f64, f64)]) -> Vec<Option<usize>> {
        points.iter().map(|&(x, y)| finder.find(x, y)).collect()
    }

    #[test]
    fn interiors_edges_and_vertices_find_matplotlibs_triangles() {
        let tri = grid3(None);
        let finder = tri.trifinder().unwrap();
        let points = [
            (0.3, 0.1),
            (0.2, 0.6),
            (0.5, 0.5),
            (0.5, 1.0),
            (0.5, 0.0),
            (1.0, 1.0),
            (0.0, 0.0),
            (2.0, 2.0),
            (1.0, 2.0),
            (2.5, 1.0),
        ];
        let expected = [
            Some(0),
            Some(1),
            Some(1),
            Some(4),
            Some(0),
            Some(0),
            Some(0),
            Some(6),
            Some(4),
            None,
        ];
        assert_eq!(found(&finder, &points), expected);
    }

    #[test]
    fn non_finite_points_end_where_the_search_does() {
        let tri = grid3(None);
        let finder = tri.trifinder().unwrap();
        let points = [
            (f64::NAN, 1.0),
            (0.5, f64::NAN),
            (f64::NAN, f64::NAN),
            (f64::INFINITY, 1.0),
            (1.0, f64::NEG_INFINITY),
        ];
        // Where a search for NaN ends depends on the map, built in the order
        // of the C++ library's `std::shuffle`: MSVC's (matplotlib's Windows
        // wheel) ends it elsewhere.
        let ends = if cfg!(windows) { 5 } else { 1 };
        let expected = [None, Some(ends), None, None, None];
        assert_eq!(found(&finder, &points), expected);
    }

    #[test]
    fn masked_triangles_are_not_found() {
        let mask = vec![false, false, false, true, false, true, false, false];
        let tri = grid3(Some(mask));
        let finder = tri.trifinder().unwrap();
        let points = [(1.6, 0.3), (1.3, 0.8), (0.0, 2.0), (0.2, 1.9)];
        assert_eq!(found(&finder, &points), [Some(2), None, None, None]);
    }

    #[test]
    fn duplicate_triangles_are_invalid() {
        let tri = Triangulation::new(
            &[0.0, 1.0, 0.0, 1.0],
            &[0.0, 0.0, 1.0, 1.0],
            vec![[0, 1, 2], [0, 1, 3]],
        )
        .unwrap();
        assert_eq!(tri.trifinder().err(), Some(TriError::InvalidTriangulation));
    }

    #[test]
    fn bounds_grow_as_matplotlibs_bounding_box() {
        let points = [
            Point::new(1.0, 1.0),
            Point::new(-1.0, 0.0),
            Point::new(f64::NAN, 3.0),
            Point::new(2.0, f64::NAN),
        ];
        let (lower, upper) = bounds(&points).unwrap();
        assert_eq!(
            (lower, upper),
            (Point::new(-1.0, 0.0), Point::new(2.0, 3.0))
        );
        assert_eq!(bounds(&[]), None);
    }
}
