//! Delaunay triangulation of a planar point set, identical to matplotlib's.
//!
//! matplotlib triangulates with qhull 2020.2: its `_qhull.delaunay`
//! subtracts the mean from the points and runs `qhull d Qt Qbb Qc Qz`, the
//! lower convex hull of the points lifted to the paraboloid
//! `z = x² + y²`. The triangles and neighbours it returns are qhull's
//! facets in qhull's own list order, so reproducing them exactly means
//! reproducing qhull's construction step by step. The submodules port the
//! parts of qhull these options use, with qhull's floating-point operation
//! order wherever a result feeds a decision:
//!
//! * [`precision`] — the `Qbb` scaling and the roundoff constants;
//! * [`init`] — the initial simplex and hull;
//! * [`partition`], [`findbest`] — outside and coplanar point sets;
//! * [`cone`], [`matching`] — adding a point: horizon, cone of new facets;
//! * [`merge`] — merging coplanar and concave facets (cocircular sites);
//! * [`triangulate`] — splitting merged facets into triangles (`Qt`);
//! * [`hull`], [`lists`], [`sets`] — storage, facet lists and set order;
//! * [`geom`], [`planes`] — geometry with qhull's rounding.
//!
//! `Qz` adds a point "at infinity" above the paraboloid so cocircular input
//! is not degenerate; `Qc` keeps points that are not vertices (such as all
//! but one of a set of duplicate sites) in coplanar sets instead of
//! failing. No random numbers are involved for these options.

mod cone;
mod findbest;
mod geom;
mod hull;
mod init;
mod lists;
mod matching;
mod merge;
mod partition;
mod planes;
mod precision;
mod sets;
mod triangulate;

use plt_core::Point;

use self::geom::Vec3;
use self::hull::{Hull, HullResult};
use self::precision::{Precision, QhullMinMax};
use super::TriError;

/// A Delaunay triangulation as matplotlib's qhull wrapper returns it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Delaunay {
    /// Anticlockwise triangles of point indices, in qhull's facet order.
    pub(crate) triangles: Vec<[usize; 3]>,
    /// For each triangle, the triangle across the edge from vertex `i` to
    /// vertex `(i + 1) % 3`, if any.
    pub(crate) neighbors: Vec<[Option<usize>; 3]>,
}

/// Computes the Delaunay triangulation of `points` exactly as matplotlib
/// does. Duplicate points are left out; fewer than three distinct points
/// or (nearly) collinear points are an error.
pub(crate) fn triangulate(points: &[Point]) -> Result<Delaunay, TriError> {
    if points.iter().any(|p| !p.is_finite()) {
        return Err(TriError::NonFinite);
    }
    if points.len() < 3 {
        return Err(TriError::TooFewPoints);
    }
    if !has_three_unique(points) {
        return Err(TriError::TooFewUniquePoints);
    }
    let hull = build(lift(points)).map_err(|_| TriError::Collinear)?;
    Ok(hull.delaunay())
}

/// Whether `points` contains at least three distinct points (matplotlib's
/// `at_least_3_unique_points`).
fn has_three_unique(points: &[Point]) -> bool {
    let mut unique: Vec<Point> = Vec::with_capacity(3);
    for p in points {
        if !unique.contains(p) {
            unique.push(*p);
            if unique.len() == 3 {
                return true;
            }
        }
    }
    false
}

/// The points centred on their mean and lifted to the paraboloid, followed
/// by qhull's point at infinity (option `Qz`): the centroid raised above the
/// highest lifted point (matplotlib's wrapper and `qh_projectinput`).
fn lift(points: &[Point]) -> Vec<Vec3> {
    let count = points.len() as f64;
    let (mut x_mean, mut y_mean) = (0.0, 0.0);
    for p in points {
        x_mean += p.x;
        y_mean += p.y;
    }
    x_mean /= count;
    y_mean /= count;
    let mut lifted = Vec::with_capacity(points.len() + 1);
    let mut infinity = [0.0; 3];
    let mut max_lifted = 0.0_f64;
    for p in points {
        let (x, y) = (p.x - x_mean, p.y - y_mean);
        let mut lift = 0.0;
        lift += x * x;
        infinity[0] += x;
        lift += y * y;
        infinity[1] += y;
        lifted.push([x, y, lift]);
        max_lifted = max_lifted.max_qhull(lift);
    }
    infinity[0] /= count;
    infinity[1] /= count;
    infinity[2] = max_lifted * 1.1;
    lifted.push(infinity);
    lifted
}

