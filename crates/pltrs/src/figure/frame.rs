//! The figure frame artist: the root figure and every subfigure.

use pltrs_core::{Bbox, Path};

use crate::artist::{Artist, ArtistBase, ArtistId, ArtistStore, DrawContext};
use crate::axes::AxesState;
use crate::colors::Color;
use crate::figure::Figure;
use crate::gridspec::{SubplotParams, SubplotSpec};
use crate::image::{Compositor, FigureImageArtist};
use crate::legend::LegendArtist;
use crate::mplot3d::is_axes3d;
use crate::patches::{PatchPaint, PatchStyle};
use crate::patheffects::with_path_effects;
use crate::text::TextArtist;
use crate::transforms::unit_to_bbox;

/// A rectangular region that owns axes and figure-level artists: the
/// figure itself, or a subfigure nested inside it (matplotlib's
/// `FigureBase`).
#[derive(Debug)]
pub(crate) struct FigureFrame {
    base: ArtistBase,
    /// The background colour, the colour and width of the border, and the
    /// rest of the style of the background patch (matplotlib's
    /// `Figure.patch`).
    pub(crate) style: PatchStyle,
    /// Whether the background and border are drawn.
    pub(crate) frameon: bool,
    /// The id of the background's group in SVG output (matplotlib's
    /// `Figure.patch.get_gid`).
    pub(crate) patch_gid: Option<String>,
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

pltrs_core::archive_struct!(FigureFrame {
    base,
    style,
    frameon,
    patch_gid,
    relative_extent,
    children,
    subplot_params,
    super_texts,
    subplot_spec
});

/// The texts spanning a whole figure (matplotlib's `_suptitle`,
/// `_supxlabel`, `_supylabel`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SuperTexts {
    pub(crate) title: Option<SuperText>,
    pub(crate) xlabel: Option<SuperText>,
    pub(crate) ylabel: Option<SuperText>,
}

pltrs_core::archive_struct!(SuperTexts {
    title,
    xlabel,
    ylabel
});

/// A figure-wide text and how it is placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SuperText {
    /// The text artist.
    pub(crate) id: ArtistId,
    /// Whether constrained layout moves the text to the figure's edge and
    /// makes room for it, as it does until the text is placed by hand
    /// across that edge (matplotlib's `_autopos`).
    pub(crate) auto_placed: bool,
}

pltrs_core::archive_struct!(SuperText { id, auto_placed });

impl SuperTexts {
    /// Forgets `text` if it is one of the figure-wide texts (it was
    /// removed), so that the figure no longer reports it and makes it
    /// anew when asked.
    fn forget(&mut self, text: ArtistId) {
        for slot in [&mut self.title, &mut self.xlabel, &mut self.ylabel] {
            if slot.is_some_and(|known| known.id == text) {
                *slot = None;
            }
        }
    }
}

impl FigureFrame {
    /// A frame covering `relative_extent` of its parent.
    pub(crate) fn new(
        relative_extent: Bbox,
        facecolor: Color,
        edgecolor: Color,
        subplot_params: SubplotParams,
    ) -> Self {
        Self {
            base: ArtistBase::default(),
            style: PatchStyle {
                facecolor: Some(facecolor),
                edgecolor: Some(edgecolor),
                linewidth: Some(0.0),
                ..PatchStyle::from_rc(&crate::rc::params())
            },
            frameon: true,
            patch_gid: None,
            relative_extent,
            children: Vec::new(),
            subplot_params,
            super_texts: SuperTexts::default(),
            subplot_spec: None,
        }
    }
}

