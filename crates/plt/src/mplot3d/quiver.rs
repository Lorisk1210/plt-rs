//! Arrows in 3-D (matplotlib's `Axes3D.quiver`).

use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::{Vec3, norm, sub};
use super::plot::columns;
use crate::collections::{Collection, CollectionProps};
use crate::quiver::Pivot;

/// The angle between an arrow head's barbs and its shaft.
const HEAD_ANGLE_DEGREES: f64 = 15.0;

/// Options of [`Axes3D::quiver`].
#[derive(Clone, Debug, PartialEq)]
pub struct Quiver3DOptions {
    /// The length of every arrow in data units, as a factor of its
    /// vector.
    pub length: f64,
    /// The length of the head barbs as a fraction of the arrow.
    pub arrow_length_ratio: f64,
    /// The part of the arrow at its position.
    pub pivot: Pivot,
    /// Whether every vector is scaled to unit length first.
    pub normalize: bool,
    /// Line properties.
    pub props: CollectionProps,
}

impl Default for Quiver3DOptions {
    fn default() -> Self {
        Self {
            length: 1.0,
            arrow_length_ratio: 0.3,
            pivot: Pivot::Tail,
            normalize: false,
            props: CollectionProps::default(),
        }
    }
}

impl Axes3D {
    /// Arrows from the points `positions` along `vectors`, drawn as a
    /// shaft and two head barbs each (matplotlib's `ax.quiver(X, Y, Z, U,
    /// V, W)`).
    ///
    /// # Errors
    ///
    /// When the positions and vectors differ in number.
    pub fn quiver(
        &self,
        positions: &[Vec3],
        vectors: &[Vec3],
        options: Quiver3DOptions,
    ) -> Result<Collection, Axes3DError> {
        check_len("vectors", positions.len(), vectors.len())?;
        let had_data = self.has_data();
        if positions.is_empty() {
            return Ok(self.add_segments(Vec::new(), options.props));
        }
        let length = options.length;
        let shaft_offsets = match options.pivot {
            Pivot::Tail => [-length, 0.0],
            Pivot::Middle => [-length / 2.0, length / 2.0],
            Pivot::Tip => [0.0, length],
        };
        let head_offsets = [0.0, length * options.arrow_length_ratio];
        let vectors: Vec<Vec3> = vectors
            .iter()
            .map(|&v| {
                let n = norm(v);
                if options.normalize && n != 0.0 {
                    v.map(|c| c / n)
                } else {
                    v
                }
            })
            .collect();
        let shafts: Vec<Vec<Vec3>> = positions
            .iter()
            .zip(&vectors)
            .map(|(&p, &v)| {
                shaft_offsets
                    .iter()
                    .map(|&d| sub(p, v.map(|c| d * c)))
                    .collect()
            })
            .collect();
        let barbs: Vec<[Vec3; 2]> = vectors.iter().map(|&v| head_barbs(v)).collect();
        let head = |side: usize| {
            shafts.iter().zip(&barbs).map(move |(shaft, barb)| {
                head_offsets
                    .iter()
                    .map(|&d| sub(shaft[0], barb[side].map(|c| d * c)))
                    .collect::<Vec<Vec3>>()
            })
        };
        let lines: Vec<Vec<Vec3>> = shafts
            .iter()
            .cloned()
            .chain(head(0))
            .chain(head(1))
            .collect();
        let collection = self.add_segments(lines, options.props);
        let [xs, ys, zs] = columns(positions);
        self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)?;
        Ok(collection)
    }
}

/// The directions of the two head barbs of an arrow along `v`: `v`
/// rotated by ±15° about the horizontal axis perpendicular to it
/// (matplotlib's `calc_arrows`).
fn head_barbs(v: Vec3) -> [Vec3; 2] {
    let [x, y, _] = v;
    let horizontal = norm([x, y, 0.0]);
    let (x_p, y_p) = if horizontal == 0.0 {
        (0.0, 1.0)
    } else {
        (y / horizontal, -x / horizontal)
    };
    let (s, c) = HEAD_ANGLE_DEGREES.to_radians().sin_cos();
    let (r13, r32, r12) = (y_p * s, x_p * s, x_p * y_p * (1.0 - c));
    let positive = [
        [c + (x_p * x_p) * (1.0 - c), r12, r13],
        [r12, c + (y_p * y_p) * (1.0 - c), -r32],
        [-r13, r32, c],
    ];
    let mut negative = positive;
    for (i, j) in [(0, 2), (1, 2), (2, 0), (2, 1)] {
        negative[i][j] = -negative[i][j];
    }
    let apply = |m: [[f64; 3]; 3]| m.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2]);
    [apply(positive), apply(negative)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barbs_keep_the_length_and_open_symmetrically() {
        let v = [1.0, 0.0, 0.0];
        let [a, b] = head_barbs(v);
        assert!((norm(a) - 1.0).abs() < 1e-12 && (norm(b) - 1.0).abs() < 1e-12);
        let cos = |p: Vec3| (p[0] * v[0] + p[1] * v[1] + p[2] * v[2]) / norm(p);
        assert!((cos(a) - 15_f64.to_radians().cos()).abs() < 1e-12);
        assert!((cos(a) - cos(b)).abs() < 1e-12);
    }

    #[test]
    fn vertical_vectors_get_barbs_too() {
        let [a, b] = head_barbs([0.0, 0.0, 2.0]);
        assert!(a.iter().all(|c| c.is_finite()));
        assert!((0..3).any(|i| (a[i] - b[i]).abs() > 1e-9));
    }
}
