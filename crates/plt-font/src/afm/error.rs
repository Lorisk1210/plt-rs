//! Why Adobe Font Metrics could not be read or used.

use std::fmt;

/// The part of an AFM file a problem was found in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AfmSection {
    /// The global font information before `StartCharMetrics`.
    Header,
    /// The `StartCharMetrics` ... `EndCharMetrics` block.
    CharMetrics,
    /// The `StartKernPairs` ... `EndKernPairs` block.
    KernPairs,
    /// The `StartComposites` ... `EndComposites` block.
    Composites,
}

impl fmt::Display for AfmSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Header => "header",
            Self::CharMetrics => "character metrics",
            Self::KernPairs => "kerning pairs",
            Self::Composites => "composites",
        })
    }
}

/// Everything that can go wrong with an AFM file: the cases where
/// matplotlib's `_afm` module raises.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AfmError {
    /// The first line does not start with `StartFontMetrics`.
    NotAfm,
    /// The data ends inside `section`.
    UnexpectedEnd(AfmSection),
    /// Line `line` (1-based) of `section` cannot be read.
    BadLine {
        /// The section the line belongs to.
        section: AfmSection,
        /// The line number in the file.
        line: usize,
    },
    /// A header entry the operation needs is absent (or was unreadable).
    MissingField(&'static str),
    /// A glyph the operation needs is not in the font.
    MissingGlyph(String),
    /// No AFM file of this name is bundled.
    NotBundled(String),
}

impl fmt::Display for AfmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAfm => f.write_str("not an AFM file"),
            Self::UnexpectedEnd(section) => write!(f, "AFM data ends inside the {section}"),
            Self::BadLine { section, line } => {
                write!(f, "bad AFM {section} line {line}")
            }
            Self::MissingField(key) => write!(f, "the AFM header has no {key}"),
            Self::MissingGlyph(name) => write!(f, "the AFM font has no glyph {name:?}"),
            Self::NotBundled(file) => write!(f, "the AFM metrics {file} are not bundled"),
        }
    }
}

impl std::error::Error for AfmError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_problem() {
        let err = AfmError::BadLine {
            section: AfmSection::KernPairs,
            line: 12,
        };
        assert_eq!(err.to_string(), "bad AFM kerning pairs line 12");
        assert!(
            AfmError::MissingField("Ascender")
                .to_string()
                .contains("Ascender")
        );
        assert!(
            AfmError::UnexpectedEnd(AfmSection::Composites)
                .to_string()
                .contains("composites")
        );
    }
}
