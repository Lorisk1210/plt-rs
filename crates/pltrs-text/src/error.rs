//! Errors raised while laying out text.

use std::fmt;
use std::path::PathBuf;

/// Why text could not be laid out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextError {
    /// A font could not be found, read or parsed.
    Font(String),
    /// A mathtext expression is malformed.
    Math(String),
    /// A language range reaches outside the text it was given for.
    LanguageRange {
        /// The range's language, as written.
        language: String,
        /// Index of its first character.
        start: usize,
        /// Index one past its last character.
        end: usize,
    },
    /// A name that a text property does not have, such as a horizontal
    /// alignment of `"middle"`.
    UnknownName {
        /// What was being parsed (`"horizontal alignment"`, ...).
        kind: &'static str,
        /// The unknown name.
        name: String,
    },
    /// TeX could not typeset or draw usetex text.
    Tex(crate::tex::TexError),
    /// The font file a text names (matplotlib's `FontProperties(fname=
    /// ...)`) does not exist; matplotlib accepts the name and fails with a
    /// `FileNotFoundError` when it opens the file to measure the text.
    MissingFontFile(PathBuf),
}

impl TextError {
    /// The error of a name that `kind` does not have.
    pub(crate) fn unknown_name(kind: &'static str, name: &str) -> Self {
        Self::UnknownName {
            kind,
            name: name.to_owned(),
        }
    }
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Font(why) => write!(f, "font error: {why}"),
            Self::Math(why) => write!(f, "mathtext error: {why}"),
            Self::LanguageRange {
                language,
                start,
                end,
            } => write!(
                f,
                "failed to set language between {start} and {end} characters to '{language}' \
                 for layout"
            ),
            Self::UnknownName { kind, name } => write!(f, "{name:?} is not a valid {kind}"),
            Self::Tex(err) => err.fmt(f),
            Self::MissingFontFile(path) => write!(
                f,
                "[Errno 2] No such file or directory: '{}'",
                path.display()
            ),
        }
    }
}

impl std::error::Error for TextError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Tex(err) => Some(err),
            _ => None,
        }
    }
}

impl From<crate::tex::TexError> for TextError {
    fn from(err: crate::tex::TexError) -> Self {
        Self::Tex(err)
    }
}

impl From<pltrs_font::FontError> for TextError {
    fn from(err: pltrs_font::FontError) -> Self {
        Self::Font(err.to_string())
    }
}

impl From<pltrs_font::AfmError> for TextError {
    fn from(err: pltrs_font::AfmError) -> Self {
        Self::Font(err.to_string())
    }
}
