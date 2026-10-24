//! Two-dimensional histograms (`np.histogram2d`, i.e. `np.histogramdd`
//! with two dimensions).
//!
//! Unlike [`histogram`](super::histogram), numpy assigns every sample with
//! a binary search over the edges (`searchsorted(side="right")`), counts
//! samples on the last edge in the last bin and drops everything else
//! outside the edges.

use super::{StatsError, outer_edges, pairwise_sum};
use crate::numeric::{Grid, linspace};

/// The bins along one dimension of a multi-dimensional histogram (numpy
/// accepts an integer or explicit edges there, no estimator).
#[derive(Clone, Debug, PartialEq)]
pub enum AxisBins {
    /// This many equal-width bins over the range.
    Count(usize),
    /// Explicit, monotonically increasing bin edges.
    Edges(Vec<f64>),
}

impl Default for AxisBins {
    /// Ten equal-width bins, numpy's and matplotlib's default.
    fn default() -> Self {
        Self::Count(10)
    }
}

impl From<usize> for AxisBins {
    fn from(count: usize) -> Self {
        Self::Count(count)
    }
}

impl From<Vec<f64>> for AxisBins {
    fn from(edges: Vec<f64>) -> Self {
        Self::Edges(edges)
    }
}

/// The optional arguments of [`histogram2d`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Histogram2dOptions<'a> {
    /// The x and y bins (default: ten equal-width bins each).
    pub bins: [AxisBins; 2],
    /// The x and y range of equal-width bins (default: the data's extent).
    pub range: [Option<(f64, f64)>; 2],
    /// Normalise to a probability density (integral one over the bins).
    pub density: bool,
    /// One weight per sample.
    pub weights: Option<&'a [f64]>,
}

/// Counts per cell and the bin edges of both dimensions.
#[derive(Clone, Debug, PartialEq)]
pub struct Histogram2d {
    /// Counts (or summed weights, or densities) indexed `(x bin, y bin)`.
    pub counts: Grid<f64>,
    /// The x bin edges.
    pub xedges: Vec<f64>,
    /// The y bin edges.
    pub yedges: Vec<f64>,
}

/// Computes the histogram of the points `(x[i], y[i])` exactly as
/// `np.histogram2d` does.
///
/// ```
/// use plt::stats::{AxisBins, Histogram2dOptions, histogram2d};
/// let opts = Histogram2dOptions {
///     bins: [AxisBins::Count(2), AxisBins::Edges(vec![0.0, 1.0])],
///     ..Histogram2dOptions::default()
/// };
/// let h = histogram2d(&[0.0, 1.0, 2.0], &[0.0, 1.0, 5.0], opts)?;
/// assert_eq!(h.xedges, vec![0.0, 1.0, 2.0]);
/// assert_eq!(h.counts.as_slice(), &[1.0, 1.0]); // y = 5 lies outside
/// # Ok::<(), plt::stats::StatsError>(())
/// ```
///
/// # Errors
///
/// When x and y (or the weights) differ in length, a bin count is zero,
/// edges decrease, or a range is inverted or not finite.
pub fn histogram2d(
    x: &[f64],
    y: &[f64],
    opts: Histogram2dOptions<'_>,
) -> Result<Histogram2d, StatsError> {
    if x.len() != y.len() {
        return Err(StatsError::SampleLengthMismatch {
            x: x.len(),
            y: y.len(),
        });
    }
    if let Some(weights) = opts.weights.filter(|w| w.len() != x.len()) {
        return Err(StatsError::WeightsMismatch {
            expected: x.len(),
            actual: weights.len(),
        });
    }
    let xedges = axis_edges(x, &opts.bins[0], opts.range[0])?;
    let yedges = axis_edges(y, &opts.bins[1], opts.range[1])?;
    let (nx, ny) = (xedges.len() - 1, yedges.len() - 1);
    let mut counts = Grid::filled(nx, ny, 0.0);
    for (i, (&xi, &yi)) in x.iter().zip(y).enumerate() {
        if let (Some(bx), Some(by)) = (bin_index(&xedges, xi), bin_index(&yedges, yi)) {
            counts.as_mut_slice()[bx * ny + by] += opts.weights.map_or(1.0, |w| w[i]);
        }
    }
    if opts.density {
        let total = pairwise_sum(counts.as_slice());
        counts = Grid::from_fn(nx, ny, |r, c| {
            counts[(r, c)] / (xedges[r + 1] - xedges[r]) / (yedges[c + 1] - yedges[c]) / total
        });
    }
    Ok(Histogram2d {
        counts,
        xedges,
        yedges,
    })
}

