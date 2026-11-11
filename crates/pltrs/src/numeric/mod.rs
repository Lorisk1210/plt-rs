//! numpy-style numeric helpers: ranges, grids, NaN-aware reductions,
//! numpy's (unstable) `argsort`, index regions (runs of a mask, block
//! perimeters of a grid) and numpy's scalar arithmetic with its runtime
//! warnings ([`numpy_arith`]).
//!
//! The range constructors reproduce numpy's floating-point arithmetic
//! exactly (`linspace` computes `i * step + start` and pins the endpoint),
//! because matplotlib's results depend on those precise values;
//! [`linspace`], [`linspace_with`], [`remainder`] and [`pow`] (Python's
//! float `**`) come from [`pltrs_core::numeric`], which the lower-level
//! crates share.

mod argsort;
mod grid;
mod interp;
mod number;
pub mod numpy_arith;
mod regions;

pub use argsort::argsort;
pub(crate) use grid::cell_buffer;
pub use grid::{Grid, GridShapeError};
pub use interp::interp;
pub use number::Number;
pub(crate) use number::{to_f64s, with_numbers};
use pltrs_core::numeric::fused;
pub use pltrs_core::numeric::{linspace, linspace_with, pow, remainder};
pub use regions::{block_perimeters, contiguous_regions};

/// Values `start, start + step, ...` strictly before `stop` (numpy `arange`).
///
/// numpy stores `start` and `start + step`, then fills the rest with
/// `start + i * delta` where `delta = (start + step) - start`, which can
/// differ from `step` in the last bits: `arange(-3.0, 3.0, 0.025)[173]` is
/// `1.324999999999985`, not `1.3250000000000002`. numpy's wheel for arm64
/// macOS is built like matplotlib's, with that product fused
/// (`pltrs_core::numeric::fused`): the value is `1.3249999999999846` there.
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
            _ => fused(i as f64, delta, start),
        })
        .collect()
}

/// `num` samples spaced evenly on a log scale: `base ** linspace(start, stop)`.
pub fn logspace(start: f64, stop: f64, num: usize, base: f64) -> Vec<f64> {
    linspace(start, stop, num)
        .into_iter()
        .map(|e| pow(base, e))
        .collect()
}

/// `num` samples spaced evenly on a log scale from `start` to `stop`, both
/// included exactly (numpy `geomspace`); both must have the same sign.
///
/// ```
/// let v = pltrs::numeric::geomspace(1.0, 1000.0, 4);
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

/// Minimum ignoring NaN; `None` if no finite-or-infinite value exists.
pub fn nanmin<I: IntoIterator<Item = f64>>(values: I) -> Option<f64> {
    values.into_iter().filter(|v| !v.is_nan()).reduce(f64::min)
}

/// Maximum ignoring NaN; `None` if no finite-or-infinite value exists.
pub fn nanmax<I: IntoIterator<Item = f64>>(values: I) -> Option<f64> {
    values.into_iter().filter(|v| !v.is_nan()).reduce(f64::max)
}

/// [`nanmin`] and [`nanmax`] together, in one pass over the values.
///
/// ```
/// use pltrs::numeric::nan_extent;
/// assert_eq!(nan_extent([2.0, f64::NAN, -1.0, 5.0]), Some((-1.0, 5.0)));
/// assert_eq!(nan_extent([f64::NAN]), None);
/// ```
pub fn nan_extent<I: IntoIterator<Item = f64>>(values: I) -> Option<(f64, f64)> {
    let mut values = values.into_iter().filter(|v| !v.is_nan());
    let first = values.next()?;
    Some(values.fold((first, first), |(lo, hi), v| (lo.min(v), hi.max(v))))
}

#[cfg(test)]
mod tests {
    use pltrs_core::numeric::FUSES_PRODUCTS;

    use super::*;

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
        // numpy 2.5.4: its x86-64 wheel's values and its arm64 macOS
        // wheel's, which fuses `i * delta` into the sum.
        let (far, near) = if FUSES_PRODUCTS {
            (1.324_999_999_999_984_6, 0.700_000_000_000_000_2)
        } else {
            (1.324_999_999_999_985, 0.700_000_000_000_000_1)
        };
        assert_eq!(grid[173], far);
        assert_eq!(arange(0.1, 1.0, 0.2)[2], 0.500_000_000_000_000_1);
        assert_eq!(arange(0.1, 1.0, 0.2)[3], near);
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
