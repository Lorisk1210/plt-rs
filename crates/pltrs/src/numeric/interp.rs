//! One-dimensional linear interpolation (numpy's `interp`).

/// numpy's `interp(x, xp, fp, left, right)` for non-decreasing `xp`:
/// `bounds` replaces the end values used outside the range.
///
/// ```
/// use pltrs::numeric::interp;
/// assert_eq!(interp(0.5, &[0.0, 1.0], &[10.0, 20.0], None), 15.0);
/// assert_eq!(interp(2.0, &[0.0, 1.0], &[10.0, 20.0], Some((-1.0, 99.0))), 99.0);
/// ```
pub fn interp(x: f64, xp: &[f64], fp: &[f64], bounds: Option<(f64, f64)>) -> f64 {
    let (Some(&first), Some(&last)) = (fp.first(), fp.last()) else {
        return f64::NAN;
    };
    let (left, right) = bounds.unwrap_or((first, last));
    let n = xp.len();
    if x.is_nan() {
        return x;
    }
    if n == 1 {
        return if x < xp[0] {
            left
        } else if x > xp[0] {
            right
        } else {
            fp[0]
        };
    }
    if x > xp[n - 1] {
        return right;
    }
    if x < xp[0] {
        return left;
    }
    // The last index whose sample does not exceed x; none (numpy's -1)
    // only when NaN samples break the order.
    let Some(j) = xp.partition_point(|&v| v <= x).checked_sub(1) else {
        return left;
    };
    #[expect(
        clippy::float_cmp,
        reason = "numpy returns the sample itself on an exact hit"
    )]
    if j == n - 1 || xp[j] == x {
        return fp[j];
    }
    let slope = (fp[j + 1] - fp[j]) / (xp[j + 1] - xp[j]);
    let value = slope * (x - xp[j]) + fp[j];
    if !value.is_nan() {
        return value;
    }
    let other = slope * (x - xp[j + 1]) + fp[j + 1];
    #[expect(clippy::float_cmp, reason = "numpy's fallback for equal neighbours")]
    let flat = fp[j] == fp[j + 1];
    if other.is_nan() && flat { fp[j] } else { other }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interp_matches_numpy() {
        let xp = [0.0, 1.0, 1.0, 3.0];
        let fp = [0.0, 10.0, 20.0, 40.0];
        assert_eq!(interp(0.5, &xp, &fp, None), 5.0);
        assert_eq!(
            interp(1.0, &xp, &fp, None),
            20.0,
            "duplicates take the last sample"
        );
        assert_eq!(interp(2.0, &xp, &fp, None), 30.0);
        assert_eq!(interp(-1.0, &xp, &fp, Some((-1.0, -1.0))), -1.0);
        assert_eq!(interp(5.0, &xp, &fp, None), 40.0);
        assert!(interp(f64::NAN, &xp, &fp, None).is_nan());
        assert_eq!(interp(2.0, &[1.0], &[7.0], Some((0.0, 9.0))), 9.0);
        assert!(interp(1.0, &[], &[], None).is_nan());
        // Found by fuzzing: a NaN first sample underflowed the search.
        let unordered = [f64::NAN, f64::NAN, 2.0];
        assert_eq!(interp(1.0, &unordered, &fp[..3], Some((-1.0, 9.0))), -1.0);
    }
}
