//! Container boxes that lay out artists relative to each other
//! (matplotlib.offsetbox).
//!
//! An [`OffsetBox`] tree packs texts ([`TextArea`]) and fixed-size areas of
//! artists ([`DrawingArea`]) horizontally or vertically ([`Packer`]),
//! exactly as matplotlib's `HPacker`, `VPacker`, `DrawingArea` and
//! `TextArea` do. Laying a tree out at an origin yields a [`Placement`]
//! for every visible leaf: where its text is anchored, or the transform
//! taking its artists' coordinates (points) to display pixels. The owner
//! of the tree applies the placements to its artists; legends are built
//! this way.
//!
//! [`anchored_bbox`] places a box at one of the nine [`AnchorLoc`]
//! positions of a container, as `AnchoredOffsetbox` and legends do, and
//! [`packed_offsets`] / [`aligned_offsets`] are the one-dimensional
//! packing rules underneath.

mod anchored;
mod boxes;
mod pack;
#[cfg(test)]
mod tests;

use std::fmt;

pub use anchored::{AnchorLoc, UnknownLocation, anchored_bbox};
pub use boxes::{DrawingArea, Leaf, OffsetBox, Packer, Placement, TextArea};
pub use pack::{PackAlign, PackMode, aligned_offsets, packed_offsets};

use crate::artist::ArtistId;
use crate::text::TextError;

/// Why an offset-box tree cannot be laid out.
#[derive(Clone, Debug, PartialEq)]
pub enum OffsetBoxError {
    /// A text area refers to an artist that is not a text of the figure.
    MissingText(ArtistId),
    /// A text cannot be measured (for example malformed math).
    Text(TextError),
}

impl fmt::Display for OffsetBoxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingText(id) => write!(f, "text area artist {id} is not a text"),
            Self::Text(err) => write!(f, "cannot measure text: {err}"),
        }
    }
}

impl std::error::Error for OffsetBoxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Text(err) => Some(err),
            Self::MissingText(_) => None,
        }
    }
}

impl From<TextError> for OffsetBoxError {
    fn from(err: TextError) -> Self {
        Self::Text(err)
    }
}
