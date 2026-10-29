//! numpy-compatible histograms (`np.histogram`, `np.histogram_bin_edges`).
//!
//! Bin edges, bin assignment and the automatic bin-width estimators follow
//! numpy's floating-point arithmetic step by step, so that counts and edges
//! match numpy exactly, including the closed last bin and the integer-data
//! rules (integer samples never get bins narrower than 1).

mod counting;
mod estimators;
mod sample;

use std::str::FromStr;

pub use estimators::bin_width;
pub use sample::BinSample;

use super::StatsError;
use crate::numeric::linspace;

/// Largest number of equal-width bins accepted (of a 2-D histogram: of
/// bins in all), whether counted or estimated; more are refused with
/// [`StatsError::TooManyBins`]. numpy allocates whatever is asked, which
/// for a narrower estimate or a larger count exhausts memory long before
/// producing a useful plot.
pub const MAX_EQUAL_BINS: usize = 1 << 24;

/// numpy's automatic bin-width estimators (`bins="auto"`, `"fd"`, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinEstimator {
    /// The smaller of the Sturges and Freedman–Diaconis widths (the latter
    /// at least half the square-root width).
    Auto,
    /// Freedman–Diaconis: `2·IQR·n^(-1/3)`.
    Fd,
    /// Doane's skewness-corrected Sturges rule.
    Doane,
    /// Scott's normal-reference rule: `(24·√π / n)^(1/3)·σ`.
    Scott,
    /// Stone's leave-one-out cross-validation estimate.
    Stone,
    /// Rice: `ptp / (2·n^(1/3))`.
    Rice,
    /// Sturges: `ptp / (log2(n) + 1)`.
    Sturges,
    /// Square root: `ptp / √n`.
    Sqrt,
}

impl BinEstimator {
    /// Every estimator, in numpy's documentation order.
    pub const ALL: [Self; 8] = [
        Self::Auto,
        Self::Fd,
        Self::Doane,
        Self::Scott,
        Self::Stone,
        Self::Rice,
        Self::Sturges,
        Self::Sqrt,
    ];

    /// numpy's name for the estimator.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Fd => "fd",
            Self::Doane => "doane",
            Self::Scott => "scott",
            Self::Stone => "stone",
            Self::Rice => "rice",
            Self::Sturges => "sturges",
            Self::Sqrt => "sqrt",
        }
    }
}

impl FromStr for BinEstimator {
    type Err = StatsError;

    fn from_str(name: &str) -> Result<Self, StatsError> {
        Self::ALL
            .into_iter()
            .find(|e| e.name() == name)
            .ok_or_else(|| StatsError::UnknownEstimator(name.to_owned()))
    }
}

/// How the bins of a histogram are chosen (numpy's `bins=` argument).
#[derive(Clone, Debug, PartialEq)]
pub enum Bins {
    /// This many equal-width bins over the range.
    Count(usize),
    /// Equal-width bins whose width an estimator derives from the data.
    Estimator(BinEstimator),
    /// Explicit, monotonically increasing bin edges.
    Edges(Vec<f64>),
}

impl Default for Bins {
    /// Ten equal-width bins, numpy's and matplotlib's default.
    fn default() -> Self {
        Self::Count(10)
    }
}

/// The optional arguments of [`histogram`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HistogramOptions<'a> {
    /// The bins (default: ten equal-width bins).
    pub bins: Bins,
    /// Lower and upper range of equal-width bins; samples outside are
    /// ignored. Defaults to the data's minimum and maximum.
    pub range: Option<(f64, f64)>,
    /// Normalise to a probability density (integral one over the range).
    pub density: bool,
    /// One weight per sample; each sample contributes its weight instead
    /// of one.
    pub weights: Option<&'a [f64]>,
}

/// Counts per bin and the bin edges (`counts.len() + 1` of them).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Histogram {
    /// Number (or summed weight, or density) of samples per bin.
    pub counts: Vec<f64>,
    /// The bin edges; every bin is half-open except the last, which also
    /// contains its upper edge.
    pub edges: Vec<f64>,
}

/// Computes the histogram of `data` exactly as `np.histogram` does.
///
/// ```
/// use pltrs::stats::{Bins, HistogramOptions, histogram};
/// let opts = HistogramOptions { bins: Bins::Count(5), ..HistogramOptions::default() };
/// let h = histogram(&[0, 1, 2, 3, 4, 5], opts).unwrap();
/// assert_eq!(h.counts, vec![1.0, 1.0, 1.0, 1.0, 2.0]); // the last bin is closed
/// assert_eq!(h.edges, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
/// ```
///
/// # Errors
///
/// When the weights do not match the data, the bins are invalid, or
/// equal-width bins would be more than [`MAX_EQUAL_BINS`] or too narrow
/// to tell apart.
pub fn histogram<T: BinSample>(
    data: &[T],
    opts: HistogramOptions<'_>,
) -> Result<Histogram, StatsError> {
    check_weights(data.len(), opts.weights)?;
    let plan = plan_edges(data, &opts.bins, opts.range, opts.weights.is_some())?;
    let counts = match plan.uniform {
        Some(uniform) => counting::uniform(data, opts.weights, &plan.edges, uniform),
        None => counting::by_edges(data, opts.weights, &plan.edges),
    };
    let counts = if opts.density {
        counting::density(&counts, &plan.edges)
    } else {
        counts
    };
    Ok(Histogram {
        counts,
        edges: plan.edges,
    })
}

