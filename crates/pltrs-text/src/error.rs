//! Errors raised while laying out text.

use std::fmt;
use std::path::PathBuf;

/// The line of matplotlib's `ft2font.cpp` its `FT_Set_Char_Size` errors
/// name (`__LINE__` in the `FT_CHECK` call spanning lines 248 to 250) on
/// this system: GCC, which builds matplotlib's Linux wheels, gives the
/// line the call starts on; clang, which builds its macOS wheels, and
/// MSVC, which builds its Windows wheels, the line it ends on.
pub const SET_CHAR_SIZE_LINE: u32 = if cfg!(any(target_os = "macos", windows)) {
    250
} else {
    248
};

/// Why text could not be laid out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextError {
    /// A font could not be found, read or parsed.
    Font(String),
    /// FreeType refuses the size of a text's font at the resolution it is
    /// laid out at (fewer than one pixel per em, or more than 65 535).
    FontSize(pltrs_font::SizeError),
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
            // As matplotlib's `FT2Font.set_size` reports FreeType's error.
            Self::FontSize(err) => write!(
                f,
                "FT_Set_Char_Size (ft2font.cpp line {SET_CHAR_SIZE_LINE}) failed with error \
                 {:#04x}: {err}",
                err.code()
            ),
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
            // Python's message names the file by its `repr` (Windows'
            // separators doubled).
            Self::MissingFontFile(path) => write!(
                f,
                "[Errno 2] No such file or directory: {}",
                pltrs_core::pyfmt::str_repr(&path.to_string_lossy())
            ),
        }
    }
}

impl std::error::Error for TextError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Tex(err) => Some(err),
            Self::FontSize(err) => Some(err),
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
        match err {
            pltrs_font::FontError::Size(err) => Self::FontSize(err),
            err => Self::Font(err.to_string()),
        }
    }
}

impl From<pltrs_font::AfmError> for TextError {
    fn from(err: pltrs_font::AfmError) -> Self {
        Self::Font(err.to_string())
    }
}
