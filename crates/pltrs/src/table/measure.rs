//! Measuring the text of a table cell where the table draws it
//! (matplotlib's `Cell.get_text_bounds`, `get_required_width` and
//! `auto_set_font_size`).

use pltrs_core::Bbox;

use super::TableArtist;
use super::cell::{Cell, PlacedCell};
use super::handle::TableCell;
use super::layout::AxesMaps;
use crate::artist::{ArtistHandle, LayoutContext};
use crate::rc::RcParams;
use crate::text::{Text, TextDrawError};
use crate::transforms::Transform;

impl TableCell {
    /// `f` of the cell, its table, the figure's current layout (laid out
    /// first) and the maps of the table's axes; `None` for a table in no
    /// axes, or an axes of no area.
    fn measured<R>(
        &self,
        f: impl FnOnce(&Cell, usize, &TableArtist, LayoutContext<'_>, &AxesMaps) -> R,
    ) -> Option<R> {
        let raw = self.table().raw();
        let state = raw.figure_state().prepared();
        let layout = LayoutContext::new(&state);
        let table = state.store.get_as::<TableArtist>(raw.id())?;
        let maps = AxesMaps::of(&layout.axes_extent(table.axes?)?)?;
        let index = table.cells.iter().position(|(key, _)| *key == self.key())?;
        Some(f(&table.cells[index].1, index, table, layout, &maps))
    }

    /// The box the table places the cell in, in the fractions of the
    /// table's axes, in the figure's current layout; `None` for a table
    /// in no axes or one that cannot be placed.
    pub(crate) fn placed_box(&self) -> Option<Bbox> {
        self.placed().map(|placed| placed.bbox)
    }

    /// Where and at what font size the table places the cell in the
    /// figure's current layout; `None` for a table in no axes or one that
    /// cannot be placed.
    pub(crate) fn placed(&self) -> Option<PlacedCell> {
        self.measured(|_, index, table, layout, maps| {
            table
                .place(layout, maps)
                .ok()
                .map(|mut placed| placed.swap_remove(index))
        })
        .flatten()
    }

    /// The box of the cell's text where the table draws it, in the
    /// fractions of the table's axes (matplotlib's `get_text_bounds`):
    /// laid out at the cell's font size in the figure's current layout;
    /// `None` for a table in no axes, or a cell drawn through a transform
    /// that is no invertible affine map.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::table::TableOptions;
    /// let ax = Figure::new().subplot();
    /// let table = ax.table(TableOptions::with_text(vec![vec!["wide text".into()]]))?;
    /// let cell = table.cell(0, 0).expect("a cell");
    /// let bounds = cell.text_bounds()?.expect("in an axes");
    /// assert!(bounds.width() > 0.0 && bounds.width() < cell.width());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the text cannot be laid out.
    pub fn text_bounds(&self) -> Result<Option<Bbox>, TextDrawError> {
        let measured = self.measured(|cell, index, table, layout, maps| {
            let placed = table.place(layout, maps)?[index];
            cell.maps(layout, maps)
                .map(|cell_maps| cell.text_box(&placed, &cell_maps, layout, &table.rc))
                .transpose()
        });
        Ok(measured.transpose()?.flatten())
    }

    /// The width the cell needs for its text with its padding, in the
    /// fractions of the table's axes (matplotlib's `get_required_width`);
    /// `None` for a table in no axes.
    ///
    /// # Errors
    ///
    /// When the text cannot be laid out.
    pub fn required_width(&self) -> Result<Option<f64>, TextDrawError> {
        self.measured(|cell, index, table, layout, maps| {
            let to_axes = &table.cell_maps(layout, maps)[index].to_axes;
            cell.required_width(cell.fontsize(), layout, &table.rc, to_axes)
        })
        .transpose()
    }

    /// Shrinks the font of the cell a point at a time, down to 1 point,
    /// until its text fits in the cell's width, and returns the size
    /// (matplotlib's `Cell.auto_set_font_size`); the size stays for a table
    /// in no axes.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::table::TableOptions;
    /// let ax = Figure::new().subplot();
    /// let table = ax.table(TableOptions::with_text(vec![vec!["a rather long text".into()]]))?;
    /// let cell = table.cell(0, 0).expect("a cell");
    /// cell.set_width(0.05);
    /// let size = cell.auto_set_font_size()?;
    /// assert!(size < 10.0 && cell.fontsize() == size);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the text cannot be laid out.
    pub fn auto_set_font_size(&self) -> Result<f64, TextDrawError> {
        let fitted = self.measured(|cell, index, table, layout, maps| {
            fitted_size(
                cell,
                layout,
                &table.rc,
                &table.cell_maps(layout, maps)[index],
            )
        });
        let size = match fitted {
            Some(size) => size?,
            None => return Ok(self.fontsize()),
        };
        self.set_fontsize(size);
        Ok(size)
    }
}

impl TableCell {
    /// Draws the cell in the coordinates `transform`, its place in the
    /// table being in them, or in the fractions of the table's axes again
    /// with `None` (matplotlib's `Cell.set_transform`).
    pub fn set_transform(&self, transform: Option<Transform>) -> &Self {
        self.update(|cell| cell.transform = transform)
    }

