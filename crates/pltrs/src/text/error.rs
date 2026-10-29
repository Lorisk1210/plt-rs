//! Why a text could not be drawn.

use std::fmt;

use pltrs_text::TextError;

use crate::colors::ColorError;
use crate::coords::CoordError;
use crate::patches::PatchError;

/// A problem met while drawing a text, its box or its annotation arrow.
#[derive(Clone, Debug, PartialEq)]
pub enum TextDrawError {
    /// The text cannot be laid out (e.g. malformed math).
    Layout(TextError),
    /// The text colour is invalid.
    Color(ColorError),
    /// The box or arrow cannot be drawn.
    Box(PatchError),
    /// An annotation coordinate cannot be resolved.
    Coords(CoordError),
}

impl fmt::Display for TextDrawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout(err) => err.fmt(f),
            Self::Color(err) => err.fmt(f),
            Self::Box(err) => err.fmt(f),
            Self::Coords(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for TextDrawError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Layout(err) => Some(err),
            Self::Color(err) => Some(err),
            Self::Box(err) => Some(err),
            Self::Coords(err) => Some(err),
        }
    }
}

impl From<TextError> for TextDrawError {
    fn from(err: TextError) -> Self {
        Self::Layout(err)
    }
}

impl From<ColorError> for TextDrawError {
    fn from(err: ColorError) -> Self {
        Self::Color(err)
    }
}

impl From<CoordError> for TextDrawError {
    fn from(err: CoordError) -> Self {
        Self::Coords(err)
    }
}
