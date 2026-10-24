//! Errors of the plotting methods.

use std::fmt;

use super::pie::PieError;
use crate::colors::{ColorizerError, ColormapError, NORM_WITH_LIMITS};
use crate::lines::{FormatStringError, MarkEveryError};
use crate::markers::MarkerError;
use crate::mlab::MlabError;
use crate::patches::StepPatchError;
use crate::spines::SpineSide;
use crate::stats::StatsError;
use crate::ticker::{AxisName, TickerError};

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
    Format(FormatStringError),
    /// Inconsistent step data.
    Steps(StepPatchError),
    /// Negative error bar sizes (the argument named).
    NegativeErrors(&'static str),
    /// Error bar sizes that do not add to the values they extend from, or
    /// do not scale, in their units (the argument named; matplotlib's
    /// `TypeError` from `x + -1 * xerr`).
    UnaddableErrors(&'static str),
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
    /// The axis is already shared with a different axes.
    AlreadyShared(AxisName),
    /// The axes belong to different figures.
    ForeignAxes,
    /// The artist is not drawn in this axes.
    NotInAxes,
    /// The collection was not made by `fill_between` or `fill_betweenx`.
    NotFillBetween,
    /// The axis does not label its major ticks with a `ScalarFormatter`.
    NotScalarFormatter(AxisName),
    /// The axis does not place its major ticks with a `MaxNLocator`.
    NotMaxNLocator(AxisName),
    /// Invalid tick helper settings.
    Ticker(TickerError),
    /// Invalid spectral analysis input.
    Mlab(MlabError),
    /// An invalid pie chart.
    Pie(PieError),
    /// A per-dataset argument with neither one value nor one per dataset.
    DatasetCount {
        /// The argument.
        argument: &'static str,
        /// Number of values given.
        len: usize,
        /// Number of datasets.
        datasets: usize,
    },
    /// A view limit that is NaN or infinite.
    NonFiniteLimit(AxisName),
    /// An autoscaling margin of -0.5 or less, or not finite.
    InvalidMargin(f64),
    /// A side that the axis cannot be drawn on (e.g. `Left` for x).
    WrongSide {
        /// The axis.
        axis: AxisName,
        /// The requested side.
        side: SpineSide,
    },
    /// Another unit converter was already set explicitly on the axis.
    ExplicitConverter(AxisName),
    /// The axis belongs to axes whose projection fixes their limits, such
    /// as geographic axes, which therefore cannot share it (matplotlib's
    /// `TypeError` from `set_xlim` in `sharex`).
    FixedLimits(AxisName),
    /// An infinite line given by a slope on axes with a non-linear scale.
    SlopeOnNonlinearScale,
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
            Self::UnaddableErrors(argument) => write!(
                f,
                "'{argument}' does not add to the values it extends from in their units"
            ),
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
            Self::AlreadyShared(axis) => write!(f, "{}-axis is already shared", axis.name()),
            Self::ForeignAxes => f.write_str("the axes belong to different figures"),
            Self::NotInAxes => f.write_str("Argument must be an image or collection in this Axes"),
            Self::NotFillBetween => {
                f.write_str("the collection was not made by fill_between or fill_betweenx")
            }
            Self::NotMaxNLocator(axis) => write!(
                f,
                "the {}-axis ticks are not placed by a MaxNLocator",
                axis.name()
            ),
            Self::Ticker(err) => err.fmt(f),
            Self::Mlab(err) => err.fmt(f),
            Self::Pie(err) => err.fmt(f),
            Self::NotScalarFormatter(axis) => write!(
                f,
                "the {}-axis tick labels do not use a ScalarFormatter",
                axis.name()
            ),
            Self::DatasetCount {
                argument, len: 0, ..
            } => {
                write!(f, "{argument} cannot be empty")
            }
            Self::DatasetCount {
                argument,
                len,
                datasets,
            } => write!(
                f,
                "{argument} has {len} values for {datasets} datasets; give one or one per dataset"
            ),
            Self::WrongSide { axis, side } => {
                write!(
                    f,
                    "{:?} is not a side of the {}-axis",
                    side.name(),
                    axis.name()
                )
            }
            Self::NonFiniteLimit(axis) => {
                write!(f, "{}-axis limits cannot be NaN or Inf", axis.name())
            }
            Self::InvalidMargin(margin) => {
                write!(
                    f,
                    "margin must be finite and greater than -0.5, got {margin}"
                )
            }
            Self::ExplicitConverter(axis) => write!(
                f,
                "the {}-axis already has an explicit converter set",
                axis.name()
            ),
            Self::FixedLimits(axis) => write!(
                f,
                "the {}-axis cannot be shared: the projection of its axes fixes their limits",
                axis.name()
            ),
            Self::NormWithLimits => f.write_str(NORM_WITH_LIMITS),
            Self::SlopeOnNonlinearScale => {
                f.write_str("'slope' cannot be used with non-linear scales")
            }
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

impl From<ColorizerError> for PlotError {
    fn from(err: ColorizerError) -> Self {
        match err {
            ColorizerError::Colormap(err) => Self::Colormap(err),
            ColorizerError::NormWithLimits => Self::NormWithLimits,
        }
    }
}

impl From<FormatStringError> for PlotError {
    fn from(err: FormatStringError) -> Self {
        Self::Format(err)
    }
}

impl From<StepPatchError> for PlotError {
    fn from(err: StepPatchError) -> Self {
        Self::Steps(err)
    }
}
