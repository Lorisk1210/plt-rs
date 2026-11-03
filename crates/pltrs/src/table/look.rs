//! The look of a table cell as a rectangle (matplotlib's `Cell` is a
//! `Rectangle`): a loose rectangle with it, for a program to read and
//! change, and the cell restyled from such a rectangle.

use pltrs_core::Point;

use super::handle::TableCell;
use crate::artist::{Artist, ArtistBase, ArtistHandle};
use crate::patches::{Patch, PatchArtist, PatchProps, Rectangle};

/// Copies onto `into` the artist properties of `from` a cell takes: all
/// but its place in a figure (its parent, transform and clipping
/// artists, which are its table's) and what a figure keeps of it.
fn take_look(from: &ArtistBase, into: &mut ArtistBase) {
    into.visible = from.visible;
    into.alpha = from.alpha;
    into.zorder = from.zorder;
    into.label.clone_from(&from.label);
    into.clip_on = from.clip_on;
    into.rasterized = from.rasterized;
    into.animated = from.animated;
    into.in_layout = from.in_layout;
    into.snap = from.snap;
    into.sketch.clone_from(&from.sketch);
    into.path_effects.clone_from(&from.path_effects);
    into.gid.clone_from(&from.gid);
    into.url.clone_from(&from.url);
    into.picker.clone_from(&from.picker);
    into.mouseover = from.mouseover;
    into.agg_filter.clone_from(&from.agg_filter);
}

impl TableCell {
    /// A rectangle outside any figure with the cell's look: its size, its
    /// face, edge, line and hatch style and its artist properties, at the
    /// corner where its table last placed it (in the fractions of the
    /// table's axes; the origin for a table not placed yet).
    ///
    /// ```
    /// use pltrs::colors::Color;
    /// use pltrs::prelude::*;
    /// use pltrs::table::TableOptions;
    /// let table = Figure::new().subplot().table(TableOptions::with_text(vec![vec!["a".into()]]))?;
    /// let cell = table.cell(0, 0).expect("a cell");
    /// let rectangle = cell.rectangle();
    /// rectangle.set_facecolor("red");
    /// cell.restyle_from(&rectangle);
    /// assert_eq!(cell.rectangle().style().facecolor, Some(Color::from("red")));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn rectangle(&self) -> Patch<Rectangle> {
        let corner = self.placed_corner().unwrap_or(Point::new(0.0, 0.0));
        let (width, height, style, base) = self.read(|cell| {
            (
                cell.width,
                cell.height,
                cell.style.clone(),
                cell.base.clone(),
            )
        });
        let rectangle = Patch::loose(Rectangle::new(corner, width, height), PatchProps::default());
        rectangle
            .raw()
            .with_mut::<PatchArtist<Rectangle>, _>(|patch| {
                patch.style = style;
                take_look(&base, patch.base_mut());
            });
        rectangle
    }

    /// The corner where the table places the cell in the figure's current
    /// layout, in the fractions of the table's axes (matplotlib's cell
    /// `xy` once its table is drawn); `None` for a table in no axes or
    /// one that cannot be placed.
    pub fn placed_corner(&self) -> Option<Point> {
        self.placed_box().map(|bbox| bbox.p0)
    }

    /// Takes the look of `rectangle` (one [`TableCell::rectangle`] made):
    /// its style and its artist properties, but for its transform and
    /// clipping artists; the cell keeps its size (which its table may
    /// change) and its table places it.
    pub fn restyle_from(&self, rectangle: &Patch<Rectangle>) -> &Self {
        let (style, base) = rectangle
            .raw()
            .with::<PatchArtist<Rectangle>, _>(|patch| (patch.style.clone(), patch.base().clone()));
        self.update(|cell| {
            cell.style = style;
            take_look(&base, &mut cell.base);
        })
    }
}
