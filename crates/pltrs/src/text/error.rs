//! Why a text could not be drawn.

use std::fmt;

use pltrs_text::TextError;
use pltrs_text::tex::TexError;

use crate::artist::DrawErrorKind;
use crate::colors::ColorError;
use crate::coords::CoordError;
use crate::patches::PatchError;

use super::UnroundablePosition;

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
    /// The renderer rounds glyph positions, and one is not finite.
    Position(UnroundablePosition),
}

impl TextDrawError {
    /// The kind of draw error it makes: malformed mathtext, an unknown
    /// property name, a language range outside the text and an invalid
    /// colour are values of the program's that matplotlib refuses with a
    /// `ValueError` while drawing, as it refuses a TeX font without a
    /// Type 1 file (`DviFont.resolve_path`) and a text the PGF backend's
    /// TeX system cannot measure. A glyph position the renderer cannot
    /// round is Python's `ValueError` for NaN, its `OverflowError` for an
    /// infinity.
    pub fn kind(&self) -> DrawErrorKind {
        match self {
            Self::Layout(err) => layout_error_kind(err),
            Self::Color(_) => DrawErrorKind::InvalidValue,
            Self::Box(_) | Self::Coords(_) => DrawErrorKind::Other,
            Self::Position(position) if position.is_nan() => DrawErrorKind::InvalidValue,
            Self::Position(_) => DrawErrorKind::Overflow,
        }
    }
}

/// The kind of draw error a text that cannot be laid out makes, wherever
/// it is drawn (a text artist, an offset box's text area): see
/// [`TextDrawError::kind`]; a font file or TeX file that is not there is
/// matplotlib's `FileNotFoundError`, and a font size `FreeType` refuses, a
/// TeX program failing on the text, not installed or not starting (the PGF
/// backend's measuring) its `RuntimeError`.
pub(crate) fn layout_error_kind(err: &TextError) -> DrawErrorKind {
    match err {
        TextError::Math(_)
        | TextError::UnknownName { .. }
        | TextError::LanguageRange { .. }
        | TextError::Tex(TexError::NoType1Font { .. } | TexError::Measuring { .. }) => {
            DrawErrorKind::InvalidValue
        }
        TextError::MissingFontFile(_) | TextError::Tex(TexError::NotFound { .. }) => {
            DrawErrorKind::FileNotFound
        }
        TextError::FontSize(_)
        | TextError::Tex(
            TexError::Failed { .. } | TexError::Missing { .. } | TexError::Unstartable { .. },
        ) => DrawErrorKind::Library,
        _ => DrawErrorKind::Other,
    }
}

/// A text an axis could not lay out or colour, as its draw reports it:
/// the message, and the kind of draw error it makes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextProblem {
    /// What kind of problem it is (see [`layout_error_kind`]).
    pub(crate) kind: DrawErrorKind,
    /// What went wrong.
    pub(crate) message: String,
}

impl From<TextError> for TextProblem {
    fn from(err: TextError) -> Self {
        Self {
            kind: layout_error_kind(&err),
            message: err.to_string(),
        }
    }
}

impl From<ColorError> for TextProblem {
    fn from(err: ColorError) -> Self {
        Self {
            kind: DrawErrorKind::Other,
            message: err.to_string(),
        }
    }
}

impl From<String> for TextProblem {
    /// A problem of no kind of its own (laying out what is no text).
    fn from(message: String) -> Self {
        Self {
            kind: DrawErrorKind::Other,
            message,
        }
    }
}

impl From<TextProblem> for String {
    fn from(problem: TextProblem) -> Self {
        problem.message
    }
}

impl fmt::Display for TextProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// A text that cannot be laid out, in the words of matplotlib's draw:
/// malformed mathtext as the parser's explanation alone.
pub(crate) fn fmt_layout_error(err: &TextError, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match err {
        TextError::Math(why) => f.write_str(why),
        err => fmt::Display::fmt(err, f),
    }
}

impl fmt::Display for TextDrawError {
    /// The problem as matplotlib's draw raises it: malformed mathtext as
    /// the parser's explanation alone, the message of its `ValueError`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout(err) => fmt_layout_error(err, f),
            Self::Color(err) => err.fmt(f),
            Self::Box(err) => err.fmt(f),
            Self::Coords(err) => err.fmt(f),
            Self::Position(err) => err.fmt(f),
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
            Self::Position(err) => Some(err),
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

impl From<UnroundablePosition> for TextDrawError {
    fn from(err: UnroundablePosition) -> Self {
        Self::Position(err)
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
