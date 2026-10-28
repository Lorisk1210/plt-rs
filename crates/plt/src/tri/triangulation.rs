//! The [`Triangulation`]: points, triangles, mask and derived topology.

use std::collections::{BTreeSet, HashMap};

use plt_core::Point;

use super::{TriError, TriFinder, delaunay};

/// An unstructured triangular grid (matplotlib's `Triangulation`).
///
/// Triangles are stored anticlockwise: explicit triangles given clockwise
/// are corrected on construction (`[a, b, c]` becomes `[a, c, b]`; zero-area
/// triangles are kept as given), as matplotlib's C++ triangulation does;
/// the vertex order they were given in, which matplotlib's plotting
/// functions draw, stays available ([`Triangulation::given_triangles`]).
/// Masked triangles are ignored by every derived quantity
/// ([`Triangulation::edges`], [`Triangulation::neighbors`], point
/// location, interpolation and contouring) but keep their index.
#[derive(Clone, Debug, PartialEq)]
pub struct Triangulation {
    x: Vec<f64>,
    y: Vec<f64>,
    triangles: Vec<[usize; 3]>,
    /// The explicit triangles as given, when some had to be turned
    /// anticlockwise.
    given: Option<Vec<[usize; 3]>>,
    mask: Option<Vec<bool>>,
    neighbors: Vec<[Option<usize>; 3]>,
    is_delaunay: bool,
}

/// A triangle edge: `(triangle, edge)` where edge `e` runs from vertex `e`
/// to vertex `(e + 1) % 3`.
pub(crate) type TriEdge = (usize, usize);

impl Triangulation {
    /// A triangulation with explicit `triangles` (point indices).
    pub fn new(x: &[f64], y: &[f64], triangles: Vec<[usize; 3]>) -> Result<Self, TriError> {
        check_lengths(x, y)?;
        if let Some(&index) = triangles.iter().flatten().find(|&&v| v >= x.len()) {
            return Err(TriError::PointIndex {
                index,
                npoints: x.len(),
            });
        }
        let mut tri = Self {
            x: x.to_vec(),
            y: y.to_vec(),
            triangles,
            given: None,
            mask: None,
            neighbors: Vec::new(),
            is_delaunay: false,
        };
        tri.correct_orientation();
        tri.neighbors = tri.compute_neighbors();
        Ok(tri)
    }

    /// The Delaunay triangulation of the points.
    ///
    /// The triangles, their vertex order and their neighbours are exactly
    /// matplotlib's (qhull's). Duplicate points are left out (one of each
    /// set of duplicates is used). Fails for non-finite coordinates, fewer
    /// than three distinct points or collinear points.
    pub fn delaunay(x: &[f64], y: &[f64]) -> Result<Self, TriError> {
        check_lengths(x, y)?;
        let points: Vec<Point> = x.iter().zip(y).map(|(&x, &y)| Point::new(x, y)).collect();
        let delaunay = delaunay::triangulate(&points)?;
        Ok(Self {
            x: x.to_vec(),
            y: y.to_vec(),
            triangles: delaunay.triangles,
            given: None,
            mask: None,
            neighbors: delaunay.neighbors,
            is_delaunay: true,
        })
    }

    /// The same triangulation with a triangle mask (`true` = masked out).
    pub fn with_mask(mut self, mask: Option<Vec<bool>>) -> Result<Self, TriError> {
        self.set_mask(mask)?;
        Ok(self)
    }

    /// Sets or clears the triangle mask, recomputing the topology.
    pub fn set_mask(&mut self, mask: Option<Vec<bool>>) -> Result<(), TriError> {
        if let Some(mask) = &mask
            && mask.len() != self.triangles.len()
        {
            return Err(TriError::MaskLength {
                expected: self.triangles.len(),
                actual: mask.len(),
            });
        }
        self.mask = mask;
        self.neighbors = self.compute_neighbors();
        Ok(())
    }

    /// Point x coordinates.
    pub fn x(&self) -> &[f64] {
        &self.x
    }

    /// Point y coordinates.
    pub fn y(&self) -> &[f64] {
        &self.y
    }

    /// Number of points.
    pub fn npoints(&self) -> usize {
        self.x.len()
    }

    /// Number of triangles, masked or not.
    pub fn ntri(&self) -> usize {
        self.triangles.len()
    }

    /// Point `index` as a [`Point`].
    pub fn point(&self, index: usize) -> Point {
        Point::new(self.x[index], self.y[index])
    }

    /// All triangles (anticlockwise point indices), masked or not.
    pub fn triangles(&self) -> &[[usize; 3]] {
        &self.triangles
    }

