//! Why a removed artist could not be added back to its figure.

use std::fmt;

/// Why [`Axes::attach`](crate::axes::Axes::attach) or
/// [`Figure::attach`](crate::figure::Figure::attach) could not add an
/// artist back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttachError {
    /// The artist was not removed: it is still in its figure (matplotlib
    /// refuses to put an artist in a second axes).
    NotRemoved,
    /// The artist was removed together with an artist that owns it; it
    /// comes back with that one.
    OwnedByRemoved,
    /// The artist belongs to another figure.
    OtherFigure,
    /// The artist cannot go there: figures take any artist, axes any but
    /// figures and 3-D axes, and no artist takes one that owns it.
    Misplaced,
}

impl fmt::Display for AttachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotRemoved => "the artist is in its figure; remove it first",
            Self::OwnedByRemoved => "the artist was removed with the artist that owns it",
            Self::OtherFigure => "the artist belongs to another figure",
            Self::Misplaced => "the artist cannot be added there",
        })
    }
}

impl std::error::Error for AttachError {}
