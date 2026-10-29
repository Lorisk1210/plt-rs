//! Point location in a triangulation.

use std::collections::HashMap;

use pltrs_core::Point;

use super::{TriError, Triangulation};

/// Finds the unmasked triangle containing a point, with the tie-breaking
/// rules of matplotlib's `TrapezoidMapTriFinder`:
///
/// * a point strictly inside a triangle gets that triangle;
/// * a point on the interior of an edge gets the triangle on the left of
///   the edge directed from its lexicographically smaller endpoint (by x,
///   then y) to the larger one, or the triangle on the right if there is
///   none on the left;
/// * a point on a vertex gets the lowest-index triangle using the vertex;
/// * points outside, inside masked triangles, or with non-finite
///   coordinates get `None`.
///
/// Edge tests use the same (non-robust) cross products as matplotlib, with
/// each edge oriented canonically, so neighbouring triangles always agree
/// on which side of a shared edge a point lies.
#[derive(Clone, Debug)]
pub struct TriFinder<'a> {
    tri: &'a Triangulation,
    /// The lowest-index unmasked triangle of each vertex, by coordinates.
    vertex_triangle: HashMap<(u64, u64), usize>,
    grid: BucketGrid,
}

/// Where a point lies relative to one triangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Placement {
    Inside,
    OnEdgeLeft,
    OnEdgeRight,
}

impl<'a> TriFinder<'a> {
    /// Indexes the unmasked triangles of `tri`.
    ///
    /// Fails with [`TriError::InvalidTriangulation`] if two unmasked
    /// triangles traverse the same edge in the same direction (duplicate or
    /// overlapping triangles).
    pub fn new(tri: &'a Triangulation) -> Result<Self, TriError> {
        let mut directed = HashMap::new();
        let mut vertex_triangle = HashMap::new();
        for t in tri.unmasked() {
            let v = tri.triangles()[t];
            for e in 0..3 {
                if directed.insert((v[e], v[(e + 1) % 3]), t).is_some() {
                    return Err(TriError::InvalidTriangulation);
                }
                vertex_triangle
                    .entry(point_key(tri.point(v[e])))
                    .or_insert(t);
            }
        }
        Ok(Self {
            tri,
            vertex_triangle,
            grid: BucketGrid::new(tri),
        })
    }

    /// The triangulation being searched.
    pub fn triangulation(&self) -> &'a Triangulation {
        self.tri
    }

    /// The index of the triangle containing `(x, y)`, if any.
    pub fn find(&self, x: f64, y: f64) -> Option<usize> {
        if !(x.is_finite() && y.is_finite()) {
            return None;
        }
        let p = Point::new(x, y);
        if let Some(&t) = self.vertex_triangle.get(&point_key(p)) {
            return Some(t);
        }
        self.grid
            .candidates(p)
            .iter()
            .filter_map(|&t| self.placement(t, p).map(|place| (place, t)))
            .min()
            .map(|(_, t)| t)
    }

    /// How `p` lies relative to unmasked triangle `t`, if in its closure.
    fn placement(&self, t: usize, p: Point) -> Option<Placement> {
        let v = self.tri.triangles()[t];
        let mut on_edge = None;
        for e in 0..3 {
            let (a, b) = (self.tri.point(v[e]), self.tri.point(v[(e + 1) % 3]));
            // The triangle lies on the left of a -> b (anticlockwise).
            let forward = is_right_of(b, a);
            let (left, right) = if forward { (a, b) } else { (b, a) };
            let side = orientation(left, right, p);
            let inside = if forward { side < 0 } else { side > 0 };
            if side == 0 {
                if on_edge.is_some() {
                    return None;
                }
                on_edge = Some(if forward {
                    Placement::OnEdgeLeft
                } else {
                    Placement::OnEdgeRight
                });
            } else if !inside {
                return None;
            }
        }
        Some(on_edge.unwrap_or(Placement::Inside))
    }
}

/// matplotlib's `XY::is_right_of`: lexicographic comparison by x, then y.
#[expect(
    clippy::float_cmp,
    reason = "exact lexicographic order, as in matplotlib"
)]
fn is_right_of(p: Point, other: Point) -> bool {
    if p.x == other.x {
        p.y > other.y
    } else {
        p.x > other.x
    }
}

