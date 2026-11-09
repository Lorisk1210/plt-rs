//! How an anchored box sits in the box it is anchored to ([`Anchoring`],
//! shared by the anchored boxes added to an axes or a figure and those
//! inside another box's tree), and anchored boxes as nodes of another
//! box's tree (matplotlib's `AnchoredOffsetbox` as the child of a
//! `PaddedBox` or a packer).

use pltrs_core::{Bbox, Point};

use super::anchored_box::{AnchoredBox, AnchoredOptions, AnchoredParts};
use super::{
    AnchorLoc, ArtistRef, BoxItem, OffsetBox, OffsetBoxError, Placement, TextArea, TreeLayout,
    anchored_bbox, transformed_bbox,
};
use crate::artist::{ArtistId, LayoutContext};
use crate::fonts::FontProperties;
use crate::text::TextArtist;
use crate::transforms::Transform;

/// Where an anchored box goes in the box it is anchored to (matplotlib's
/// `AnchoredOffsetbox` settings `loc`, `pad`, `borderpad`,
/// `bbox_to_anchor` and `bbox_transform`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Anchoring {
    /// Where the box goes within its anchor.
    pub(crate) loc: AnchorLoc,
    /// Padding around the child, as a fraction of the font size.
    pub(crate) pad: f64,
    /// Padding between the frame and the anchor, horizontally and
    /// vertically, as fractions of the font size.
    pub(crate) borderpad: (f64, f64),
    /// `bbox_to_anchor` with its coordinates; `None` anchors in the axes
    /// or figure the box belongs to.
    pub(crate) anchor: Option<(Bbox, Transform)>,
}

pltrs_core::archive_struct!(Anchoring {
    loc,
    pad,
    borderpad,
    anchor
});

/// Where an anchored box and its parts go.
pub(crate) struct AnchoredLayout<A> {
    /// The framed box, in display pixels.
    pub(crate) extent: Bbox,
    /// Where the frame, then the child's artists go, and where the origin
    /// of each box of the child went (matplotlib's `get_offset` for the
    /// child itself).
    pub(crate) tree: TreeLayout<A>,
}

impl From<&AnchoredOptions> for Anchoring {
    fn from(options: &AnchoredOptions) -> Self {
        Self {
            loc: options.loc,
            pad: options.pad,
            borderpad: options.borderpad,
            anchor: options.bbox_to_anchor.map(|anchor| {
                let transform = options.bbox_transform.clone().unwrap_or(Transform::Display);
                (anchor.bbox(), transform)
            }),
        }
    }
}

impl Anchoring {
    /// The display box the box is anchored in (matplotlib's
    /// `get_bbox_to_anchor`): `bbox_to_anchor`, else the box of `owner`,
    /// its axes or figure.
    fn anchor_box(&self, ctx: LayoutContext<'_>, owner: ArtistId) -> Option<Bbox> {
        match &self.anchor {
            None => ctx.axes_extent(owner).or_else(|| ctx.frame_extent(owner)),
            Some((bbox, transform)) => Some(transformed_bbox(bbox, &ctx.resolve(transform)?)),
        }
    }

    /// Where the padded child box `bbox`'s origin goes for the box to sit
    /// at `loc` in its anchor (matplotlib's `get_offset`).
    fn offset(
        &self,
        ctx: LayoutContext<'_>,
        owner: ArtistId,
        bbox: &Bbox,
        fontsize: f64,
    ) -> Result<Point, OffsetBoxError> {
        let anchor = self
            .anchor_box(ctx, owner)
            .ok_or(OffsetBoxError::Detached)?;
        let corner = anchored_bbox(
            self.loc,
            &Bbox::from_bounds(0.0, 0.0, bbox.width(), bbox.height()),
            &anchor,
            self.borderpad.0 * fontsize,
            self.borderpad.1 * fontsize,
        );
        Ok(corner - bbox.p0)
    }

    /// The extent of `child` padded as the box pads it, around its origin
    /// (matplotlib's `AnchoredOffsetbox.get_bbox`), for paddings that are
    /// fractions of `fontsize` pixels.
    fn padded<A: ArtistRef>(
        &self,
        ctx: LayoutContext<'_>,
        child: &OffsetBox<A>,
        fontsize: f64,
    ) -> Result<Bbox, OffsetBoxError> {
        let pad = self.pad * fontsize;
        Ok(child.bbox(ctx)?.padded(pad, pad))
    }

    /// Lays `framed` out in `owner`, its axes or figure: at its pinned
    /// place when the box was dragged there, else at its anchor
    /// (matplotlib's `AnchoredOffsetbox.draw`). The frame and the child's
    /// parts are placed into `layout`, where the child is the box
    /// `child_index`; returns the framed box.
    pub(crate) fn lay_out_into<A: ArtistRef>(
        &self,
        ctx: LayoutContext<'_>,
        owner: ArtistId,
        framed: Framed<'_, A>,
        layout: &mut TreeLayout<A>,
        child_index: usize,
    ) -> Result<Bbox, OffsetBoxError> {
        let Framed {
            child,
            frame,
            fontsize,
            pinned,
        } = framed;
        let bbox = self.padded(ctx, child, fontsize)?;
        let offset = pinned.map_or_else(|| self.offset(ctx, owner, &bbox, fontsize), Ok)?;
        let extent = bbox.translated(offset.x, offset.y);
        layout.placements.push(Placement::Frame {
            frame: frame.clone(),
            bbox: extent,
            mutation_scale: Some(fontsize),
        });
        child.place_into(ctx, offset, None, layout, child_index)?;
        Ok(extent)
    }
}

