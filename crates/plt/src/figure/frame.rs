//! The figure frame artist: the root figure and every subfigure.

use plt_core::{Bbox, Path, Rgba};

use crate::artist::{Artist, ArtistBase, ArtistId, DrawContext};
use crate::transforms::unit_to_bbox;

/// A rectangular region that owns axes and figure-level artists: the
/// figure itself, or a subfigure nested inside it (matplotlib's
/// `FigureBase`).
#[derive(Debug)]
pub(crate) struct FigureFrame {
    base: ArtistBase,
    /// Background colour.
    pub(crate) facecolor: Rgba,
    /// Border colour.
    pub(crate) edgecolor: Rgba,
    /// Border width in points.
    pub(crate) linewidth: f64,
    /// Whether the background and border are drawn.
    pub(crate) frameon: bool,
    /// Extent as a fraction of the parent frame (the unit box for the root).
    pub(crate) relative_extent: Bbox,
    /// Owned artists in insertion order (axes included).
    pub(crate) children: Vec<ArtistId>,
}

impl FigureFrame {
    /// A frame covering `relative_extent` of its parent.
    pub(crate) fn new(relative_extent: Bbox, facecolor: Rgba, edgecolor: Rgba) -> Self {
        Self {
            base: ArtistBase::default(),
            facecolor,
            edgecolor,
            linewidth: 0.0,
            frameon: true,
            relative_extent,
            children: Vec::new(),
        }
    }
}

impl Artist for FigureFrame {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        if self.frameon
            && let Some(extent) = ctx.layout().frame_extent(self.base.id())
        {
            let mut gc = ctx.base_gc(&self.base);
            gc.linewidth = self.linewidth;
            gc.foreground = self.edgecolor;
            ctx.renderer()
                .draw_path(&gc, &Path::unit_rectangle(), &unit_to_bbox(&extent), Some(self.facecolor));
        }
        ctx.draw_children_sorted(&self.children);
    }

    fn children(&self) -> Vec<ArtistId> {
        self.children.clone()
    }

    fn remove_child(&mut self, child: ArtistId) {
        self.children.retain(|&id| id != child);
    }
}
