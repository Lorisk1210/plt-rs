//! Python and numpy numeric semantics that tick placement depends on.
//!
//! Tick placement is full of `divmod`, `//`, `round` and `np.arange` calls
//! whose exact floating-point behaviour decides which ticks appear. These
//! helpers reproduce `CPython`'s and numpy's algorithms bit for bit; the
//! float arithmetic shared beyond ticking (`divmod`, `//`, `%`, `min`,
//! `max`, `sign`) is in [`plt_core::numeric`].

use super::TickerError;

/// Python's `math.isclose(a, b)` with the default `rel_tol=1e-9`.
pub(crate) fn isclose(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    if a.is_infinite() || b.is_infinite() {
        return false;
    }
    let diff = (b - a).abs();
    diff <= (1e-9 * b).abs() || diff <= (1e-9 * a).abs()
}

/// numpy's `isclose(a, b, rtol, atol)`; `b` is the reference value.
pub(crate) fn np_isclose(a: f64, b: f64, rtol: f64, atol: f64) -> bool {
    if a.is_infinite() || b.is_infinite() {
        return a == b;
    }
    (a - b).abs() <= atol + rtol * b.abs()
}

/// matplotlib's `_is_close_to_int`: `math.isclose(x, round(x))`.
///
/// Python raises for non-finite `x`; callers check finiteness first.
pub(crate) fn is_close_to_int(x: f64) -> bool {
    isclose(x, x.round_ties_even())
}

/// numpy's `power_of_ten`, used by `np.round`.
fn power_of_ten(n: u32) -> f64 {
    const P10: [f64; 9] = [1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8];
    match P10.get(n as usize) {
        Some(&p) => p,
        None => (9..n).fold(1e9, |acc, _| acc * 10.0),
    }
}

/// numpy's `np.round(x, decimals)`: scale, round half to even, unscale.
pub(crate) fn np_round(x: f64, decimals: i32) -> f64 {
    let scale = power_of_ten(decimals.unsigned_abs());
    if decimals >= 0 {
        (x * scale).round_ties_even() / scale
    } else {
        (x / scale).round_ties_even() * scale
    }
}

/// Python's `round(x, ndigits)` for floats: the double nearest to the
/// correctly rounded decimal value.
pub(crate) fn py_round(x: f64, ndigits: usize) -> f64 {
    if !x.is_finite() {
        return x;
    }
    format!("{x:.ndigits$}")
        .parse()
        .expect("Rust's fixed-point output always parses back")
}

/// `10.0 ** e` as computed by C's `pow` (Python's float power).
pub(crate) fn pow10(e: f64) -> f64 {
    10.0_f64.powf(e)
}

/// `10**e` as Python evaluates it for an integer `e`: the double
/// nearest to the exact power.
pub(crate) fn pow10_int(e: i32) -> f64 {
    format!("1e{e}")
        .parse()
        .expect("`1e<int>` is a valid float literal")
}

/// Python's `math.log(x, base)`: `log(x) / log(base)`.
pub(crate) fn log_base(x: f64, base: f64) -> f64 {
    x.ln() / base.ln()
}

/// numpy's `abs(np.spacing(x))`: the gap to the next float away from zero.
pub(crate) fn spacing(x: f64) -> f64 {
    let a = x.abs();
    if !a.is_finite() {
        return f64::NAN;
    }
    f64::from_bits(a.to_bits() + 1) - a
}

/// Python's `math.floor` as an exact integer, refusing NaN and infinity
/// like Python does.
pub(crate) fn floor_int(x: f64, what: &'static str) -> Result<i64, TickerError> {
    to_int(x.floor(), what)
}

/// Python's `math.ceil` as an exact integer.
pub(crate) fn ceil_int(x: f64, what: &'static str) -> Result<i64, TickerError> {
    to_int(x.ceil(), what)
}

/// Python's `round(x)` (half to even) as an exact integer.
pub(crate) fn round_int(x: f64, what: &'static str) -> Result<i64, TickerError> {
    to_int(x.round_ties_even(), what)
}

