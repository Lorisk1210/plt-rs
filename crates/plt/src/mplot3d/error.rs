//! What 3-D plotting can fail with.

use std::fmt;

use super::coords::Axis3;
use super::math::Vec3;
use crate::axes::PlotError;
use crate::colors::ColorError;
use crate::contour::ContourError;
use crate::gridspec::GridSpecError;
use crate::scale::ScaleError;
use crate::tri::TriError;

/// An invalid argument to a 3-D axes or one of its plotting methods.
#[derive(Clone, Debug, PartialEq)]
pub enum Axes3DError {
    /// A perspective focal length that is not positive.
    FocalLength(f64),
    /// A box zoom that is not positive.
    Zoom(f64),
    /// A box aspect with a component that is not positive.
    BoxAspect(Vec3),
    /// An autoscaling margin that is not above -0.5.
    Margin(f64),
    /// Axis limits that are infinite or NaN.
    NonFiniteLimits(f64, f64),
    /// Coordinate arrays of different lengths or shapes.
    Shape {
        /// The argument that does not match.
        argument: &'static str,
        /// The expected number of elements.
        expected: usize,
        /// The number of elements given.
        actual: usize,
    },
    /// Both wireframe strides are zero.
    NoStride,
    /// A tick locator failed while autoscaling.
    Ticker(String),
    /// A colour that cannot be resolved.
    Color(ColorError),
    /// Colours or styles that cannot be resolved.
    Style(String),
    /// An error of the underlying 2-D plotting method.
    Plot(PlotError),
    /// An error while computing contours.
    Contour(ContourError),
    /// An error while triangulating.
    Tri(TriError),
    /// An invalid subplot grid or cell.
    Grid(GridSpecError),
    /// A mouse rotation style other than `azel`, `trackball`, `sphere` and
    /// `arcball`.
    UnknownRotationStyle(String),
    /// An aspect other than `auto`, `equal`, `equalxy`, `equalxz` and
    /// `equalyz`.
    UnknownAspect(String),
    /// A scale that cannot be created.
    Scale(ScaleError),
    /// An axis or the view is already shared with another axes (what is
    /// shared, as matplotlib names it).
    AlreadyShared(&'static str),
    /// Axes of different figures cannot share an axis or the view.
    OtherFigure,
    /// Only polygon and line collections can be lifted to 3-D.
    UnsupportedCollection,
    /// The artist is not a 3-D artist of the axes.
    Not3DArtist,
    /// The axis does not label its major ticks with a `ScalarFormatter`.
    NotScalarFormatter(Axis3),
}

impl fmt::Display for Axes3DError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FocalLength(value) => {
                write!(f, "focal_length = {value} must be greater than 0")
            }
            Self::Zoom(value) => write!(f, "zoom = {value} must be > 0"),
            Self::BoxAspect(aspect) => {
                write!(f, "box aspect {aspect:?} must have positive components")
            }
            Self::Margin(value) => write!(f, "margin {value} must be greater than -0.5"),
            Self::NonFiniteLimits(low, high) => {
                write!(f, "axis limits {low}, {high} cannot be infinite or NaN")
            }
            Self::Shape {
                argument,
                expected,
                actual,
            } => write!(
                f,
                "{argument} has {actual} elements where {expected} were expected"
            ),
            Self::NoStride => f.write_str("either rstride or cstride must be non zero"),
            Self::UnknownRotationStyle(name) => write!(
                f,
                "{name:?} is not a mouse rotation style (azel, trackball, sphere or arcball)"
            ),
            Self::UnknownAspect(name) => write!(
                f,
                "{name:?} is not a 3-D aspect (auto, equal, equalxy, equalxz or equalyz)"
            ),
            Self::AlreadyShared(what) => write!(f, "{what} is already shared"),
            Self::OtherFigure => f.write_str("axes of another figure cannot be shared with"),
            Self::UnsupportedCollection => {
                f.write_str("only polygon and line collections can be added to 3-D axes")
            }
            Self::Not3DArtist => f.write_str("the artist is not a 3-D artist of these axes"),
            Self::NotScalarFormatter(axis) => write!(
                f,
                "the {}-axis tick labels do not use a ScalarFormatter",
                axis.name()
            ),
            Self::Scale(err) => err.fmt(f),
            Self::Ticker(message) => write!(f, "cannot autoscale: {message}"),
            Self::Color(err) => err.fmt(f),
            Self::Style(message) => f.write_str(message),
            Self::Plot(err) => err.fmt(f),
            Self::Contour(err) => err.fmt(f),
            Self::Tri(err) => err.fmt(f),
            Self::Grid(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Axes3DError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Color(err) => Some(err),
            Self::Plot(err) => Some(err),
            Self::Contour(err) => Some(err),
            Self::Tri(err) => Some(err),
            Self::Grid(err) => Some(err),
            Self::Scale(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ScaleError> for Axes3DError {
    fn from(err: ScaleError) -> Self {
        Self::Scale(err)
    }
}

impl From<ColorError> for Axes3DError {
    fn from(err: ColorError) -> Self {
        Self::Color(err)
    }
}

impl From<PlotError> for Axes3DError {
    fn from(err: PlotError) -> Self {
        Self::Plot(err)
    }
}

impl From<ContourError> for Axes3DError {
    fn from(err: ContourError) -> Self {
        Self::Contour(err)
    }
}

impl From<TriError> for Axes3DError {
    fn from(err: TriError) -> Self {
        Self::Tri(err)
    }
}

impl From<GridSpecError> for Axes3DError {
    fn from(err: GridSpecError) -> Self {
        Self::Grid(err)
    }
}

/// Checks that `argument` has `expected` elements.
pub(crate) fn check_len(
    argument: &'static str,
    expected: usize,
    actual: usize,
) -> Result<(), Axes3DError> {
    if expected == actual {
        Ok(())
    } else {
        Err(Axes3DError::Shape {
            argument,
            expected,
            actual,
        })
    }
}