/// The bin edges [`histogram`] would use (`np.histogram_bin_edges`); the
/// `density` option is irrelevant here.
///
/// # Errors
///
/// As [`histogram`].
pub fn histogram_bin_edges<T: BinSample>(
    data: &[T],
    opts: HistogramOptions<'_>,
) -> Result<Vec<f64>, StatsError> {
    check_weights(data.len(), opts.weights)?;
    Ok(plan_edges(data, &opts.bins, opts.range, opts.weights.is_some())?.edges)
}

/// The outer edges of equal-width bins: the given `range` (validated) or the
/// data's extent, widened by ±0.5 when degenerate (numpy
/// `_get_outer_edges`). Empty data spans `(0, 1)`.
pub fn outer_edges<T: BinSample>(
    data: &[T],
    range: Option<(f64, f64)>,
) -> Result<(f64, f64), StatsError> {
    let (lo, hi) = match range {
        Some((lo, hi)) => {
            if lo > hi {
                return Err(StatsError::InvertedRange { lo, hi });
            }
            if !(lo.is_finite() && hi.is_finite()) {
                return Err(StatsError::NonFiniteRange {
                    lo,
                    hi,
                    autodetected: false,
                });
            }
            (lo, hi)
        }
        None if data.is_empty() => (0.0, 1.0),
        None => {
            let (lo, hi) = extent(data);
            if !(lo.is_finite() && hi.is_finite()) {
                return Err(StatsError::NonFiniteRange {
                    lo,
                    hi,
                    autodetected: true,
                });
            }
            (lo, hi)
        }
    };
    Ok(if lo == hi {
        (lo - 0.5, hi + 0.5)
    } else {
        (lo, hi)
    })
}

/// Minimum and maximum; NaN if any sample is NaN (numpy `min`/`max`).
fn extent<T: BinSample>(data: &[T]) -> (f64, f64) {
    data.iter()
        .map(|v| v.to_f64())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            if v.is_nan() || lo.is_nan() {
                (f64::NAN, f64::NAN)
            } else {
                (lo.min(v), hi.max(v))
            }
        })
}

fn check_weights(len: usize, weights: Option<&[f64]>) -> Result<(), StatsError> {
    match weights {
        Some(w) if w.len() != len => Err(StatsError::WeightsMismatch {
            expected: len,
            actual: w.len(),
        }),
        _ => Ok(()),
    }
}

/// Equal-width bins: `count` bins spanning `first..=last`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Uniform {
    first: f64,
    last: f64,
    count: usize,
}

/// The edges and, for equal-width bins, their parameters (which enable
/// numpy's fast index computation).
struct EdgePlan {
    edges: Vec<f64>,
    uniform: Option<Uniform>,
}

/// numpy `_get_bin_edges`.
fn plan_edges<T: BinSample>(
    data: &[T],
    bins: &Bins,
    range: Option<(f64, f64)>,
    weighted: bool,
) -> Result<EdgePlan, StatsError> {
    let (first, last, count) = match bins {
        Bins::Edges(edges) => {
            if edges.windows(2).any(|w| w[0] > w[1]) {
                return Err(StatsError::NonMonotonicEdges);
            }
            return Ok(EdgePlan {
                edges: edges.clone(),
                uniform: None,
            });
        }
        Bins::Count(0) => return Err(StatsError::ZeroBins),
        Bins::Count(n) => {
            let (first, last) = outer_edges(data, range)?;
            (first, last, *n)
        }
        Bins::Estimator(estimator) => {
            if weighted {
                return Err(StatsError::WeightedEstimator);
            }
            let (first, last) = outer_edges(data, range)?;
            let count = estimated_bin_count(data, *estimator, range, (first, last));
            (first, last, count)
        }
    };
    if count > MAX_EQUAL_BINS {
        return Err(StatsError::TooManyBins(count));
    }
    let edges = linspace(first, last, count + 1);
    if edges.windows(2).any(|w| w[0] >= w[1]) {
        return Err(StatsError::TooManyBins(count));
    }
    Ok(EdgePlan {
        edges,
        uniform: Some(Uniform { first, last, count }),
    })
}

