//! Why a Type 1 font could not be read, changed or drawn.

use std::fmt;

/// A dictionary entry of a Type 1 font that is parsed by its own rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Type1Section {
    /// The `/Subrs` array of subroutines.
    Subrs,
    /// The `/CharStrings` dictionary of glyph programs.
    CharStrings,
    /// The `/Encoding` array.
    Encoding,
    /// The `/OtherSubrs` PostScript procedures.
    OtherSubrs,
}

impl Type1Section {
    /// The dictionary key, without the slash.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Subrs => "Subrs",
            Self::CharStrings => "CharStrings",
            Self::Encoding => "Encoding",
            Self::OtherSubrs => "OtherSubrs",
        }
    }
}

impl fmt::Display for Type1Section {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/{}", self.key())
    }
}

/// Everything that can go wrong with a Type 1 font: the cases where
/// matplotlib's `_type1font` module raises, and the glyphs FreeType
/// refuses to load.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Type1Error {
    /// A PFB segment does not start with the byte 128.
    BrokenPfb(u8),
    /// A PFB segment of a type other than ASCII (1), binary (2) or end of
    /// file (3).
    UnknownSegment(u8),
    /// A PFB segment header is cut short or announces a negative length.
    BadSegmentHeader,
    /// The font has no `eexec` operator.
    MissingEexec,
    /// The font has no `cleartomark` operator.
    MissingCleartomark,
    /// The data ends where the font's three parts are being located.
    TruncatedParts,
    /// The encrypted part is not an even number of hexadecimal digits.
    InvalidHex,
    /// A `(` string that is never closed, starting at the offset.
    UnterminatedString(usize),
    /// A `<` hexadecimal string that is never closed, starting at the
    /// offset.
    UnterminatedHexString(usize),
    /// A `<` hexadecimal string with characters other than hexadecimal
    /// digits and white space, starting at the offset.
    MalformedHexString(usize),
    /// A delimiter that does not belong where it is found, at the offset.
    UnbalancedDelimiter(usize),
    /// The font ends where a token was expected.
    UnexpectedEnd,
    /// The font ends inside the named entry.
    Incomplete(Type1Section),
    /// A token of the wrong kind inside an entry.
    UnexpectedToken {
        /// The entry being read.
        section: Type1Section,
        /// What was expected there.
        expected: &'static str,
        /// The offset of the token found instead.
        at: usize,
    },
    /// A number token that does not read as the integer or real it should
    /// be.
    InvalidNumber(String),
    /// A binary token announced with a negative length that reaches back
    /// before the start of the font.
    NegativeLength(i64),
    /// Negative binary lengths make the parser read the same tokens over
    /// and over (matplotlib never finishes).
    ParseLimit,
    /// The `/Subrs` indices do not cover 0 to the count minus one.
    SubrsIndices,
    /// The `/FontBBox` is not an array of four numbers.
    FontBBox(String),
    /// An entry the operation needs is absent.
    MissingKey(&'static str),
    /// An entry whose value has the wrong type for the operation.
    InvalidValue(&'static str),
    /// A replacement starting in the cleartext part ends in the encrypted
    /// part.
    SpansEexecBoundary,
    /// Text written into the font has characters outside Latin-1.
    NotLatin1,
    /// A glyph name that is not in `/CharStrings`.
    MissingGlyph(String),
    /// A subroutine index that is not in `/Subrs`.
    MissingSubr(i64),
    /// A character code that is not in the Standard Encoding.
    NotStandardCode(i64),
    /// A charstring that cannot be decoded or run.
    BadCharstring(&'static str),
    /// A charstring calls more subroutines than allowed or runs too long.
    CharstringLimit,
    /// A glyph index beyond the font's glyphs.
    GlyphOutOfRange(u16),
    /// The font is not a usable Type 1 font for FreeType.
    InvalidFont(&'static str),
}

impl fmt::Display for Type1Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BrokenPfb(byte) => {
                write!(f, "broken PFB file (expected byte 128, got {byte})")
            }
            Self::UnknownSegment(kind) => write!(f, "unknown segment type {kind} in PFB file"),
            Self::BadSegmentHeader => f.write_str("bad PFB segment header"),
            Self::MissingEexec => f.write_str("no eexec operator in the Type 1 font"),
            Self::MissingCleartomark => f.write_str("no cleartomark operator in the Type 1 font"),
            Self::TruncatedParts => f.write_str("the Type 1 font ends while finding its parts"),
            Self::InvalidHex => f.write_str("the encrypted part is not hexadecimal"),
            Self::UnterminatedString(at) => write!(f, "unterminated string starting at {at}"),
            Self::UnterminatedHexString(at) => {
                write!(f, "unterminated hex string starting at {at}")
            }
            Self::MalformedHexString(at) => write!(f, "malformed hex string starting at {at}"),
            Self::UnbalancedDelimiter(at) => write!(f, "unbalanced delimiter at {at}"),
            Self::UnexpectedEnd => f.write_str("the Type 1 font ends unexpectedly"),
            Self::Incomplete(section) => {
                write!(f, "malformed Type 1 font file: incomplete {section}")
            }
            Self::UnexpectedToken {
                section,
                expected,
                at,
            } => write!(f, "expected {expected} in {section}, found a token at {at}"),
            Self::InvalidNumber(text) => write!(f, "invalid number {text:?}"),
            Self::NegativeLength(length) => write!(f, "negative binary length {length}"),
            Self::ParseLimit => f.write_str("the font's binary lengths make the parser loop"),
            Self::SubrsIndices => {
                f.write_str("malformed Type 1 font file: /Subrs indices do not cover the count")
            }
            Self::FontBBox(value) => write!(f, "FontBBox should be a size-4 array, was {value}"),
            Self::MissingKey(key) => write!(f, "the Type 1 font has no /{key}"),
            Self::InvalidValue(key) => write!(f, "the Type 1 font's /{key} has the wrong type"),
            Self::SpansEexecBoundary => f.write_str("text to be replaced spans the eexec boundary"),
            Self::NotLatin1 => f.write_str("text outside Latin-1 cannot be written to the font"),
            Self::MissingGlyph(name) => write!(f, "the Type 1 font has no glyph {name:?}"),
            Self::MissingSubr(index) => write!(f, "the Type 1 font has no subroutine {index}"),
            Self::NotStandardCode(code) => {
                write!(f, "code {code} is not in the Standard Encoding")
            }
            Self::BadCharstring(why) => write!(f, "bad charstring: {why}"),
            Self::CharstringLimit => f.write_str("charstring exceeds the nesting or run limit"),
            Self::GlyphOutOfRange(glyph) => write!(f, "glyph {glyph} is beyond the font's glyphs"),
            Self::InvalidFont(why) => write!(f, "invalid Type 1 font: {why}"),
        }
    }
}

impl std::error::Error for Type1Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_problem() {
        assert_eq!(
            Type1Error::Incomplete(Type1Section::Subrs).to_string(),
            "malformed Type 1 font file: incomplete /Subrs"
        );
        assert!(Type1Error::BrokenPfb(7).to_string().contains("got 7"));
        let token = Type1Error::UnexpectedToken {
            section: Type1Section::CharStrings,
            expected: "a number",
            at: 12,
        };
        assert!(token.to_string().contains("/CharStrings"));
        assert_eq!(Type1Section::OtherSubrs.key(), "OtherSubrs");
    }
}
