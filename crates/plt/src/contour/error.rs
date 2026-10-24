//! Errors of contour generation, level selection and labelling.

use std::fmt;

use crate::colors::{ColorError, ColormapError, NormError};
use crate::text::TextError;
use crate::ticker::TickerError;
use crate::ticker::pyfmt::FormatError;
use crate::transforms::TransformError;
use crate::tri::TriError;

/// Why a contour plot or label could not be made.
#[derive(Clone, Debug, PartialEq)]
pub enum ContourError {
    /// The grid has fewer than 2 rows or columns.
    GridTooSmall {
        /// Rows of z.
        rows: usize,
        /// Columns of z.
        cols: usize,
    },
    /// An array does not have the shape of z (`(rows, cols)`).
    ShapeMismatch {
        /// The shape of z.
        expected: (usize, usize),
        /// The offending shape.
        actual: (usize, usize),
    },
    /// 1-D x has a length other than the number of columns of z.
    XLength {
        /// Length of x.
        len: usize,
        /// Columns of z.
        cols: usize,
    },
    /// 1-D y has a length other than the number of rows of z.
    YLength {
        /// Length of y.
        len: usize,
        /// Rows of z.
        rows: usize,
    },
    /// The z values of a triangular grid are not one per point.
    ZLength {
        /// Number of z values.
        len: usize,
        /// Number of points.
        points: usize,
    },
    /// A point used by an unmasked triangle has a non-finite z.
    NonFiniteZ,
    /// Every triangle is masked.
    NoTriangles,
    /// A level is NaN.
    NanLevel,
    /// No contouring algorithm has this name.
    UnknownAlgorithm(String),
    /// Corners are to be masked by an algorithm that cannot mask them.
    CornerMaskUnsupported(super::ContourAlgorithm),
    /// A filled contour's lower level is not below its upper level.
    LevelOrder {
        /// Lower level.
        lower: f64,
        /// Upper level.
        upper: f64,
    },
    /// The levels are not strictly increasing.
    LevelsNotIncreasing,
    /// Filled contours need at least two levels.
    TooFewFilledLevels(usize),
    /// Log-scaled triangular contours need positive z.
    LogOfNonPositive,
    /// Explicit colours and a colormap were both given.
    ColorsAndCmap,
    /// Labels were requested for levels the contour set does not have.
    UnknownLabelLevels(Vec<f64>),
    /// The label position is not near any labelled contour line.
    NoContourToLabel,
    /// A label was added near a point before the set was labelled.
    Unlabelled,
    /// The operation needs line contours, not filled ones.
    FilledUnsupported,
    /// A level index is out of range.
    LevelIndex {
        /// The index given.
        index: usize,
        /// How many levels there are.
        levels: usize,
    },
    /// There is no label to remove: none at the index given, or none at
    /// all (`None`).
    NoLabel(Option<usize>),
    /// More levels (this many) to label than
    /// [`MAX_LABELLED_LEVELS`](super::MAX_LABELLED_LEVELS).
    TooManyLabelledLevels(usize),
    /// Given segments are not one list per level (lines) or band (fills).
    SegmentCount {
        /// Levels (lines) or bands (fills).
        expected: usize,
        /// Lists of segments given.
        actual: usize,
    },
    /// The given segments have no points.
    NoSegmentPoints,
    /// Level selection failed in the locator.
    Ticker(TickerError),
    /// The colour mapping failed.
    Norm(NormError),
    /// A colormap could not be built.
    Colormap(ColormapError),
    /// A colour specification is invalid.
    Color(ColorError),
    /// A label could not be laid out.
    Text(TextError),
    /// The triangulation is invalid.
    Tri(TriError),
    /// A label format failed.
    Format(FormatError),
    /// The axes' data transform cannot be resolved or inverted.
    Transform(TransformError),
}