fn to_int(integral: f64, what: &'static str) -> Result<i64, TickerError> {
    // Integral values within ±2^63 convert exactly.
    if integral.is_finite() && integral.abs() < 9.2e18 {
        Ok(integral as i64)
    } else {
        Err(TickerError::NonFinite(what))
    }
}

/// The largest array `np.arange` may build here; matplotlib would exhaust
/// memory long before, plt-rs reports [`TickerError::TooManyTicks`].
const MAX_ARANGE: f64 = 1e7;

/// numpy's `np.arange(start, stop, step)` for floats ([`numeric::arange`],
/// whose last-bit rounding matters for tick positions), with numpy's
/// errors for a zero step and a NaN length.
///
/// [`numeric::arange`]: crate::numeric::arange
pub(crate) fn arange(start: f64, stop: f64, step: f64) -> Result<Vec<f64>, TickerError> {
    if step == 0.0 {
        return Err(TickerError::invalid("step", "arange step must be non-zero"));
    }
    let len = ((stop - start) / step).ceil();
    if len.is_nan() {
        return Err(TickerError::NonFinite("arange length"));
    }
    if len > MAX_ARANGE {
        return Err(TickerError::TooManyTicks(len as usize));
    }
    Ok(crate::numeric::arange(start, stop, step))
}

/// numpy's `np.linspace(start, stop, num)` ([`numeric::linspace`]), with
/// [`TickerError::TooManyTicks`] for more than [`arange`] builds.
///
/// [`numeric::linspace`]: crate::numeric::linspace
pub(crate) fn linspace(start: f64, stop: f64, num: usize) -> Result<Vec<f64>, TickerError> {
    if num as f64 > MAX_ARANGE {
        return Err(TickerError::TooManyTicks(num));
    }
    Ok(crate::numeric::linspace(start, stop, num))
}

/// `np.unique`: sorted values without duplicates (NaN sorted last).
pub(crate) fn sorted_unique(values: &[f64]) -> Vec<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted.dedup_by(|a, b| a == b);
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_helpers() {
        assert_eq!(np_round(2.675, 2), 2.68);
        assert_eq!(np_round(0.5, 0), 0.0);
        assert_eq!(np_round(1250.0, -2), 1200.0);
        assert_eq!(py_round(2.675, 2), 2.67);
        assert_eq!(py_round(0.123_456_789_012_345, 10), 0.123_456_789);
        assert_eq!(round_int(2.5, "x"), Ok(2));
        assert!(round_int(f64::NAN, "x").is_err());
        assert_eq!(floor_int(-0.5, "x"), Ok(-1));
        assert_eq!(ceil_int(-0.5, "x"), Ok(0));
    }

    #[test]
    fn closeness() {
        assert!(isclose(1.0, 1.0 + 1e-12));
        assert!(!isclose(1e-12, 0.0));
        assert!(is_close_to_int(3.000_000_000_1));
        assert!(np_isclose(2.500_001, 2.5, 1e-5, 1e-8));
        assert!(!np_isclose(f64::NAN, f64::NAN, 1e-5, 1e-8));
        assert!(np_isclose(f64::INFINITY, f64::INFINITY, 1e-5, 1e-8));
    }

    #[test]
    fn arange_matches_numpy() {
        // numpy: np.arange(0.1, 1.0, 0.2)[2] == 0.5000000000000001
        let values = arange(0.1, 1.0, 0.2).unwrap();
        assert_eq!(values.len(), 5);
        assert_eq!(values[1], 0.300_000_000_000_000_04);
        assert_eq!(values[2], 0.500_000_000_000_000_1);
        assert_eq!(arange(1.0, 0.0, 1.0).unwrap(), Vec::<f64>::new());
        assert!(arange(0.0, 1.0, 0.0).is_err());
        assert!(arange(0.0, 1e300, 1.0).is_err());
        assert!(arange(0.0, f64::NAN, 1.0).is_err());
    }

    #[test]
    fn spacing_and_unique() {
        assert_eq!(spacing(1.0), f64::EPSILON);
        assert_eq!(spacing(-1.0), f64::EPSILON);
        assert_eq!(sorted_unique(&[3.0, 1.0, 3.0, 2.0]), vec![1.0, 2.0, 3.0]);
    }
}