    /// The coordinates the cell is drawn in, `None` for the fractions of
    /// the table's axes.
    pub fn transform(&self) -> Option<Transform> {
        self.read(|cell| cell.transform.clone())
    }

    /// A loose text with the look of the cell's text: its string, colour,
    /// font and horizontal alignment, centred vertically (what a program
    /// reads and changes of the text matplotlib's `Cell.get_text` returns;
    /// the table places the text when it is drawn). Changes come back with
    /// [`TableCell::restyle_text_from`].
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::table::TableOptions;
    /// let ax = Figure::new().subplot();
    /// let table = ax.table(TableOptions::with_text(vec![vec!["old".into()]]))?;
    /// let cell = table.cell(0, 0).expect("a cell");
    /// let text = cell.text_artist();
    /// assert_eq!(text.text(), "old");
    /// text.set_text("new");
    /// cell.restyle_text_from(&text);
    /// assert_eq!(cell.text(), "new");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn text_artist(&self) -> Text {
        let (string, color, font, halign, valign, rotation, alpha) = self.read(|cell| {
            (
                cell.text.clone(),
                cell.text_color.clone(),
                cell.font(),
                cell.loc,
                cell.text_valign,
                cell.text_rotation,
                cell.text_alpha,
            )
        });
        let text = Text::loose(0.0, 0.0, string);
        text.set_color(color)
            .set_font(font)
            .set_halign(halign)
            .set_valign(valign)
            .set_rotation(rotation);
        text.set_alpha(alpha);
        text
    }

    /// Takes the string, colour, font, alignments, rotation and opacity
    /// of `text` (made by [`TableCell::text_artist`]).
    pub fn restyle_text_from(&self, text: &Text) -> &Self {
        let (string, color, font) = (text.text(), text.color(), text.font());
        let (halign, valign, rotation, alpha) =
            (text.halign(), text.valign(), text.rotation(), text.alpha());
        self.update(|cell| {
            cell.text = string;
            cell.text_color = color;
            cell.set_font(font);
            cell.loc = halign;
            cell.text_valign = valign;
            cell.text_rotation = rotation;
            cell.text_alpha = alpha;
        })
    }
}

/// The font size, stepped down from the cell's a point at a time to 1, at
/// which its text fits in its width.
fn fitted_size(
    cell: &Cell,
    layout: LayoutContext<'_>,
    rc: &RcParams,
    maps: &AxesMaps,
) -> Result<f64, TextDrawError> {
    let mut size = cell.fontsize();
    let mut required = cell.required_width(size, layout, rc, &maps.to_axes)?;
    while size > 1.0 && required > cell.width {
        size -= 1.0;
        required = cell.required_width(size, layout, rc, &maps.to_axes)?;
    }
    Ok(size)
}