impl fmt::Display for ContourError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GridTooSmall { rows, cols } => write!(
                f,
                "z must be at least a (2, 2) shaped array, but has shape ({rows}, {cols})"
            ),
            Self::ShapeMismatch { expected, actual } => {
                write!(
                    f,
                    "shape {actual:?} does not match the shape of z {expected:?}"
                )
            }
            Self::XLength { len, cols } => {
                write!(
                    f,
                    "length of x ({len}) must match number of columns in z ({cols})"
                )
            }
            Self::YLength { len, rows } => {
                write!(
                    f,
                    "length of y ({len}) must match number of rows in z ({rows})"
                )
            }
            Self::ZLength { len, points } => write!(
                f,
                "z array must have same length as triangulation x and y arrays ({len} != {points})"
            ),
            Self::NonFiniteZ => {
                f.write_str("z array must not contain non-finite values within the triangulation")
            }
            Self::NoTriangles => f.write_str("the triangulation has no unmasked triangles"),
            Self::NanLevel => f.write_str("contour levels cannot be NaN"),
            Self::UnknownAlgorithm(name) => {
                write!(f, "unrecognised contour generator name: {name}")
            }
            Self::CornerMaskUnsupported(algorithm) => {
                write!(
                    f,
                    "{algorithm} contour generator does not support corner_mask=True"
                )
            }
            Self::LevelOrder { lower, upper } => {
                write!(
                    f,
                    "upper level ({upper}) must be larger than lower level ({lower})"
                )
            }
            Self::LevelsNotIncreasing => f.write_str("contour levels must be increasing"),
            Self::TooFewFilledLevels(n) => {
                write!(f, "filled contours require at least 2 levels, got {n}")
            }
            Self::LogOfNonPositive => f.write_str("cannot contour log of negative values"),
            Self::ColorsAndCmap => f.write_str("either colors or cmap must be None"),
            Self::UnknownLabelLevels(levels) => {
                write!(
                    f,
                    "specified levels {levels:?} don't match available levels"
                )
            }
            Self::NoContourToLabel => f.write_str("no labelled contour line to place a label on"),
            Self::Unlabelled => f.write_str("label the contour set with clabel first"),
            Self::FilledUnsupported => f.write_str("method does not support filled contours"),
            Self::LevelIndex { index, levels } => {
                write!(f, "level index {index} is out of range for {levels} levels")
            }
            Self::NoLabel(Some(index)) => write!(f, "the contour set has no label {index}"),
            Self::NoLabel(None) => f.write_str("the contour set has no labels"),
            Self::TooManyLabelledLevels(count) => write!(
                f,
                "{count} levels to label are more than {}",
                super::MAX_LABELLED_LEVELS
            ),
            Self::SegmentCount { expected, actual } => write!(
                f,
                "{actual} lists of segments given for {expected} levels or bands"
            ),
            Self::NoSegmentPoints => f.write_str("the contour segments have no points"),
            Self::Ticker(err) => write!(f, "level selection failed: {err}"),
            Self::Norm(err) => write!(f, "colour mapping failed: {err}"),
            Self::Colormap(err) => write!(f, "invalid colormap: {err}"),
            Self::Color(err) => write!(f, "invalid colour: {err}"),
            Self::Text(err) => write!(f, "label layout failed: {err}"),
            Self::Tri(err) => write!(f, "invalid triangulation: {err}"),
            Self::Format(err) => write!(f, "label format failed: {err}"),
            Self::Transform(err) => write!(f, "data transform unavailable: {err}"),
        }
    }
}

impl std::error::Error for ContourError {}

macro_rules! from_error {
    ($($source:ty => $variant:ident),* $(,)?) => {
        $(impl From<$source> for ContourError {
            fn from(err: $source) -> Self {
                Self::$variant(err)
            }
        })*
    };
}

from_error! {
    TickerError => Ticker,
    NormError => Norm,
    ColormapError => Colormap,
    ColorError => Color,
    TextError => Text,
    TriError => Tri,
    FormatError => Format,
    TransformError => Transform,
}
