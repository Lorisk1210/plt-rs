//! Drawing an axes (matplotlib's `_AxesBase.draw`): the background, then
//! the children and both axes in z-order; with a rasterization z-order,
//! the background and everything up to it become one image in vector
//! output.

use pltrs_render::ClipPath;

use super::Axes;
use super::axis_draw::with_agg_filter;
use super::axis_layout::AxisPainter;
use super::state::AxesState;
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistStore, DrawContext};
use crate::axes_grid1::{is_parasite, parasites_of};
use crate::image::Compositor;
use crate::legend::LegendArtist;
use crate::patches::{PatchError, PatchPaint};
use crate::patheffects::with_path_effects;
use crate::ticker::AxisName;
use crate::transforms::{Evaluation, unit_to_bbox};

/// Something an axes draws in z-order.
#[derive(Clone, Copy, Debug)]
pub(super) enum Item {
    /// A child artist.
    Child(ArtistId),
    /// The x or y axis of this axes or of one of its parasites.
    Axis(ArtistId, AxisName),
    /// The background patch again, as the child of a parasite sharing it.
    Patch,
}

/// Child `id` with its z-order, unless it was removed.
fn zordered(store: &ArtistStore, id: ArtistId) -> Option<(f64, Item)> {
    Some((store.get(id)?.base().zorder, Item::Child(id)))
}

impl AxesState {
    /// Draws the background, then the children and both axes in z-order;
    /// those at or below the rasterization z-order are drawn into one
    /// rasterized image together with the background, when the lowest
    /// item lies below it.
    pub(super) fn paint(&self, ctx: &mut DrawContext<'_>) {
        if ctx.layout().axes_extent(self.base().id()).is_none() {
            return;
        }
        let id = self.base().id();
        ctx.evaluated(false, || Some(Evaluation::Axes(id)));
        if self
            .polar_view()
            .is_some_and(|polar| !polar.has_finite_wedge())
        {
            // matplotlib's polar draw places its wedge first, an arc it
            // cannot make of angles that are not finite.
            let err = PatchError::ArcAngles;
            ctx.report_kind(self.base().id(), err.draw_kind(), err);
            return;
        }
        ctx.renderer()
            .open_group("axes", self.base().gid.as_deref());
        let mut items = self.items(ctx.layout().store());
        items.retain(|&(_, item)| match item {
            Item::Child(id) => !ctx.layout().skips_animated(id, true),
            Item::Axis(owner, name) => !ctx.layout().skips_animated_axis(owner, name),
            Item::Patch => true,
        });
        let (below, above) = items.split_at(self.rasterized_count(&items));
        if below.is_empty() {
            self.draw_background(ctx);
        } else {
            // matplotlib's `_draw_rasterized`: images among them are not
            // composited.
            ctx.draw_artist(true, |ctx| {
                self.draw_background(ctx);
                for &(_, item) in below {
                    self.draw_item(ctx, item);
                }
            });
        }
        self.draw_composited(ctx, above);
        ctx.renderer().close_group("axes");
    }

    /// The children and both axes sorted by z-order, stable for ties in the
    /// order of matplotlib's `get_children`: plotted artists, the children
    /// of every parasite (which matplotlib's `HostAxesBase.draw` appends to
    /// the plotted artists), spines, both axes, the axes the program
    /// draws, the centre, left and right titles, child axes, the legend.
    pub(super) fn items(&self, store: &ArtistStore) -> Vec<(f64, Item)> {
        let mut items: Vec<(f64, Item)> = self
            .plotted(store)
            .filter_map(|id| zordered(store, id))
            .collect();
        let parasites = parasites_of(store, self.base().id());
        for parasite in parasites
            .into_iter()
            .filter_map(|id| store.get_as::<Self>(id))
        {
            items.extend(parasite.plotted(store).filter_map(|id| zordered(store, id)));
            // matplotlib draws every child of a parasite that is visible:
            // its frame and axis settings do not count.
            items.extend(parasite.decoration_items(store, true, true));
            if parasite.patch.host_owned {
                items.push((self.patch.zorder, Item::Patch));
            }
        }
        items.extend(self.decoration_items(
            store,
            self.decorations.framed(),
            self.decorations.axis_on,
        ));
        items.sort_by(|a, b| a.0.total_cmp(&b.0));
        items
    }

