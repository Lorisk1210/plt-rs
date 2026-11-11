//! qhull's 3-d geometry (`geom_r.c`, `geom2_r.c`), with qhull's exact
//! floating-point operation order: every decision of the hull construction
//! compares results of these functions, so they must round identically.
//!
//! Where matplotlib's build of qhull contracts a product and a sum into one
//! fused multiply-add (its arm64 macOS wheel), so do these functions:
//! each such expression goes through [`fused`], in clang's operand order.

use pltrs_core::numeric::{difference_of_products, fused, sum_of_products};

use super::precision::Precision;

/// A 3-d point or vector.
pub(super) type Vec3 = [f64; 3];

/// Signed distance from `point` to the hyperplane `normal . x + offset = 0`
/// (qhull's `qh_distplane`).
pub(super) fn dist_plane(point: &Vec3, normal: &Vec3, offset: f64) -> f64 {
    let dist = fused(point[0], normal[0], offset);
    let dist = fused(point[1], normal[1], dist);
    fused(point[2], normal[2], dist)
}

/// The dot product accumulated from zero (qhull's `qh_getangle`).
pub(super) fn angle(a: &Vec3, b: &Vec3) -> f64 {
    let mut angle = 0.0;
    for k in 0..3 {
        angle = fused(a[k], b[k], angle);
    }
    angle
}

/// Squared distance between two points (qhull's `qh_pointdist` with a
/// negative dimension).
pub(super) fn point_dist2(a: &Vec3, b: &Vec3, dim: usize) -> f64 {
    let mut dist = 0.0;
    for k in 0..dim {
        let diff = a[k] - b[k];
        dist = fused(diff, diff, dist);
    }
    dist
}

/// The arithmetic centre of `points` (qhull's `qh_getcenter`).
pub(super) fn center(points: &[Vec3]) -> Vec3 {
    let count = points.len() as f64;
    let mut center = [0.0; 3];
    for (k, c) in center.iter_mut().enumerate() {
        for p in points {
            *c += p[k];
        }
        *c /= count;
    }
    center
}

/// `point` moved by `dist` against `normal` (qhull's `qh_projectpoint`).
pub(super) fn project(point: &Vec3, normal: &Vec3, dist: f64) -> Vec3 {
    [0, 1, 2].map(|k| fused(-dist, normal[k], point[k]))
}

/// `numer / denom` unless the quotient would overflow (qhull's
/// `qh_divzero`); `None` signals a zero divide.
pub(super) fn div_zero(numer: f64, denom: f64, min_denom_1: f64) -> Option<f64> {
    if numer < min_denom_1 && numer > -min_denom_1 {
        return (numer.abs() < denom.abs()).then(|| numer / denom);
    }
    let temp = denom / numer;
    (temp > min_denom_1 || temp < -min_denom_1).then(|| numer / denom)
}

/// Normalises `normal`, negated unless `toporient` (qhull's `qh_normalize2`).
pub(super) fn normalize(normal: &mut Vec3, toporient: bool, prec: &Precision) {
    let mut norm = dot_from_products(normal, normal).sqrt();
    if norm > prec.min_denom {
        if !toporient {
            norm = -norm;
        }
        for c in normal.iter_mut() {
            *c /= norm;
        }
    } else if norm == 0.0 {
        *normal = [(1.0_f64 / 3.0).sqrt(); 3];
    } else {
        if !toporient {
            norm = -norm;
        }
        for k in 0..3 {
            if let Some(value) = div_zero(normal[k], norm, prec.min_denom_1) {
                normal[k] = value;
            } else {
                let max = (0..3).fold(0, |best, i| {
                    if normal[i].abs() > normal[best].abs() {
                        i
                    } else {
                        best
                    }
                });
                let sign = if normal[max] * norm >= 0.0 { 1.0 } else { -1.0 };
                *normal = [0.0; 3];
                normal[max] = sign;
                return;
            }
        }
    }
}

