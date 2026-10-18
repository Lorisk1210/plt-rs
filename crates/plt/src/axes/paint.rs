//! Drawing an axes (matplotlib's `_AxesBase.draw`): the background, then
//! the children and both axes in z-order; with a rasterization z-order,
//! the background and everything up to it become one image in vector
//! output.

use plt_core::{Bbox, Rgba};
use plt_render::{ClipPath, JoinStyle};

use super::Axes;
use super::axis_layout::AxisPainter;
use super::state::AxesState;
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistStore, DrawContext};
use crate::axes_grid1::{is_parasite, parasites_of};
use crate::image::Compositor;
use crate::legend::LegendArtist;
use crate::patheffects::with_path_effects;
use crate::ticker::AxisName;
use crate::transforms::unit_to_bbox;

/// Something an axes draws in z-order.
#[derive(Clone, Copy, Debug)]
enum Item {
    /// A child artist.
    Child(ArtistId),
    /// The x or y axis of this axes or of one of its parasites.
    Axis(ArtistId, AxisName),
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
        let Some(extent) = ctx.layout().axes_extent(self.base().id()) else {
            return;
        };
        let items = self.items(ctx.layout().store());
        let (below, above) = items.split_at(self.rasterized_count(&items));
        if below.is_empty() {
            self.draw_background(ctx, extent);
        } else {
            // matplotlib's `_draw_rasterized`: images among them are not
            // composited.
            ctx.draw_artist(true, |ctx| {
                self.draw_background(ctx, extent);
                for &(_, item) in below {
                    self.draw_item(ctx, item);
                }
            });
        }
        self.draw_composited(ctx, above);
    }

    /// The children and both axes sorted by z-order, stable for ties in the
    /// order of matplotlib's `get_children`: plotted artists, the children
    /// of every parasite (which matplotlib's `HostAxesBase.draw` appends to
    /// the plotted artists), spines, both axes, the centre, left and right
    /// titles, child axes, the legend.
    fn items(&self, store: &ArtistStore) -> Vec<(f64, Item)> {
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
        }
        items.extend(self.decoration_items(
            store,
            self.decorations.framed(),
            self.decorations.axis_on,
        ));
        items.sort_by(|a, b| a.0.total_cmp(&b.0));
        items
    }

    /// The plotted children: neither spines, titles, legends nor child
    /// axes.
    fn plotted<'a>(&'a self, store: &'a ArtistStore) -> impl Iterator<Item = ArtistId> + 'a {
        self.children.iter().copied().filter(move |&id| {
            !self.spines.contains(&id)
                && !self.titles.contains(&id)
                && store.get_as::<LegendArtist>(id).is_none()
                && store.get_as::<Self>(id).is_none()
        })
    }

    /// The decorations in the order of matplotlib's `get_children`: the
    /// spines (with `spines`), both axes (with `axes`), the centre, left
    /// and right titles, the child axes other than parasites, the legends.
    fn decoration_items(&self, store: &ArtistStore, spines: bool, axes: bool) -> Vec<(f64, Item)> {
        let id = self.base().id();
        let mut items: Vec<(f64, Item)> = self
            .spines_in_children_order()
            .filter(|_| spines)
            .filter_map(|id| zordered(store, id))
            .collect();
        if axes {
            items.push((self.axis_zorder, Item::Axis(id, AxisName::X)));
            items.push((self.axis_zorder, Item::Axis(id, AxisName::Y)));
        }
        // `titles` holds the left, centre and right title.
        let titles = [1, 0, 2]
            .into_iter()
            .filter_map(|i| self.titles.get(i).copied());
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

    /// Draws the background patch of a framed axes, along its outline.
    fn draw_background(&self, ctx: &mut DrawContext<'_>, extent: Bbox) {
        if !(self.decorations.framed() && self.patch.visible) {
            return;
        }
        let base = self.base();
        match self.patch.facecolor.resolve() {
            Ok(face) => {
                // The patch has edge colour 'none' and no width, and is
                // left unfilled when transparent.
                let mut gc = ctx.base_gc(base);
                gc.linewidth = 0.0;
                gc.foreground = Rgba::TRANSPARENT;
                gc.join = JoinStyle::Miter;
                let face = (!face.is_transparent()).then_some(face);
                let outline = ctx
                    .layout()
                    .clip_outline(base.id())
                    .unwrap_or_else(|| ClipPath {
                        path: self.patch_path(),
                        transform: unit_to_bbox(&extent),
                    });
                ctx.draw_artist(false, |ctx| {
                    with_path_effects(ctx, &base.path_effects, |ctx| {
                        ctx.renderer()
                            .draw_path(&gc, &outline.path, &outline.transform, face);
                    });
                });
            }
            Err(err) => ctx.report(base.id(), err),
        }
    }

    /// Draws one item on its own.
    fn draw_item(&self, ctx: &mut DrawContext<'_>, item: Item) {
        match item {
            Item::Child(id) => ctx.draw_child(id),
            Item::Axis(owner, name) => {
                let axes = if owner == self.base().id() {
                    Some(self)
                } else {
                    ctx.layout().store().get_as::<Self>(owner)
                };
                if let Some(axes) = axes {
                    ctx.draw_artist(false, |ctx| {
                        AxisPainter::new(owner, axes, name).draw(ctx, &axes.rc);
                    });
                }
            }
        }
    }

    /// Draws `items` in order, compositing consecutive images
    /// (matplotlib's `_draw_list_compositing_images`).
    fn draw_composited(&self, ctx: &mut DrawContext<'_>, items: &[(f64, Item)]) {
        let children: Vec<ArtistId> = items
            .iter()
            .filter_map(|&(_, item)| match item {
                Item::Child(id) => Some(id),
                Item::Axis(..) => None,
            })
            .collect();
        let clip = ctx.layout().axes_extent(self.base().id());
        let mut compositor = Compositor::new(ctx, &children, (clip, None));
        for &(_, item) in items {
            match item {
                Item::Child(id) => compositor.draw(ctx, id),
                Item::Axis(..) => {
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
