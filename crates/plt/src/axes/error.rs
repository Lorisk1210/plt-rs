//! Errors of the plotting methods.

use std::fmt;

use crate::colors::ColormapError;
use crate::lines::{FormatError, MarkEveryError};
use crate::markers::MarkerError;
use crate::patches::StepPatchError;
use crate::stats::StatsError;

/// Invalid data or options passed to a plotting method.
#[derive(Clone, Debug, PartialEq)]
pub enum PlotError {
    /// x and y have different lengths.
    LengthMismatch {
        /// Number of x values.
        x: usize,
        /// Number of y values.
        y: usize,
    },
    /// A per-point argument whose length is neither one nor the number of
    /// points.
    Broadcast {
        /// The argument (`"s"`, `"c"`, ...).
        argument: &'static str,
        /// Its length.
        len: usize,
        /// The number of points.
        points: usize,
    },
    /// An invalid marker.
    Marker(MarkerError),
    /// An unknown or invalid colormap.
    Colormap(ColormapError),
    /// Both a norm and explicit `vmin`/`vmax` were given.
    NormWithLimits,
    /// An invalid format string.
    Format(FormatError),
    /// Inconsistent step data.
    Steps(StepPatchError),
    /// Negative error bar sizes (the argument named).
    NegativeErrors(&'static str),
    /// An invalid `errorevery` selection.
    ErrorEvery(MarkEveryError),
    /// Per-item legend labels whose number differs from the number of
    /// items.
    LabelCount {
        /// Number of labels.
        labels: usize,
        /// Number of items.
        items: usize,
    },
    /// Invalid statistics input (binning, weights, ...).
    Stats(StatsError),
    /// Tick labels whose number differs from the number of fixed ticks.
    TickLabelCount {
        /// Number of ticks.
        ticks: usize,
        /// Number of labels.
        labels: usize,
    },
}

impl fmt::Display for PlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { x, y } => write!(
                f,
                "x and y must have same first dimension, but have shapes ({x},) and ({y},)"
            ),
            Self::Broadcast {
                argument,
                len,
                points,
            } => write!(
                f,
                "'{argument}' has {len} elements, which is inconsistent with 'x' and 'y' \
                 with size {points}"
            ),
            Self::Marker(err) => err.fmt(f),
            Self::Colormap(err) => err.fmt(f),
            Self::Format(err) => err.fmt(f),
            Self::Steps(err) => err.fmt(f),
            Self::Stats(err) => err.fmt(f),
            Self::NegativeErrors(argument) => {
                write!(f, "'{argument}' must not contain negative values")
            }
            Self::ErrorEvery(err) => write!(f, "errorevery: {err}"),
            Self::LabelCount { labels, items } => write!(
                f,
                "number of labels ({labels}) does not match number of bars ({items})."
            ),
            Self::TickLabelCount { ticks, labels } => write!(
                f,
                "The number of FixedLocator locations ({ticks}), usually from a call to \
                 set_ticks, does not match the number of labels ({labels})."
            ),
            Self::NormWithLimits => f.write_str(
                "passing a Normalize instance simultaneously with vmin/vmax is not supported",
            ),
        }
    }
}

impl std::error::Error for PlotError {}

impl From<MarkerError> for PlotError {
    fn from(err: MarkerError) -> Self {
        Self::Marker(err)
    }
}

impl From<ColormapError> for PlotError {
    fn from(err: ColormapError) -> Self {
        Self::Colormap(err)
    }
}

impl From<FormatError> for PlotError {
    fn from(err: FormatError) -> Self {
        Self::Format(err)
    }
}

impl From<StepPatchError> for PlotError {
    fn from(err: StepPatchError) -> Self {
        Self::Steps(err)
    }
}
