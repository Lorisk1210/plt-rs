//! The data side of `Axes.hist`: binning several datasets with shared
//! edges, stacking, cumulating and normalising, plus the bars or step
//! outlines that represent the result.
//!
//! [`hist`] performs every computation matplotlib's `Axes.hist` does
//! before it creates artists; `Axes::hist` draws [`HistPatches::Bars`]
//! with `bar`/`barh` and [`HistPatches::Steps`] as `Polygon` patches.

mod geometry;

pub use geometry::{HistAlign, HistBars, HistOrientation, HistPatches, HistType, StepOutline};

use super::histogram::{Bins, HistogramOptions, histogram, histogram_bin_edges};
use super::summary::pairwise_sum;
use super::{BinSample, StatsError};

/// Whether and in which direction `Axes.hist` accumulates counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cumulative {
    /// Plain counts.
    #[default]
    Off,
    /// Each bin holds the total of itself and all bins to its left.
    Forward,
    /// Each bin holds the total of itself and all bins to its right
    /// (matplotlib's negative `cumulative`).
    Reverse,
}

/// The baseline of the bars or steps (`bottom=`).
#[derive(Clone, Debug, PartialEq)]
pub enum HistBottom {
    /// The same baseline for every bin.
    Uniform(f64),
    /// One baseline per bin.
    PerBin(Vec<f64>),
}

impl Default for HistBottom {
    fn default() -> Self {
        Self::Uniform(0.0)
    }
}

impl HistBottom {
    fn at(&self, bin: usize) -> f64 {
        match self {
            Self::Uniform(v) => *v,
            Self::PerBin(values) => values.get(bin).copied().unwrap_or(f64::NAN),
        }
    }
}

/// Every option of `Axes.hist` that influences counts or geometry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HistOptions<'a> {
    /// Bins (`None`: ten equal-width bins; [`crate::axes::Axes::hist`]
    /// uses `hist.bins` instead).
    pub bins: Option<Bins>,
    /// Range of equal-width bins; defaults to the NaN-ignoring extent of
    /// all datasets.
    pub range: Option<(f64, f64)>,
    /// Normalise to densities.
    pub density: bool,
    /// One weight array per dataset.
    pub weights: Option<&'a [Vec<f64>]>,
    /// Accumulate counts along the bins.
    pub cumulative: Cumulative,
    /// Stack datasets on top of each other (implied by
    /// [`HistType::BarStacked`]).
    pub stacked: bool,
    /// Bars or steps.
    pub histtype: HistType,
    /// Bar/step placement relative to the bin edges.
    pub align: HistAlign,
    /// Vertical bars grow upwards; horizontal ones rightwards.
    pub orientation: HistOrientation,
    /// Relative bar width in `[0, 1]`; `None` for matplotlib's default.
    pub rwidth: Option<f64>,
    /// Baseline of the first dataset.
    pub bottom: HistBottom,
}

impl HistOptions<'_> {
    /// Whether datasets stack (explicitly or through
    /// [`HistType::BarStacked`]).
    pub fn is_stacked(&self) -> bool {
        self.stacked || self.histtype == HistType::BarStacked
    }
}

/// The result of [`hist`].
#[derive(Clone, Debug, PartialEq)]
pub struct Hist {
    /// Per dataset, the bar tops after stacking, normalisation and
    /// accumulation (matplotlib's `n`).
    pub tops: Vec<Vec<f64>>,
    /// The shared bin edges.
    pub edges: Vec<f64>,
    /// The patches representing each dataset.
    pub patches: HistPatches,
}

