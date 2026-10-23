//! numpy-style numeric helpers: ranges, grids, NaN-aware reductions,
//! numpy's (unstable) `argsort` and index regions (runs of a mask, block
//! perimeters of a grid).
//!
//! The range constructors reproduce numpy's floating-point arithmetic
//! exactly (`linspace` computes `i * step + start` and pins the endpoint),
//! because matplotlib's results depend on those precise values.

mod argsort;
mod grid;
mod interp;
mod regions;

pub use argsort::argsort;
pub use grid::{Grid, GridError};
pub use interp::interp;
pub use regions::{block_perimeters, contiguous_regions};

/// `num` evenly spaced samples over `[start, stop]` (numpy `linspace`).
///
/// ```
/// assert_eq!(plt::numeric::linspace(0.0, 1.0, 5), vec![0.0, 0.25, 0.5, 0.75, 1.0]);
/// ```
pub fn linspace(start: f64, stop: f64, num: usize) -> Vec<f64> {
    linspace_with(start, stop, num, true)
}

/// numpy `linspace` with an explicit `endpoint` flag.
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

/// Values `start, start + step, ...` strictly before `stop` (numpy `arange`).
///
/// numpy stores `start` and `start + step`, then fills the rest with
/// `start + i * delta` where `delta = (start + step) - start`, which can
/// differ from `step` in the last bits: `arange(-3.0, 3.0, 0.025)[173]` is
/// `1.324999999999985`, not `1.3250000000000002`.
///
/// Returns an empty vector for a zero or wrongly-signed step.
pub fn arange(start: f64, stop: f64, step: f64) -> Vec<f64> {
    if step == 0.0 || !step.is_finite() {
        return Vec::new();
    }
    let count = ((stop - start) / step).ceil();
    if !count.is_finite() || count <= 0.0 {
        return Vec::new();
    }
    let next = start + step;
    let delta = next - start;
    (0..count as usize)
        .map(|i| match i {
            0 => start,
            1 => next,
            _ => start + i as f64 * delta,
        })
        .collect()
}

/// `num` samples spaced evenly on a log scale: `base ** linspace(start, stop)`.
pub fn logspace(start: f64, stop: f64, num: usize, base: f64) -> Vec<f64> {
    linspace(start, stop, num)
        .into_iter()
        .map(|e| base.powf(e))
        .collect()
}

/// `num` samples spaced evenly on a log scale from `start` to `stop`, both
/// included exactly (numpy `geomspace`); both must have the same sign.
///
/// ```
/// let v = plt::numeric::geomspace(1.0, 1000.0, 4);
/// assert_eq!((v[0], v[3]), (1.0, 1000.0));
/// assert!((v[1] - 10.0).abs() < 1e-12);
/// ```
pub fn geomspace(start: f64, stop: f64, num: usize) -> Vec<f64> {
    let sign = start.signum();
    let mut values = logspace((start / sign).log10(), (stop / sign).log10(), num, 10.0);
    if let Some(first) = values.first_mut() {
        *first = start / sign;
    }
    if num > 1 {
        values[num - 1] = stop / sign;
    }
    values.iter().map(|v| v * sign).collect()
}

/// Coordinate matrices from coordinate vectors (numpy `meshgrid`, `xy`
/// indexing): both grids have `y.len()` rows and `x.len()` columns.
pub fn meshgrid(x: &[f64], y: &[f64]) -> (Grid<f64>, Grid<f64>) {
    let xs = Grid::from_fn(y.len(), x.len(), |_, c| x[c]);
    let ys = Grid::from_fn(y.len(), x.len(), |r, _| y[r]);
    (xs, ys)
}

/// The remainder of `a / b` with the sign of `b` (numpy `remainder`,
/// Python's `%`); a zero result takes the sign of `b`.
///
/// ```
/// assert_eq!(plt::numeric::remainder(-1.0, 3.0), 2.0);
/// assert_eq!(plt::numeric::remainder(1.0, -3.0), -2.0);
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

/// Minimum ignoring NaN; `None` if no finite-or-infinite value exists.
pub fn nanmin<I: IntoIterator<Item = f64>>(values: I) -> Option<f64> {
    values.into_iter().filter(|v| !v.is_nan()).reduce(f64::min)
}

/// Maximum ignoring NaN; `None` if no finite-or-infinite value exists.
pub fn nanmax<I: IntoIterator<Item = f64>>(values: I) -> Option<f64> {
    values.into_iter().filter(|v| !v.is_nan()).reduce(f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn arange_matches_numpy() {
        assert_eq!(arange(0.0, 1.0, 0.25), vec![0.0, 0.25, 0.5, 0.75]);
        assert_eq!(arange(3.0, 0.0, -1.0), vec![3.0, 2.0, 1.0]);
        assert_eq!(arange(0.0, 1.0, -1.0), Vec::<f64>::new());
        assert_eq!(arange(0.0, 1.0, 0.0), Vec::<f64>::new());
        assert_eq!(arange(0.0, 0.3, 0.1).len(), 3);
        // numpy fills with (start + step) - start, not step.
        let grid = arange(-3.0, 3.0, 0.025);
        assert_eq!(grid.len(), 240);
        assert_eq!(grid[1], -2.975);
        assert_eq!(grid[173], 1.324_999_999_999_985);
        assert_eq!(arange(0.1, 1.0, 0.2)[2], 0.500_000_000_000_000_1);
    }

    #[test]
    fn geomspace_pins_the_ends() {
        let v = geomspace(1e-2, 1e3, 40);
        assert_eq!((v[0], v[39]), (1e-2, 1e3));
        let negative = geomspace(-1.0, -100.0, 3);
        assert_eq!(negative[0], -1.0);
        assert!((negative[1] + 10.0).abs() < 1e-12);
        assert_eq!(geomspace(1.0, 10.0, 1), [1.0]);
        assert_eq!(geomspace(1.0, 10.0, 0), Vec::<f64>::new());
    }

    #[test]
    fn remainder_matches_numpy() {
        assert_eq!(remainder(7.5, 2.0), 1.5);
        assert_eq!(remainder(-7.5, 2.0), 0.5);
        assert_eq!(remainder(7.5, -2.0), -0.5);
        assert!(remainder(-4.0, 2.0).is_sign_positive());
        assert!(remainder(4.0, -2.0).is_sign_negative());
        assert!(remainder(1.0, 0.0).is_nan());
        assert!(remainder(f64::NAN, 2.0).is_nan());
        assert_eq!(remainder(1.0, f64::INFINITY), 1.0);
    }

    #[test]
    fn logspace_and_meshgrid() {
        assert_eq!(logspace(0.0, 2.0, 3, 10.0), vec![1.0, 10.0, 100.0]);
        let (xs, ys) = meshgrid(&[1.0, 2.0, 3.0], &[10.0, 20.0]);
        assert_eq!((xs.rows(), xs.cols()), (2, 3));
        assert_eq!(xs[(1, 2)], 3.0);
        assert_eq!(ys[(1, 2)], 20.0);
    }

    #[test]
    fn nan_reductions() {
        assert_eq!(nanmin([3.0, f64::NAN, 1.0]), Some(1.0));
        assert_eq!(nanmax([f64::NAN]), None);
        assert_eq!(nanmax([f64::NEG_INFINITY, -1.0]), Some(-1.0));
    }
}
