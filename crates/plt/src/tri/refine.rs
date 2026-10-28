//! Uniform mesh refinement (matplotlib's `UniformTriRefiner`).

use super::{CubicKind, CubicTriInterpolator, TriError, TriInterpolator, Triangulation};

/// Refines a triangulation by splitting every triangle into four at its
/// edge midpoints, recursively.
#[derive(Clone, Copy, Debug)]
pub struct UniformTriRefiner<'a> {
    tri: &'a Triangulation,
}

/// A field refined together with its triangulation.
#[derive(Clone, Debug, PartialEq)]
pub struct RefinedField {
    /// The refined triangulation.
    pub triangulation: Triangulation,
    /// The interpolated value at each refined point; `None` for points in
    /// no triangle or where the interpolant is not finite.
    pub z: Vec<Option<f64>>,
}

impl<'a> UniformTriRefiner<'a> {
    /// A refiner for `tri`.
    pub fn new(tri: &'a Triangulation) -> Self {
        Self { tri }
    }

    /// The triangulation with each triangle split into `4^subdiv`
    /// children; children of masked triangles are masked.
    ///
    /// Midpoints are `(p + q) * 0.5`. An edge between a masked and an
    /// unmasked triangle gets two (coincident) midpoints, because masked
    /// triangles have no neighbours.
    pub fn refine_triangulation(&self, subdiv: usize) -> Triangulation {
        self.refine_with_ancestors(subdiv).0
    }

    /// Like [`UniformTriRefiner::refine_triangulation`], also returning for
    /// each refined point the index of the original triangle containing it
    /// (preferring unmasked ancestors, then the highest index).
    pub fn refine_triangulation_with_index(
        &self,
        subdiv: usize,
    ) -> (Triangulation, Vec<Option<usize>>) {
        let (refined, ancestors) = self.refine_with_ancestors(subdiv);
        let mut found = vec![None; refined.npoints()];
        let masked_ancestor = |a: usize| self.tri.is_masked(a);
        for pass_masked in [true, false] {
            for (child, &ancestor) in refined.triangles().iter().zip(&ancestors) {
                if masked_ancestor(ancestor) == pass_masked {
                    for &v in child {
                        found[v] = Some(ancestor);
                    }
                }
            }
        }
        (refined, found)
    }

    /// Refines the triangulation and interpolates `z` at the new points
    /// with `interpolator` (default: a [`CubicTriInterpolator`] minimising
    /// the bending energy). Each point is evaluated in its ancestor
    /// triangle as reported by
    /// [`UniformTriRefiner::refine_triangulation_with_index`].
    pub fn refine_field(
        &self,
        z: &[f64],
        interpolator: Option<&dyn TriInterpolator>,
        subdiv: usize,
    ) -> Result<RefinedField, TriError> {
        let default;
        let interp: &dyn TriInterpolator = if let Some(interp) = interpolator {
            interp
        } else {
            default = CubicTriInterpolator::new(self.tri, z, &CubicKind::MinE)?;
            &default
        };
        let (triangulation, found) = self.refine_triangulation_with_index(subdiv);
        let z = found
            .iter()
            .enumerate()
            .map(|(i, parent)| {
                let p = triangulation.point(i);
                parent
                    .map(|t| interp.value_in(t, p.x, p.y))
                    .filter(|v| v.is_finite())
            })
            .collect();
        Ok(RefinedField { triangulation, z })
    }

    fn refine_with_ancestors(self, subdiv: usize) -> (Triangulation, Vec<usize>) {
        let mut tri = self.tri.clone();
        let mut ancestors: Vec<usize> = (0..tri.ntri()).collect();
        for _ in 0..subdiv {
            tri = refine_once(&tri);
            ancestors = ancestors.iter().flat_map(|&a| [a; 4]).collect();
        }
        (tri, ancestors)
    }
}

