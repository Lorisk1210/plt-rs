//! The figure frame artist: the root figure and every subfigure.

use plt_core::{Bbox, Path, Rgba};
use plt_render::JoinStyle;

use crate::artist::{Artist, ArtistBase, ArtistId, DrawContext};
use crate::gridspec::{SubplotParams, SubplotSpec};
use crate::image::Compositor;
use crate::patheffects::with_path_effects;
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
    /// Margins and spacing of subplot grids in this frame.
    pub(crate) subplot_params: SubplotParams,
    /// The figure title and super-labels, once created.
    pub(crate) super_texts: SuperTexts,
    /// For subfigures: the cells of the parent's grid covered (matplotlib's
    /// `SubFigure._subplotspec`).
    pub(crate) subplot_spec: Option<SubplotSpec>,
}

/// The texts spanning a whole figure (matplotlib's `_suptitle`,
/// `_supxlabel`, `_supylabel`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SuperTexts {
    pub(crate) title: Option<ArtistId>,
    pub(crate) xlabel: Option<ArtistId>,
    pub(crate) ylabel: Option<ArtistId>,
}

impl FigureFrame {
    /// A frame covering `relative_extent` of its parent.
    pub(crate) fn new(
        relative_extent: Bbox,
        facecolor: Rgba,
        edgecolor: Rgba,
        subplot_params: SubplotParams,
    ) -> Self {
        Self {
            base: ArtistBase::default(),
            facecolor,
            edgecolor,
            linewidth: 0.0,
            frameon: true,
            relative_extent,
            children: Vec::new(),
            subplot_params,
            super_texts: SuperTexts::default(),
            subplot_spec: None,
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
            && let Some(extent) = ctx.layout().frame_patch_extent(self.base.id())
        {
            // The figure patch is a `Rectangle`: mitred, without an edge
            // when it has no colour, unfilled when transparent, and drawn
            // without anti-aliasing.
            let mut gc = ctx.base_gc(&self.base);
            gc.linewidth = if self.edgecolor.is_transparent() {
                0.0
            } else {
                self.linewidth
            };
            gc.foreground = self.edgecolor;
            gc.join = JoinStyle::Miter;
            gc.antialiased = false;
            let face = (!self.facecolor.is_transparent()).then_some(self.facecolor);
            ctx.draw_artist(false, |ctx| {
                with_path_effects(ctx, &self.base.path_effects, |ctx| {
                    ctx.renderer().draw_path(
                        &gc,
                        &Path::unit_rectangle(),
                        &unit_to_bbox(&extent),
                        face,
                    );
                });
            });
        }
        let mut children = ctx.sorted_children(&self.children);
        children.retain(|&id| !ctx.layout().skips_animated(id, false));
        let clip = ctx.layout().frame_extent(self.base.id());
        let mut compositor = Compositor::new(ctx, &children, (clip, None));
        for id in children {
            compositor.draw(ctx, id);
        }
        compositor.flush(ctx);
    }

    fn children(&self) -> Vec<ArtistId> {
        self.children.clone()
    }

    fn remove_child(&mut self, child: ArtistId) {
        self.children.retain(|&id| id != child);
    }
}
