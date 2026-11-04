//! The artist of an inset indicator: it owns its rectangle and its
//! connectors and draws them as matplotlib's `InsetIndicator.draw` does.

use pltrs_core::{Bbox, Path, Point};

use super::geometry::{bounds, data_transform};
use crate::artist::{Artist, ArtistBase, ArtistId, DrawContext, LayoutContext};
use crate::patches::{ConnectionPatch, DisplayPath, PatchArtist, Rectangle};

/// An inset indicator (matplotlib's `InsetIndicator`): a [`Rectangle`]
/// marking an area of its axes and, when it indicates an inset axes, four
/// [`ConnectionPatch`]es joining the rectangle to the inset's corners, two
/// of them visible.
#[derive(Clone, Debug)]
pub(crate) struct IndicatorArtist {
    pub(super) base: ArtistBase,
    /// The frame.
    pub(super) rectangle: ArtistId,
    /// The connectors to the inset's lower-left, upper-left, lower-right
    /// and upper-right corners; none without an inset.
    pub(super) connectors: Vec<ArtistId>,
    /// The inset axes the connectors lead to.
    pub(super) inset: Option<ArtistId>,
    /// Whether the rectangle follows the view limits of the inset
    /// (matplotlib's `_auto_update_bounds`).
    pub(super) follows_inset: bool,
    /// Whether the connectors' visibility was chosen: matplotlib chooses
    /// it once, when it makes them (at the first draw, or when the program
    /// first asks for them); until then the connectors that do not cross
    /// the rectangle are drawn.
    pub(super) chosen: bool,
}

pltrs_core::archive_struct!(IndicatorArtist {
    base,
    rectangle,
    connectors,
    inset,
    follows_inset,
    chosen
});

impl IndicatorArtist {
    /// The rectangle's outline in display pixels, as matplotlib draws it
    /// from `rectangle.get_transform().transform_path(rectangle.get_path())`.
    fn outline(layout: LayoutContext<'_>, rectangle: &PatchArtist<Rectangle>) -> Option<Path> {
        let transform = layout.resolve(&data_transform(rectangle))?;
        let [x, y, width, height] = bounds(rectangle);
        let corners = [
            (x, y),
            (x + width, y),
            (x + width, y + height),
            (x, y + height),
            (x, y),
        ];
        Some(Path::closed(
            corners
                .into_iter()
                .map(|(cx, cy)| transform.transform_point(Point::new(cx, cy)))
                .collect(),
        ))
    }

    /// The connectors drawn: the visible ones once chosen, else those that
    /// do not cross the rectangle.
    fn shown_connectors<'a>(
        &self,
        layout: LayoutContext<'a>,
    ) -> Vec<(ArtistId, &'a PatchArtist<ConnectionPatch>)> {
        let automatic = if self.chosen {
            None
        } else {
            Some(self.automatic_visibility(layout).unwrap_or([false; 4]))
        };
        self.connectors
            .iter()
            .enumerate()
            .filter_map(|(index, &id)| {
                let connector = layout.store().get_as::<PatchArtist<ConnectionPatch>>(id)?;
                let shown = automatic.map_or(connector.base().visible, |shown| shown[index]);
                shown.then_some((id, connector))
            })
            .collect()
    }

    /// Draws the connectors whose style differs from the rectangle's on
    /// their own, then the rectangle with the others as one path in the
    /// rectangle's style.
    fn draw_parts(&self, ctx: &mut DrawContext<'_>) -> Result<(), String> {
        let layout = ctx.layout();
        let Some(rectangle) = layout
            .store()
            .shared_as::<PatchArtist<Rectangle>>(self.rectangle)
        else {
            return Ok(());
        };
        let outline = Self::outline(layout, &rectangle)
            .ok_or("the rectangle's coordinate system no longer exists")?;
        let mut outlines = vec![outline];
        let dpi_cor = layout.points_to_pixels(1.0);
        // The connectors drawn on their own, in order, until one whose
        // outline cannot be found.
        let mut apart = Vec::new();
        let mut found = Ok(());
        for (id, connector) in self.shown_connectors(layout) {
            if connector.shares_edge_style(&rectangle) {
                match connector.display_paths_at(layout, dpi_cor) {
                    Ok(pieces) => outlines.extend(pieces.iter().map(DisplayPath::to_display)),
                    Err(err) => {
                        found = Err(err.to_string());
                        break;
                    }
                }
            } else {
                apart.push(id);
            }
        }
        for id in apart {
            ctx.draw_child(id);
        }
        found?;
        if !rectangle.base().visible {
            return Ok(());
        }
        rectangle
            .paint_display_paths(
                ctx,
                &[DisplayPath::display(Path::compound(&outlines), true)],
            )
            .map_err(|err| err.to_string())
    }
}

impl Artist for IndicatorArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        if let Err(problem) = self.draw_parts(ctx) {
            ctx.report(self.base.id(), problem);
        }
    }

    /// `InsetIndicator.draw` draws the parts without making the indicator
    /// fresh.
    fn freshened_by_draw(&self, _ctx: &LayoutContext<'_>) -> bool {
        false
    }

    fn children(&self) -> Vec<ArtistId> {
        std::iter::once(self.rectangle)
            .chain(self.connectors.iter().copied())
            .collect()
    }

    fn remove_child(&mut self, child: ArtistId) {
        self.connectors.retain(|&id| id != child);
    }

    /// The extent of the rectangle and of the connectors drawn.
    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let layout = *ctx;
        let rectangle = layout
            .store()
            .get_as::<PatchArtist<Rectangle>>(self.rectangle)?;
        let dpi_cor = layout.measured_dpi_cor();
        let mut extent = Self::outline(layout, rectangle)?.extents();
        for (_, connector) in self.shown_connectors(layout) {
            for piece in connector.display_paths_at(layout, dpi_cor).ok()? {
                extent = extent.union(&piece.to_display().extents());
            }
        }
        Some(extent)
    }
}
