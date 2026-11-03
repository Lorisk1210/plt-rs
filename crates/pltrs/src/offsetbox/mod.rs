//! Container boxes that lay out artists relative to each other
//! (matplotlib.offsetbox).
//!
//! An [`OffsetBox`] tree packs texts ([`TextArea`]), fixed-size areas of
//! artists ([`DrawingArea`]), artists in their own coordinates
//! ([`AuxTransformBox`]) and images ([`OffsetImage`]) horizontally or
//! vertically ([`Packer`]), optionally padded and framed
//! ([`PaddedBox`]), exactly as matplotlib's offset boxes do. Laying a tree
//! out at an origin yields a [`Placement`] for every visible leaf: where
//! its text is anchored, the transform taking its artists' coordinates to
//! display pixels, or the box an image or a frame fills. Legends are built
//! this way.
//!
//! Users build trees whose artists are [`BoxItem`]s and add them to an
//! axes in one of three ways:
//!
//! * anchored at one of nine positions of the axes ([`AnchoredBox`],
//!   matplotlib's `AnchoredOffsetbox` and `AnchoredText`), see
//!   [`Axes::add_anchored_box`](crate::axes::Axes::add_anchored_box);
//! * placed at a point, possibly away from an annotated point with an
//!   arrow pointing to it ([`AnnotationBbox`]), see
//!   [`Axes::add_annotation_bbox`](crate::axes::Axes::add_annotation_bbox)
//!   (or [`Figure::add_annotation_bbox`](crate::figure::Figure::add_annotation_bbox)
//!   for a box of the figure);
//! * with its origin at a display point ([`PlacedOffsetBox`], matplotlib's
//!   plain `OffsetBox` such as a `PaddedBox`), see
//!   [`Axes::add_offset_box`](crate::axes::Axes::add_offset_box).
//!
//! An [`AnchoredBox`] can also be a node of another box's tree
//! ([`AnchoredNode`]): its parent lays it out, but it is drawn at its
//! anchor, as matplotlib's anchored boxes are.
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, BoxItem, DrawingArea};
//! use pltrs::offsetbox::{OffsetBox, PackAlign, Packer, TextArea};
//! use pltrs::patches::{Circle, PatchProps};
//! use pltrs::text::TextProps;
//! let ax = Figure::new().subplot();
//! let dot = BoxItem::patch(Circle::new((5.0, 5.0), 5.0), PatchProps::default().facecolor("r"));
//! let child = OffsetBox::HPacker(Packer::new(0.0, 5.0, PackAlign::Center, vec![
//!     TextArea::from_text("red dot:", TextProps::default()).into(),
//!     DrawingArea::new(10.0, 10.0, 0.0, 0.0).with_artist(dot).into(),
//! ]));
//! let anchored = ax.add_anchored_box(AnchoredBox::new(child, AnchoredOptions::new(AnchorLoc::UpperLeft)));
//! assert_eq!(anchored.texts()[0].text(), "red dot:");
//! ```
//!
//! [`anchored_bbox`] places a box at one of the nine [`AnchorLoc`]
//! positions of a container, and [`packed_offsets`] /
//! [`aligned_offsets`] are the one-dimensional packing rules underneath.

mod anchored;
mod anchored_box;
mod anchored_node;
mod annotation_bbox;
mod apply;
mod boxes;
mod drag;
pub(crate) use drag::reconnect_drag;
mod leaves;
mod pack;
mod pending;
mod placed_box;
#[cfg(test)]
mod tests;

use std::fmt;

pub(crate) use anchored::transformed_bbox;
pub use anchored::{AnchorLoc, BboxToAnchor, UnknownLocation, anchored_bbox};
pub(crate) use anchored_box::AnchoredBoxArtist;
pub use anchored_box::{AnchoredBox, AnchoredOffsetbox, AnchoredOptions};
pub use anchored_node::AnchoredNode;
pub(crate) use annotation_bbox::AnnotationBboxArtist;
pub use annotation_bbox::{AnnotationBbox, AnnotationBboxOptions};
pub(crate) use apply::{apply_placements, lay_out_box, lay_out_boxes};
pub use boxes::{OffsetBox, Packer, Placement};
pub use leaves::{AuxTransformBox, DrawingArea, OffsetImage, PaddedBox, TextArea};
pub use pack::{PackAlign, PackMode, aligned_offsets, packed_offsets};
pub use pending::BoxItem;
pub(crate) use placed_box::PlacedBoxArtist;
pub use placed_box::PlacedOffsetBox;

use crate::artist::{ArtistId, ArtistStore};
use crate::coords::CoordError;
use crate::text::TextError;
use crate::transforms::Transform;

/// How the leaves of an offset-box tree refer to the artists they lay
/// out.
pub trait ArtistRef: Clone {
    /// The artist referred to.
    fn artist_id(&self) -> ArtistId;

    /// Makes the artist's coordinates map to display pixels through
    /// `transform`.
    fn attach(&self, store: &mut ArtistStore, transform: Transform) {
        if let Some(artist) = store.get_mut(self.artist_id()) {
            artist.base_mut().transform = Some(transform);
        }
    }
}

impl ArtistRef for ArtistId {
    fn artist_id(&self) -> ArtistId {
        *self
    }
}

/// Why an offset-box tree cannot be laid out.
#[derive(Clone, Debug, PartialEq)]
pub enum OffsetBoxError {
    /// A text area refers to an artist that is not a text of the figure.
    MissingText(ArtistId),
    /// An offset image refers to an artist that is not an image of the
    /// figure.
    MissingImage(ArtistId),
    /// A text cannot be measured (for example malformed math).
    Text(TextError),
    /// A point of an annotation box cannot be located.
    Coords(CoordError),
    /// The axes or the anchor box the box is placed in no longer exists.
    Detached,
    /// An anchored text was given a vertical alignment, which text areas
    /// cannot honour (matplotlib refuses it too).
    VerticalAlignment,
    /// The frame given to a box is not a fancy box patch.
    NotAFrame,
}

impl fmt::Display for OffsetBoxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingText(id) => write!(f, "text area artist {id} is not a text"),
            Self::MissingImage(id) => write!(f, "offset image artist {id} is not an image"),
            Self::Text(err) => write!(f, "cannot measure text: {err}"),
            Self::Coords(err) => write!(f, "cannot place the annotation box: {err}"),
            Self::Detached => f.write_str("the box's axes or anchor no longer exists"),
            Self::VerticalAlignment => {
                f.write_str("mixing a vertical alignment with an anchored text is not supported")
            }
            Self::NotAFrame => f.write_str("the frame of a box must be a FancyBboxPatch"),
        }
    }
}

impl std::error::Error for OffsetBoxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Text(err) => Some(err),
            Self::Coords(err) => Some(err),
            Self::MissingText(_)
            | Self::MissingImage(_)
            | Self::Detached
            | Self::VerticalAlignment
            | Self::NotAFrame => None,
        }
    }
}

impl From<TextError> for OffsetBoxError {
    fn from(err: TextError) -> Self {
        Self::Text(err)
    }
}

impl From<CoordError> for OffsetBoxError {
    fn from(err: CoordError) -> Self {
        Self::Coords(err)
    }
}
