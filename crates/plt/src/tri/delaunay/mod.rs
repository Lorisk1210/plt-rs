//! Delaunay triangulation of a planar point set.
//!
//! An incremental Bowyer–Watson construction with exact predicates and
//! "ghost" triangles (triangles joining each convex-hull edge to a vertex
//! at infinity), so points outside the current hull, on hull edges and
//! collinear with hull edges need no special casing. Points are inserted in
//! Hilbert-curve order and located by a visibility walk from the previous
//! insertion, giving `O(n log n)` behaviour in practice.
//!
//! Like matplotlib (qhull with `Qbb Qc Qz`), exact duplicate points are
//! ignored except for their first occurrence, all other points (including
//! collinear hull points) are vertices, and fewer than three distinct or
//! only collinear points are an error.

mod predicates;

use std::collections::HashMap;

use plt_core::Point;

pub(crate) use predicates::orient2d;
use predicates::incircle;

use super::TriError;

/// Marker for the vertex at infinity.
const GHOST: usize = usize::MAX;
/// Marker for an unset neighbour (only during construction).
const NONE: usize = usize::MAX;

/// Computes the Delaunay triangulation of `points`; triangles are
/// anticlockwise and reference indices into `points`.
pub(crate) fn triangulate(points: &[Point]) -> Result<Vec<[usize; 3]>, TriError> {
    if points.iter().any(|p| !p.is_finite()) {
        return Err(TriError::NonFinite);
    }
    if points.len() < 3 {
        return Err(TriError::TooFewPoints);
    }
    let unique = unique_points(points);
    if unique.len() < 3 {
        return Err(TriError::TooFewUniquePoints);
    }
    let order = hilbert_order(points, &unique);
    let mut mesh = Mesh::new(points, &order)?;
    for &index in &order {
        if !mesh.is_vertex(index) {
            mesh.insert(index);
        }
    }
    Ok(mesh.real_triangles())
}

/// Indices of the first occurrence of each distinct point.
fn unique_points(points: &[Point]) -> Vec<usize> {
    let mut seen = HashMap::with_capacity(points.len());
    let key = |p: &Point| ((p.x + 0.0).to_bits(), (p.y + 0.0).to_bits());
    points
        .iter()
        .enumerate()
        .filter(|(i, p)| *seen.entry(key(p)).or_insert(*i) == *i)
        .map(|(i, _)| i)
        .collect()
}

/// `indices` sorted along a Hilbert curve over the points' bounding box.
fn hilbert_order(points: &[Point], indices: &[usize]) -> Vec<usize> {
    const ORDER: u32 = 16;
    let (mut lo, mut hi) = (points[indices[0]], points[indices[0]]);
    for &i in indices {
        lo = lo.min(points[i]);
        hi = hi.max(points[i]);
    }
    let side = f64::from((1u32 << ORDER) - 1);
    let cell = |v: f64, lo: f64, hi: f64| {
        if hi > lo {
            ((v - lo) / (hi - lo) * side) as u32
        } else {
            0
        }
    };
    let mut keyed: Vec<(u64, usize)> = indices
        .iter()
        .map(|&i| {
            let p = points[i];
            let key = hilbert_key(cell(p.x, lo.x, hi.x), cell(p.y, lo.y, hi.y), ORDER);
            (key, i)
        })
        .collect();
    keyed.sort_unstable();
    keyed.into_iter().map(|(_, i)| i).collect()
}

/// Distance of cell `(x, y)` along a Hilbert curve of the given order.
fn hilbert_key(mut x: u32, mut y: u32, order: u32) -> u64 {
    let max = (1u32 << order) - 1;
    let mut key = 0u64;
    let mut s = 1u32 << (order - 1);
    while s > 0 {
        let rx = u32::from(x & s > 0);
        let ry = u32::from(y & s > 0);
        key += u64::from(s) * u64::from(s) * u64::from((3 * rx) ^ ry);
        if ry == 0 {
            if rx == 1 {
                x = max - x;
                y = max - y;
            }
            std::mem::swap(&mut x, &mut y);
        }
        s >>= 1;
    }
    key
}

