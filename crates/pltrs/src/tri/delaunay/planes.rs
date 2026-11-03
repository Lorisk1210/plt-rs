//! Facet hyperplanes and distances on the hull (`qh_setfacetplane`,
//! `qh_checkflipped`, `qh_getcentrum`, `qh_getdistance`, ...).

use super::geom::{self, Vec3};
use super::hull::{FacetId, Hull, VertexId};

/// qhull's `qh_ZEROdelaunay`: facets whose normal points down by less than
/// this many angle roundoffs are upper Delaunay facets.
const ZERO_DELAUNAY: f64 = 2.0;

impl Hull {
    /// Signed distance from `point` to facet `f` (qhull's `qh_distplane`).
    pub(super) fn dist(&self, point: &Vec3, f: FacetId) -> f64 {
        geom::dist_plane(point, &self.normal(f), self.facet(f).offset)
    }

    /// Distance from point `p` (an index into the points) to facet `f`.
    pub(super) fn point_dist(&self, p: usize, f: FacetId) -> f64 {
        self.dist(&self.points[p], f)
    }

    /// Sets the hyperplane of simplicial facet `f` from its vertices
    /// (qhull's `qh_setfacetplane`), flags upper Delaunay facets and checks
    /// the orientation against the interior point.
    pub(super) fn set_facet_plane(&mut self, f: FacetId) {
        let vertices = &self.facet(f).vertices;
        let rows = [0, 1, 2].map(|i| self.vertex_point(vertices[i]));
        let toporient = self.facet(f).toporient;
        let (mut normal, mut offset, nearzero) = geom::hyperplane_det(&rows, toporient, &self.prec);
        let mut orient_outside = false;
        if nearzero {
            let p0 = rows[0];
            let gauss_rows = [rows[1], rows[2]].map(|q| [q[0] - p0[0], q[1] - p0[1], q[2] - p0[2]]);
            let gauss_nearzero;
            (normal, offset, gauss_nearzero) =
                geom::hyperplane_gauss(gauss_rows, &p0, toporient, &self.prec);
            orient_outside = gauss_nearzero;
        }
        let facet = self.facet_mut(f);
        facet.normal = Some(normal);
        facet.offset = offset;
        if orient_outside {
            self.orient_outside(f);
        }
        let threshold = -self.prec.angle_round * ZERO_DELAUNAY;
        self.facet_mut(f).upperdelaunay = self.normal(f)[2] > threshold;
        self.check_flipped(f);
    }

    /// Flags `f` as flipped if the interior point is not clearly below it
    /// (qhull's `qh_checkflipped` with `allerror`); returns `false` when
    /// flipped.
    pub(super) fn check_flipped(&mut self, f: FacetId) -> bool {
        if self.facet(f).flipped {
            return false;
        }
        let dist = self.dist(&self.interior_point, f);
        if dist >= -self.prec.dist_round {
            self.facet_mut(f).flipped = true;
            return false;
        }
        true
    }

    /// Negates the hyperplane of `f` if the interior point is above it
    /// (qhull's `qh_orientoutside`).
    pub(super) fn orient_outside(&mut self, f: FacetId) {
        if self.dist(&self.interior_point, f) > 0.0 {
            let facet = self.facet_mut(f);
            let normal = facet
                .normal
                .as_mut()
                .expect("oriented facets have a hyperplane");
            for c in normal.iter_mut() {
                *c = -*c;
            }
            facet.offset = -facet.offset;
        }
    }

    /// The centrum of `f`: its vertex centre projected onto its hyperplane
    /// (qhull's `qh_getcentrum`).
    pub(super) fn centrum(&self, f: FacetId) -> Vec3 {
        let points: Vec<Vec3> = self
            .facet(f)
            .vertices
            .iter()
            .map(|&v| self.vertex_point(v))
            .collect();
        let center = geom::center(&points);
        let dist = self.dist(&center, f);
        geom::project(&center, &self.normal(f), dist)
    }

    /// The cached centrum of `f`, computed on first use.
    pub(super) fn cached_centrum(&mut self, f: FacetId) -> Vec3 {
        if let Some(center) = self.facet(f).center.as_deref() {
            return *center;
        }
        let center = self.centrum(f);
        self.facet_mut(f).center = Some(Box::new(center));
        center
    }

    /// Minimum and maximum distance to `neighbor` of the vertices of
    /// `facet` that are not in `neighbor`, and the larger magnitude
    /// (qhull's `qh_getdistance`).
    pub(super) fn get_distance(&mut self, facet: FacetId, neighbor: FacetId) -> (f64, f64, f64) {
        for v in self.facet(facet).vertices.clone() {
            self.vertex_mut(v).seen = false;
        }
        for v in self.facet(neighbor).vertices.clone() {
            self.vertex_mut(v).seen = true;
        }
        let (mut min, mut max) = (0.0_f64, 0.0_f64);
        for &v in &self.facet(facet).vertices {
            if !self.vertex(v).seen {
                let dist = self.dist(&self.vertex_point(v), neighbor);
                if dist < min {
                    min = dist;
                } else if dist > max {
                    max = dist;
                }
            }
        }
        let worst = if max > -min { max } else { -min };
        (min, max, worst)
    }

    /// Minimum distance between two vertices of `vertices` and that pair
    /// (qhull's `qh_vertex_bestdist2`).
    pub(super) fn vertex_bestdist(
        &self,
        vertices: &[VertexId],
    ) -> (f64, Option<(VertexId, VertexId)>) {
        let mut best = f64::MAX;
        let mut pair = None;
        for (i, &a) in vertices.iter().enumerate() {
            for &b in &vertices[i + 1..] {
                let dist = geom::point_dist2(&self.vertex_point(a), &self.vertex_point(b), 3);
                if dist < best {
                    best = dist;
                    pair = Some((a, b));
                }
            }
        }
        (best.sqrt(), pair)
    }
}
