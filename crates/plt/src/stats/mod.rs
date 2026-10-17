//! Statistics for plotting: box/violin stats, kernel density estimates and
//! histogram binning (matplotlib.cbook, numpy.histogram).
//!
//! Every routine reproduces the numpy/matplotlib arithmetic it replaces, so
//! the numbers match matplotlib's to the last bit where numpy's are exact:
//!
//! * [`mean`], [`median`], [`var`], [`std`](fn@std), [`percentile`], [`quantile`] —
//!   numpy's reductions (pairwise summation, 'linear' interpolation).
//! * [`histogram`] / [`histogram_bin_edges`] — `np.histogram` with every
//!   `bins=` form ([`Bins`], [`BinEstimator`]), ranges, weights and
//!   densities; [`hist`] adds what `Axes.hist` does on top (shared edges,
//!   stacking, cumulative counts, bar and step geometry).
//! * [`histogram2d`] — `np.histogram2d` ([`AxisBins`], [`Histogram2d`]).
//! * [`boxplot_stats`] — `cbook.boxplot_stats` ([`BoxStats`]).
//! * [`violin_stats`] / [`GaussianKde`] — `cbook.violin_stats` and
//!   `mlab.GaussianKDE`.
//!
//! ```
//! use plt::stats::{BoxplotOptions, boxplot_stats, percentile};
//! assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 25.0), Ok(1.75));
//! let b = &boxplot_stats(&[[1.0, 2.0, 3.0, 4.0]], &BoxplotOptions::default()).unwrap()[0];
//! assert_eq!(b.iqr, 1.5);
//! ```
#![allow(
    clippy::float_cmp,
    reason = "numpy's algorithms branch on exact float comparisons; the ports keep them"
)]

mod boxplot;
mod error;
mod hist;
mod histogram;
mod histogram2d;
mod kde;
mod linalg;
mod summary;
mod violin;

pub use boxplot::{BoxStats, BoxplotOptions, Whis, box_stats, boxplot_stats};
pub use error::StatsError;
pub use hist::{
    Cumulative, Hist, HistAlign, HistBars, HistBottom, HistOptions, HistOrientation, HistPatches,
    HistType, StepOutline, hist,
};
pub use histogram::{
    BinEstimator, BinSample, Bins, Histogram, HistogramOptions, bin_width, histogram,
    histogram_bin_edges, outer_edges,
};
pub use histogram2d::{AxisBins, Histogram2d, Histogram2dOptions, histogram2d};
pub use kde::{BandwidthMethod, GaussianKde};
pub use summary::{mean, median, pairwise_sum, percentile, percentiles, quantile, std, var};
pub use violin::{ViolinOptions, ViolinStats, violin_stats, violin_stats_with};

pub(crate) use summary::pairwise;
