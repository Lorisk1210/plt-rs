//! Mesh quality analysis (matplotlib's `TriAnalyzer`).

use super::Triangulation;
use super::interpolate::ptp;

/// Flatness measures for the triangles of a triangulation.
#[derive(Clone, Copy, Debug)]
pub struct TriAnalyzer<'a> {
    tri: &'a Triangulation,
}

impl<'a> TriAnalyzer<'a> {
    /// An analyzer for `tri`.
    pub fn new(tri: &'a Triangulation) -> Self {
        Self { tri }
    }

    /// Factors `[kx, ky]` scaling the points used by unmasked triangles
    /// into a unit square: `1 / ptp(x)` and `1 / ptp(y)`.
    pub fn scale_factors(&self) -> [f64; 2] {
        let mut used = vec![false; self.tri.npoints()];
        for t in self.tri.unmasked() {
            for v in self.tri.triangles()[t] {
                used[v] = true;
            }
        }
        let pick = |values: &[f64]| -> Vec<f64> {
            values
                .iter()
                .zip(&used)
                .filter(|(_, u)| **u)
                .map(|(v, _)| *v)
                .collect()
        };
        [
            1.0 / ptp(&pick(self.tri.x())),
            1.0 / ptp(&pick(self.tri.y())),
        ]
    }

    /// The ratio of incircle to circumcircle radius of each triangle (at most
    /// 0.5, for equilateral triangles; 0 for flat ones), after rescaling to
    /// the unit square if `rescale`. Masked triangles give `None`.
    pub fn circle_ratios(&self, rescale: bool) -> Vec<Option<f64>> {
        let [kx, ky] = if rescale {
            self.scale_factors()
        } else {
            [1.0, 1.0]
        };
        let point = |v: usize| [self.tri.x()[v] * kx, self.tri.y()[v] * ky];
        self.tri
            .triangles()
            .iter()
            .enumerate()
            .map(|(t, tri)| {
                let ratio = circle_ratio(tri.map(point));
                (!self.tri.is_masked(t)).then_some(ratio)
            })
            .collect()
    }

    /// A mask removing excessively flat triangles from the border: starting
    /// from the current mask, border triangles (with a missing neighbour)
    /// whose circle ratio is below `min_circle_ratio` are masked
    /// repeatedly until none remain, so no new holes appear.
    pub fn flat_tri_mask(&self, min_circle_ratio: f64, rescale: bool) -> Vec<bool> {
        let ntri = self.tri.ntri();
        let bad: Vec<bool> = self
            .circle_ratios(rescale)
            .iter()
            .map(|r| r.is_some_and(|r| r < min_circle_ratio))
            .collect();
        let mut mask: Vec<bool> = (0..ntri).map(|t| self.tri.is_masked(t)).collect();
        let mut neighbors: Vec<[Option<usize>; 3]> = self.tri.neighbors().to_vec();
        loop {
            let added: Vec<usize> = (0..ntri)
                .filter(|&t| !mask[t] && bad[t] && neighbors[t].contains(&None))
                .collect();
            if added.is_empty() {
                return mask;
            }
            for &t in &added {
                mask[t] = true;
                neighbors[t] = [None; 3];
            }
            for row in &mut neighbors {
                for n in row.iter_mut() {
                    if n.is_some_and(|n| mask[n]) {
                        *n = None;
                    }
                }
            }
        }
    }
}

/// Incircle over circumcircle radius of one triangle (matplotlib's
/// formula, with zero for flat triangles).
fn circle_ratio([p0, p1, p2]: [[f64; 2]; 3]) -> f64 {
    let length = |a: [f64; 2], b: [f64; 2]| (b[0] - a[0]).hypot(b[1] - a[1]);
    let (a, b, c) = (length(p0, p1), length(p1, p2), length(p2, p0));
    let s = (a + b + c) * 0.5;
    let prod = s * (a + b - s) * (a + c - s) * (b + c - s);
    let abc = a * b * c;
    let circum = if prod == 0.0 {
        f64::INFINITY
    } else {
        abc / (4.0 * prod.sqrt())
    };
    let inner = abc / (4.0 * circum * s);
    inner / circum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratios_of_right_and_flat_triangles() {
        let tri = Triangulation::new(
            &[0.0, 1.0, 2.0, 1.0],
            &[0.0, 0.0, 0.0, 1.0],
            vec![[0, 2, 1], [0, 1, 3], [1, 2, 3]],
        )
        .unwrap();
        let analyzer = TriAnalyzer::new(&tri);
        assert_eq!(analyzer.scale_factors(), [0.5, 1.0]);
        let ratios = analyzer.circle_ratios(false);
        assert_eq!(ratios[0], Some(0.0));
        let right_isoceles = 2f64.sqrt() - 1.0;
        assert!((ratios[1].unwrap() - right_isoceles).abs() < 1e-15);
        // The flat triangle is on the border and removed; the others are fine.
        assert_eq!(analyzer.flat_tri_mask(0.01, false), [true, false, false]);
    }

    #[test]
    fn masked_triangles_stay_masked() {
        let tri = Triangulation::new(&[0.0, 1.0, 0.0], &[0.0, 0.0, 1.0], vec![[0, 1, 2]])
            .unwrap()
            .with_mask(Some(vec![true]))
            .unwrap();
        let analyzer = TriAnalyzer::new(&tri);
        assert_eq!(analyzer.circle_ratios(false), [None]);
        assert_eq!(analyzer.flat_tri_mask(0.5, false), [true]);
    }
}
