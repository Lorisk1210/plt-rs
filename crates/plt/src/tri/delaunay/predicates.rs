//! Exact geometric predicates: orientation and in-circle tests.
//!
//! Both predicates first evaluate the determinant in plain floating point
//! and accept the result when it exceeds Shewchuk's forward error bound;
//! otherwise they re-evaluate it exactly with floating-point expansion
//! arithmetic. The sign of the returned value is therefore always correct,
//! which is what a robust Delaunay triangulation needs.

use plt_core::Point;

/// Half an ulp of 1.0: the unit roundoff of `f64`.
const EPSILON: f64 = f64::EPSILON * 0.5;
/// Shewchuk's `ccwerrboundA`.
const ORIENT_BOUND: f64 = (3.0 + 16.0 * EPSILON) * EPSILON;
/// Shewchuk's `iccerrboundA`.
const INCIRCLE_BOUND: f64 = (10.0 + 96.0 * EPSILON) * EPSILON;

/// Twice the signed area of triangle `abc`: positive when `a, b, c` turn
/// anticlockwise, negative when clockwise, zero when collinear (exact sign).
pub(crate) fn orient2d(a: Point, b: Point, c: Point) -> f64 {
    let left = (a.x - c.x) * (b.y - c.y);
    let right = (a.y - c.y) * (b.x - c.x);
    let det = left - right;
    if det.abs() >= ORIENT_BOUND * (left.abs() + right.abs()) {
        return det;
    }
    let acx = two_diff(a.x, c.x);
    let bcy = two_diff(b.y, c.y);
    let acy = two_diff(a.y, c.y);
    let bcx = two_diff(b.x, c.x);
    let det = diff(&product(&acx, &bcy), &product(&acy, &bcx));
    estimate(&det)
}

/// Positive when `d` lies strictly inside the circle through the
/// anticlockwise triangle `abc`, negative outside, zero on it (exact sign).
pub(crate) fn incircle(a: Point, b: Point, c: Point, d: Point) -> f64 {
    let (adx, ady) = (a.x - d.x, a.y - d.y);
    let (bdx, bdy) = (b.x - d.x, b.y - d.y);
    let (cdx, cdy) = (c.x - d.x, c.y - d.y);
    let (bdxcdy, cdxbdy) = (bdx * cdy, cdx * bdy);
    let (cdxady, adxcdy) = (cdx * ady, adx * cdy);
    let (adxbdy, bdxady) = (adx * bdy, bdx * ady);
    let alift = adx * adx + ady * ady;
    let blift = bdx * bdx + bdy * bdy;
    let clift = cdx * cdx + cdy * cdy;
    let det = alift * (bdxcdy - cdxbdy) + blift * (cdxady - adxcdy) + clift * (adxbdy - bdxady);
    let permanent = (bdxcdy.abs() + cdxbdy.abs()) * alift
        + (cdxady.abs() + adxcdy.abs()) * blift
        + (adxbdy.abs() + bdxady.abs()) * clift;
    if det.abs() > INCIRCLE_BOUND * permanent {
        return det;
    }
    incircle_exact(a, b, c, d)
}

fn incircle_exact(a: Point, b: Point, c: Point, d: Point) -> f64 {
    let (adx, ady) = (two_diff(a.x, d.x), two_diff(a.y, d.y));
    let (bdx, bdy) = (two_diff(b.x, d.x), two_diff(b.y, d.y));
    let (cdx, cdy) = (two_diff(c.x, d.x), two_diff(c.y, d.y));
    let lift = |x: &[f64], y: &[f64]| sum(&product(x, x), &product(y, y));
    let cross =
        |x1: &[f64], y1: &[f64], x2: &[f64], y2: &[f64]| diff(&product(x1, y2), &product(x2, y1));
    let terms = [
        product(&lift(&adx, &ady), &cross(&bdx, &bdy, &cdx, &cdy)),
        product(&lift(&bdx, &bdy), &cross(&cdx, &cdy, &adx, &ady)),
        product(&lift(&cdx, &cdy), &cross(&adx, &ady, &bdx, &bdy)),
    ];
    estimate(&sum(&sum(&terms[0], &terms[1]), &terms[2]))
}

/// `a - b` as an exact two-component expansion (smallest first).
fn two_diff(a: f64, b: f64) -> [f64; 2] {
    let x = a - b;
    let bv = a - x;
    let av = x + bv;
    let err = (a - av) + (bv - b);
    [err, x]
}

