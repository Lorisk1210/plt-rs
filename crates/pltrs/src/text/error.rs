//! Why a text could not be drawn.

use std::fmt;

use pltrs_text::TextError;
use pltrs_text::tex::TexError;

use crate::artist::DrawErrorKind;
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

impl TextDrawError {
    /// The kind of draw error it makes: malformed mathtext, an unknown
    /// property name, a language range outside the text and an invalid
    /// colour are values of the program's that matplotlib refuses with a
    /// `ValueError` while drawing, as it refuses a TeX font without a
    /// Type 1 file (`DviFont.resolve_path`) and a text the PGF backend's
    /// TeX system cannot measure.
    pub fn kind(&self) -> DrawErrorKind {
        match self {
            Self::Layout(
                TextError::Math(_)
                | TextError::UnknownName { .. }
                | TextError::LanguageRange { .. }
                | TextError::Tex(TexError::NoType1Font { .. } | TexError::Measuring { .. }),
            )
            | Self::Color(_) => DrawErrorKind::InvalidValue,
            _ => DrawErrorKind::Other,
        }
    }
}

impl fmt::Display for TextDrawError {
    /// The problem as matplotlib's draw raises it: malformed mathtext as
    /// the parser's explanation alone, the message of its `ValueError`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout(TextError::Math(why)) => f.write_str(why),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_math_reads_as_matplotlibs_value_error() {
        let why = "\n\\sqrt{\n     ^\nParseSyntaxException: Expected \\sqrt{value}";
        let err = TextDrawError::from(TextError::Math(why.to_owned()));
        assert_eq!(err.to_string(), why);
        assert_eq!(err.kind(), DrawErrorKind::InvalidValue);
        let font = TextDrawError::from(TextError::Font("no file".to_owned()));
        assert_eq!(font.to_string(), "font error: no file");
    }
}