    /// The children the axes draws, in drawing order: its z-ordered items
    /// that are artists (see [`items`](Self::items)), without the spines
    /// of an unframed axes and the axes the program draws of an axes
    /// whose axes are off, which matplotlib's draw leaves out.
    pub(crate) fn drawn_children(&self, store: &ArtistStore) -> Vec<ArtistId> {
        self.items(store)
            .into_iter()
            .filter_map(|(_, item)| match item {
                Item::Child(id) => Some(id),
                Item::Axis(..) | Item::Patch => None,
            })
            .collect()
    }

    /// The plotted children: neither spines, titles, the axes the program
    /// draws, legends nor child axes.
    fn plotted<'a>(&'a self, store: &'a ArtistStore) -> impl Iterator<Item = ArtistId> + 'a {
        self.children.iter().copied().filter(move |&id| {
            !self.is_own_part(id)
                && store.get_as::<LegendArtist>(id).is_none()
                && store.get_as::<Self>(id).is_none()
        })
    }

    /// The decorations in the order of matplotlib's `get_children`: the
    /// spines (with `spines`), both axes and those the program draws
    /// (with `axes`), the centre, left and right titles, the child axes
    /// other than parasites, the legends.
    fn decoration_items(&self, store: &ArtistStore, spines: bool, axes: bool) -> Vec<(f64, Item)> {
        let id = self.base().id();
        let mut items: Vec<(f64, Item)> = self
            .spines_in_children_order()
            .into_iter()
            .filter(|_| spines)
            .filter_map(|id| zordered(store, id))
            .collect();
        if axes {
            let zorder = self.axisbelow.zorder();
            items.push((zorder, Item::Axis(id, AxisName::X)));
            items.push((zorder, Item::Axis(id, AxisName::Y)));
            items.extend((self.program_axes.iter()).filter_map(|&axis| zordered(store, axis)));
        }
        let titles = self.titles_centre_first();
        let child_axes =
            self.children.iter().copied().filter(|&child| {
                store.get_as::<Self>(child).is_some() && !is_parasite(store, child)
            });
        let legends = self
            .children
            .iter()
            .copied()
            .filter(|&child| store.get_as::<LegendArtist>(child).is_some());
        items.extend(
            titles
                .chain(child_axes)
                .chain(legends)
                .filter_map(|id| zordered(store, id)),
        );
        items
    }

    /// How many of the z-ordered `items` are rasterized: those at or below
    /// the rasterization z-order, when the lowest lies below it
    /// (matplotlib's `searchsorted(..., side="right")`).
    fn rasterized_count(&self, items: &[(f64, Item)]) -> usize {
        match (self.rasterization_zorder, items.first()) {
            (Some(limit), Some(&(lowest, _))) if lowest < limit => {
                items.partition_point(|&(zorder, _)| zorder <= limit)
            }
            _ => 0,
        }
    }

    /// Draws the background patch of a framed axes.
    fn draw_background(&self, ctx: &mut DrawContext<'_>) {
        if self.decorations.framed() {
            self.draw_patch(ctx);
        }
    }

    /// Draws the background patch, if visible, along its outline, in its
    /// SVG group (matplotlib's `Patch.draw`).
    fn draw_patch(&self, ctx: &mut DrawContext<'_>) {
        let base = self.base();
        let Some(extent) = ctx.layout().axes_extent(base.id()) else {
            return;
        };
        if !self.patch.visible {
            return;
        }
        ctx.evaluated(false, || Some(Evaluation::Background(base.id())));
        ctx.renderer()
            .open_part_group("patch", self.patch.gid.as_deref());
        // A transparent face is left unfilled, a transparent edge
        // unstroked, as for any patch.
        match PatchPaint::new(&self.patch.style, &self.rc, self.patch.alpha, false) {
            Ok(paint) => {
                let mut gc = ctx.base_gc(base);
                if self
                    .patch_outline
                    .as_ref()
                    .is_some_and(|o| o.clipped_to_box)
                {
                    // Floating axes clip their outline to the axes box
                    // (matplotlib's `patch.set_clip_path(orig_patch)`).
                    gc.clip_rect = Some(extent);
                }
                let outline = ctx
                    .layout()
                    .background_outline(base.id())
                    .unwrap_or_else(|| ClipPath {
                        path: self.patch_path(),
                        transform: unit_to_bbox(&extent),
                    });
                let drawn = ctx.draw_artist(false, |ctx| {
                    with_path_effects(ctx, &base.path_effects, |ctx| {
                        paint.paint(gc.clone(), |gc, face| {
                            ctx.renderer()
                                .draw_path(gc, &outline.path, &outline.transform, face);
                        });
                    })
                });
                if let Err(err) = drawn {
                    ctx.report(base.id(), err);
                }
            }
            Err(err) => ctx.report(base.id(), err),
        }
        ctx.renderer().close_part_group("patch");
    }