/// `a + b` exactly: `(sum, roundoff)`.
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let x = a + b;
    let bv = x - a;
    let av = x - bv;
    (x, (a - av) + (b - bv))
}

/// `a * b` exactly: `(product, roundoff)` (fused multiply-add is exact).
fn two_product(a: f64, b: f64) -> (f64, f64) {
    let x = a * b;
    (x, a.mul_add(b, -x))
}

/// Adds a scalar to an expansion (Shewchuk's `grow_expansion_zeroelim`).
fn grow(e: &[f64], b: f64) -> Vec<f64> {
    let mut out = Vec::with_capacity(e.len() + 1);
    let mut q = b;
    for &component in e {
        let (sum, err) = two_sum(q, component);
        if err != 0.0 {
            out.push(err);
        }
        q = sum;
    }
    if q != 0.0 || out.is_empty() {
        out.push(q);
    }
    out
}

/// The exact sum of two expansions.
fn sum(e: &[f64], f: &[f64]) -> Vec<f64> {
    f.iter()
        .fold(e.to_vec(), |acc, &component| grow(&acc, component))
}

/// The exact difference of two expansions.
fn diff(e: &[f64], f: &[f64]) -> Vec<f64> {
    let negated: Vec<f64> = f.iter().map(|v| -v).collect();
    sum(e, &negated)
}

/// An expansion times a scalar (Shewchuk's `scale_expansion_zeroelim`).
fn scale(e: &[f64], b: f64) -> Vec<f64> {
    let mut out = Vec::with_capacity(2 * e.len());
    let Some((&first, rest)) = e.split_first() else {
        return vec![0.0];
    };
    let (mut q, low) = two_product(first, b);
    if low != 0.0 {
        out.push(low);
    }
    for &component in rest {
        let (product, product_low) = two_product(component, b);
        let (sum, err) = two_sum(q, product_low);
        if err != 0.0 {
            out.push(err);
        }
        let (next, err) = two_sum(product, sum);
        if err != 0.0 {
            out.push(err);
        }
        q = next;
    }
    if q != 0.0 || out.is_empty() {
        out.push(q);
    }
    out
}

/// The exact product of two expansions.
fn product(e: &[f64], f: &[f64]) -> Vec<f64> {
    f.iter()
        .fold(vec![0.0], |acc, &component| sum(&acc, &scale(e, component)))
}

/// The most significant component, which carries the expansion's sign.
fn estimate(e: &[f64]) -> f64 {
    e.iter().rev().copied().find(|v| *v != 0.0).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn orientation_signs() {
        assert!(orient2d(p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)) > 0.0);
        assert!(orient2d(p(0.0, 0.0), p(0.0, 1.0), p(1.0, 0.0)) < 0.0);
        assert_eq!(orient2d(p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0)), 0.0);
    }

    #[test]
    fn orientation_is_exact_near_degeneracy() {
        // Points on the line y = x, perturbed by one ulp: plain floating point
        // gets these wrong for many offsets.
        let base = 0.5;
        for i in 0..64 {
            let x = base + f64::from(i) * f64::EPSILON;
            let collinear = orient2d(p(12.0, 12.0), p(24.0, 24.0), p(x, x));
            assert_eq!(collinear, 0.0);
            let above = orient2d(p(12.0, 12.0), p(24.0, 24.0), p(x, x.next_up()));
            assert!(above > 0.0, "{i}");
        }
    }

    #[test]
    fn incircle_signs_and_cocircular_points() {
        let (a, b, c) = (p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0));
        assert!(incircle(a, b, c, p(0.5, 0.5)) > 0.0);
        assert!(incircle(a, b, c, p(2.0, 2.0)) < 0.0);
        assert_eq!(incircle(a, b, c, p(1.0, 1.0)), 0.0);
        // Cocircular points of a large shifted grid square.
        let o = 1e9 + 0.1;
        let (a, b, c, d) = (p(o, o), p(o + 3.0, o), p(o + 3.0, o + 3.0), p(o, o + 3.0));
        assert_eq!(incircle(a, b, c, d), 0.0);
        assert!(incircle(a, b, c, p(o + 1.5, o + 3.0 - 1e-6)) > 0.0);
    }

    #[test]
    fn expansion_arithmetic() {
        let e = sum(&[1e-30], &[1.0]);
        assert_eq!(estimate(&diff(&e, &[1.0])), 1e-30);
        assert_eq!(estimate(&product(&[3.0], &[0.0])), 0.0);
        assert_eq!(estimate(&scale(&two_diff(1.0, 1e-20), 2.0)), 2.0);
    }
}