impl FigureFrame {
    /// Draws the background and border over `extent`. The figure patch
    /// is a `Rectangle` painted as any patch (without an edge when it has
    /// no colour, unfilled when transparent, its alpha replacing the alpha
    /// of its colours), but always without anti-aliasing.
    fn draw_patch(&self, ctx: &mut DrawContext<'_>, extent: Bbox) {
        let style = PatchStyle {
            antialiased: Some(false),
            ..self.style.clone()
        };
        let paint = match PatchPaint::new(&style, &crate::rc::params(), self.base.alpha, false) {
            Ok(paint) => paint,
            Err(err) => {
                ctx.report(self.base.id(), err);
                return;
            }
        };
        let gc = ctx.base_gc(&self.base);
        let drawn = ctx.draw_artist(false, |ctx| {
            with_path_effects(ctx, &self.base.path_effects, |ctx| {
                paint.paint(gc.clone(), |gc, face| {
                    ctx.renderer().draw_path(
                        gc,
                        &Path::unit_rectangle(),
                        &unit_to_bbox(&extent),
                        face,
                    );
                });
            })
        });
        if let Err(err) = drawn {
            ctx.report(self.base.id(), err);
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
        let group = if self.base.parent.is_some() {
            "subfigure"
        } else {
            "figure"
        };
        ctx.renderer().open_group(group, self.base.gid.as_deref());
        self.draw_contents(ctx);
        ctx.renderer().close_group(group);
    }

    fn children(&self) -> Vec<ArtistId> {
        self.children.clone()
    }

    fn remove_child(&mut self, child: ArtistId) {
        self.children.retain(|&id| id != child);
        self.super_texts.forget(child);
    }
}

impl FigureFrame {
    /// Draws the background in its SVG group (matplotlib's `Patch.draw`),
    /// then the children, compositing images.
    fn draw_contents(&self, ctx: &mut DrawContext<'_>) {
        if self.frameon
            && let Some(extent) = ctx.layout().frame_patch_extent(self.base.id())
        {
            ctx.renderer()
                .open_part_group("patch", self.patch_gid.as_deref());
            self.draw_patch(ctx, extent);
            ctx.renderer().close_part_group("patch");
        }
        let mut children =
            ctx.sorted_children(&listing_order(ctx.layout().store(), &self.children));
        children.retain(|&id| !ctx.layout().skips_animated(id, false));
        let clip = ctx.layout().frame_extent(self.base.id());
        let mut compositor = Compositor::new(ctx, &children, (clip, None));
        for id in children {
            compositor.draw(ctx, id);
        }
        compositor.flush(ctx);
    }
}

impl Figure {
    /// Names the SVG group of the background `gid` (matplotlib's
    /// `fig.patch.set_gid`), or numbers it as a `patch` (`None`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// fig.set_patch_gid(Some("canvas".into()));
    /// assert_eq!(fig.patch_gid().as_deref(), Some("canvas"));
    /// assert!(fig.to_svg()?.contains("<g id=\"canvas\">"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_patch_gid(&self, gid: Option<String>) -> &Self {
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.patch_gid = gid);
        self
    }

    /// The id of the background's SVG group, if it was given one.
    pub fn patch_gid(&self) -> Option<String> {
        self.root
            .with::<FigureFrame, _>(|frame| frame.patch_gid.clone())
    }

    /// The style of the background patch: its colours, border width, line
    /// style, hatch, caps and joins, as a patch's (matplotlib's
    /// `Figure.patch`; it is drawn without anti-aliasing whatever its
    /// style says).
    pub fn patch_style(&self) -> PatchStyle {
        self.root
            .with::<FigureFrame, _>(|frame| frame.style.clone())
    }

    /// Gives the background patch the style of a patch (matplotlib's
    /// `fig.patch` setters).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::patches::PatchStyle;
    /// let fig = Figure::new();
    /// fig.set_patch_style(PatchStyle {
    ///     hatch: Some("x".into()),
    ///     ..fig.patch_style()
    /// });
    /// assert!(fig.to_svg()?.contains("<pattern"), "the hatch is drawn");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_patch_style(&self, style: PatchStyle) -> &Self {
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.style = style);
        self
    }
}

/// `children` in the order matplotlib's `Figure.get_children` lists them,
/// which its z-order sort keeps among equal z-orders: artists added with
/// `add_artist`, axes, texts, figure images, legends, then subfigures.
pub(super) fn listing_order(store: &ArtistStore, children: &[ArtistId]) -> Vec<ArtistId> {
    let group = |id: ArtistId| -> u8 {
        if store.get_as::<AxesState>(id).is_some() || is_axes3d(store, id) {
            1
        } else if store.get_as::<TextArtist>(id).is_some() {
            2
        } else if store.get_as::<FigureImageArtist>(id).is_some() {
            3
        } else if store.get_as::<LegendArtist>(id).is_some() {
            4
        } else if store.get_as::<FigureFrame>(id).is_some() {
            5
        } else {
            0
        }
    };
    let mut ordered = children.to_vec();
    ordered.sort_by_key(|&id| group(id));
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::gridspec::GridSpec;

    #[test]
    fn subfigures_come_after_axes_and_texts() {
        let fig = Figure::new();
        let cell = GridSpec::new(1, 2)
            .and_then(|grid| grid.cell(0, 0))
            .expect("a cell");
        let sub = fig.add_subfigure(&cell);
        let text = fig.text(0.5, 0.5, "t");
        let ax = fig.subplot();
        let state = fig.raw().figure_state().borrow();
        let created = [sub.id(), text.id(), ax.id()];
        assert_eq!(
            listing_order(&state.store, &created),
            [ax.id(), text.id(), sub.id()]
        );
    }

    #[test]
    fn the_alpha_of_the_figure_replaces_that_of_its_colours() {
        let fig = Figure::with_size(1.0, 1.0, 10.0);
        let opaque = fig.to_svg().expect("an svg");
        assert!(opaque.contains("fill: #ffffff"));
        fig.set_alpha(Some(0.0));
        let clear = fig.to_svg().expect("an svg");
        assert!(
            !clear.contains("fill: #ffffff"),
            "a transparent face is not filled"
        );
        assert!(clear.contains("opacity: 0"));
    }
}