/// `a . b` as qhull writes it out for three dimensions, `a0 * b0 + a1 * b1 +
/// a2 * b2`: the first sum takes the first product fused and the second
/// rounded, the second sum the third product fused.
fn dot_from_products(a: &Vec3, b: &Vec3) -> f64 {
    fused(a[2], b[2], sum_of_products(a[0], b[0], a[1], b[1]))
}

/// The determinant `a1 * b2 - a2 * b1` (qhull's `det2_`).
fn det2(a1: f64, a2: f64, b1: f64, b2: f64) -> f64 {
    difference_of_products(a1, b2, a2, b1)
}

/// A hyperplane through three points: `(normal, offset, nearzero)`.
pub(super) type Plane = (Vec3, f64, bool);

/// The oriented hyperplane through `rows` (qhull's `qh_sethyperplane_det`
/// for 3-d); `nearzero` reports a vertex off the plane by more than the
/// distance roundoff. `rows[0]` is the facet's first vertex.
pub(super) fn hyperplane_det(rows: &[Vec3; 3], toporient: bool, prec: &Precision) -> Plane {
    let p0 = rows[0];
    let d = |i: usize, k: usize| rows[i][k] - rows[0][k];
    let mut normal = [
        det2(d(2, 1), d(2, 2), d(1, 1), d(1, 2)),
        det2(d(1, 0), d(1, 2), d(2, 0), d(2, 2)),
        det2(d(2, 0), d(2, 1), d(1, 0), d(1, 1)),
    ];
    normalize(&mut normal, toporient, prec);
    let offset = -dot_from_products(&p0, &normal);
    let nearzero = rows[1..].iter().rev().any(|p| {
        let dist = offset + dot_from_products(p, &normal);
        dist > prec.dist_round || dist < -prec.dist_round
    });
    (normal, offset, nearzero)
}

/// The oriented hyperplane through `point0` and `point0 + rows[i]` by
/// Gaussian elimination (qhull's `qh_sethyperplane_gauss` for 3-d).
pub(super) fn hyperplane_gauss(
    mut rows: [Vec3; 2],
    point0: &Vec3,
    toporient: bool,
    prec: &Precision,
) -> Plane {
    let mut sign = toporient;
    let mut nearzero = gauss_elim(&mut rows, &mut sign, prec);
    for k in (0..2).rev() {
        if rows[k][k] < 0.0 {
            sign = !sign;
        }
    }
    let (normal, nearzero2) = back_normal(&rows, sign, prec);
    nearzero |= nearzero2;
    let mut normal = normal;
    normalize(&mut normal, true, prec);
    let mut offset = -(point0[0] * normal[0]);
    offset = fused(-point0[1], normal[1], offset);
    offset = fused(-point0[2], normal[2], offset);
    (normal, offset, nearzero)
}

/// Gaussian elimination with partial pivoting on two rows (qhull's
/// `qh_gausselim`); returns whether a pivot was near zero.
fn gauss_elim(rows: &mut [Vec3; 2], sign: &mut bool, prec: &Precision) -> bool {
    let mut nearzero = false;
    for k in 0..2 {
        let mut pivot_abs = rows[k][k].abs();
        let mut pivot_i = k;
        for (i, row) in rows.iter().enumerate().skip(k + 1) {
            if row[k].abs() > pivot_abs {
                pivot_abs = row[k].abs();
                pivot_i = i;
            }
        }
        if pivot_i != k {
            rows.swap(pivot_i, k);
            *sign = !*sign;
        }
        if pivot_abs <= prec.near_zero[k] {
            nearzero = true;
            if pivot_abs == 0.0 {
                continue;
            }
        }
        let pivot_row = rows[k];
        for row in rows.iter_mut().skip(k + 1) {
            let n = row[k] / pivot_row[k];
            for (value, pivot) in row.iter_mut().zip(pivot_row).skip(k + 1) {
                *value = fused(-n, pivot, *value);
            }
        }
    }
    nearzero
}