/// An anchored box as [`Anchoring::lay_out_into`] lays it out.
pub(crate) struct Framed<'a, A> {
    /// The child box.
    pub(crate) child: &'a OffsetBox<A>,
    /// The frame (a `FancyBboxPatch`).
    pub(crate) frame: &'a A,
    /// The font size the paddings are fractions of, in pixels.
    pub(crate) fontsize: f64,
    /// Where the child's origin was pinned by dragging, in display pixels.
    pub(crate) pinned: Option<Point>,
}

/// An anchored box inside another box's tree (matplotlib's
/// `AnchoredOffsetbox` as a child of a `PaddedBox` or a packer, made from
/// an [`AnchoredBox`]): its parent lays it out with its padded extent,
/// but it is drawn at its anchor in the axes or figure the tree belongs
/// to, wherever the parent puts it, as matplotlib's anchored boxes ignore
/// the offset their parent gives them.
#[derive(Clone, Debug, PartialEq)]
pub struct AnchoredNode<A> {
    /// The anchored box (matplotlib's `get_child()`).
    pub(crate) child: Box<OffsetBox<A>>,
    /// The frame (a `FancyBboxPatch`).
    pub(crate) frame: A,
    /// Where it goes in its anchor.
    pub(crate) anchoring: Anchoring,
    /// The font size its paddings are fractions of, in points; `None`
    /// follows the text of its child text area (anchored texts).
    pub(crate) fontsize: Option<f64>,
    /// The axes or figure the tree belongs to, once it joined one.
    pub(crate) owner: Option<ArtistId>,
    /// Whether the box takes part in its parent's layout.
    pub(crate) visible: bool,
}

pltrs_core::archive_struct!(AnchoredNode<A> {
    child,
    frame,
    anchoring,
    fontsize,
    owner,
    visible
});

impl From<AnchoredBox> for AnchoredNode<BoxItem> {
    fn from(spec: AnchoredBox) -> Self {
        let AnchoredParts {
            child,
            frame,
            options,
            follows_text,
        } = spec.into_parts();
        let fontsize = (!follows_text).then(|| pad_font_points(options.prop.as_ref()));
        Self {
            child: Box::new(child),
            frame,
            anchoring: Anchoring::from(&options),
            fontsize,
            owner: None,
            visible: true,
        }
    }
}

impl From<AnchoredBox> for OffsetBox<BoxItem> {
    /// The anchored box as a node of another box's tree.
    fn from(spec: AnchoredBox) -> Self {
        Self::Anchored(spec.into())
    }
}

/// The size in points the paddings of an anchored box with font `prop`
/// are fractions of: its size, else `legend.fontsize`.
pub(crate) fn pad_font_points(prop: Option<&FontProperties>) -> f64 {
    prop.map_or_else(
        || {
            let rc = crate::rc::params();
            rc.legend_fontsize.to_points(rc.font_size)
        },
        |prop| prop.size,
    )
}

impl<A> AnchoredNode<A> {
    /// `child` anchored as `options` say, framed by `frame` (a
    /// `FancyBboxPatch`): an anchored box as a node of another box's tree
    /// whose artists are given (handles of loose artists, say), its
    /// paddings fractions of the size of `options.prop`.
    pub fn new(child: OffsetBox<A>, options: &AnchoredOptions, frame: A) -> Self {
        Self {
            child: Box::new(child),
            frame,
            anchoring: Anchoring::from(options),
            fontsize: Some(pad_font_points(options.prop.as_ref())),
            owner: None,
            visible: true,
        }
    }

    /// The same node with every artist reference replaced by `f` of it,
    /// the frame first.
    pub(super) fn try_map<B, E>(
        self,
        f: &mut impl FnMut(A) -> Result<B, E>,
    ) -> Result<AnchoredNode<B>, E> {
        let frame = f(self.frame)?;
        Ok(AnchoredNode {
            child: Box::new(self.child.try_map(f)?),
            frame,
            anchoring: self.anchoring,
            fontsize: self.fontsize,
            owner: self.owner,
            visible: self.visible,
        })
    }
}

impl<A: ArtistRef> AnchoredNode<A> {
    /// The font size the paddings are fractions of, in pixels.
    fn fontsize(&self, ctx: LayoutContext<'_>) -> Result<f64, OffsetBoxError> {
        let points = match (self.fontsize, &*self.child) {
            (Some(points), _) => points,
            (None, OffsetBox::TextArea(TextArea { text, .. })) => {
                let id = text.artist_id();
                ctx.store()
                    .get_as::<TextArtist>(id)
                    .ok_or(OffsetBoxError::MissingText(id))?
                    .font
                    .size
            }
            (None, _) => pad_font_points(None),
        };
        Ok(ctx.measuring_points_to_pixels(points))
    }

    /// The padded extent of the child around its origin (matplotlib's
    /// `AnchoredOffsetbox.get_bbox`), which the parent lays out.
    pub(super) fn bbox(&self, ctx: LayoutContext<'_>) -> Result<Bbox, OffsetBoxError> {
        self.anchoring.padded(ctx, &self.child, self.fontsize(ctx)?)
    }

    /// Places the frame and the child's artists into `layout`, where the
    /// child is the box `child_index`: at the anchor, whatever the
    /// parent's offset.
    pub(super) fn place_into(
        &self,
        ctx: LayoutContext<'_>,
        layout: &mut TreeLayout<A>,
        child_index: usize,
    ) -> Result<(), OffsetBoxError> {
        let owner = self.owner.ok_or(OffsetBoxError::Detached)?;
        let framed = Framed {
            child: &self.child,
            frame: &self.frame,
            fontsize: self.fontsize(ctx)?,
            pinned: None,
        };
        self.anchoring
            .lay_out_into(ctx, owner, framed, layout, child_index)?;
        Ok(())
    }
}
