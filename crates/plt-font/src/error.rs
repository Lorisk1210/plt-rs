//! The crate-wide error type.

use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::hinting::{BytecodeError, BytecodeProgram};
use crate::types::Tag;

/// Pseudo-tag used for errors in the font file header / table directory.
pub(crate) const DIRECTORY: Tag = Tag::new(b"sfnt");

/// Everything that can go wrong while reading or managing fonts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontError {
    /// The data does not start with a known sfnt or collection signature.
    UnknownFormat(u32),
    /// A collection was asked for a face it does not contain.
    FaceIndexOutOfRange {
        /// The requested face index.
        index: u32,
        /// Number of faces in the collection.
        count: u32,
    },
    /// A table required for the operation is absent.
    MissingTable(Tag),
    /// A structure extends beyond the end of its table.
    Truncated(Tag),
    /// A table is structurally invalid.
    Malformed {
        /// The table containing the problem.
        table: Tag,
        /// What is wrong.
        reason: &'static str,
    },
    /// A table uses a version or subtable format this crate does not read.
    UnsupportedFormat {
        /// The table containing the format field.
        table: Tag,
        /// The unsupported format or version number.
        format: u32,
    },
    /// A character map index beyond the face's usable subtables.
    CharmapOutOfRange {
        /// The requested index.
        index: usize,
        /// Number of usable character maps.
        count: usize,
    },
    /// Reading a font file from disk failed.
    Io {
        /// The file being read.
        path: PathBuf,
        /// The I/O error category.
        kind: io::ErrorKind,
    },
    /// No font in a database matched a query and fallback was disabled.
    NoMatchingFont(String),
    /// A font id that does not belong to the database.
    UnknownFont(usize),
    /// The font or control value program of a TrueType font failed, so
    /// FreeType refuses to load its glyphs hinted.
    Bytecode {
        /// The program that failed.
        program: BytecodeProgram,
        /// Why it stopped.
        error: BytecodeError,
    },
}

impl FontError {
    pub(crate) const fn malformed(table: Tag, reason: &'static str) -> Self {
        Self::Malformed { table, reason }
    }
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFormat(magic) => write!(f, "unknown font format (magic {magic:#010x})"),
            Self::FaceIndexOutOfRange { index, count } => {
                write!(
                    f,
                    "face index {index} out of range (collection has {count} faces)"
                )
            }
            Self::MissingTable(tag) => write!(f, "missing required table '{tag}'"),
            Self::Truncated(tag) => write!(f, "table '{tag}' is truncated"),
            Self::Malformed { table, reason } => write!(f, "malformed table '{table}': {reason}"),
            Self::UnsupportedFormat { table, format } => {
                write!(f, "unsupported format {format} in table '{table}'")
            }
            Self::CharmapOutOfRange { index, count } => {
                write!(f, "charmap index {index} out of range (face has {count})")
            }
            Self::Io { path, kind } => write!(f, "cannot read {}: {kind}", path.display()),
            Self::NoMatchingFont(query) => write!(f, "no font matches {query}"),
            Self::UnknownFont(id) => write!(f, "unknown font id {id}"),
            Self::Bytecode { program, error } => write!(f, "the {program} failed: {error}"),
        }
    }
}

impl std::error::Error for FontError {}

/// Convenience alias used throughout the crate.
pub type Result<T, E = FontError> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_mention_the_table() {
        let err = FontError::Truncated(Tag::new(b"glyf"));
        assert_eq!(err.to_string(), "table 'glyf' is truncated");
        let err = FontError::malformed(Tag::new(b"cmap"), "bad segment count");
        assert!(err.to_string().contains("cmap"));
        let err = FontError::FaceIndexOutOfRange { index: 3, count: 2 };
        assert!(err.to_string().contains('3'));
        let err = FontError::Bytecode {
            program: BytecodeProgram::ControlValue,
            error: BytecodeError::StackOverflow,
        };
        assert!(err.to_string().contains("prep"));
    }
}
