//! Why a patch cannot be built or drawn.

use std::fmt;

use crate::colors::ColorError;
use crate::coords::CoordError;

/// A problem met while drawing a patch.
#[derive(Clone, Debug, PartialEq)]
pub enum PatchError {
    /// A coordinate system the patch refers to no longer exists.
    Detached,
    /// A colour of the patch is invalid.
    Color(ColorError),
    /// The arrow style needs a single quadratic Bézier curve (`simple`,
    /// `fancy` and `wedge` arrows), but the connection is not one.
    NotQuadratic,
    /// The connection's two angles are parallel, so its corner is undefined.
    ParallelAngles,
    /// The arrow is too short for its head or tail.
    NotIntersecting,
    /// Two points of a connection coincide, so its direction is undefined
    /// (matplotlib divides by their zero distance).
    CoincidentPoints,
    /// An end's coordinates cannot be resolved.
    Coords(CoordError),
    /// Connection patches do not support offset, font-size, artist or
    /// blended coordinates.
    UnsupportedCoords,
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Detached => f.write_str("its coordinate system no longer exists"),
            Self::Color(err) => err.fmt(f),
            Self::NotQuadratic => f.write_str("'path' is not a valid quadratic Bezier curve"),
            Self::ParallelAngles => f.write_str(
                "given lines do not intersect: the angles are equal or differ by 180 degrees",
            ),
            Self::NotIntersecting => {
                f.write_str("the arrow path does not cross the circle around its head or tail")
            }
            Self::CoincidentPoints => {
                f.write_str("the connection's points coincide, so its direction is undefined")
            }
            Self::Coords(err) => err.fmt(f),
            Self::UnsupportedCoords => f.write_str(
                "connection patches take data, polar, fraction, pixel or point coordinates",
            ),
        }
    }
}

impl std::error::Error for PatchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Color(err) => Some(err),
            Self::Coords(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ColorError> for PatchError {
    fn from(err: ColorError) -> Self {
        Self::Color(err)
    }
}

/// An invalid style description such as `"round,pad=0.2"` or a parameter
/// out of range.
#[derive(Clone, Debug, PartialEq)]
pub enum StyleError {
    /// No style of that name exists.
    UnknownStyle(String),
    /// A `key=value` attribute is malformed or its value is not a number.
    BadAttribute(String),
    /// The style has no parameter of that name.
    UnknownParameter {
        /// The style.
        style: String,
        /// The parameter given.
        parameter: String,
    },
    /// A parameter value is out of range.
    InvalidValue(&'static str),
}

impl fmt::Display for StyleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownStyle(name) => write!(f, "unknown style: {name:?}"),
            Self::BadAttribute(attr) => write!(f, "incorrect style argument: {attr:?}"),
            Self::UnknownParameter { style, parameter } => {
                write!(f, "style {style:?} has no parameter {parameter:?}")
            }
            Self::InvalidValue(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for StyleError {}

/// Geometry a patch cannot have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeError {
    /// An annulus ring wider than its smaller semi-axis.
    AnnulusTooWide,
    /// A shadow shade outside `[0, 1]`.
    ShadeOutOfRange,
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AnnulusTooWide => {
                "width of annulus must be less than or equal to semi-minor axis"
            }
            Self::ShadeOutOfRange => "shade must be between 0 and 1",
        })
    }
}

impl std::error::Error for ShapeError {}
