//! Why the engine could not run a charstring.

/// A charstring the engine refused, as FreeType fails the glyph's load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CharstringError {
    /// A charstring that cannot be decoded or run.
    Malformed(&'static str),
    /// Subroutines nested too deep, or too many operators run.
    Limit,
    /// A subroutine number with no subroutine.
    MissingSubr(i64),
    /// A `seac` character code outside the Standard Encoding.
    NotStandardCode(i64),
    /// A `seac` component the font does not have.
    MissingGlyph(String),
    /// The size or font cannot be drawn by the engine.
    InvalidFont(&'static str),
}

impl CharstringError {
    /// A short description, for errors that keep a static reason.
    pub(crate) const fn reason(&self) -> &'static str {
        match self {
            Self::Malformed(why) | Self::InvalidFont(why) => why,
            Self::Limit => "charstring nesting or length limit",
            Self::MissingSubr(_) => "subroutine index out of range",
            Self::NotStandardCode(_) | Self::MissingGlyph(_) => "seac component not in the font",
        }
    }
}
