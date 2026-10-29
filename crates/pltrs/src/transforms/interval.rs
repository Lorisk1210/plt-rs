//! Interval membership tests (matplotlib's `transforms._interval_contains*`).
//!
//! Intervals are `(a, b)` pairs in either orientation. Comparisons with
//! NaN are false, so a NaN value is never contained.

/// Whether `val` lies in the closed interval, in either orientation.
pub fn interval_contains((a, b): (f64, f64), val: f64) -> bool {
    let (lo, hi) = sorted(a, b);
    lo <= val && val <= hi
}

/// Whether `val` lies strictly inside the interval, in either orientation.
pub fn interval_contains_open((a, b): (f64, f64), val: f64) -> bool {
    (a < val && val < b) || (a > val && val > b)
}

/// Whether `val` lies in the closed interval widened on both sides by
/// `rtol` times its length (matplotlib uses `rtol = 1e-10` to keep ticks
/// that sit on a limit up to rounding error).
pub fn interval_contains_close((a, b): (f64, f64), val: f64, rtol: f64) -> bool {
    let (lo, hi) = sorted(a, b);
    let slack = (hi - lo) * rtol;
    lo - slack <= val && val <= hi + slack
}

/// Swaps the endpoints when `a > b` (so a NaN endpoint stays in place).
fn sorted(a: f64, b: f64) -> (f64, f64) {
    if a > b { (b, a) } else { (a, b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_interval_in_either_orientation() {
        assert!(interval_contains((1.0, 0.0), 1.0));
        assert!(interval_contains((0.0, 1.0), 0.0));
        assert!(!interval_contains((0.0, 1.0), 1.0 + 1e-11));
        assert!(interval_contains(
            (f64::NEG_INFINITY, 0.0),
            f64::NEG_INFINITY
        ));
        assert!(!interval_contains((0.0, 1.0), f64::NAN));
    }

    #[test]
    fn open_interval_excludes_endpoints() {
        assert!(!interval_contains_open((0.0, 1.0), 1.0));
        assert!(interval_contains_open((1.0, 0.0), 0.5));
        assert!(!interval_contains_open((1.0, 1.0), 1.0), "degenerate");
        assert!(!interval_contains_open((0.0, 1.0), f64::NAN));
    }

    #[test]
    fn close_interval_tolerates_rounding() {
        assert!(interval_contains_close((0.0, 1.0), 1.0 + 1e-11, 1e-10));
        assert!(!interval_contains_close((0.0, 1.0), 1.0 + 1e-9, 1e-10));
        assert!(interval_contains_close((1.0, 0.0), -0.0005, 1e-3));
        assert!(!interval_contains_close((1.0, 0.0), 1.002, 1e-3));
        assert!(
            interval_contains_close((1.0, 1.0), 1.0, 1e-10),
            "degenerate"
        );
    }

    #[test]
    fn close_interval_with_infinite_length() {
        // The slack is infinite, so every non-NaN value is contained...
        assert!(interval_contains_close(
            (f64::NEG_INFINITY, 0.0),
            5.0,
            1e-10
        ));
        assert!(!interval_contains_close(
            (f64::NEG_INFINITY, 0.0),
            f64::NAN,
            1e-10
        ));
        // ...unless rtol is zero: inf * 0 is NaN and every comparison fails.
        assert!(!interval_contains_close(
            (f64::NEG_INFINITY, 0.0),
            -1.0,
            0.0
        ));
    }
}
