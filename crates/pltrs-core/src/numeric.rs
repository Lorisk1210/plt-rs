//! Python's and numpy's float semantics that matplotlib's results depend
//! on, shared by every crate.
//!
//! matplotlib computes with `CPython` floats and numpy arrays, and the exact
//! floating-point behaviour of their operations decides tick positions,
//! limits and layouts. These functions reproduce `CPython`'s and numpy's
//! algorithms bit for bit:
//!
//! * `CPython`'s float `divmod`, `//` and `%` ([`divmod`], [`floor_div`],
//!   [`remainder`]) and numpy's `divmod` ([`np_divmod`]);
//! * `CPython`'s builtin `min` and `max` of two floats ([`py_min`],
//!   [`py_max`]), which keep their first argument unless the second
//!   compares smaller (larger), so NaN behaves according to its position;
//! * numpy's NaN-propagating `minimum` and `maximum` ([`np_minimum`],
//!   [`np_maximum`]), `clip` ([`np_clip`]) and `sign` ([`np_sign`]);
//! * numpy's `linspace` ([`linspace`], [`linspace_with`]);
//! * numpy's `arcsinh` ([`np_arcsinh`]), correctly rounded on Linux where
//!   Rust's `f64::asinh` is a bit off;
//! * `CPython`'s float `**` and numpy's `power` ([`pow`]): the C library's
//!   `pow`, whatever the base;
//! * numpy's cast of a float to `int64` ([`np_int`]), as the processor
//!   converts what no `int64` holds;
//! * `a * b + c` as matplotlib's compiled extensions compute it on this
//!   platform ([`fused`], [`sum_of_products`], [`difference_of_products`],
//!   [`FUSES_PRODUCTS`]): in one rounding on arm64 macOS, in two
//!   elsewhere.

/// The remainder of `a / b` with the sign of `b`: Python's float `%` and
/// numpy's `remainder`. A zero result takes the sign of `b`.
///
/// ```
/// use pltrs_core::numeric::remainder;
/// assert_eq!(remainder(-1.0, 3.0), 2.0);
/// assert_eq!(remainder(1.0, -3.0), -2.0);
/// ```
pub fn remainder(a: f64, b: f64) -> f64 {
    let fmod = a % b;
    if b == 0.0 {
        fmod
    } else if fmod == 0.0 {
        0.0_f64.copysign(b)
    } else if (b < 0.0) != (fmod < 0.0) {
        fmod + b
    } else {
        fmod
    }
}

/// Python's float `divmod(x, y)`: the floor quotient and a remainder with
/// the sign of `y` (`CPython`'s `float_divmod`). Where Python raises for a
/// zero `y`, both results are NaN.
pub fn divmod(x: f64, y: f64) -> (f64, f64) {
    let mut rem = x % y;
    let mut div = (x - rem) / y;
    if rem == 0.0 {
        rem = 0.0_f64.copysign(y);
    } else if (y < 0.0) != (rem < 0.0) {
        rem += y;
        div -= 1.0;
    }
    let floor = if div == 0.0 {
        0.0_f64.copysign(x / y)
    } else {
        let mut floor = div.floor();
        if div - floor > 0.5 {
            floor += 1.0;
        }
        floor
    };
    (floor, rem)
}

/// numpy's `divmod` for floats (`npy_divmod`): [`divmod`], except that a
/// zero divisor gives the quotient `x / y` (an infinity or NaN).
pub fn np_divmod(x: f64, y: f64) -> (f64, f64) {
    if y == 0.0 {
        return (x / y, x % y);
    }
    divmod(x, y)
}

/// Python's float `x // y`.
pub fn floor_div(x: f64, y: f64) -> f64 {
    divmod(x, y).0
}

/// Python's `max(a, b)`: `a` unless `b > a` (so a NaN `a` wins).
pub fn py_max(a: f64, b: f64) -> f64 {
    if b > a { b } else { a }
}

/// Python's `min(a, b)`: `a` unless `b < a` (so a NaN `a` wins).
pub fn py_min(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}

/// numpy's `minimum(a, b)`, also the step of its `min` reduction: NaN if
/// either value is NaN.
pub fn np_minimum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// numpy's `maximum(a, b)`, also the step of its `max` reduction: NaN if
/// either value is NaN.
pub fn np_maximum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// numpy's `clip(v, lo, hi)` for floats: `minimum(maximum(v, lo), hi)`
/// by comparisons, so a NaN in any argument propagates.
pub fn np_clip(v: f64, lo: f64, hi: f64) -> f64 {
    let raised = if v.is_nan() || v > lo { v } else { lo };
    if raised.is_nan() || raised < hi {
        raised
    } else {
        hi
    }
}