/// The number of equal-width bins an estimator asks for over `bounds`.
fn estimated_bin_count<T: BinSample>(
    data: &[T],
    estimator: BinEstimator,
    range: Option<(f64, f64)>,
    (first, last): (f64, f64),
) -> usize {
    let kept: Vec<T> = match range {
        Some(_) => data
            .iter()
            .copied()
            .filter(|v| {
                let v = v.to_f64();
                v >= first && v <= last
            })
            .collect(),
        None => data.to_vec(),
    };
    if kept.is_empty() {
        return 1;
    }
    let mut width = bin_width(&kept, estimator, (first, last));
    if width == 0.0 || width.is_nan() {
        return 1;
    }
    if T::INTEGER && width < 1.0 {
        width = 1.0;
    }
    let bins = ((last - first) / width).ceil();
    if bins >= MAX_EQUAL_BINS as f64 {
        MAX_EQUAL_BINS + 1
    } else {
        bins as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edges_of<T: BinSample>(data: &[T], bins: Bins) -> Result<Vec<f64>, StatsError> {
        histogram_bin_edges(
            data,
            HistogramOptions {
                bins,
                ..HistogramOptions::default()
            },
        )
    }

    #[test]
    fn equal_width_bins_follow_linspace() {
        assert_eq!(
            edges_of(&[0.1, 0.7], Bins::Count(3)).unwrap(),
            linspace(0.1, 0.7, 4)
        );
        assert_eq!(
            edges_of::<f64>(&[], Bins::Count(2)).unwrap(),
            vec![0.0, 0.5, 1.0]
        );
        assert_eq!(edges_of(&[7], Bins::Count(2)).unwrap(), vec![6.5, 7.0, 7.5]);
    }

    #[test]
    fn errors_match_numpy() {
        assert_eq!(edges_of(&[1.0], Bins::Count(0)), Err(StatsError::ZeroBins));
        assert_eq!(
            edges_of(&[1.0], Bins::Edges(vec![3.0, 1.0])),
            Err(StatsError::NonMonotonicEdges)
        );
        assert!(matches!(
            edges_of(&[1.0, f64::NAN], Bins::Count(3)),
            Err(StatsError::NonFiniteRange {
                autodetected: true,
                ..
            })
        ));
        assert_eq!(
            edges_of(&[1.0, 1.0 + f64::EPSILON], Bins::Count(10)),
            Err(StatsError::TooManyBins(10))
        );
        let opts = HistogramOptions {
            range: Some((5.0, 1.0)),
            ..HistogramOptions::default()
        };
        assert_eq!(
            histogram(&[1.0], opts),
            Err(StatsError::InvertedRange { lo: 5.0, hi: 1.0 })
        );
        let weights = [1.0; 2];
        let opts = HistogramOptions {
            bins: Bins::Estimator(BinEstimator::Auto),
            weights: Some(&weights),
            ..HistogramOptions::default()
        };
        assert_eq!(
            histogram(&[1.0, 2.0], opts.clone()),
            Err(StatsError::WeightedEstimator)
        );
        assert_eq!(
            histogram(&[1.0], opts),
            Err(StatsError::WeightsMismatch {
                expected: 1,
                actual: 2
            })
        );
        assert_eq!(
            "bogus".parse::<BinEstimator>(),
            Err(StatsError::UnknownEstimator("bogus".into()))
        );
    }

    #[test]
    fn estimators_choose_counts() {
        let data: Vec<f64> = (0..100).map(|i| f64::from(i) * 0.1).collect();
        // sqrt: ptp / 10 -> 10 bins.
        assert_eq!(
            edges_of(&data, Bins::Estimator(BinEstimator::Sqrt))
                .unwrap()
                .len(),
            11
        );
        // Integer data never gets bins narrower than one.
        let ints = [0, 1, 1, 1, 9];
        let edges = edges_of(&ints, Bins::Estimator(BinEstimator::Stone)).unwrap();
        assert_eq!(edges.len(), 10);
        let floats = [0.0, 1.0, 1.0, 1.0, 9.0];
        let edges = edges_of(&floats, Bins::Estimator(BinEstimator::Stone)).unwrap();
        assert!(edges.len() > 10);
        assert_eq!(
            edges_of::<f64>(&[], Bins::Estimator(BinEstimator::Fd)).unwrap(),
            vec![0.0, 1.0]
        );
    }

    #[test]
    fn density_and_weights() {
        let opts = HistogramOptions {
            bins: Bins::Edges(vec![0.0, 1.0, 3.0]),
            density: true,
            ..HistogramOptions::default()
        };
        let h = histogram(&[0.5, 1.5, 2.5, 3.0], opts).unwrap();
        assert_eq!(h.counts, vec![0.25, 0.375]);
        let weights = [2.0, 0.5, 1.0];
        let opts = HistogramOptions {
            bins: Bins::Count(2),
            weights: Some(&weights),
            ..HistogramOptions::default()
        };
        assert_eq!(
            histogram(&[0.0, 1.0, 2.0], opts).unwrap().counts,
            vec![2.0, 1.5]
        );
    }
}