/// One triangle of the working mesh. Edge `i` runs from `v[i]` to
/// `v[(i + 1) % 3]` and `n[i]` is the triangle across it. Ghost triangles
/// store the vertex at infinity as `v[2]`, so their edge 0 is the hull edge
/// (traversed clockwise around the hull).
#[derive(Clone, Copy, Debug)]
struct Tri {
    v: [usize; 3],
    n: [usize; 3],
    alive: bool,
}

impl Tri {
    fn is_ghost(&self) -> bool {
        self.v[2] == GHOST
    }
}

/// The mesh under construction.
struct Mesh<'a> {
    points: &'a [Point],
    tris: Vec<Tri>,
    free: Vec<usize>,
    /// Whether each point is already a vertex.
    inserted: Vec<bool>,
    /// Membership of each triangle in the cavity being built.
    in_cavity: Vec<bool>,
    /// A recently created triangle: the start of the next walk.
    last: usize,
    /// State of the deterministic generator choosing walk directions.
    seed: u64,
}

impl<'a> Mesh<'a> {
    /// Starts from the first non-degenerate triangle of `order`.
    fn new(points: &'a [Point], order: &[usize]) -> Result<Self, TriError> {
        let a = order[0];
        let b = order[1];
        let c = order[2..]
            .iter()
            .copied()
            .find(|&c| orient2d(points[a], points[b], points[c]) != 0.0)
            .ok_or(TriError::Collinear)?;
        let (b, c) = if orient2d(points[a], points[b], points[c]) > 0.0 {
            (b, c)
        } else {
            (c, b)
        };
        let mut inserted = vec![false; points.len()];
        for v in [a, b, c] {
            inserted[v] = true;
        }
        // Triangle 0 is real; 1..=3 are the ghosts across its edges.
        let tris = vec![
            Tri {
                v: [a, b, c],
                n: [1, 2, 3],
                alive: true,
            },
            Tri {
                v: [b, a, GHOST],
                n: [0, 3, 2],
                alive: true,
            },
            Tri {
                v: [c, b, GHOST],
                n: [0, 1, 3],
                alive: true,
            },
            Tri {
                v: [a, c, GHOST],
                n: [0, 2, 1],
                alive: true,
            },
        ];
        Ok(Self {
            points,
            tris,
            free: Vec::new(),
            inserted,
            in_cavity: vec![false; 4],
            last: 0,
            seed: 0x9e37_79b9_7f4a_7c15,
        })
    }

    fn is_vertex(&self, index: usize) -> bool {
        self.inserted[index]
    }

    fn point(&self, v: usize) -> Point {
        self.points[v]
    }

    /// Whether `p` invalidates triangle `t` (lies in its open circumcircle,
    /// or for a ghost, strictly outside its hull edge or on its interior).
    fn in_conflict(&self, t: usize, p: Point) -> bool {
        let [a, b, c] = self.tris[t].v;
        if c == GHOST {
            let (pa, pb) = (self.point(a), self.point(b));
            let side = orient2d(pa, pb, p);
            side > 0.0 || (side == 0.0 && strictly_between(pa, pb, p))
        } else {
            incircle(self.point(a), self.point(b), self.point(c), p) > 0.0
        }
    }

    /// A triangle in conflict with `p`, found by a visibility walk.
    fn locate(&mut self, p: Point) -> usize {
        let mut t = self.last;
        if self.tris[t].is_ghost() {
            if self.in_conflict(t, p) {
                return t;
            }
            t = self.tris[t].n[0];
        }
        let limit = 4 * self.tris.len() + 16;
        for _ in 0..limit {
            match self.step(t, p) {
                Some(next) if self.tris[next].is_ghost() => return next,
                Some(next) => t = next,
                None => return t,
            }
        }
        self.scan(p)
    }

