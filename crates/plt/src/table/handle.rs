//! Handles to a table and to its cells (matplotlib's `Table` and the
//! `Cell`s of `table[row, col]`).

use plt_text::HAlign;

use super::TableArtist;
use super::cell::{Cell, CellEdges};
use crate::artist::{ArtistHandle, RawHandle};
use crate::colors::Color;
use crate::fonts::FontProperties;

/// A handle to a table (matplotlib's `Table`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    raw: RawHandle,
}

impl ArtistHandle for Table {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl Table {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    fn update(&self, f: impl FnOnce(&mut TableArtist)) -> &Self {
        self.raw.with_mut::<TableArtist, _>(f);
        self
    }

    /// Turns the shrinking of the font size to fit the cells on or off.
    pub fn auto_set_font_size(&self, on: bool) -> &Self {
        self.update(|t| t.auto_fontsize = on)
    }

    /// Sets the text size of every cell in points (still shrunk to fit
    /// unless [`auto_set_font_size`](Self::auto_set_font_size) is off).
    pub fn set_fontsize(&self, size: f64) -> &Self {
        self.update(|t| t.set_fontsize(size))
    }

    /// Scales column widths by `xscale` and row heights by `yscale`.
    pub fn scale(&self, xscale: f64, yscale: f64) -> &Self {
        self.update(|t| {
            for (_, cell) in &mut t.cells {
                cell.width *= xscale;
                cell.height *= yscale;
            }
        })
    }

    /// Fits the widths of these columns to their texts at draw time
    /// (`-1` is the row header column).
    pub fn auto_set_column_width(&self, columns: &[i32]) -> &Self {
        self.update(|t| t.auto_columns.extend_from_slice(columns))
    }

    /// The number of cells.
    pub fn len(&self) -> usize {
        self.raw.with::<TableArtist, _>(|t| t.cells.len())
    }

    /// Whether the table has no cells.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `(row, column)` keys of the cells in the order they were made
    /// (matplotlib's `get_celld()`): the cells row by row, then the column
    /// headers in row 0, then the row headers in column -1.
    pub fn cell_keys(&self) -> Vec<(i32, i32)> {
        self.raw
            .with::<TableArtist, _>(|t| t.cells.iter().map(|(key, _)| *key).collect())
    }

    /// The cell at `(row, column)`, if there is one (matplotlib's
    /// `table[row, col]`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::table::TableOptions;
    /// let ax = Figure::new().subplot();
    /// let table = ax.table(&TableOptions {
    ///     col_labels: Some(vec!["day".into()]),
    ///     ..TableOptions::with_text(vec![vec!["Mon".into()]])
    /// })?;
    /// let header = table.cell(0, 0).expect("the column header");
    /// header.set_facecolor("#40466e").set_text_color("w");
    /// assert_eq!(header.text(), "day");
    /// assert!(table.cell(3, 0).is_none());
    /// # Ok::<(), plt::table::TableError>(())
    /// ```
    pub fn cell(&self, row: i32, column: i32) -> Option<TableCell> {
        self.raw
            .with::<TableArtist, _>(|t| t.cells.iter().any(|(key, _)| *key == (row, column)))
            .then(|| TableCell {
                table: self.clone(),
                key: (row, column),
            })
    }
}

/// A handle to one cell of a table (matplotlib's `Cell`): a rectangle,
/// some of whose edges are drawn, with a text inside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableCell {
    table: Table,
    key: (i32, i32),
}

impl TableCell {
    fn read<R>(&self, f: impl FnOnce(&Cell) -> R) -> R {
        self.table.raw.with::<TableArtist, _>(|t| {
            let (_, cell) = t
                .cells
                .iter()
                .find(|(key, _)| *key == self.key)
                .expect("table cells are never removed");
            f(cell)
        })
    }

