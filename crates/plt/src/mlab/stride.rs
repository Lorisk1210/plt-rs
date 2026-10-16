//! Overlapping segment views (`mlab._stride_windows`).

use super::MlabError;
use crate::numeric::Grid;

/// The overlapping length-`n` segments of `x` as the columns of an
/// `n × count` grid; segment `j` starts at `j·(n - noverlap)` and
/// `count = (len - noverlap) / (n - noverlap)`.
///
/// ```
/// let w = plt::mlab::stride_windows(&[0.0, 1.0, 2.0, 3.0, 4.0], 3, 1).unwrap();
/// assert_eq!(w.shape(), (3, 2));
/// assert_eq!(w.column(1), vec![2.0, 3.0, 4.0]);
/// ```
pub fn stride_windows(x: &[f64], n: usize, noverlap: usize) -> Result<Grid<f64>, MlabError> {
    if noverlap >= n {
        return Err(MlabError::InvalidOverlap { noverlap, nfft: n });
    }
    let step = n - noverlap;
    let count = x.len().saturating_sub(noverlap) / step;
    Ok(Grid::from_fn(n, count, |r, c| x[c * step + r]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments() {
        let x: Vec<f64> = (0..10).map(f64::from).collect();
        let w = stride_windows(&x, 4, 2).unwrap();
        assert_eq!(w.shape(), (4, 4));
        assert_eq!(w.column(3), vec![6.0, 7.0, 8.0, 9.0]);
        assert_eq!(stride_windows(&x, 10, 0).unwrap().shape(), (10, 1));
        assert_eq!(stride_windows(&x, 11, 0).unwrap().shape(), (11, 0));
        assert!(stride_windows(&x, 4, 4).is_err());
    }
}