    /// The neighbour of real triangle `t` across an edge that separates it
    /// from `p`, or `None` if `p` lies in the closed triangle.
    fn step(&mut self, t: usize, p: Point) -> Option<usize> {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        let start = (self.seed % 3) as usize;
        let tri = self.tris[t];
        (0..3).map(|k| (start + k) % 3).find_map(|i| {
            let (a, b) = (self.point(tri.v[i]), self.point(tri.v[(i + 1) % 3]));
            (orient2d(a, b, p) < 0.0).then_some(tri.n[i])
        })
    }

    /// Exhaustive fallback for [`Mesh::locate`].
    fn scan(&self, p: Point) -> usize {
        (0..self.tris.len())
            .find(|&t| self.tris[t].alive && self.in_conflict(t, p))
            .expect("every point outside the existing vertices conflicts with some triangle")
    }

    /// Inserts point `index` (Bowyer–Watson).
    fn insert(&mut self, index: usize) {
        let p = self.point(index);
        let first = self.locate(p);
        let cavity = self.cavity(first, p);
        // Boundary edges of the cavity, anticlockwise around it, with the
        // outside triangle across each.
        let mut boundary = Vec::new();
        for &t in &cavity {
            let tri = self.tris[t];
            for i in 0..3 {
                if !self.in_cavity[tri.n[i]] {
                    boundary.push((tri.v[i], tri.v[(i + 1) % 3], tri.n[i]));
                }
            }
        }
        for &t in &cavity {
            self.tris[t].alive = false;
            self.in_cavity[t] = false;
            self.free.push(t);
        }
        let mut by_start = HashMap::with_capacity(boundary.len());
        let mut by_end = HashMap::with_capacity(boundary.len());
        let mut created = Vec::with_capacity(boundary.len());
        for &(s, e, across) in &boundary {
            let t = self.allocate(new_triangle(s, e, index));
            self.relink(across, s, e, t);
            by_start.insert(s, t);
            by_end.insert(e, t);
            created.push((t, s, e));
        }
        for &(t, s, e) in &created {
            let v = self.tris[t].v;
            let (next, prev) = (by_start[&e], by_end[&s]);
            self.tris[t].n[edge_slot(&v, e, index)] = next;
            self.tris[t].n[edge_slot(&v, index, s)] = prev;
        }
        self.inserted[index] = true;
        self.last = created
            .iter()
            .map(|(t, _, _)| *t)
            .find(|&t| !self.tris[t].is_ghost())
            .unwrap_or(created[0].0);
    }

    /// All triangles in conflict with `p`, connected to `first`; they are
    /// flagged in `in_cavity` until the caller clears them.
    fn cavity(&mut self, first: usize, p: Point) -> Vec<usize> {
        let mut cavity = vec![first];
        self.in_cavity[first] = true;
        let mut next = 0;
        while let Some(&t) = cavity.get(next) {
            next += 1;
            for across in self.tris[t].n {
                if !self.in_cavity[across] && self.in_conflict(across, p) {
                    self.in_cavity[across] = true;
                    cavity.push(across);
                }
            }
        }
        cavity
    }

    fn allocate(&mut self, tri: Tri) -> usize {
        if let Some(t) = self.free.pop() {
            self.tris[t] = tri;
            t
        } else {
            self.tris.push(tri);
            self.in_cavity.push(false);
            self.tris.len() - 1
        }
    }

    /// Points the neighbour slot of `outside` for edge `(e, s)` at `t`.
    fn relink(&mut self, outside: usize, s: usize, e: usize, t: usize) {
        let tri = &mut self.tris[outside];
        let slot = (0..3)
            .find(|&i| tri.v[i] == e && tri.v[(i + 1) % 3] == s)
            .expect("outside triangle shares the cavity edge");
        tri.n[slot] = t;
        let slot = edge_slot(&self.tris[t].v, s, e);
        self.tris[t].n[slot] = outside;
    }