    /// All triangles, masked or not, in the vertex order they were given
    /// (matplotlib's `triangles` until its C++ triangulation turns the
    /// clockwise ones in place).
    ///
    /// ```
    /// use plt::tri::Triangulation;
    /// let tri = Triangulation::new(&[0.0, 1.0, 0.0], &[0.0, 0.0, 1.0], vec![[0, 2, 1]])?;
    /// assert_eq!(tri.triangles(), [[0, 1, 2]]);
    /// assert_eq!(tri.given_triangles(), [[0, 2, 1]]);
    /// # Ok::<(), plt::tri::TriError>(())
    /// ```
    pub fn given_triangles(&self) -> &[[usize; 3]] {
        self.given.as_deref().unwrap_or(&self.triangles)
    }

    /// The unmasked triangles in the vertex order they were given, which
    /// matplotlib's `tripcolor` and `plot_trisurf` draw (they read
    /// `get_masked_triangles` before anything corrects the orientation).
    pub(crate) fn drawn_triangles(&self) -> Vec<[usize; 3]> {
        let given = self.given_triangles();
        self.unmasked().map(|t| given[t]).collect()
    }

    /// The triangle mask, if any.
    pub fn mask(&self) -> Option<&[bool]> {
        self.mask.as_deref()
    }

    /// Whether triangle `tri` is masked out.
    pub fn is_masked(&self, tri: usize) -> bool {
        self.mask.as_ref().is_some_and(|m| m[tri])
    }

    /// Whether the triangles were computed by [`Triangulation::delaunay`].
    pub fn is_delaunay(&self) -> bool {
        self.is_delaunay
    }

    /// The unmasked triangles, in order (matplotlib's
    /// `get_masked_triangles`).
    pub fn masked_triangles(&self) -> Vec<[usize; 3]> {
        self.unmasked().map(|t| self.triangles[t]).collect()
    }