    fn update(&self, f: impl FnOnce(&mut Cell)) -> &Self {
        self.table.update(|t| {
            if let Some((_, cell)) = t.cells.iter_mut().find(|(key, _)| *key == self.key) {
                f(cell);
            }
        });
        self
    }

    /// Its `(row, column)` key.
    pub fn key(&self) -> (i32, i32) {
        self.key
    }

    /// The text.
    pub fn text(&self) -> String {
        self.read(|cell| cell.text.clone())
    }

    /// Replaces the text (matplotlib's `get_text().set_text`).
    pub fn set_text(&self, text: impl Into<String>) -> &Self {
        let text = text.into();
        self.update(|cell| cell.text = text)
    }

    /// The font of the text.
    pub fn font(&self) -> FontProperties {
        self.read(|cell| cell.font.clone())
    }

    /// Sets the font of the text: its weight, style, family and size
    /// (matplotlib's `set_text_props` with font keywords).
    pub fn set_font(&self, font: FontProperties) -> &Self {
        self.update(|cell| cell.font = font)
    }

    /// Sets the colour of the text (`set_text_props(color=...)`).
    pub fn set_text_color(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|cell| cell.text_color = color)
    }

    /// Aligns the text to the left, centre or right of the cell, a tenth
    /// of the cell width from the edge it aligns to
    /// (`set_text_props(ha=...)`).
    pub fn set_text_halign(&self, halign: HAlign) -> &Self {
        self.update(|cell| cell.loc = halign)
    }

    /// Sets the background colour.
    pub fn set_facecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|cell| cell.facecolor = color)
    }

    /// Sets the colour of the drawn edges.
    pub fn set_edgecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|cell| cell.edgecolor = color)
    }

    /// Sets the width of the drawn edges in points.
    pub fn set_linewidth(&self, width: f64) -> &Self {
        self.update(|cell| cell.linewidth = Some(width))
    }

    /// Sets the opacity of the background and edges; the text keeps its
    /// own.
    pub fn set_alpha(&self, alpha: Option<f64>) -> &Self {
        self.update(|cell| cell.alpha = alpha)
    }

    /// Chooses which edges are drawn (matplotlib's `visible_edges`). The
    /// background is filled only when all four are.
    pub fn set_visible_edges(&self, edges: CellEdges) -> &Self {
        self.update(|cell| cell.edges = edges)
    }

    /// Sets the width in axes fractions.
    pub fn set_width(&self, width: f64) -> &Self {
        self.update(|cell| cell.width = width)
    }

    /// Sets the height in axes fractions.
    pub fn set_height(&self, height: f64) -> &Self {
        self.update(|cell| cell.height = height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::table::TableOptions;

    #[test]
    fn cells_are_keyed_like_matplotlib_and_styled_one_by_one() {
        let ax = Figure::new().subplot();
        let table = ax
            .table(&TableOptions {
                col_labels: Some(vec!["A".into(), "B".into()]),
                row_labels: Some(vec!["r".into()]),
                ..TableOptions::with_text(vec![vec!["1".into(), "2".into()]])
            })
            .expect("a valid table");
        assert_eq!(table.cell_keys(), [(1, 0), (1, 1), (0, 0), (0, 1), (1, -1)]);
        assert!(table.cell(2, 0).is_none());
        let header = table.cell(0, 1).expect("a header cell");
        header
            .set_text("B2")
            .set_text_color("w")
            .set_facecolor("k")
            .set_linewidth(2.0)
            .set_alpha(Some(0.5))
            .set_visible_edges(CellEdges::OPEN)
            .set_height(0.2);
        assert_eq!(header.text(), "B2");
        header.read(|cell| {
            assert_eq!(cell.text_color, Color::from("w"));
            assert_eq!(cell.facecolor, Color::from("k"));
            assert_eq!((cell.linewidth, cell.alpha), (Some(2.0), Some(0.5)));
            assert_eq!((cell.edges, cell.height), (CellEdges::OPEN, 0.2));
        });
    }
}
