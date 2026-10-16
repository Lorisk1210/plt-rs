//! C1 cubic interpolation with reduced Hsieh–Clough–Tocher elements
//! (matplotlib's `CubicTriInterpolator`).

use super::dof::{self, Mesh, jacobian};
use super::hct::{self, pseudo_inv22sym};
use super::{TriError, TriFinder, TriInterpolator, Triangulation};

/// How the nodal gradients of a [`CubicTriInterpolator`] are chosen.
#[derive(Clone, Debug, PartialEq)]
pub enum CubicKind {
    /// Minimise the bending energy of the interpolant (the default).
    MinE,
    /// Angle-weighted average of the adjacent triangles' gradients; faster.
    Geom,
    /// Gradients given by the caller, one per point.
    User {
        /// `dz/dx` at each point.
        dzdx: Vec<f64>,
        /// `dz/dy` at each point.
        dzdy: Vec<f64>,
    },
}

/// Smooth (C1) piecewise-cubic interpolation over a triangulation.
///
/// The unmasked triangles and the points they use are rescaled to the
/// unit square; each triangle carries a reduced HCT element whose nodal
/// gradients come from the chosen [`CubicKind`].
#[derive(Clone, Debug)]
pub struct CubicTriInterpolator<'a> {
    finder: TriFinder<'a>,
    /// Compressed triangle index of each triangle (`None` when masked).
    tri_renum: Vec<Option<usize>>,
    unit: [f64; 2],
    corners: Vec<[[f64; 2]; 3]>,
    eccs: Vec<[f64; 3]>,
    dofs: Vec<[f64; 9]>,
}

impl<'a> CubicTriInterpolator<'a> {
    /// Interpolates `z` (one value per point) over `tri`.
    pub fn new(tri: &'a Triangulation, z: &[f64], kind: &CubicKind) -> Result<Self, TriError> {
        tri.check_values(z)?;
        if let CubicKind::User { dzdx, dzdy } = kind {
            tri.check_values(dzdx)?;
            tri.check_values(dzdy)?;
        }
        let finder = tri.trifinder()?;
        let compressed = Compressed::new(tri);
        let unit = [ptp(&compressed.x), ptp(&compressed.y)];
        let pts: Vec<[f64; 2]> = compressed
            .x
            .iter()
            .zip(&compressed.y)
            .map(|(x, y)| [x / unit[0], y / unit[1]])
            .collect();
        let zc = compressed.gather(z);
        let mesh = Mesh {
            pts: &pts,
            tris: &compressed.triangles,
            z: &zc,
        };
        let corners: Vec<[[f64; 2]; 3]> = compressed
            .triangles
            .iter()
            .map(|t| t.map(|v| pts[v]))
            .collect();
        let eccs: Vec<[f64; 3]> = corners.iter().map(|&c| hct::eccentricities(c)).collect();
        let dz = match kind {
            CubicKind::MinE => dof::min_energy_gradients(mesh, &eccs),
            CubicKind::Geom => dof::geom_gradients(mesh),
            // Like matplotlib, user derivatives are indexed by compressed
            // node number without renumbering: once unused points are
            // dropped, node `i` takes the derivatives given for point `i`.
            CubicKind::User { dzdx, dzdy } => dzdx
                .iter()
                .zip(dzdy)
                .take(pts.len())
                .map(|(gx, gy)| [gx * unit[0], gy * unit[1]])
                .collect(),
        };
        let dofs = dof::element_dofs(mesh, &dz);
        Ok(Self {
            finder,
            tri_renum: compressed.tri_renum,
            unit,
            corners,
            eccs,
            dofs,
        })
    }

    /// The compressed triangle evaluated for triangle `tri`; masked
    /// triangles map to the last unmasked one, as numpy's `-1` index does.
    fn element(&self, tri: usize) -> Option<usize> {
        self.tri_renum[tri].or_else(|| self.corners.len().checked_sub(1))
    }

    /// Barycentric coordinates of the rescaled point in element `e`.
    fn barycentric(&self, e: usize, x: f64, y: f64) -> [f64; 3] {
        let [p0, p1, p2] = self.corners[e];
        let (xs, ys) = (x / self.unit[0], y / self.unit[1]);
        let a = [p1[0] - p0[0], p1[1] - p0[1]];
        let b = [p2[0] - p0[0], p2[1] - p0[1]];
        let om = [xs - p0[0], ys - p0[1]];
        let metric = [
            [a[0] * a[0] + a[1] * a[1], a[0] * b[0] + a[1] * b[1]],
            [b[0] * a[0] + b[1] * a[1], b[0] * b[0] + b[1] * b[1]],
        ];
        let inv = pseudo_inv22sym(metric);
        let covar = [a[0] * om[0] + a[1] * om[1], b[0] * om[0] + b[1] * om[1]];
        let k0 = inv[0][0] * covar[0] + inv[0][1] * covar[1];
        let k1 = inv[1][0] * covar[0] + inv[1][1] * covar[1];
        [1.0 - k0 - k1, k0, k1]
    }
}

impl TriInterpolator for CubicTriInterpolator<'_> {
    fn finder(&self) -> &TriFinder<'_> {
        &self.finder
    }

    fn value_in(&self, tri: usize, x: f64, y: f64) -> f64 {
        self.element(tri).map_or(f64::NAN, |e| {
            hct::value(self.barycentric(e, x, y), self.eccs[e], &self.dofs[e])
        })
    }

    fn gradient_in(&self, tri: usize, x: f64, y: f64) -> [f64; 2] {
        self.element(tri).map_or([f64::NAN; 2], |e| {
            let alpha = self.barycentric(e, x, y);
            let j = jacobian(self.corners[e]);
            let [gx, gy] = hct::gradient(alpha, j, self.eccs[e], &self.dofs[e]);
            [gx * (1.0 / self.unit[0]), gy * (1.0 / self.unit[1])]
        })
    }
}

