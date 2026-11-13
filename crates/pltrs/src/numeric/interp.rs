//! One-dimensional linear interpolation (numpy's `interp`).

use pltrs_core::numeric::fused;

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
    // The last index whose sample does not exceed x, as numpy searches
    // for it: none (its -1) or the one past the last only when NaN
    // samples break the order.
    let j = match search(x, xp) {
        Found::Before => return left,
        Found::After => return right,
        Found::At(j) => j,
    };
    #[expect(
        clippy::float_cmp,
        reason = "numpy returns the sample itself on an exact hit"
    )]
    if j == n - 1 || xp[j] == x {
        return fp[j];
    }
    let slope = (fp[j + 1] - fp[j]) / (xp[j + 1] - xp[j]);
    // numpy's build for arm64 macOS fuses the product into the sum.
    let value = fused(slope, x - xp[j], fp[j]);
    if !value.is_nan() {
        return value;
    }
    let other = fused(slope, x - xp[j + 1], fp[j + 1]);
    #[expect(clippy::float_cmp, reason = "numpy's fallback for equal neighbours")]
    let flat = fp[j] == fp[j + 1];
    if other.is_nan() && flat { fp[j] } else { other }
}

/// Where numpy's search puts a value among the samples.
enum Found {
    /// Before the first sample.
    Before,
    /// At or after the sample of this index, before the next.
    At(usize),
    /// After the last sample.
    After,
}

/// How far from its guess numpy's search looks before it bisects
/// (`LIKELY_IN_CACHE_SIZE`).
const NEAR: usize = 8;

/// numpy's `binary_search_with_guess(key, arr, len, 0)`, the search of
/// `interp`: for samples in order, the last index whose sample does not
/// exceed `key`; NaN samples compare false with everything, and where
/// they send the search (linear up to four samples, around its guess and
/// then by bisection for more) is what numpy interpolates from.
fn search(key: f64, arr: &[f64]) -> Found {
    let len = arr.len();
    if key > arr[len - 1] {
        return Found::After;
    }
    if key < arr[0] {
        return Found::Before;
    }
    if len <= 4 {
        #[expect(
            clippy::neg_cmp_op_on_partial_ord,
            reason = "a NaN sample ends the scan, as numpy's `key >= arr[i]` does"
        )]
        let next = (1..len).find(|&i| !(key >= arr[i])).unwrap_or(len);
        return Found::At(next - 1);
    }
    // The guess 0, moved inside: 1.
    let guess = 1;
    let (mut imin, mut imax) = (0, len);
    if key < arr[guess] {
        if key < arr[guess - 1] {
            imax = guess - 1;
        } else {
            return Found::At(guess - 1);
        }
    } else if key < arr[guess + 1] {
        return Found::At(guess);
    } else if key < arr[guess + 2] {
        return Found::At(guess + 1);
    } else {
        imin = guess + 2;
        if guess + NEAR + 1 < len && key < arr[guess + NEAR] {
            imax = guess + NEAR;
        }
    }
    while imin < imax {
        let imid = imin + ((imax - imin) >> 1);
        if key >= arr[imid] {
            imin = imid + 1;
        } else {
            imax = imid;
        }
    }
    imin.checked_sub(1).map_or(Found::Before, Found::At)
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
        // numpy 2.5.4 on x86-64 and, the product fused, on arm64 macOS.
        let rounded = interp(1.7, &[0.1, 2.3, 4.1], &[0.33, 0.71, 0.12], None);
        let expected = if pltrs_core::numeric::FUSES_PRODUCTS {
            0.606_363_636_363_636_3
        } else {
            0.606_363_636_363_636_4
        };
        assert_eq!(rounded, expected);
        assert_eq!(interp(-1.0, &xp, &fp, Some((-1.0, -1.0))), -1.0);
        assert_eq!(interp(5.0, &xp, &fp, None), 40.0);
        assert!(interp(f64::NAN, &xp, &fp, None).is_nan());
        assert_eq!(interp(2.0, &[1.0], &[7.0], Some((0.0, 9.0))), 9.0);
        assert!(interp(1.0, &[], &[], None).is_nan());
    }

    #[test]
    fn nan_samples_send_the_search_where_numpys_goes() {
        let nan = f64::NAN;
        let outside = Some((-1.0, -1.0));
        // Found by fuzzing: a NaN first sample underflowed the search.
        assert!(interp(1.0, &[nan, nan, 2.0], &[0.0, 10.0, 20.0], Some((-1.0, 9.0))).is_nan());
        let fp: Vec<f64> = (0..11).map(f64::from).collect();
        // A line at NaN on the canvas (matplotlib's contour labels).
        let lost = [nan; 11];
        assert!(interp(-7.0, &lost, &fp, outside).is_nan());
        assert!(interp(7.0, &lost, &fp, outside).is_nan());
        let mut from_zero = lost;
        from_zero[0] = 0.0;
        assert_eq!(interp(-7.0, &from_zero, &fp, outside), -1.0);
        assert!(interp(7.0, &from_zero, &fp, outside).is_nan());
        let gap = [0.0, 1.0, nan, 3.0, 4.0, 5.0];
        assert!(interp(1.5, &gap, &fp[..6], outside).is_nan());
        assert_eq!(interp(3.5, &gap, &fp[..6], None), 3.5);
        assert_eq!(interp(0.5, &gap, &fp[..6], None), 0.5);
        assert_eq!(
            interp(2.5, &[nan, 1.0, 2.0, 3.0, 4.0, 5.0], &fp[..6], outside),
            2.5
        );
        assert_eq!(
            interp(2.5, &[0.0, 1.0, 2.0, 3.0, 4.0, nan], &fp[..6], outside),
            2.5
        );
    }

    #[test]
    fn long_ordered_samples_are_bisected() {
        let xp: Vec<f64> = (0..40).map(f64::from).collect();
        for x in [0.0, 0.5, 1.0, 2.5, 8.5, 9.0, 9.5, 20.25, 38.5, 39.0] {
            assert_eq!(interp(x, &xp, &xp, None), x);
        }
    }
}
