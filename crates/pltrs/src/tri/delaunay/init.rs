//! The initial simplex and hull (`qh_initbuild`, `qh_maxsimplex`,
//! `qh_initialvertices`, `qh_createsimplex`, `qh_initialhull`).

use super::geom::{self, Vec3};
use super::hull::{FacetId, Hull, HullError, HullResult, VertexId};
use super::precision::{ExtremePoints, Precision};
use super::sets;

/// qhull's `qh_RATIOmaxsimplex`: a determinant this much smaller than the
/// expected one triggers a search of all points.
const RATIO_MAX_SIMPLEX: f64 = 1.0e-3;

/// qhull's `qh_MAXnarrow`: neighbouring initial facets this close to
/// opposite make a narrow hull.
const MAX_NARROW: f64 = -0.999_999_99;

impl Hull {
    /// Builds the initial simplex, partitions the points into outside and
    /// coplanar sets and selects the first facet to process.
    pub(super) fn init_build(&mut self, extremes: &ExtremePoints) -> HullResult<()> {
        let simplex = max_simplex(&self.points, extremes, &self.prec)?;
        let mut vertices = Vec::with_capacity(4);
        for &point in &simplex {
            let v = self.new_vertex(point);
            vertices.insert(0, v);
        }
        self.initial_hull(&vertices)?;
        self.partition_all(&vertices)?;
        self.reset_lists(true);
        self.facet_next = Some(self.facet_list);
        self.furthest_next();
        Ok(())
    }

    /// Creates the simplex's four facets with full neighbour sets
    /// (qhull's `qh_createsimplex`).
    fn create_simplex(&mut self, vertices: &[VertexId]) {
        let mut toporient = true;
        for (i, &vertex) in vertices.iter().enumerate() {
            let f = self.new_facet();
            let facet = self.facet_mut(f);
            facet.vertices = sets::without_nth(vertices, i);
            facet.toporient = toporient;
            self.append_facet(f);
            self.facet_mut(f).newfacet = true;
            self.append_vertex(vertex);
            toporient = !toporient;
        }
        let facets = self.facets_from(self.newfacet_list);
        for &f in &facets {
            self.facet_mut(f).neighbors = facets.iter().copied().filter(|&g| g != f).collect();
        }
    }

    /// Creates the initial hull and orients it around its centre
    /// (qhull's `qh_initialhull`).
    fn initial_hull(&mut self, vertices: &[VertexId]) -> HullResult<()> {
        self.create_simplex(vertices);
        self.reset_lists(true);
        self.facet_next = Some(self.facet_list);
        let corners: Vec<Vec3> = vertices.iter().map(|&v| self.vertex_point(v)).collect();
        self.interior_point = geom::center(&corners);
        let first = self.facet_list;
        self.set_facet_plane(first);
        self.facet_mut(first).flipped = false;
        if self.dist(&self.interior_point, first) > self.prec.dist_round {
            for f in self.facets_from(Some(self.facet_list)) {
                self.facet_mut(f).toporient ^= true;
            }
            self.set_facet_plane(first);
        }
        let facets = self.facets_from(Some(self.facet_list));
        for &f in &facets {
            if f != first {
                self.set_facet_plane(f);
            }
        }
        for &f in &facets {
            self.facet_mut(f).flipped = false;
            if self.dist(&self.interior_point, f) > self.prec.dist_round {
                for &g in &facets {
                    self.facet_mut(g).flipped = false;
                    self.facet_mut(g).toporient ^= true;
                    self.orient_outside(g);
                }
                break;
            }
        }
        let mut min_angle = f64::MAX;
        for &f in &facets {
            if !self.check_flipped(f) {
                return Err(HullError::Singular);
            }
            for &n in &self.facet(f).neighbors {
                let angle = geom::angle(&self.normal(f), &self.normal(n));
                if min_angle > angle {
                    min_angle = angle;
                }
            }
        }
        self.narrow = min_angle < MAX_NARROW;
        self.check_initial_convexity(&facets)
    }

    /// Fails unless every vertex is clearly below the facet opposite it
    /// (qhull's `qh_checkconvex` for the initial simplex).
    fn check_initial_convexity(&self, facets: &[FacetId]) -> HullResult<()> {
        for &f in facets {
            let facet = self.facet(f);
            for (&n, &v) in facet.neighbors.iter().zip(&facet.vertices) {
                if self.dist(&self.vertex_point(v), n) >= -self.prec.dist_round {
                    return Err(HullError::Singular);
                }
            }
        }
        Ok(())
    }
}

/// The points of a simplex with a large volume, preferring the extreme
/// points (qhull's `qh_maxsimplex` for an empty starting simplex).
fn max_simplex(
    points: &[Vec3],
    extremes: &ExtremePoints,
    prec: &Precision,
) -> HullResult<Vec<usize>> {
    let (mut max_coord, mut min_coord) = (-f64::MAX, f64::MAX);
    let (mut max_x, mut min_x) = (0, 0);
    for &p in extremes {
        if max_coord < points[p][0] {
            max_coord = points[p][0];
            max_x = p;
        }
        if min_coord > points[p][0] {
            min_coord = points[p][0];
            min_x = p;
        }
    }
    let mut max_det = max_coord - min_coord;
    let mut simplex = vec![min_x];
    sets::append_unique(&mut simplex, max_x);
    if simplex.len() < 2 {
        return Err(HullError::Singular);
    }
    for i in 2..4 {
        let prev_det = max_det;
        let mut max_point = None;
        let mut max_nearzero = false;
        max_det = -1.0;
        let base: Vec<Vec3> = simplex.iter().map(|&p| points[p]).collect();
        for &p in extremes {
            if !simplex.contains(&p) && max_point != Some(p) {
                let (det, nearzero) = geom::det_simplex(&points[p], &base, i, prec);
                if det.abs() > max_det {
                    max_det = det.abs();
                    max_point = Some(p);
                    max_nearzero = nearzero;
                }
            }
        }
        let target_det = prev_det * prec.max_width;
        let min_det = 10.0 * RATIO_MAX_SIMPLEX * target_det;
        let maybe_false_narrow = max_det > 0.0 && max_det / target_det < RATIO_MAX_SIMPLEX;
        if max_point.is_none() || max_nearzero || maybe_false_narrow {
            for (p, point) in points.iter().enumerate() {
                if extremes.contains(&p) || simplex.contains(&p) {
                    continue;
                }
                let (det, nearzero) = geom::det_simplex(point, &base, i, prec);
                if det.abs() > max_det {
                    max_det = det.abs();
                    max_point = Some(p);
                    max_nearzero = nearzero;
                    if !max_nearzero && max_det > min_det {
                        break;
                    }
                }
            }
        }
        simplex.push(max_point.ok_or(HullError::Singular)?);
    }
    Ok(simplex)
}
