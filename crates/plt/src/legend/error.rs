//! Why a legend cannot be made.

use std::fmt;

use crate::colors::ColorError;
use crate::patches::ShadowError;

/// An invalid legend option.
#[derive(Clone, Debug, PartialEq)]
pub enum LegendError {
    /// `loc` is not a location name, a code from 0 to 10 or a point.
    InvalidLocation(String),
    /// `loc="best"` was asked of a figure legend.
    BestForFigure,
    /// An `"outside ..."` location (this one) was asked of an axes legend.
    OutsideForAxes(String),
    /// `numpoints` is zero.
    NoPoints,
    /// `scatteryoffsets` is empty.
    NoScatterOffsets,
    /// `ncols` is zero while there are entries to split.
    NoColumns,
    /// `title_fontsize` and `title_fontproperties` were both given.
    TitleFontConflict,
    /// A frame or label colour is invalid.
    Color(ColorError),
    /// The colours of an artist shown in the legend cannot be resolved.
    HandleColors(String),
    /// The frame's shadow cannot be made.
    Shadow(ShadowError),
    /// A handler was given a handle of a kind it does not draw (this
    /// handler), such as [`HandlerPatch`](super::HandlerPatch) for a line.
    UnsupportedHandle(&'static str),
}

impl fmt::Display for LegendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocation(loc) => write!(
                f,
                "loc must be a location name, a coordinate pair or an integer 0-10, not {loc}"
            ),
            Self::BestForFigure => f.write_str(
                "automatic legend placement (loc=\"best\") is not implemented for figure legends",
            ),
            Self::OutsideForAxes(loc) => write!(
                f,
                "the 'outside' option of loc={loc:?} only works for figure legends"
            ),
            Self::NoPoints => f.write_str("numpoints must be > 0"),
            Self::NoScatterOffsets => f.write_str("scatteryoffsets must not be empty"),
            Self::NoColumns => f.write_str("ncols must be > 0"),
            Self::TitleFontConflict => f.write_str(
                "title_fontsize and title_fontproperties can't be specified at the same time",
            ),
            Self::Color(err) => write!(f, "invalid legend colour: {err}"),
            Self::HandleColors(why) => write!(f, "invalid colours of a legend entry: {why}"),
            Self::Shadow(err) => write!(f, "invalid legend shadow: {err}"),
            Self::UnsupportedHandle(handler) => {
                write!(f, "{handler} cannot draw this kind of legend handle")
            }
        }
    }
}

impl std::error::Error for LegendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Color(err) => Some(err),
            Self::Shadow(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ColorError> for LegendError {
    fn from(err: ColorError) -> Self {
        Self::Color(err)
    }
}

impl From<ShadowError> for LegendError {
    fn from(err: ShadowError) -> Self {
        Self::Shadow(err)
    }
}