    fn real_triangles(&self) -> Vec<[usize; 3]> {
        self.tris
            .iter()
            .filter(|t| t.alive && !t.is_ghost())
            .map(|t| t.v)
            .collect()
    }
}

/// The triangle `(s, e, p)`, rotated so a ghost vertex comes last.
fn new_triangle(s: usize, e: usize, p: usize) -> Tri {
    let v = if s == GHOST {
        [e, p, GHOST]
    } else if e == GHOST {
        [p, s, GHOST]
    } else {
        [s, e, p]
    };
    Tri {
        v,
        n: [NONE; 3],
        alive: true,
    }
}

/// The edge index of `(from, to)` within vertex triple `v`.
fn edge_slot(v: &[usize; 3], from: usize, to: usize) -> usize {
    (0..3)
        .find(|&i| v[i] == from && v[(i + 1) % 3] == to)
        .expect("triangle contains the edge")
}

/// Whether `p`, collinear with `a` and `b`, lies strictly between them.
fn strictly_between(a: Point, b: Point, p: Point) -> bool {
    if a.x == b.x {
        (a.y < p.y && p.y < b.y) || (b.y < p.y && p.y < a.y)
    } else {
        (a.x < p.x && p.x < b.x) || (b.x < p.x && p.x < a.x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(coords: &[(f64, f64)]) -> Vec<Point> {
        coords.iter().map(|&(x, y)| Point::new(x, y)).collect()
    }

    fn check_delaunay(points: &[Point], tris: &[[usize; 3]]) {
        for t in tris {
            let [a, b, c] = t.map(|v| points[v]);
            assert!(orient2d(a, b, c) > 0.0, "triangle {t:?} not anticlockwise");
            for (i, &p) in points.iter().enumerate() {
                if !t.contains(&i) {
                    assert!(incircle(a, b, c, p) <= 0.0, "point {i} inside {t:?}");
                }
            }
        }
    }

    #[test]
    fn square_grid_uses_every_point() {
        let mut coords = Vec::new();
        for j in 0..4 {
            for i in 0..5 {
                coords.push((f64::from(i), f64::from(j)));
            }
        }
        let points = pts(&coords);
        let tris = triangulate(&points).unwrap();
        assert_eq!(tris.len(), 2 * 20 - 2 - 14);
        check_delaunay(&points, &tris);
    }

    #[test]
    fn random_points_are_delaunay() {
        let mut state = 12345u64;
        let mut next = || {
            state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        let points: Vec<Point> = (0..300).map(|_| Point::new(next(), next())).collect();
        let tris = triangulate(&points).unwrap();
        check_delaunay(&points, &tris);
    }

    #[test]
    fn duplicates_keep_first_occurrence() {
        let points = pts(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.2)]);
        let tris = triangulate(&points).unwrap();
        assert_eq!(tris.len(), 2);
        assert!(tris.iter().all(|t| !t.contains(&3)));
    }

    #[test]
    fn degenerate_inputs_are_rejected() {
        assert_eq!(triangulate(&pts(&[(0.0, 0.0), (1.0, 1.0)])), Err(TriError::TooFewPoints));
        assert_eq!(
            triangulate(&pts(&[(0.0, 0.0), (0.0, 0.0), (1.0, 0.0)])),
            Err(TriError::TooFewUniquePoints)
        );
        assert_eq!(
            triangulate(&pts(&[(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)])),
            Err(TriError::Collinear)
        );
        assert_eq!(
            triangulate(&pts(&[(0.0, 0.0), (1.0, 0.0), (0.0, f64::NAN)])),
            Err(TriError::NonFinite)
        );
    }

    #[test]
    fn collinear_start_then_offset_point() {
        let points = pts(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0), (1.5, 1.0)]);
        let tris = triangulate(&points).unwrap();
        assert_eq!(tris.len(), 3);
        check_delaunay(&points, &tris);
    }
}
