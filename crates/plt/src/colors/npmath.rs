//! numpy's floating-point semantics that colour code reproduces.

/// `x ** exponent` as numpy evaluates it for an array and a Python float:
/// exponents 1, 2, 0.5, -1 and 0 take exact fast paths, others call `pow`.
pub(crate) fn np_power(x: f64, exponent: f64) -> f64 {
    match exponent {
        1.0 => x,
        2.0 => x * x,
        0.5 => x.sqrt(),
        -1.0 => 1.0 / x,
        0.0 => 1.0,
        _ => x.powf(exponent),
    }
}

/// numpy's sort order for floats: NaN sorts after every number.
pub(crate) fn np_less(a: f64, b: f64) -> bool {
    a < b || (b.is_nan() && !a.is_nan())
}

/// `np.clip(v, lo, hi)`: NaN in any argument propagates.
pub(crate) fn np_clip(v: f64, lo: f64, hi: f64) -> f64 {
    if v.is_nan() || lo.is_nan() || hi.is_nan() {
        f64::NAN
    } else {
        v.max(lo).min(hi)
    }
}

/// The smaller of two values as numpy's `min` reduction sees it: NaN wins.
pub(crate) fn nan_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// The larger of two values as numpy's `max` reduction sees it: NaN wins.
pub(crate) fn nan_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numpy_semantics() {
        assert_eq!(np_power(9.0, 0.5), 3.0);
        assert_eq!(np_power(3.0, 2.0), 9.0);
        assert_eq!(np_power(2.0, 3.0), 8.0);
        assert!(np_clip(f64::NAN, 0.0, 1.0).is_nan());
        assert!(np_clip(0.5, f64::NAN, 1.0).is_nan());
        assert_eq!(np_clip(2.0, 0.0, 1.0), 1.0);
        assert!(np_less(1.0, f64::NAN));
        assert!(!np_less(f64::NAN, 1.0));
        assert!(nan_min(1.0, f64::NAN).is_nan());
        assert_eq!(nan_max(1.0, 3.0), 3.0);
    }
}