/// numpy's `sign`: -1 or 1, and the value itself for zeros (keeping their
/// sign) and NaN.
pub fn np_sign(x: f64) -> f64 {
    if x == 0.0 || x.is_nan() {
        x
    } else {
        x.signum()
    }
}

/// The inverse hyperbolic sine as numpy's `arcsinh` computes it: beyond
/// 2²⁸, `ln(2x)` rounded once on Linux, where Rust's `f64::asinh` and
/// fdlibm's `ln(x) + ln 2` are a bit off, but that formula on macOS, whose
/// `asinh` numpy calls there (Apple's libm differs from these formulas
/// below 2²⁸ too, at about one point in a hundred); below, the C library's
/// formulas (fdlibm's).
///
/// ```
/// use pltrs_core::numeric::np_arcsinh;
/// // numpy on Linux: np.arcsinh(1e12) == 28.324168296488494 (`f64::asinh`
/// // and macOS' `asinh` give 28.32416829648849, one bit below the exact
/// // 28.3241682964884935...).
/// let expected = if cfg!(target_os = "macos") {
///     28.324_168_296_488_49
/// } else {
///     28.324_168_296_488_494
/// };
/// assert_eq!(np_arcsinh(1e12), expected);
/// assert_eq!(np_arcsinh(-0.0).to_bits(), (-0.0_f64).to_bits());
/// ```
pub fn np_arcsinh(x: f64) -> f64 {
    const LN2: f64 = std::f64::consts::LN_2;
    let ax = x.abs();
    if !ax.is_finite() {
        return x + x;
    }
    let w = if ax < 2f64.powi(-28) {
        return x;
    } else if ax > 2f64.powi(28) {
        let double = 2.0 * ax;
        if double.is_finite() && !cfg!(target_os = "macos") {
            double.ln()
        } else {
            ax.ln() + LN2
        }
    } else if ax > 2.0 {
        (2.0 * ax + 1.0 / ((ax * ax + 1.0).sqrt() + ax)).ln()
    } else {
        let t = ax * ax;
        (ax + t / (1.0 + (1.0 + t).sqrt())).ln_1p()
    };
    w.copysign(x)
}

/// `base ** exponent` as `CPython`'s floats and numpy's `power` compute
/// it: by the C library's `pow`.
///
/// `f64::powf` calls `pow` too, unless LLVM sees a constant base: it then
/// computes `2.0.powf(x)` as `exp2(x)` and, on Apple's platforms,
/// `10.0.powf(x)` as `__exp10(x)`, whose last bits differ from `pow`'s
/// (macOS 26: 350 of 200,000 exponents drawn from `[-10, 10]`, while
/// numpy's `power(10.0, x)` equals `pow` on every one). Hiding the base
/// from the optimiser keeps the call to `pow`.
///
/// ```
/// use pltrs_core::numeric::pow;
/// assert_eq!(pow(10.0, 3.0), 1000.0);
/// assert_eq!(pow(2.0, -1.0), 0.5);
/// ```
#[inline]
pub fn pow(base: f64, exponent: f64) -> f64 {
    std::hint::black_box(base).powf(exponent)
}

/// numpy's `astype(np.int64)` of a float, as the processor converts it:
/// truncated towards zero where it fits; otherwise x86-64 gives the
/// "integer indefinite" `i64::MIN` (for NaN and both infinities alike),
/// while other processors (arm64's `fcvtzs`) saturate, as Rust's casts do,
/// and turn NaN into 0.
///
/// ```
/// use pltrs_core::numeric::np_int;
/// assert_eq!((np_int(2.9), np_int(-2.9)), (2, -2));
/// let x86 = cfg!(target_arch = "x86_64");
/// assert_eq!(np_int(f64::NAN), if x86 { i64::MIN } else { 0 });
/// assert_eq!(np_int(f64::INFINITY), if x86 { i64::MIN } else { i64::MAX });
/// ```
pub fn np_int(x: f64) -> i64 {
    /// 2⁶³, the first float above `int64`'s range.
    const LIMIT: f64 = 9_223_372_036_854_775_808.0;
    if cfg!(target_arch = "x86_64") && (x.is_nan() || x.abs() >= LIMIT) {
        i64::MIN
    } else {
        // Truncation within the range, saturation beyond it.
        x as i64
    }
}

