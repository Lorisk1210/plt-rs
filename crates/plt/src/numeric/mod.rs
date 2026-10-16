//! numpy-style numeric helpers: ranges, grids and NaN-aware reductions.
//!
//! The range constructors reproduce numpy's floating-point arithmetic
//! exactly (`linspace` computes `i * step + start` and pins the endpoint),
//! because matplotlib's results depend on those precise values.

mod grid;

pub use grid::{Grid, GridError};

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
/// Returns an empty vector for a zero or wrongly-signed step.
pub fn arange(start: f64, stop: f64, step: f64) -> Vec<f64> {
    if step == 0.0 || !step.is_finite() {
        return Vec::new();
    }
    let count = ((stop - start) / step).ceil();
    if !count.is_finite() || count <= 0.0 {
        return Vec::new();
    }
    (0..count as usize)
        .map(|i| start + i as f64 * step)
        .collect()
}

/// `num` samples spaced evenly on a log scale: `base ** linspace(start, stop)`.
pub fn logspace(start: f64, stop: f64, num: usize, base: f64) -> Vec<f64> {
    linspace(start, stop, num)
        .into_iter()
        .map(|e| base.powf(e))
        .collect()
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