/// The unmasked triangles renumbered over the points they use
/// (matplotlib's `TriAnalyzer._get_compressed_triangulation`).
#[derive(Clone, Debug)]
pub(crate) struct Compressed {
    pub(crate) triangles: Vec<[usize; 3]>,
    pub(crate) x: Vec<f64>,
    pub(crate) y: Vec<f64>,
    pub(crate) tri_renum: Vec<Option<usize>>,
    pub(crate) node_renum: Vec<Option<usize>>,
}

impl Compressed {
    pub(crate) fn new(tri: &Triangulation) -> Self {
        let mut used = vec![false; tri.npoints()];
        for t in tri.unmasked() {
            for v in tri.triangles()[t] {
                used[v] = true;
            }
        }
        let node_renum = renumber(&used);
        let live: Vec<bool> = (0..tri.ntri()).map(|t| !tri.is_masked(t)).collect();
        let tri_renum = renumber(&live);
        let triangles = tri
            .unmasked()
            .map(|t| tri.triangles()[t].map(|v| node_renum[v].expect("used node")))
            .collect();
        let pick = |values: &[f64]| -> Vec<f64> {
            values
                .iter()
                .zip(&used)
                .filter(|(_, u)| **u)
                .map(|(v, _)| *v)
                .collect()
        };
        Self {
            triangles,
            x: pick(tri.x()),
            y: pick(tri.y()),
            tri_renum,
            node_renum,
        }
    }

    /// Per-point values restricted to the used points, in compressed order.
    pub(crate) fn gather(&self, values: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.x.len()];
        for (value, renum) in values.iter().zip(&self.node_renum) {
            if let Some(i) = renum {
                out[*i] = *value;
            }
        }
        out
    }
}

/// Consecutive indices for the `true` entries.
fn renumber(valid: &[bool]) -> Vec<Option<usize>> {
    let mut next = 0;
    valid
        .iter()
        .map(|&v| {
            v.then(|| {
                next += 1;
                next - 1
            })
        })
        .collect()
}

/// numpy `ptp`: `max - min` (NaN if empty).
pub(crate) fn ptp(values: &[f64]) -> f64 {
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    if values.is_empty() {
        f64::NAN
    } else {
        max - min
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(n: usize) -> Triangulation {
        let mut x = Vec::new();
        let mut y = Vec::new();
        for j in 0..n {
            for i in 0..n {
                x.push(i as f64);
                y.push(j as f64);
            }
        }
        let mut t = Vec::new();
        for j in 0..n - 1 {
            for i in 0..n - 1 {
                let a = j * n + i;
                t.push([a, a + 1, a + n + 1]);
                t.push([a, a + n + 1, a + n]);
            }
        }
        Triangulation::new(&x, &y, t).unwrap()
    }

    #[test]
    fn all_kinds_reproduce_linear_fields() {
        let tri = grid(4);
        let f = |x: f64, y: f64| 3.0 + 0.75 * x - 0.5 * y;
        let z: Vec<f64> = tri
            .x()
            .iter()
            .zip(tri.y())
            .map(|(&x, &y)| f(x, y))
            .collect();
        let user = CubicKind::User {
            dzdx: vec![0.75; 16],
            dzdy: vec![-0.5; 16],
        };
        for kind in [CubicKind::MinE, CubicKind::Geom, user] {
            let interp = CubicTriInterpolator::new(&tri, &z, &kind).unwrap();
            for (x, y) in [(0.3, 0.1), (1.7, 2.2), (2.5, 0.5), (3.0, 3.0)] {
                let v = interp.value(x, y).unwrap();
                assert!((v - f(x, y)).abs() < 1e-9, "{kind:?} {x} {y} {v}");
                let [gx, gy] = interp.gradient(x, y).unwrap();
                assert!((gx - 0.75).abs() < 1e-7 && (gy + 0.5).abs() < 1e-7);
            }
            assert_eq!(interp.value(-1.0, 0.0), None);
        }
    }

    #[test]
    fn values_at_nodes_are_exact() {
        let tri = grid(3);
        let z: Vec<f64> = (0..9).map(|i| (f64::from(i) * 0.7).sin()).collect();
        let interp = CubicTriInterpolator::new(&tri, &z, &CubicKind::MinE).unwrap();
        for (i, zi) in z.iter().enumerate() {
            let v = interp.value(tri.x()[i], tri.y()[i]).unwrap();
            assert!((v - zi).abs() < 1e-12);
        }
    }

    #[test]
    fn user_gradients_use_compressed_numbering() {
        // Point 0 is unused, so node 0 of the compressed mesh is point 1
        // and takes the derivatives given for point 0, as in matplotlib.
        let tri = Triangulation::new(
            &[5.0, 0.0, 1.0, 0.0],
            &[5.0, 0.0, 0.0, 1.0],
            vec![[1, 2, 3]],
        )
        .unwrap();
        let kind = CubicKind::User {
            dzdx: vec![1.0, 2.0, 3.0, 4.0],
            dzdy: vec![0.0; 4],
        };
        let interp = CubicTriInterpolator::new(&tri, &[0.0; 4], &kind).unwrap();
        let [gx, gy] = interp.gradient(0.0, 0.0).unwrap();
        assert!((gx - 1.0).abs() < 1e-12 && gy.abs() < 1e-12);
    }

    #[test]
    fn user_gradients_need_one_per_point() {
        let tri = grid(3);
        let kind = CubicKind::User {
            dzdx: vec![0.0; 3],
            dzdy: vec![0.0; 3],
        };
        assert!(CubicTriInterpolator::new(&tri, &[0.0; 9], &kind).is_err());
    }
}