    /// Indices of the unmasked triangles.
    pub fn unmasked(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.triangles.len()).filter(|&t| !self.is_masked(t))
    }

    /// Whether each point is a corner of an unmasked triangle.
    pub(crate) fn used_points(&self) -> Vec<bool> {
        let mut used = vec![false; self.npoints()];
        for t in self.unmasked() {
            for v in self.triangles[t] {
                used[v] = true;
            }
        }
        used
    }

    /// The x and y coordinates of the points `used` marks (see
    /// [`used_points`](Self::used_points)), in order.
    pub(crate) fn coordinates_of(&self, used: &[bool]) -> (Vec<f64>, Vec<f64>) {
        let pick = |values: &[f64]| -> Vec<f64> {
            values
                .iter()
                .zip(used)
                .filter(|(_, used)| **used)
                .map(|(value, _)| *value)
                .collect()
        };
        (pick(&self.x), pick(&self.y))
    }

    /// For each triangle, the triangles across its edges: entry `[t][e]` is
    /// the unmasked triangle sharing edge `e` (from vertex `e` to vertex
    /// `(e + 1) % 3`) of unmasked triangle `t`; masked triangles have none.
    pub fn neighbors(&self) -> &[[Option<usize>; 3]] {
        &self.neighbors
    }

    /// Every edge of an unmasked triangle once, as `[j, i]` with `j > i`,
    /// sorted.
    pub fn edges(&self) -> Vec<[usize; 2]> {
        let set: BTreeSet<[usize; 2]> = self
            .unmasked()
            .flat_map(|t| {
                let v = self.triangles[t];
                (0..3).map(move |e| {
                    let (a, b) = (v[e], v[(e + 1) % 3]);
                    [a.max(b), a.min(b)]
                })
            })
            .collect();
        set.into_iter().collect()
    }

    /// Coefficients `[a, b, c]` of the plane `z = a x + b y + c` through each
    /// triangle (zeros for masked triangles). Degenerate (collinear)
    /// triangles use the Moore–Penrose pseudo-inverse.
    pub fn plane_coefficients(&self, z: &[f64]) -> Result<Vec<[f64; 3]>, TriError> {
        self.check_values(z)?;
        Ok((0..self.ntri())
            .map(|t| {
                if self.is_masked(t) {
                    [0.0; 3]
                } else {
                    plane(self.triangles[t].map(|v| [self.x[v], self.y[v], z[v]]))
                }
            })
            .collect())
    }

    /// A point locator for the unmasked triangles.
    pub fn trifinder(&self) -> Result<TriFinder<'_>, TriError> {
        TriFinder::new(self)
    }

    /// Fails unless `values` has one entry per point.
    pub(crate) fn check_values(&self, values: &[f64]) -> Result<(), TriError> {
        if values.len() == self.npoints() {
            Ok(())
        } else {
            Err(TriError::ValueLength {
                expected: self.npoints(),
                actual: values.len(),
            })
        }
    }

    /// The edge of the neighbouring triangle that matches edge `edge` of
    /// `tri` (it starts at this edge's end point).
    pub(crate) fn neighbor_edge(&self, tri: usize, edge: usize) -> Option<TriEdge> {
        let neighbor = self.neighbors[tri][edge]?;
        let end = self.triangles[tri][(edge + 1) % 3];
        let index = self.triangles[neighbor].iter().position(|&v| v == end)?;
        Some((neighbor, index))
    }

    /// The boundaries of the unmasked region: closed loops of triangle edges
    /// without a neighbour, each starting at its smallest `(tri, edge)` and
    /// ordered by that start (matplotlib's `calculate_boundaries`).
    pub(crate) fn boundaries(&self) -> Vec<Vec<TriEdge>> {
        let mut remaining: BTreeSet<TriEdge> = self
            .unmasked()
            .flat_map(|t| (0..3).map(move |e| (t, e)))
            .filter(|&(t, e)| self.neighbors[t][e].is_none())
            .collect();
        let mut boundaries = Vec::new();
        while let Some(start) = remaining.pop_first() {
            let mut boundary = vec![start];
            let (mut tri, mut edge) = start;
            loop {
                edge = (edge + 1) % 3;
                let point = self.triangles[tri][edge];
                // A walk round `point` through more triangle edges than
                // there are repeats itself, which only a triangle that is
                // its own neighbour makes happen; it ends there, where
                // matplotlib's goes on forever.
                for _ in 0..3 * self.triangles.len() {
                    let Some(next) = self.neighbors[tri][edge] else {
                        break;
                    };
                    tri = next;
                    edge = self.triangles[tri]
                        .iter()
                        .position(|&v| v == point)
                        .expect("neighbouring triangles share the pivot point");
                }
                if (tri, edge) == start || !remaining.remove(&(tri, edge)) {
                    break;
                }
                boundary.push((tri, edge));
            }
            boundaries.push(boundary);
        }
        boundaries
    }

    /// Makes every positive-area triangle anticlockwise.
    fn correct_orientation(&mut self) {
        let given = self.triangles.clone();
        for t in 0..self.triangles.len() {
            let [a, b, c] = self.triangles[t].map(|v| self.point(v));
            let (ab, ac) = (b - a, c - a);
            if ab.x * ac.y - ab.y * ac.x < 0.0 {
                self.triangles[t].swap(1, 2);
            }
        }
        if given != self.triangles {
            self.given = Some(given);
        }
    }

    /// matplotlib's neighbour matching: each directed edge pairs with the
    /// first unmatched opposite edge seen so far. A triangle that repeats a
    /// point can be its own neighbour, as in matplotlib.
    fn compute_neighbors(&self) -> Vec<[Option<usize>; 3]> {
        let mut neighbors = vec![[None; 3]; self.triangles.len()];
        let mut open: HashMap<(usize, usize), TriEdge> = HashMap::new();
        for t in self.unmasked() {
            let v = self.triangles[t];
            for e in 0..3 {
                let (start, end) = (v[e], v[(e + 1) % 3]);
                if let Some((other, other_edge)) = open.remove(&(end, start)) {
                    neighbors[t][e] = Some(other);
                    neighbors[other][other_edge] = Some(t);
                } else {
                    open.insert((start, end), (t, e));
                }
            }
        }
        neighbors
    }
}

fn check_lengths(x: &[f64], y: &[f64]) -> Result<(), TriError> {
    if x.len() == y.len() {
        Ok(())
    } else {
        Err(TriError::CoordinateLengths {
            x: x.len(),
            y: y.len(),
        })
    }
}