/// Back substitution for the normal of an upper-triangular system (qhull's
/// `qh_backnormal`); also returns whether a diagonal was zero.
fn back_normal(rows: &[Vec3; 2], sign: bool, prec: &Precision) -> (Vec3, bool) {
    let unit = if sign { -1.0 } else { 1.0 };
    let mut normal = [0.0, 0.0, unit];
    let mut zero = false;
    for i in (0..2).rev() {
        let mut value = 0.0;
        for j in i + 1..3 {
            value = fused(-rows[i][j], normal[j], value);
        }
        let diagonal = rows[i][i];
        if diagonal.abs() > prec.min_denom_2 {
            normal[i] = value / diagonal;
        } else if let Some(q) = div_zero(value, diagonal, prec.min_denom_1_2) {
            normal[i] = q;
        } else {
            zero = true;
            normal[i] = unit;
            for c in &mut normal[i + 1..] {
                *c = 0.0;
            }
        }
    }
    (normal, zero)
}

/// Determinant of the simplex `apex, base...` in the first `dim`
/// coordinates and whether it is near zero (qhull's `qh_detsimplex`).
pub(super) fn det_simplex(apex: &Vec3, base: &[Vec3], dim: usize, prec: &Precision) -> (f64, bool) {
    let row = |i: usize, k: usize| base[i][k] - apex[k];
    if dim == 2 {
        let det = det2(row(0, 0), row(0, 1), row(1, 0), row(1, 1));
        (det, det.abs() < 10.0 * prec.near_zero[1])
    } else {
        // qhull's `det3_`, `a1 * minor1 - b1 * minor2 + c1 * minor3`: the
        // difference takes its first product fused, the sum its last.
        let minor1 = det2(row(1, 1), row(1, 2), row(2, 1), row(2, 2));
        let minor2 = det2(row(0, 1), row(0, 2), row(2, 1), row(2, 2));
        let minor3 = det2(row(0, 1), row(0, 2), row(1, 1), row(1, 2));
        let difference = difference_of_products(row(0, 0), minor1, row(1, 0), minor2);
        let det = fused(row(2, 0), minor3, difference);
        (det, det.abs() < 10.0 * prec.near_zero[2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prec() -> Precision {
        Precision::for_tests(1e-15)
    }

    #[test]
    fn plane_through_three_points() {
        let rows = [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0]];
        let (normal, offset, nearzero) = hyperplane_det(&rows, true, &prec());
        assert!(!nearzero);
        assert_eq!(normal, [0.0, 0.0, -1.0]);
        assert_eq!(offset, 1.0);
        assert_eq!(dist_plane(&[0.3, 0.2, 0.0], &normal, offset), 1.0);
        let (flipped, _, _) = hyperplane_det(&rows, false, &prec());
        assert_eq!(flipped, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn gauss_matches_determinant_plane() {
        let p0 = [0.0, 0.0, 1.0];
        let (normal, offset, nearzero) =
            hyperplane_gauss([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], &p0, true, &prec());
        assert!(!nearzero);
        assert_eq!(normal, [0.0, 0.0, -1.0]);
        assert_eq!(offset, 1.0);
    }

    #[test]
    fn helpers() {
        assert_eq!(div_zero(1.0, 0.0, f64::MIN_POSITIVE), None);
        assert_eq!(div_zero(1.0, 4.0, f64::MIN_POSITIVE), Some(0.25));
        assert_eq!(center(&[[0.0, 0.0, 0.0], [2.0, 4.0, 6.0]]), [1.0, 2.0, 3.0]);
        assert_eq!(
            project(&[1.0, 1.0, 1.0], &[0.0, 0.0, 1.0], 0.5),
            [1.0, 1.0, 0.5]
        );
        assert_eq!(point_dist2(&[1.0, 1.0, 9.0], &[0.0, 0.0, 0.0], 2), 2.0);
        let (det, nearzero) =
            det_simplex(&[0.0; 3], &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], 2, &prec());
        assert_eq!(det, 1.0);
        assert!(!nearzero);
        let mut zero = [0.0; 3];
        normalize(&mut zero, true, &prec());
        assert_eq!(zero, [(1.0_f64 / 3.0).sqrt(); 3]);
    }
}
