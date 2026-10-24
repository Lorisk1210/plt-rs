//! Errors raised while laying out text.

use std::fmt;

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
        }
    }
}

impl std::error::Error for TextError {}

impl From<plt_font::FontError> for TextError {
    fn from(err: plt_font::FontError) -> Self {
        Self::Font(err.to_string())
    }
}

impl From<plt_font::AfmError> for TextError {
    fn from(err: plt_font::AfmError) -> Self {
        Self::Font(err.to_string())
    }
}