/// Whether the compiled code pltrs reproduces here (matplotlib's
/// extensions, with the Agg, qhull and `HarfBuzz` they build in,
/// contourpy's, and numpy's own loops) rounds `a * b + c` once instead of
/// twice.
///
/// C and C++ allow a compiler to contract a product and the sum or
/// difference it feeds within one expression into a fused multiply-add.
/// clang does so by default where the processor has the instruction and
/// GCC does not for x86-64's baseline, so the wheels for arm64 macOS fuse
/// and those for x86-64 (Linux, Windows, and macOS under Rosetta) do not.
/// The reference is the binary wheel from `PyPI`; another build of
/// matplotlib for the same platform may contract elsewhere.
pub const FUSES_PRODUCTS: bool = cfg!(all(target_os = "macos", target_arch = "aarch64"));

/// Whether numpy's matrix and vector products are Apple's Accelerate's
/// here: numpy's wheel for arm64 macOS links that BLAS, where its other
/// wheels bring `OpenBLAS`. The two round their small products in ways of
/// their own (`pltrs::mplot3d::math`, `Affine2D::then`). numpy's x86-64
/// wheel for macOS links Accelerate too, whose kernels for that processor
/// are others again: it is not followed.
pub const ACCELERATE_BLAS: bool = cfg!(all(target_os = "macos", target_arch = "aarch64"));

/// `a * b + c` where matplotlib's compiled code computes it by a fused
/// multiply-add on the platforms of [`FUSES_PRODUCTS`]: rounded once
/// there, and the product and the sum rounded one after the other
/// elsewhere.
///
/// A ported expression names the product clang fuses: of `a * b + c * d`
/// the first, so `fused(a, b, c * d)`; `c - a * b` is `fused(-a, b, c)` and
/// `a * b - c` is `fused(a, b, -c)`, negation being exact. Without fusing
/// each of these is the expression as written, bit for bit.
///
/// ```
/// use pltrs_core::numeric::{FUSES_PRODUCTS, fused};
/// assert_eq!(fused(3.0, 4.0, 5.0), 17.0);
/// // 0.1 * 10.0 is 1 + 2^-54 before rounding.
/// let residue = fused(0.1, 10.0, -1.0);
/// assert_eq!(residue, if FUSES_PRODUCTS { 2f64.powi(-54) } else { 0.0 });
/// ```
#[inline]
#[must_use]
pub fn fused(a: f64, b: f64, c: f64) -> f64 {
    if FUSES_PRODUCTS {
        rounded_once(a, b, c)
    } else {
        rounded_twice(a, b, c)
    }
}

/// [`fused`] for single-precision operands (Agg's `float` colour types).
///
/// ```
/// use pltrs_core::numeric::fused_f32;
/// assert_eq!(fused_f32(3.0, 4.0, 5.0), 17.0);
/// ```
#[inline]
#[must_use]
pub fn fused_f32(a: f32, b: f32, c: f32) -> f32 {
    if FUSES_PRODUCTS {
        a.mul_add(b, c)
    } else {
        a * b + c
    }
}

/// `a * b + c * d` as that code computes it: where it fuses, clang fuses
/// the first product into the sum and rounds the second.
///
/// ```
/// use pltrs_core::numeric::sum_of_products;
/// assert_eq!(sum_of_products(2.0, 3.0, 4.0, 5.0), 26.0);
/// ```
#[inline]
#[must_use]
pub fn sum_of_products(a: f64, b: f64, c: f64, d: f64) -> f64 {
    fused(a, b, c * d)
}

/// `a * b - c * d` as that code computes it: where it fuses, the first
/// product is fused into the difference and the second rounded.
///
/// ```
/// use pltrs_core::numeric::difference_of_products;
/// assert_eq!(difference_of_products(2.0, 3.0, 4.0, 5.0), -14.0);
/// ```
#[inline]
#[must_use]
pub fn difference_of_products(a: f64, b: f64, c: f64, d: f64) -> f64 {
    fused(a, b, -(c * d))
}

/// `a * b + c` with the exact product added: a fused multiply-add.
#[inline]
fn rounded_once(a: f64, b: f64, c: f64) -> f64 {
    a.mul_add(b, c)
}

/// `a * b + c` with the product rounded before it is added.
#[inline]
fn rounded_twice(a: f64, b: f64, c: f64) -> f64 {
    a * b + c
}

