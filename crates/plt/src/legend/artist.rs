//! The legend artist: a frame, a title and one handle plus label per
//! entry, laid out by an offset-box tree (matplotlib's `Legend`).

use plt_core::Bbox;

use super::LegendMode;
use super::handlers::Attach;
use super::settings::{Location, Settings};
use crate::artist::{Artist, ArtistBase, ArtistId, DrawContext, LayoutContext};
use crate::offsetbox::{Leaf, OffsetBox};
use crate::transforms::Transform;

/// What a legend is placed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LegendParent {
    /// An axes (`ax.legend()`).
    Axes(ArtistId),
    /// A figure or subfigure frame (`fig.legend()`).
    Figure(ArtistId),
}

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

/// An artist of a handle box and how it follows the box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HandleRef {
    pub(crate) id: ArtistId,
    pub(crate) attach: Attach,
}

/// State of a legend (matplotlib's `Legend`). Its parts are artists of
/// the figure owned by the legend; the layout pass places them.
#[derive(Debug)]
pub(crate) struct LegendArtist {
    pub(crate) base: ArtistBase,
    pub(crate) parent: LegendParent,
    pub(crate) settings: Settings,
    pub(crate) location: Location,
    pub(crate) mode: LegendMode,
    /// `bbox_to_anchor` in the coordinates of its transform.
    pub(crate) anchor: Option<(Bbox, Transform)>,
    /// The title and the columns of entries (matplotlib's `_legend_box`);
    /// the title is the first child.
    pub(crate) legend_box: OffsetBox<HandleRef>,
    /// The frame (matplotlib's `legendPatch`).
    pub(crate) frame: ArtistId,
    /// The frame's shadow, if any.
    pub(crate) shadow: Option<ArtistId>,
    /// The title text.
    pub(crate) title: ArtistId,
    /// One label per entry.
    pub(crate) texts: Vec<ArtistId>,
    /// Every part, for removal with the legend.
    pub(crate) parts: Vec<ArtistId>,
}

impl LegendArtist {
    /// The title box (the first child of the legend box).
    pub(crate) fn title_box_mut(&mut self) -> Option<&mut OffsetBox<HandleRef>> {
        match &mut self.legend_box {
            OffsetBox::VPacker(packer) => packer.children.first_mut(),
            _ => None,
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
        if let Some(shadow) = self.shadow {
            ctx.draw_child(shadow);
        }
        ctx.draw_child(self.frame);
        for leaf in self.legend_box.leaves() {
            match leaf {
                Leaf::Text(id) => ctx.draw_child(id),
                Leaf::Area(artists) => {
                    for artist in artists {
                        ctx.draw_child(artist.id);
                    }
                }
            }
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
        self.layout(*ctx).ok().map(|layout| layout.extent)
    }
}
