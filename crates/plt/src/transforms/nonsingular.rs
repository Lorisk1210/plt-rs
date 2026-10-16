//! Expansion of degenerate ranges (matplotlib's `transforms.nonsingular`).

/// Widens `vmin..vmax` so it is usable as an axis range: non-finite
/// endpoints become `(-expander, expander)`, ranges that are zero or
/// numerically empty relative to their magnitude are expanded by
/// `expander` times the magnitude (or to `±expander` around zero).
///
/// The result is increasing unless the input was decreasing and
/// `increasing` is `false`. This is an exact port of matplotlib's
/// `transforms.nonsingular`; locators use `expander = 0.05`.
///
/// ```
/// use plt::transforms::nonsingular;
/// assert_eq!(nonsingular(2.0, 2.0, 0.05, 1e-15, true), (1.9, 2.1));
/// assert_eq!(nonsingular(0.0, 0.0, 0.05, 1e-15, true), (-0.05, 0.05));
/// assert_eq!(nonsingular(5.0, 1.0, 0.05, 1e-15, false), (5.0, 1.0));
/// ```
pub fn nonsingular(vmin: f64, vmax: f64, expander: f64, tiny: f64, increasing: bool) -> (f64, f64) {
    if !vmin.is_finite() || !vmax.is_finite() {
        return (-expander, expander);
    }
    let swapped = vmax < vmin;
    let (mut lo, mut hi) = if swapped { (vmax, vmin) } else { (vmin, vmax) };
    let max_abs = lo.abs().max(hi.abs());
    if max_abs < (1e6 / tiny) * f64::MIN_POSITIVE {
        lo = -expander;
        hi = expander;
    } else if hi - lo <= max_abs * tiny {
        if hi == 0.0 && lo == 0.0 {
            lo = -expander;
            hi = expander;
        } else {
            lo -= expander * lo.abs();
            hi += expander * hi.abs();
        }
    }
    if swapped && !increasing {
        (hi, lo)
    } else {
        (lo, hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_matplotlib() {
        assert_eq!(
            nonsingular(f64::NAN, 1.0, 0.001, 1e-15, true),
            (-0.001, 0.001)
        );
        assert_eq!(
            nonsingular(1.0, f64::INFINITY, 0.05, 1e-15, true),
            (-0.05, 0.05)
        );
        assert_eq!(nonsingular(3.0, 1.0, 0.05, 1e-15, true), (1.0, 3.0));
        assert_eq!(nonsingular(-2.0, -2.0, 0.05, 1e-15, true), (-2.1, -1.9));
        assert_eq!(
            nonsingular(1e-310, 1e-310, 0.05, 1e-15, true),
            (-0.05, 0.05),
            "subnormal magnitudes"
        );
        assert_eq!(
            nonsingular(0.0, 10.0, 0.05, 1e-15, true),
            (0.0, 10.0),
            "regular ranges are untouched"
        );
    }
}