/// Computes everything `Axes.hist` derives from its datasets.
///
/// ```
/// use plt::stats::{Bins, HistOptions, hist};
/// let opts = HistOptions { bins: Some(Bins::Count(2)), ..HistOptions::default() };
/// let h = hist(&[vec![0.0, 1.0, 1.5, 2.0]], opts).unwrap();
/// assert_eq!(h.tops, vec![vec![1.0, 3.0]]);
/// assert_eq!(h.edges, vec![0.0, 1.0, 2.0]);
/// ```
pub fn hist<T: BinSample, D: AsRef<[T]>>(
    datasets: &[D],
    opts: HistOptions<'_>,
) -> Result<Hist, StatsError> {
    let (counts, edges) = bin_datasets(datasets, &opts)?;
    let tops = reduce(counts, &edges, &opts);
    let patches = geometry::patches(&tops, &edges, &opts);
    Ok(Hist {
        tops,
        edges,
        patches,
    })
}

/// Bins each dataset: with several non-empty datasets the edges are
/// computed once from all samples together, then every dataset is binned
/// with those explicit edges.
fn bin_datasets<T: BinSample, D: AsRef<[T]>>(
    datasets: &[D],
    opts: &HistOptions<'_>,
) -> Result<(Vec<Vec<f64>>, Vec<f64>), StatsError> {
    let weights = dataset_weights(datasets, opts.weights)?;
    let input_empty = datasets.iter().all(|d| d.as_ref().is_empty());
    let range = opts.range.or_else(|| nan_extent(datasets));
    let stacked = opts.is_stacked();
    let (bins, range) = if !input_empty && datasets.len() > 1 {
        let all: Vec<T> = datasets
            .iter()
            .flat_map(|d| d.as_ref().iter().copied())
            .collect();
        let all_weights: Option<Vec<f64>> =
            opts.weights.map(|w| w.iter().flatten().copied().collect());
        let edges = histogram_bin_edges(
            &all,
            HistogramOptions {
                bins: opts.bins.clone().unwrap_or_default(),
                range,
                density: false,
                weights: all_weights.as_deref(),
            },
        )?;
        (Bins::Edges(edges), None)
    } else {
        (opts.bins.clone().unwrap_or_default(), range)
    };
    let mut edges = Vec::new();
    let counts = datasets
        .iter()
        .zip(weights)
        .map(|(data, weights)| {
            let h = histogram(
                data.as_ref(),
                HistogramOptions {
                    bins: bins.clone(),
                    range,
                    density: opts.density && !stacked,
                    weights,
                },
            )?;
            edges = h.edges;
            Ok(h.counts)
        })
        .collect::<Result<Vec<_>, StatsError>>()?;
    if counts.is_empty() {
        edges = histogram_bin_edges::<f64>(
            &[],
            HistogramOptions {
                bins,
                range,
                ..HistogramOptions::default()
            },
        )?;
    }
    Ok((counts, edges))
}

/// Pairs each dataset with its weights, checking the shapes.
fn dataset_weights<'w, T, D: AsRef<[T]>>(
    datasets: &[D],
    weights: Option<&'w [Vec<f64>]>,
) -> Result<Vec<Option<&'w [f64]>>, StatsError> {
    let Some(weights) = weights else {
        return Ok(vec![None; datasets.len()]);
    };
    if weights.len() != datasets.len() {
        return Err(StatsError::CountMismatch {
            what: "weights",
            datasets: datasets.len(),
            actual: weights.len(),
        });
    }
    datasets
        .iter()
        .zip(weights)
        .map(|(d, w)| {
            if d.as_ref().len() == w.len() {
                Ok(Some(w.as_slice()))
            } else {
                Err(StatsError::WeightsMismatch {
                    expected: d.as_ref().len(),
                    actual: w.len(),
                })
            }
        })
        .collect()
}

/// The NaN-ignoring minimum and maximum over all datasets, if any finite
/// or infinite sample exists.
fn nan_extent<T: BinSample, D: AsRef<[T]>>(datasets: &[D]) -> Option<(f64, f64)> {
    let values = || {
        datasets
            .iter()
            .flat_map(|d| d.as_ref().iter().map(|v| v.to_f64()))
    };
    let lo = crate::numeric::nanmin(values())?;
    let hi = crate::numeric::nanmax(values())?;
    (lo <= hi).then_some((lo, hi))
}