/// The edges along one dimension (numpy `histogramdd`'s edge loop).
fn axis_edges(
    sample: &[f64],
    bins: &AxisBins,
    range: Option<(f64, f64)>,
) -> Result<Vec<f64>, StatsError> {
    match bins {
        AxisBins::Count(0) => Err(StatsError::ZeroBins),
        AxisBins::Count(n) => {
            let (first, last) = outer_edges(sample, range)?;
            Ok(linspace(first, last, n + 1))
        }
        AxisBins::Edges(edges) if edges.len() < 2 => Err(StatsError::ZeroBins),
        AxisBins::Edges(edges) if edges.windows(2).any(|w| w[0] > w[1]) => {
            Err(StatsError::NonMonotonicEdges)
        }
        AxisBins::Edges(edges) => Ok(edges.clone()),
    }
}

/// The bin of `value`, if inside the edges: `searchsorted(side="right")`
/// (NaN sorts last), with the last edge moved into the last bin.
fn bin_index(edges: &[f64], value: f64) -> Option<usize> {
    let last = edges.len() - 1;
    let found = if value.is_nan() {
        edges.len()
    } else {
        edges.partition_point(|&edge| edge <= value)
    };
    let found = if value == edges[last] {
        found - 1
    } else {
        found
    };
    (1..=last).contains(&found).then(|| found - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(bins: [AxisBins; 2]) -> Histogram2dOptions<'static> {
        Histogram2dOptions {
            bins,
            ..Histogram2dOptions::default()
        }
    }

    #[test]
    fn counts_follow_numpy_bin_assignment() {
        let x = [0.0, 0.5, 1.0, 1.5, 2.0, f64::NAN, -1.0];
        let y = [0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0];
        let opts = options([
            AxisBins::Edges(vec![0.0, 1.0, 2.0]),
            AxisBins::Edges(vec![0.0, 0.5, 1.0]),
        ]);
        let h = histogram2d(&x, &y, opts).unwrap();
        // Edges are left-closed except the last, which also takes its edge.
        assert_eq!(h.counts.as_slice(), &[2.0, 0.0, 0.0, 3.0]);
    }

    #[test]
    fn equal_width_bins_span_the_data_or_the_range() {
        let h = histogram2d(&[1.0, 3.0], &[2.0, 2.0], options([2.into(), 1.into()])).unwrap();
        assert_eq!(h.xedges, vec![1.0, 2.0, 3.0]);
        assert_eq!(h.yedges, vec![1.5, 2.5]);
        assert_eq!(h.counts.as_slice(), &[1.0, 1.0]);
        let ranged = Histogram2dOptions {
            range: [Some((0.0, 4.0)), None],
            ..options([4.into(), 1.into()])
        };
        let h = histogram2d(&[1.0, 3.0], &[2.0, 2.0], ranged).unwrap();
        assert_eq!(h.xedges, vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        assert_eq!(h.counts.as_slice(), &[0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn density_and_weights() {
        let weights = [1.0, 3.0];
        let weighted = Histogram2dOptions {
            weights: Some(&weights),
            ..options([2.into(), 1.into()])
        };
        let h = histogram2d(&[0.0, 2.0], &[0.0, 1.0], weighted.clone()).unwrap();
        assert_eq!(h.counts.as_slice(), &[1.0, 3.0]);
        let density = Histogram2dOptions {
            density: true,
            ..weighted
        };
        let h = histogram2d(&[0.0, 2.0], &[0.0, 1.0], density).unwrap();
        assert_eq!(h.counts.as_slice(), &[0.25, 0.75]);
    }

    #[test]
    fn invalid_input_is_rejected() {
        let default = Histogram2dOptions::default();
        assert_eq!(
            histogram2d(&[1.0], &[], default.clone()),
            Err(StatsError::SampleLengthMismatch { x: 1, y: 0 })
        );
        assert_eq!(
            histogram2d(&[1.0], &[1.0], options([0.into(), 1.into()])),
            Err(StatsError::ZeroBins)
        );
        let decreasing = options([vec![1.0, 0.0].into(), 1.into()]);
        assert_eq!(
            histogram2d(&[1.0], &[1.0], decreasing),
            Err(StatsError::NonMonotonicEdges)
        );
        assert!(matches!(
            histogram2d(&[f64::NAN], &[1.0], default),
            Err(StatsError::NonFiniteRange { .. })
        ));
        let weights = [1.0, 2.0];
        let mismatched = Histogram2dOptions {
            weights: Some(&weights),
            ..Histogram2dOptions::default()
        };
        assert_eq!(
            histogram2d(&[1.0], &[1.0], mismatched),
            Err(StatsError::WeightsMismatch {
                expected: 1,
                actual: 2
            })
        );
    }
}