/// Runs qhull on the lifted points (`qh_qhull` followed by the
/// triangulation of `qh_prepare_output`).
fn build(mut points: Vec<Vec3>) -> HullResult<Hull> {
    let (prec, extremes) = Precision::scale_and_measure(&mut points)?;
    let mut hull = Hull::new(points, prec);
    hull.init_build(&extremes)?;
    hull.build_hull()?;
    hull.triangulate()?;
    Ok(hull)
}

impl Hull {
    /// The lower facets as triangles and their neighbours, oriented like
    /// matplotlib's `_qhull.delaunay` does from qhull's facet orientation.
    fn delaunay(&self) -> Delaunay {
        let lower: Vec<_> = self
            .facets_from(Some(self.facet_list))
            .into_iter()
            .filter(|&f| !self.facet(f).upperdelaunay)
            .collect();
        let mut index = vec![None; self.facets.len()];
        for (i, f) in lower.iter().enumerate() {
            index[f.index()] = Some(i);
        }
        let mut triangles = Vec::with_capacity(lower.len());
        let mut neighbors = Vec::with_capacity(lower.len());
        for &f in &lower {
            let facet = self.facet(f);
            let [a, b, c] = [0, 1, 2].map(|i| self.vertex(facet.vertices[i]).point);
            let [n0, n1, n2] = [0, 1, 2].map(|i| index[facet.neighbors[i].index()]);
            if facet.toporient {
                triangles.push([a, b, c]);
                neighbors.push([n2, n0, n1]);
            } else {
                triangles.push([c, b, a]);
                neighbors.push([n0, n2, n1]);
            }
        }
        Delaunay {
            triangles,
            neighbors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(coords: &[(f64, f64)]) -> Vec<Point> {
        coords.iter().map(|&(x, y)| Point::new(x, y)).collect()
    }

    fn orient(points: &[Point], t: [usize; 3]) -> f64 {
        let [a, b, c] = t.map(|i| points[i]);
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
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
        let tri = triangulate(&points).unwrap();
        assert_eq!(tri.triangles.len(), 2 * 20 - 2 - 14);
        assert!(tri.triangles.iter().all(|&t| orient(&points, t) > 0.0));
    }

    #[test]
    fn neighbours_are_mutual() {
        let points = pts(&[(0.0, 0.0), (1.0, 0.1), (0.2, 1.0), (1.3, 1.1), (0.6, 0.4)]);
        let tri = triangulate(&points).unwrap();
        for (t, nb) in tri.neighbors.iter().enumerate() {
            for (e, n) in nb.iter().enumerate() {
                if let Some(n) = *n {
                    let [a, b] = [tri.triangles[t][e], tri.triangles[t][(e + 1) % 3]];
                    let k = (0..3).find(|&k| tri.triangles[n][k] == b).unwrap();
                    assert_eq!(tri.triangles[n][(k + 1) % 3], a);
                    assert_eq!(tri.neighbors[n][k], Some(t));
                }
            }
        }
    }

    #[test]
    fn duplicates_are_left_out() {
        let points = pts(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.2)]);
        let tri = triangulate(&points).unwrap();
        assert_eq!(tri.triangles.len(), 2);
        assert!(tri.triangles.iter().all(|t| !t.contains(&3)));
    }

    #[test]
    fn degenerate_inputs_are_rejected() {
        assert_eq!(
            triangulate(&pts(&[(0.0, 0.0), (1.0, 1.0)])),
            Err(TriError::TooFewPoints)
        );
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
}