/// `num` evenly spaced samples over `[start, stop]` (numpy `linspace`).
///
/// ```
/// assert_eq!(pltrs_core::numeric::linspace(0.0, 1.0, 5), vec![0.0, 0.25, 0.5, 0.75, 1.0]);
/// ```
pub fn linspace(start: f64, stop: f64, num: usize) -> Vec<f64> {
    linspace_with(start, stop, num, true)
}

/// numpy `linspace` with an explicit `endpoint` flag: `i * step + start`,
/// with `stop` itself as the last sample when `endpoint` is set.
pub fn linspace_with(start: f64, stop: f64, num: usize, endpoint: bool) -> Vec<f64> {
    let div = if endpoint { num.saturating_sub(1) } else { num };
    let delta = stop - start;
    let mut values: Vec<f64> = if div == 0 {
        (0..num).map(|i| i as f64 * delta + start).collect()
    } else {
        let step = delta / div as f64;
        if step == 0.0 {
            (0..num)
                .map(|i| i as f64 / div as f64 * delta + start)
                .collect()
        } else {
            (0..num).map(|i| i as f64 * step + start).collect()
        }
    };
    if endpoint && num > 1 {
        values[num - 1] = stop;
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exponents at which `exp2` and Apple's `__exp10` round differently
    /// from `pow`; `pow`'s results, as glibc and macOS' libm (and so
    /// Python and numpy) give them there.
    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn constant_bases_still_go_through_pow() {
        assert_eq!(
            pow(10.0, -5.449_396_810_393_321).to_bits(),
            0x3ecd_ce26_4eb4_2a8a
        );
        assert_eq!(
            pow(2.0, 7.909_669_318_739_461).to_bits(),
            0x406e_0ece_71fc_aa74
        );
    }

    /// Both forms of [`fused`], whichever the platform selects, at operands
    /// where the product's rounding shows.
    #[test]
    fn fused_products_round_once_or_twice() {
        // 0.1 * 10 = 1 + 2^-54 exactly, which rounds to 1.
        assert_eq!(rounded_twice(0.1, 10.0, -1.0), 0.0);
        assert_eq!(rounded_once(0.1, 10.0, -1.0), 2f64.powi(-54));
        // (1 + 2^-30)^2 = 1 + 2^-29 + 2^-60: the last term is rounded away.
        let x = 1.0 + 2f64.powi(-30);
        assert_eq!(rounded_twice(x, x, -(1.0 + 2f64.powi(-29))), 0.0);
        assert_eq!(rounded_once(x, x, -(1.0 + 2f64.powi(-29))), 2f64.powi(-60));
        // A determinant of dependent rows, `a * d - b * c`: with the first
        // product fused, the second product's rounding error is left.
        let third = 1.0 / 3.0;
        assert_eq!(rounded_twice(third, 3.0, -(third * 3.0)), 0.0);
        assert_eq!(rounded_once(third, 3.0, -(third * 3.0)), -(2f64.powi(-54)));
        // Exact products agree, and the forms written with a negation are
        // the differences as written.
        for (a, b, c) in [(3.0, 4.0, 5.0), (-0.5, 8.0, 4.0), (0.0, -1.0, 0.0)] {
            assert_eq!(
                rounded_once(a, b, c).to_bits(),
                rounded_twice(a, b, c).to_bits()
            );
        }
        let (a, b, c) = (0.3, 0.7, 0.21);
        assert_eq!(rounded_twice(-a, b, c).to_bits(), (c - a * b).to_bits());
        assert_eq!(rounded_twice(a, b, -c).to_bits(), (a * b - c).to_bits());
        assert_eq!(
            fused(a, b, -c),
            if FUSES_PRODUCTS {
                rounded_once(a, b, -c)
            } else {
                rounded_twice(a, b, -c)
            }
        );
        assert!(fused(f64::NAN, 1.0, 1.0).is_nan());
        assert_eq!(fused(f64::INFINITY, 1.0, 1.0), f64::INFINITY);
    }

    #[test]
    fn divmod_matches_python() {
        assert_eq!(divmod(7.0, 2.0), (3.0, 1.0));
        assert_eq!(divmod(-7.0, 2.0), (-4.0, 1.0));
        assert_eq!(divmod(7.0, -2.0), (-4.0, -1.0));
        let (d, m) = divmod(-0.0, 1.0);
        assert!(d == 0.0 && d.is_sign_negative());
        assert!(m == 0.0 && m.is_sign_positive());
        // Python: divmod(1.0, 0.1) == (9.0, 0.09999999999999995)
        assert_eq!(divmod(1.0, 0.1), (9.0, 0.099_999_999_999_999_95));
        assert_eq!(floor_div(0.7, 0.1), 6.0);
        let (d, m) = divmod(1.0, 0.0);
        assert!(d.is_nan() && m.is_nan());
    }

    #[test]
    fn divmod_matches_numpy() {
        assert_eq!(np_divmod(65.0, 50.0), (1.0, 15.0));
        assert_eq!(np_divmod(-7.0, 2.0), (-4.0, 1.0));
        assert_eq!(np_divmod(0.0, 10.0), (0.0, 0.0));
        assert!(np_divmod(1.0, 0.0).0.is_infinite());
        assert!(np_divmod(-0.0, 10.0).0.is_sign_negative());
    }

    #[test]
    fn remainder_matches_numpy() {
        assert_eq!(remainder(7.5, 2.0), 1.5);
        assert_eq!(remainder(-7.5, 2.0), 0.5);
        assert_eq!(remainder(7.5, -2.0), -0.5);
        assert_eq!(remainder(-0.5, 1.0), 0.5);
        assert!(remainder(-4.0, 2.0).is_sign_positive());
        assert!(remainder(4.0, -2.0).is_sign_negative());
        assert!(remainder(1.0, 0.0).is_nan());
        assert!(remainder(f64::NAN, 2.0).is_nan());
        assert_eq!(remainder(1.0, f64::INFINITY), 1.0);
    }

    #[test]
    fn min_max_and_sign() {
        assert!(py_max(f64::NAN, 1.0).is_nan());
        assert_eq!(py_max(1.0, f64::NAN), 1.0);
        assert!(py_min(f64::NAN, 1.0).is_nan());
        assert_eq!(py_min(1.0, f64::NAN), 1.0);
        assert!(np_minimum(1.0, f64::NAN).is_nan());
        assert!(np_maximum(f64::NAN, 1.0).is_nan());
        assert_eq!(np_minimum(1.0, 3.0), 1.0);
        assert_eq!(np_maximum(1.0, 3.0), 3.0);
        assert_eq!(np_clip(2.0, 0.0, 1.0), 1.0);
        assert_eq!(np_clip(-1.0, 0.0, 1.0), 0.0);
        assert_eq!(np_clip(0.25, 0.0, 1.0), 0.25);
        assert!(np_clip(f64::NAN, 0.0, 1.0).is_nan());
        assert!(np_clip(0.5, f64::NAN, 1.0).is_nan());
        assert!(np_clip(0.5, 0.0, f64::NAN).is_nan());
        assert_eq!(np_sign(-3.0), -1.0);
        assert_eq!(np_sign(2.0), 1.0);
        assert!(np_sign(-0.0) == 0.0 && np_sign(-0.0).is_sign_negative());
        assert!(np_sign(f64::NAN).is_nan());
    }

    #[test]
    fn arcsinh_matches_numpy() {
        // numpy 2.5: np.arcsinh of each (on macOS, Apple's `asinh`, one
        // bit lower for 1e300).
        assert_eq!(np_arcsinh(1e12 + 3.0), 28.324_168_296_491_493);
        assert_eq!(np_arcsinh(0.5), 0.481_211_825_059_603_47);
        assert_eq!(np_arcsinh(-3.0), -1.818_446_459_232_066_8);
        assert_eq!(np_arcsinh(1e-30), 1e-30);
        let huge = if cfg!(target_os = "macos") {
            691.468_675_078_773_6
        } else {
            691.468_675_078_773_7
        };
        assert_eq!(np_arcsinh(1e300), huge);
        assert!(np_arcsinh(f64::NAN).is_nan());
        assert_eq!(np_arcsinh(f64::NEG_INFINITY), f64::NEG_INFINITY);
    }

    #[test]
    fn linspace_matches_numpy() {
        assert_eq!(linspace(0.0, 1.0, 1), vec![0.0]);
        assert_eq!(linspace(0.0, 1.0, 0), Vec::<f64>::new());
        assert_eq!(linspace(2.0, 2.0, 3), vec![2.0, 2.0, 2.0]);
        assert_eq!(
            linspace_with(0.0, 1.0, 4, false),
            vec![0.0, 0.25, 0.5, 0.75]
        );
        // numpy: np.linspace(0, 1, 7)[3] == 0.5 exactly, [6] pinned to stop
        let v = linspace(0.0, 1.0, 7);
        assert_eq!(v[3], 0.5);
        assert_eq!(v[6], 1.0);
        assert_eq!(v[1], 1.0 / 6.0);
    }
}