/// Stacking, stacked density and accumulation, in matplotlib's order.
fn reduce(mut tops: Vec<Vec<f64>>, edges: &[f64], opts: &HistOptions<'_>) -> Vec<Vec<f64>> {
    let widths: Vec<f64> = edges.windows(2).map(|w| w[1] - w[0]).collect();
    if opts.is_stacked() {
        for i in 1..tops.len() {
            let (done, rest) = tops.split_at_mut(i);
            for (top, below) in rest[0].iter_mut().zip(&done[i - 1]) {
                *top += below;
            }
        }
        if opts.density
            && let Some(total) = tops.last().map(|last| pairwise_sum(last))
        {
            for top in &mut tops {
                for (v, w) in top.iter_mut().zip(&widths) {
                    *v = *v / w / total;
                }
            }
        }
    }
    if opts.cumulative != Cumulative::Off {
        for top in &mut tops {
            if opts.density {
                for (v, w) in top.iter_mut().zip(&widths) {
                    *v *= w;
                }
            }
            accumulate(top, opts.cumulative == Cumulative::Reverse);
        }
    }
    tops
}

fn accumulate(values: &mut [f64], reverse: bool) {
    let mut running = 0.0;
    let mut step = |v: &mut f64| {
        running += *v;
        *v = running;
    };
    if reverse {
        values.iter_mut().rev().for_each(&mut step);
    } else {
        values.iter_mut().for_each(&mut step);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(bins: usize) -> HistOptions<'static> {
        HistOptions {
            bins: Some(Bins::Count(bins)),
            ..HistOptions::default()
        }
    }

    #[test]
    fn several_datasets_share_edges() {
        let h = hist(&[vec![0.0, 1.0], vec![3.0, 4.0]], opts(2)).unwrap();
        assert_eq!(h.edges, vec![0.0, 2.0, 4.0]);
        assert_eq!(h.tops, vec![vec![2.0, 0.0], vec![0.0, 2.0]]);
    }

    #[test]
    fn stacking_and_cumulation() {
        let stacked = HistOptions {
            stacked: true,
            cumulative: Cumulative::Reverse,
            ..opts(2)
        };
        let h = hist(&[vec![0.0, 1.0, 4.0], vec![3.0, 4.0]], stacked).unwrap();
        assert_eq!(h.tops, vec![vec![3.0, 1.0], vec![5.0, 3.0]]);
        let density = HistOptions {
            stacked: true,
            density: true,
            ..opts(2)
        };
        let h = hist(&[vec![0.0, 4.0], vec![4.0, 4.0]], density).unwrap();
        assert_eq!(h.tops, vec![vec![0.125, 0.125], vec![0.125, 0.375]]);
    }

    #[test]
    fn empty_and_nan_inputs() {
        let h = hist::<f64, _>(&[Vec::new()], opts(4)).unwrap();
        assert_eq!(h.edges, vec![0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_eq!(h.tops, vec![vec![0.0; 4]]);
        let h = hist(&[vec![1.0, f64::NAN, 3.0]], opts(2)).unwrap();
        assert_eq!(h.tops, vec![vec![1.0, 1.0]]);
        assert!(hist(&[vec![f64::NAN]], opts(2)).is_err());
    }

    #[test]
    fn weight_shapes_are_checked() {
        let weights = vec![vec![1.0]];
        let o = HistOptions {
            weights: Some(&weights),
            ..opts(2)
        };
        assert!(matches!(
            hist(&[vec![1.0, 2.0]], o.clone()),
            Err(StatsError::WeightsMismatch { .. })
        ));
        assert!(matches!(
            hist(&[vec![1.0], vec![2.0]], o),
            Err(StatsError::CountMismatch { .. })
        ));
    }
}