    /// Draws one item on its own.
    fn draw_item(&self, ctx: &mut DrawContext<'_>, item: Item) {
        match item {
            Item::Child(id) => ctx.draw_child(id),
            Item::Axis(owner, name) => {
                let parasite;
                let axes = if owner == self.base().id() {
                    Some(self)
                } else {
                    parasite = ctx.layout().store().shared_as::<Self>(owner);
                    parasite.as_deref()
                };
                if let Some(axes) = axes {
                    ctx.evaluated(false, || AxisPainter::new(owner, axes, name).evaluation());
                    let filter = axes.axis(name).agg_filter.clone();
                    ctx.draw_artist(false, |ctx| {
                        with_agg_filter(ctx, filter.as_ref(), |ctx| {
                            AxisPainter::new(owner, axes, name).draw(ctx, &axes.rc);
                        });
                    });
                }
            }
            Item::Patch => self.draw_patch(ctx),
        }
    }

    /// Draws `items` in order, compositing consecutive images
    /// (matplotlib's `_draw_list_compositing_images`).
    fn draw_composited(&self, ctx: &mut DrawContext<'_>, items: &[(f64, Item)]) {
        let children: Vec<ArtistId> = items
            .iter()
            .filter_map(|&(_, item)| match item {
                Item::Child(id) => Some(id),
                Item::Axis(..) | Item::Patch => None,
            })
            .collect();
        let clip = ctx.layout().axes_extent(self.base().id());
        let mut compositor = Compositor::new(ctx, &children, (clip, None));
        for &(_, item) in items {
            match item {
                Item::Child(id) => compositor.draw(ctx, id),
                Item::Axis(..) | Item::Patch => {
                    compositor.flush(ctx);
                    self.draw_item(ctx, item);
                }
            }
        }
        compositor.flush(ctx);
    }
}

impl Axes {
    /// Rasterizes the background and the children at or below `zorder`
    /// into one image in vector output, when the lowest of them lies below
    /// it; `None` keeps everything vector (matplotlib's
    /// `set_rasterization_zorder`).
    pub fn set_rasterization_zorder(&self, zorder: Option<f64>) -> &Self {
        self.raw()
            .with_mut::<AxesState, _>(|axes| axes.rasterization_zorder = zorder);
        self
    }

    /// The z-order up to which children are rasterized in vector output.
    pub fn rasterization_zorder(&self) -> Option<f64> {
        self.raw()
            .with::<AxesState, _>(|axes| axes.rasterization_zorder)
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    fn images_in_svg(rasterization_zorder: Option<f64>, line_zorder: f64) -> usize {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_rasterization_zorder(rasterization_zorder);
        ax.plot([0.0, 1.0], [0.0, 1.0])
            .unwrap()
            .set_zorder(line_zorder);
        fig.to_svg().unwrap().matches("<image").count()
    }

    #[test]
    fn children_up_to_the_zorder_become_one_image() {
        assert_eq!(images_in_svg(None, 1.0), 0);
        assert_eq!(images_in_svg(Some(1.0), 0.5), 1);
        assert_eq!(images_in_svg(Some(1.0), 2.0), 0, "nothing below the limit");
        assert_eq!(
            images_in_svg(Some(2.5), 2.0),
            1,
            "the line and both axes (z-order 2.5) share the image"
        );
    }
}