/// The plane through three `[x, y, z]` points (matplotlib's C++ formula).
fn plane([p0, p1, p2]: [[f64; 3]; 3]) -> [f64; 3] {
    let s1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let s2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
    let normal = [
        s1[1] * s2[2] - s1[2] * s2[1],
        s1[2] * s2[0] - s1[0] * s2[2],
        s1[0] * s2[1] - s1[1] * s2[0],
    ];
    if normal[2] == 0.0 {
        let sum2 = s1[0] * s1[0] + s1[1] * s1[1] + s2[0] * s2[0] + s2[1] * s2[1];
        let a = (s1[0] * s1[2] + s2[0] * s2[2]) / sum2;
        let b = (s1[1] * s1[2] + s2[1] * s2[2]) / sum2;
        [a, b, p0[2] - a * p0[0] - b * p0[1]]
    } else {
        let dot = normal[0] * p0[0] + normal[1] * p0[1] + normal[2] * p0[2];
        [
            -normal[0] / normal[2],
            -normal[1] / normal[2],
            dot / normal[2],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid3() -> Triangulation {
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
        Triangulation::new(&x, &y, t).unwrap()
    }

    #[test]
    fn clockwise_triangles_are_corrected() {
        let tri = Triangulation::new(&[0.0, 1.0, 0.0], &[0.0, 0.0, 1.0], vec![[0, 2, 1]]).unwrap();
        assert_eq!(tri.triangles(), &[[0, 1, 2]]);
        let flat = Triangulation::new(&[0.0, 1.0, 2.0], &[0.0, 0.0, 0.0], vec![[0, 2, 1]]).unwrap();
        assert_eq!(flat.triangles(), &[[0, 2, 1]]);
    }

    #[test]
    fn drawn_triangles_keep_the_given_order() {
        let (x, y) = ([0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]);
        let given = vec![[0, 2, 1], [1, 2, 3]];
        let tri = Triangulation::new(&x, &y, given.clone()).unwrap();
        assert_eq!(tri.triangles(), &[[0, 1, 2], [1, 3, 2]]);
        assert_eq!(tri.given_triangles(), given);
        let tri = tri.with_mask(Some(vec![true, false])).unwrap();
        assert_eq!(tri.drawn_triangles(), [[1, 2, 3]]);
        let delaunay = Triangulation::delaunay(&x, &y).unwrap();
        assert_eq!(delaunay.given_triangles(), delaunay.triangles());
    }

    #[test]
    fn edges_and_neighbors_respect_the_mask() {
        let mut tri = grid3();
        assert_eq!(tri.edges().len(), 16);
        assert_eq!(tri.edges()[..3], [[1, 0], [2, 1], [3, 0]]);
        assert_eq!(tri.neighbors()[0], [None, Some(3), Some(1)]);
        let mask = vec![false, false, false, true, false, true, false, false];
        tri.set_mask(Some(mask)).unwrap();
        assert_eq!(tri.neighbors()[3], [None; 3]);
        assert_eq!(tri.neighbors()[0], [None, None, Some(1)]);
        assert!(!tri.edges().contains(&[6, 3]));
        assert_eq!(tri.masked_triangles().len(), 6);
        assert_eq!(
            tri.set_mask(Some(vec![true])),
            Err(TriError::MaskLength {
                expected: 8,
                actual: 1
            })
        );
    }

    #[test]
    fn plane_coefficients_reproduce_linear_fields() {
        let tri = grid3();
        let z: Vec<f64> = (0..9)
            .map(|i| 3.0 + 0.75 * tri.x()[i] - 0.5 * tri.y()[i])
            .collect();
        for [a, b, c] in tri.plane_coefficients(&z).unwrap() {
            assert!((a - 0.75).abs() < 1e-14 && (b + 0.5).abs() < 1e-14 && (c - 3.0).abs() < 1e-14);
        }
        assert!(tri.plane_coefficients(&[1.0]).is_err());
        let flat = Triangulation::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0], vec![[0, 1, 2]]).unwrap();
        let [a, b, c] = flat.plane_coefficients(&[1.0, 2.0, 3.0]).unwrap()[0];
        assert!((a - 0.5).abs() < 1e-15 && (b - 0.5).abs() < 1e-15 && (c - 1.0).abs() < 1e-15);
    }

    #[test]
    fn boundaries_follow_the_hull() {
        let tri = grid3();
        let boundaries = tri.boundaries();
        assert_eq!(boundaries.len(), 1);
        assert_eq!(boundaries[0].len(), 8);
        assert_eq!(boundaries[0][0], (0, 0));
    }

    #[test]
    fn boundaries_of_triangles_repeating_a_point_end() {
        let (x, y) = ([0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]);
        // [0, 1, 0] is its own neighbour, as in matplotlib, and walking
        // round its point 0 used to go on forever.
        let tri = Triangulation::new(&x, &y, vec![[0, 1, 0], [3, 3, 3]]).unwrap();
        assert_eq!(tri.neighbors()[0], [Some(0), Some(0), None]);
        assert_eq!(tri.boundaries(), [vec![(0, 2)], vec![(1, 2)]]);
    }

    #[test]
    fn invalid_inputs() {
        assert!(matches!(
            Triangulation::new(&[0.0], &[0.0, 1.0], vec![]),
            Err(TriError::CoordinateLengths { .. })
        ));
        assert!(matches!(
            Triangulation::new(&[0.0, 1.0, 0.0], &[0.0, 0.0, 1.0], vec![[0, 1, 3]]),
            Err(TriError::PointIndex { index: 3, .. })
        ));
    }
}