/// One level of 4-way subdivision (matplotlib's
/// `_refine_triangulation_once`).
fn refine_once(tri: &Triangulation) -> Triangulation {
    let ntri = tri.ntri();
    let triangles = tri.triangles();
    let neighbors = tri.neighbors();
    let mut x = tri.x().to_vec();
    let mut y = tri.y().to_vec();
    // Each edge's midpoint is created by its "master": the triangle with
    // the higher index (border edges have no other side, and a triangle
    // that repeats a point and is its own neighbour masters its edges).
    // Masters are numbered apex by apex, then triangle by triangle.
    let mut midside = vec![[usize::MAX; 3]; ntri];
    for apex in 0..3 {
        for t in 0..ntri {
            let is_master = neighbors[t][apex].is_none_or(|n| t >= n);
            if is_master {
                let (a, b) = (triangles[t][apex], triangles[t][(apex + 1) % 3]);
                x.push(f64::midpoint(tri.x()[a], tri.x()[b]));
                y.push(f64::midpoint(tri.y()[a], tri.y()[b]));
                midside[t][apex] = x.len() - 1;
            }
        }
    }
    // The other triangles take the midpoint of the master's edge they
    // share (even a flat triangle that shares all three edges with one
    // master).
    for t in 0..ntri {
        for apex in 0..3 {
            if let Some((master, master_apex)) = tri.neighbor_edge(t, apex).filter(|&(n, _)| t < n)
            {
                midside[t][apex] = midside[master][master_apex];
            }
        }
    }
    let children = triangles
        .iter()
        .zip(&midside)
        .flat_map(|(c, m)| {
            [
                [c[0], m[0], m[2]],
                [c[1], m[1], m[0]],
                [c[2], m[2], m[1]],
                [m[0], m[1], m[2]],
            ]
        })
        .collect();
    let mask = tri
        .mask()
        .map(|mask| mask.iter().flat_map(|&m| [m; 4]).collect());
    Triangulation::new(&x, &y, children)
        .and_then(|child| child.with_mask(mask))
        .expect("refined triangles reference valid points and keep the mask length")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tri::LinearTriInterpolator;

    fn square() -> Triangulation {
        Triangulation::new(
            &[0.0, 1.0, 0.0, 1.0],
            &[0.0, 0.0, 1.0, 1.0],
            vec![[0, 1, 3], [0, 3, 2]],
        )
        .unwrap()
    }

    #[test]
    fn subdivision_counts() {
        let tri = square();
        let refiner = UniformTriRefiner::new(&tri);
        assert_eq!(refiner.refine_triangulation(0), tri);
        let once = refiner.refine_triangulation(1);
        assert_eq!((once.npoints(), once.ntri()), (9, 8));
        let twice = refiner.refine_triangulation(2);
        assert_eq!((twice.npoints(), twice.ntri()), (25, 32));
    }

    #[test]
    fn masked_neighbours_duplicate_midpoints() {
        let tri = square().with_mask(Some(vec![false, true])).unwrap();
        let (refined, found) = UniformTriRefiner::new(&tri).refine_triangulation_with_index(1);
        assert_eq!(refined.npoints(), 10);
        assert_eq!(refined.mask().unwrap().iter().filter(|m| **m).count(), 4);
        // The shared corners prefer the unmasked ancestor.
        assert_eq!(found[0], Some(0));
        assert_eq!(found[2], Some(1));
    }

    #[test]
    fn degenerate_triangles_refine() {
        // Two flat triangles sharing all three edges, and one repeating a
        // point: refining them twice used to make a child triangle that is
        // its own neighbour, and then reference a midpoint never made.
        let (x, y) = ([0.0, 1.0, 2.0, 0.0], [0.0, 0.0, 0.0, 1.0]);
        for triangles in [vec![[0, 1, 2], [2, 1, 0]], vec![[0, 3, 0]]] {
            let tri = Triangulation::new(&x, &y, triangles).unwrap();
            let refined = UniformTriRefiner::new(&tri).refine_triangulation(2);
            assert_eq!(refined.ntri(), 16 * tri.ntri());
        }
    }

    #[test]
    fn refined_linear_field() {
        let tri = square();
        let z = [1.0, 2.0, 3.0, 4.0];
        let interp = LinearTriInterpolator::new(&tri, &z).unwrap();
        let refined = UniformTriRefiner::new(&tri)
            .refine_field(&z, Some(&interp), 2)
            .unwrap();
        for (i, v) in refined.z.iter().enumerate() {
            let p = refined.triangulation.point(i);
            assert!((v.unwrap() - (1.0 + p.x + 2.0 * p.y)).abs() < 1e-14);
        }
    }
}
