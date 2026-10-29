//! The demonstration surface of matplotlib's 3-D examples
//! (`mpl_toolkits.mplot3d.axes3d.get_test_data`).

use crate::numeric::{Grid, arange, meshgrid};

/// `(X, Y, Z)` grids of two overlapping Gaussian bumps, one subtracted
/// from the other, sampled every `delta` over `[-30, 30)` in x and y
/// (matplotlib's `get_test_data`, with the same arithmetic).
///
/// ```
/// let (x, y, z) = pltrs::mplot3d::test_data(0.5);
/// assert_eq!(x.shape(), (12, 12));
/// assert_eq!((x[(0, 0)], y[(0, 0)]), (-30.0, -30.0));
/// assert!(z.as_slice().iter().all(|v| v.is_finite()));
/// ```
pub fn test_data(delta: f64) -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let axis = arange(-3.0, 3.0, delta);
    let (x, y) = meshgrid(&axis, &axis);
    let tau = 2.0 * std::f64::consts::PI;
    let z = Grid::from_fn(x.rows(), x.cols(), |r, c| {
        let (x, y) = (x[(r, c)], y[(r, c)]);
        let z1 = (-(x * x + y * y) / 2.0).exp() / tau;
        let (dx, dy) = ((x - 1.0) / 1.5, (y - 1.0) / 0.5);
        let z2 = (-(dx * dx + dy * dy) / 2.0).exp() / (tau * 0.5 * 1.5);
        (z2 - z1) * 500.0
    });
    (x.map(|v| v * 10.0), y.map(|v| v * 10.0), z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_matplotlib() {
        // Values from matplotlib 3.11's get_test_data(0.05).
        let (x, _, z) = test_data(0.05);
        let (low, high) = z
            .as_slice()
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
                (lo.min(v), hi.max(v))
            });
        assert_eq!(z.shape(), (120, 120));
        assert!((low + 73.396_209_716_016_81).abs() < 1e-12, "{low}");
        assert!((high - 84.981_931_331_245_55).abs() < 1e-12, "{high}");
        assert!((z[(37, 81)] + 23.660_291_961_667_095).abs() < 1e-12);
        assert_eq!(x[(0, 119)], 29.499_999_999_999_787);
    }
}
