//! The error type of the statistics module.

use std::fmt;

use pltrs_core::pyfmt;

/// Why a statistical computation rejected its input.
#[derive(Clone, Debug, PartialEq)]
pub enum StatsError {
    /// A percentile outside `[0, 100]`.
    PercentileOutOfRange(f64),
    /// A quantile outside `[0, 1]`.
    QuantileOutOfRange(f64),
    /// A histogram with zero bins.
    ZeroBins,
    /// Explicit bin edges that decrease somewhere.
    NonMonotonicEdges,
    /// So many equal-width bins (this many) that consecutive edges
    /// coincide, or more than [`MAX_EQUAL_BINS`](super::MAX_EQUAL_BINS).
    TooManyBins(usize),
    /// A histogram range whose lower bound exceeds the upper one.
    InvertedRange {
        /// Lower bound.
        lo: f64,
        /// Upper bound.
        hi: f64,
    },
    /// A histogram range that is not finite; `autodetected` tells whether it
    /// came from the data (NaN or infinite samples) or was supplied.
    NonFiniteRange {
        /// Lower bound.
        lo: f64,
        /// Upper bound.
        hi: f64,
        /// `true` when the range was derived from the data.
        autodetected: bool,
    },
    /// Weights whose length differs from the data's.
    WeightsMismatch {
        /// Number of samples.
        expected: usize,
        /// Number of weights.
        actual: usize,
    },
    /// The coordinates of two-dimensional samples differ in length.
    SampleLengthMismatch {
        /// Number of x values.
        x: usize,
        /// Number of y values.
        y: usize,
    },
    /// Weighted data cannot use an automatic bin-width estimator.
    WeightedEstimator,
    /// A bin estimator name numpy does not know.
    UnknownEstimator(String),
    /// Labels (or per-dataset parameters) whose count differs from the
    /// number of datasets.
    CountMismatch {
        /// What was miscounted (`"labels"`, `"quantiles"`, `"weights"`).
        what: &'static str,
        /// Number of datasets.
        datasets: usize,
        /// Number supplied.
        actual: usize,
    },
    /// A kernel density estimate needs more than one sample value.
    TooFewSamples,
    /// More evaluation points (this many) than
    /// [`MAX_VIOLIN_POINTS`](super::MAX_VIOLIN_POINTS).
    TooManyPoints(usize),
    /// The sample covariance matrix cannot be inverted.
    SingularCovariance,
    /// Evaluation points whose dimension differs from the dataset's.
    DimensionMismatch {
        /// Dataset dimension.
        expected: usize,
        /// Points dimension.
        actual: usize,
    },
    /// A bootstrap without resamples.
    NoResamples,
    /// A bootstrap drawing more values (this many) than
    /// [`MAX_BOOTSTRAP_DRAWS`](super::MAX_BOOTSTRAP_DRAWS).
    TooManyBootstrapDraws(usize),
}

impl fmt::Display for StatsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PercentileOutOfRange(p) => {
                write!(f, "percentile {p} is outside the range [0, 100]")
            }
            Self::QuantileOutOfRange(q) => write!(f, "quantile {q} is outside the range [0, 1]"),
            Self::ZeroBins => f.write_str("the number of bins must be positive"),
            Self::NonMonotonicEdges => f.write_str("bin edges must increase monotonically"),
            Self::TooManyBins(n) => write!(
                f,
                "too many bins for data range: cannot create {n} finite-sized bins"
            ),
            Self::InvertedRange { lo, hi } => {
                write!(
                    f,
                    "range maximum {hi} must not be smaller than its minimum {lo}"
                )
            }
            Self::NonFiniteRange {
                lo,
                hi,
                autodetected,
            } => {
                let origin = if *autodetected {
                    "autodetected"
                } else {
                    "supplied"
                };
                // numpy's message, the bounds as Python writes floats.
                write!(
                    f,
                    "{origin} range of [{}, {}] is not finite",
                    pyfmt::repr(*lo),
                    pyfmt::repr(*hi)
                )
            }
            Self::WeightsMismatch { expected, actual } => {
                write!(f, "{actual} weights given for {expected} samples")
            }
            // numpy's message.
            Self::SampleLengthMismatch { .. } => f.write_str("x and y must have the same length."),
            Self::WeightedEstimator => f.write_str(
                "automated estimation of the number of bins is not supported for weighted data",
            ),
            Self::UnknownEstimator(name) => write!(f, "{name:?} is not a valid bin estimator"),
            Self::CountMismatch {
                what,
                datasets,
                actual,
            } => write!(f, "{actual} {what} given for {datasets} datasets"),
            Self::TooFewSamples => f.write_str("a kernel density estimate needs multiple samples"),
            Self::SingularCovariance => f.write_str("the data covariance matrix is singular"),
            Self::TooManyPoints(points) => write!(
                f,
                "{points} evaluation points are more than {}",
                super::MAX_VIOLIN_POINTS
            ),
            Self::DimensionMismatch { expected, actual } => write!(
                f,
                "points have dimension {actual}, dataset has dimension {expected}"
            ),
            Self::NoResamples => f.write_str("a bootstrap needs at least one resample"),
            Self::TooManyBootstrapDraws(draws) => write!(
                f,
                "a bootstrap drawing {draws} values draws more than {}",
                super::MAX_BOOTSTRAP_DRAWS
            ),
        }
    }
}

impl std::error::Error for StatsError {}