/// matplotlib's `Edge::get_point_orientation` for the edge `left -> right`:
/// `+1` below, `-1` above, `0` on the line.
fn orientation(left: Point, right: Point, p: Point) -> i8 {
    let cross = (p.x - left.x) * (right.y - left.y) - (p.y - left.y) * (right.x - left.x);
    if cross > 0.0 {
        1
    } else if cross < 0.0 {
        -1
    } else {
        0
    }
}

fn point_key(p: Point) -> (u64, u64) {
    ((p.x + 0.0).to_bits(), (p.y + 0.0).to_bits())
}

/// A uniform grid over the triangulation's bounding box listing the
/// triangles whose bounding box overlaps each cell.
#[derive(Clone, Debug)]
struct BucketGrid {
    origin: Point,
    extent: Point,
    cell: Point,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<usize>>,
}

impl BucketGrid {
    fn new(tri: &Triangulation) -> Self {
        let live: Vec<usize> = tri.unmasked().collect();
        let corners = live
            .iter()
            .flat_map(|&t| tri.triangles()[t].map(|v| tri.point(v)));
        let (lo, hi) = corners.fold(
            (
                Point::new(f64::INFINITY, f64::INFINITY),
                Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            ),
            |(lo, hi), p| (lo.min(p), hi.max(p)),
        );
        let side = ((live.len() as f64).sqrt().ceil() as usize).clamp(1, 1024);
        let mut grid = Self {
            origin: lo,
            extent: hi,
            cell: Point::new((hi.x - lo.x) / side as f64, (hi.y - lo.y) / side as f64),
            nx: side,
            ny: side,
            cells: vec![Vec::new(); side * side],
        };
        if live.is_empty() {
            grid.nx = 0;
            return grid;
        }
        for &t in &live {
            let pts = tri.triangles()[t].map(|v| tri.point(v));
            let tlo = pts[0].min(pts[1]).min(pts[2]);
            let thi = pts[0].max(pts[1]).max(pts[2]);
            let (i0, j0) = grid.cell_of(tlo);
            let (i1, j1) = grid.cell_of(thi);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    grid.cells[j * grid.nx + i].push(t);
                }
            }
        }
        grid
    }

    /// The (clamped) cell containing `p`.
    fn cell_of(&self, p: Point) -> (usize, usize) {
        let index = |v: f64, origin: f64, size: f64, n: usize| {
            if size > 0.0 {
                (((v - origin) / size).floor().max(0.0) as usize).min(n - 1)
            } else {
                0
            }
        };
        (
            index(p.x, self.origin.x, self.cell.x, self.nx),
            index(p.y, self.origin.y, self.cell.y, self.ny),
        )
    }

    /// Triangles that may contain `p`.
    fn candidates(&self, p: Point) -> &[usize] {
        let (lo, hi) = (self.origin, self.extent);
        let outside = p.x < lo.x || p.y < lo.y || p.x > hi.x || p.y > hi.y;
        if self.nx == 0 || outside {
            return &[];
        }
        let (i, j) = self.cell_of(p);
        &self.cells[j * self.nx + i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn interior_edge_and_vertex_rules() {
        let tri = grid3(None);
        let finder = tri.trifinder().unwrap();
        assert_eq!(finder.find(0.3, 0.1), Some(0));
        assert_eq!(finder.find(0.2, 0.6), Some(1));
        // Diagonal (0,0)-(1,1): left of the edge directed up-right is tri 1.
        assert_eq!(finder.find(0.5, 0.5), Some(1));
        // Horizontal edge y = 1 between x = 0 and 1: left (above) is tri 4.
        assert_eq!(finder.find(0.5, 1.0), Some(4));
        // Bottom boundary: only a triangle on the left of (0,0)->(1,0).
        assert_eq!(finder.find(0.5, 0.0), Some(0));
        // Vertex: lowest-index triangle using it.
        assert_eq!(finder.find(1.0, 1.0), Some(0));
        assert_eq!(finder.find(2.5, 1.0), None);
        assert_eq!(finder.find(f64::NAN, 1.0), None);
        assert_eq!(finder.find(f64::INFINITY, 1.0), None);
    }

    #[test]
    fn masked_triangles_are_not_found() {
        let mask = vec![false, false, false, true, false, true, false, false];
        let tri = grid3(Some(mask));
        let finder = tri.trifinder().unwrap();
        assert_eq!(finder.find(1.6, 0.3), Some(2));
        assert_eq!(finder.find(1.3, 0.8), None);
        assert_eq!(finder.find(0.0, 2.0), None);
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
}
