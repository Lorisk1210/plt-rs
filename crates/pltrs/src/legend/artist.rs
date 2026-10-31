//! The legend artist: a frame, a title and one handle plus label per
//! entry, laid out by an offset-box tree (matplotlib's `Legend`).

use pltrs_core::{Bbox, Point};

use super::OutsideEdge;
use super::handlers::{Attach, HandlerMap};
use super::settings::{Location, Settings};
use super::{LegendAlignment, LegendMode, LegendShadowStyle};
use crate::artist::{Artist, ArtistBase, ArtistId, ArtistStore, DrawContext, LayoutContext};
use crate::collections::CollectionArtist;
use crate::events::Dragger;
use crate::offsetbox::{ArtistRef, OffsetBox};
use crate::patches::{FancyBboxPatch, PatchArtist};
use crate::transforms::Transform;

/// What a legend is placed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LegendParent {
    /// An axes (`ax.legend()`).
    Axes(ArtistId),
    /// A figure or subfigure frame (`fig.legend()`).
    Figure(ArtistId),
}

pltrs_core::archive_enum!(LegendParent {
    Axes(v0),
    Figure(v0),
});

impl LegendParent {
    /// The parent's id.
    pub(crate) fn id(self) -> ArtistId {
        match self {
            Self::Axes(id) | Self::Figure(id) => id,
        }
    }

    /// The parent's fraction coordinates (matplotlib's
    /// `BboxTransformTo(parent.bbox)`).
    pub(crate) fn fractions(self) -> Transform {
        match self {
            Self::Axes(id) => Transform::Axes(id),
            Self::Figure(id) => Transform::Figure(id),
        }
    }
}

/// An artist of the legend box (a handle or a text) and how it follows
/// the box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HandleRef {
    pub(crate) id: ArtistId,
    pub(crate) attach: Attach,
}

pltrs_core::archive_struct!(HandleRef { id, attach });

impl HandleRef {
    /// A text of the legend, which the box moves by its anchor.
    pub(crate) fn text(id: ArtistId) -> Self {
        Self {
            id,
            attach: Attach::Transform,
        }
    }
}

impl ArtistRef for HandleRef {
    fn artist_id(&self) -> ArtistId {
        self.id
    }

    /// Collections drawn at offsets follow the box through their offset
    /// transform, everything else through its transform.
    fn attach(&self, store: &mut ArtistStore, transform: Transform) {
        match self.attach {
            Attach::Transform => {
                if let Some(artist) = store.get_mut(self.id) {
                    artist.base_mut().transform = Some(transform);
                }
            }
            Attach::Offsets => {
                if let Some(collection) = store.get_as_mut::<CollectionArtist>(self.id) {
                    collection.offset_transform = Some(transform);
                }
            }
        }
    }
}

/// State of a legend (matplotlib's `Legend`). Its parts are artists of
/// the figure owned by the legend; the layout pass places them.
#[derive(Debug)]
pub(crate) struct LegendArtist {
    pub(crate) base: ArtistBase,
    pub(crate) parent: LegendParent,
    pub(crate) settings: Settings,
    pub(crate) location: Location,
    /// Whether the location is the default `legend.loc` rather than one
    /// the program chose (matplotlib's `_loc_used_default`): only then
    /// does a slow search for the best location warn.
    pub(crate) loc_used_default: bool,
    /// The edge an `"outside ..."` figure legend makes room at.
    pub(crate) outside: Option<OutsideEdge>,
    pub(crate) mode: LegendMode,
    /// `bbox_to_anchor` in the coordinates of its transform.
    pub(crate) anchor: Option<(Bbox, Transform)>,
    /// The title and the columns of entries (matplotlib's `_legend_box`);
    /// the title is the first child.
    pub(crate) legend_box: OffsetBox<HandleRef>,
    /// How the title and the block of entries line up (the alignment of
    /// `legend_box`).
    pub(crate) alignment: LegendAlignment,
    /// The frame (matplotlib's `legendPatch`).
    pub(crate) frame: ArtistId,
    /// The frame's shadow and its style, if any.
    pub(crate) shadow: Option<(ArtistId, LegendShadowStyle)>,
    /// The title text.
    pub(crate) title: ArtistId,
    /// One label per entry.
    pub(crate) texts: Vec<ArtistId>,
    /// The first handle artist of every listed handle, `None` for handles
    /// no handler draws (matplotlib's `legend_handles`).
    pub(crate) handles: Vec<Option<ArtistId>>,
    /// Every part, for removal with the legend.
    pub(crate) parts: Vec<ArtistId>,
    /// Whether the legend stays on its axes as an ordinary artist
    /// (matplotlib's `ax.add_artist(legend)`): a new axes legend does not
    /// replace it and it is no longer the axes' legend.
    pub(crate) kept: bool,
    /// The callbacks dragging the legend, while it is draggable.
    pub(crate) dragger: Option<Dragger>,
    /// The handlers the legend was given on top of the default ones
    /// (matplotlib's `_custom_handler_map`).
    pub(crate) handler_map: HandlerMap,
}

pltrs_core::archive_struct!(LegendArtist {
    base,
    parent,
    settings,
    location,
    loc_used_default,
    outside,
    mode,
    anchor,
    legend_box,
    alignment,
    frame,
    shadow,
    title,
    texts,
    handles,
    parts,
    kept,
    dragger,
    handler_map
});

impl LegendArtist {
    /// The frame's shadow, if any.
    pub(crate) fn shadow_id(&self) -> Option<ArtistId> {
        self.shadow.as_ref().map(|(id, _)| *id)
    }

    /// The title box (the first child of the legend box).
    pub(crate) fn title_box_mut(&mut self) -> Option<&mut OffsetBox<HandleRef>> {
        match &mut self.legend_box {
            OffsetBox::VPacker(packer) => packer.children.first_mut(),
            _ => None,
        }
    }

    /// Lines up the title and the block of entries as `alignment` says
    /// (matplotlib's `set_alignment`).
    pub(crate) fn set_alignment(&mut self, alignment: LegendAlignment) {
        self.alignment = alignment;
        if let OffsetBox::VPacker(packer) = &mut self.legend_box {
            packer.align = alignment.into();
        }
    }
}

impl Artist for LegendArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    /// Draws the shadow, the frame, then the title and the entries in
    /// box order (matplotlib's `Legend.draw`).
    fn draw(&self, ctx: &mut DrawContext<'_>) {
        ctx.renderer()
            .open_group("legend", self.base.gid.as_deref());
        if let Some(shadow) = self.shadow_id() {
            ctx.draw_child(shadow);
        }
        ctx.draw_child(self.frame);
        for artist in self.legend_box.artists() {
            ctx.draw_child(artist.id);
        }
        ctx.renderer().close_group("legend");
    }

    fn children(&self) -> Vec<ArtistId> {
        self.parts.clone()
    }

    fn remove_child(&mut self, child: ArtistId) {
        self.parts.retain(|&id| id != child);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        self.extent(*ctx).ok()
    }

    /// matplotlib's `Legend.contains`: on the frame.
    fn contains(&self, ctx: &LayoutContext<'_>, point: Point) -> bool {
        ctx.store()
            .get_as::<PatchArtist<FancyBboxPatch>>(self.frame)
            .is_some_and(|frame| frame.contains_unpicked(*ctx, point))
    }
}
