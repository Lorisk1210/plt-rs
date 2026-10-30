//! Why a value could not be archived or read back.

use std::fmt;

/// Why a value could not be archived or read back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArchiveError {
    /// The bytes do not start as an archive does.
    NotAnArchive,
    /// The archive was written by another version of pltrs.
    Version {
        /// The version (and format) that wrote it.
        found: String,
        /// This version (and format).
        expected: String,
    },
    /// The bytes end before the value they hold.
    UnexpectedEnd,
    /// The bytes do not hold a value of the expected type.
    Invalid(String),
    /// Values nest deeper than [`MAX_DEPTH`](super::MAX_DEPTH).
    TooDeep,
    /// The value holds something that cannot be archived: a function
    /// without an origin, or a type of the program's that the program
    /// does not archive.
    Unsupported(String),
}

impl ArchiveError {
    /// An [`Invalid`](Self::Invalid) error saying what is wrong.
    pub fn invalid(what: impl Into<String>) -> Self {
        Self::Invalid(what.into())
    }

    /// An [`Unsupported`](Self::Unsupported) error naming what cannot be
    /// archived.
    pub fn unsupported(what: impl Into<String>) -> Self {
        Self::Unsupported(what.into())
    }
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAnArchive => f.write_str("not a pltrs archive"),
            Self::Version { found, expected } => write!(
                f,
                "the archive was written by pltrs {found}, which this pltrs {expected} cannot read"
            ),
            Self::UnexpectedEnd => f.write_str("the archive ends unexpectedly"),
            Self::Invalid(what) => write!(f, "malformed archive: {what}"),
            Self::TooDeep => f.write_str("the archive nests values too deeply"),
            Self::Unsupported(what) => write!(f, "cannot archive {what}"),
        }
    }
}

impl std::error::Error for ArchiveError {}
