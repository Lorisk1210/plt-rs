//! `_native.table`: tables of text cells (matplotlib's `table`), by
//! pltrs (Rust)'s [`Table`].
//!
//! A table is an artist; its cells are not: they are part of the table's
//! native state, reached by their `(row, column)` key. A cell crosses as
//! `(table, row, column)` ([`CellRef`]). A cell made outside any table
//! (matplotlib's `Cell(...)`) lives in a table of its own, outside any
//! figure and without an axes, until [`put_cell`] copies it into a table.

use pltrs::axes::Axes;
use pltrs::colors::Color;
use pltrs::table::{CellEdges, Table, TableCell, TableError, TableLoc, TableOptions};
use pltrs::text::HAlign;
use pltrs::transforms::Bbox;

use super::font_manager::FontProps;
use super::path::{PathArrays, path_arrays};
use crate::binding::prelude::*;

mod measure;

artist_handle!(Table);

impl From<TableError> for PyErr {
    /// matplotlib's `ValueError`, with its message for invalid edges.
    fn from(err: TableError) -> Self {
        match err {
            TableError::Edges(edges) => PyErr::value_error(format!(
                "Invalid edge param {edges}, must only be one of \
                 open, closed, horizontal, vertical or string of B, R, T, L"
            )),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// A name pltrs (Rust) parses (an alignment, a location, edges).
fn parse<T: std::str::FromStr<Err: Into<PyErr>>>(name: &str) -> PyResult<T> {
    name.parse().map_err(Into::into)
}

/// A box in axes fractions as its extents `(x0, y0, x1, y1)`.
type Extents = (f64, f64, f64, f64);

/// The box of `extents`.
fn bbox(extents: Option<Extents>) -> Option<Bbox> {
    extents.map(|(x0, y0, x1, y1)| Bbox::from_extents(x0, y0, x1, y1))
}

/// The cells: `(texts, colours, alignment, column widths)`.
type CellsArgs = (
    Vec<Vec<String>>,
    Option<Vec<Vec<Color>>>,
    String,
    Option<Vec<f64>>,
);

/// A header row or column: `(labels, colours, alignment)`.
type HeaderArgs = (Option<Vec<String>>, Option<Vec<Color>>, String);

/// The placement: `(loc, bbox, edges, fontsize)`.
type PlaceArgs = (String, Option<Extents>, String, Option<f64>);

/// The table `cells`, `rows` and `cols` describe, added to `axes`
/// (matplotlib's `table()` once its arguments are checked).
fn axes_table(
    axes: Axes,
    (cell_text, cell_colours, cell_loc, col_widths): CellsArgs,
    (row_labels, row_colours, row_loc): HeaderArgs,
    (col_labels, col_colours, col_loc): HeaderArgs,
    (loc, extents, edges, fontsize): PlaceArgs,
) -> PyResult<Table> {
    Ok(axes.table(TableOptions {
        cell_text,
        cell_colours,
        cell_loc: Some(parse::<HAlign>(&cell_loc)?),
        col_widths,
        row_labels,
        row_colours,
        row_loc: Some(parse::<HAlign>(&row_loc)?),
        col_labels,
        col_colours,
        col_loc: Some(parse::<HAlign>(&col_loc)?),
        loc: parse::<TableLoc>(&loc)?,
        bbox: bbox(extents),
        edges: parse::<CellEdges>(&edges)?,
        fontsize,
    })?)
}

/// A table without cells for `axes`, outside any figure until the axes
/// adds it (matplotlib's `Table(ax, loc, bbox)`); without an axes, the
/// holder of a cell made outside any table.
fn new(axes: Option<Axes>, loc: String, extents: Option<Extents>) -> PyResult<Table> {
    Ok(Table::loose(
        axes.as_ref(),
        parse::<TableLoc>(&loc)?,
        bbox(extents),
    ))
}

/// The tables of `axes`, in the order they were added (matplotlib's
/// `ax.tables`).
fn axes_tables(axes: Axes) -> PyResult<Vec<Table>> {
    Ok(axes.tables())
}

/// The table's box in display space where the figure of `axes` places
/// it, for a table made for `axes` and not added (`Table.get_window_extent`).
fn extent_for(table: Table, axes: Axes) -> PyResult<Option<Extents>> {
    Ok(table
        .extent_for(&axes)
        .map(|bbox| (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())))
}

/// The `(row, column)` keys of the cells, in the order they were made.
fn cell_keys(table: Table) -> PyResult<Vec<(i32, i32)>> {
    Ok(table.cell_keys())
}

/// Whether the table has a cell at `(row, column)`.
fn has_cell(table: Table, row: i32, column: i32) -> PyResult<bool> {
    Ok(table.cell(row, column).is_some())
}

/// Adds a `width` × `height` cell with matplotlib's defaults at `(row,
/// column)`, replacing any cell there.
fn add_cell(table: Table, row: i32, column: i32, width: f64, height: f64) -> PyResult<()> {
    table.add_cell(row, column, width, height);
    Ok(())
}

/// Takes the cell at `(row, column)` out of the table; whether there was
/// one (a program deleting it from the table's dict of cells).
fn remove_cell(table: Table, row: i32, column: i32) -> PyResult<bool> {
    Ok(table.remove_cell(row, column))
}

/// Puts a copy of `cell` at `(row, column)` (matplotlib's `table[row,
/// col] = cell`).
fn put_cell(table: Table, row: i32, column: i32, cell: CellRef) -> PyResult<()> {
    table.put_cell(row, column, &cell.0);
    Ok(())
}

/// Turns the shrinking of the font size to fit the cells on or off.
fn auto_set_font_size(table: Table, on: bool) -> PyResult<()> {
    table.auto_set_font_size(on);
    Ok(())
}

/// Sets the text size of every cell in points.
fn set_fontsize(table: Table, size: f64) -> PyResult<()> {
    table.set_fontsize(size);
    Ok(())
}

/// Fits the widths of `columns` to their texts when the table is drawn.
fn auto_set_column_width(table: Table, columns: Vec<i32>) -> PyResult<()> {
    table.auto_set_column_width(&columns);
    Ok(())
}

/// Has the table place its cells from now on (`_update_positions`, which
/// drawing and measuring a table run).
fn place_cells(table: Table) -> PyResult<()> {
    table.place_cells();
    Ok(())
}

/// A cell, as Python names it: `(table, row, column)`.
struct CellRef(TableCell);

impl FromPy<'_> for CellRef {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (table, row, column): (Table, i32, i32) = object.extract()?;
        table
            .cell(row, column)
            .map(Self)
            .ok_or_else(|| PyErr::key_error(format!("({row}, {column})")))
    }
}

/// The text of a cell.
fn get_text(cell: CellRef) -> PyResult<String> {
    Ok(cell.0.text())
}

/// Replaces the text of a cell.
fn set_text(cell: CellRef, text: String) -> PyResult<()> {
    cell.0.set_text(text);
    Ok(())
}

/// The text size of a cell in points.
fn get_cell_fontsize(cell: CellRef) -> PyResult<f64> {
    Ok(cell.0.fontsize())
}

/// Sets the text size of a cell in points.
fn set_cell_fontsize(cell: CellRef, size: f64) -> PyResult<()> {
    cell.0.set_fontsize(size);
    Ok(())
}

/// Sets the font of a cell's text.
fn set_font(cell: CellRef, font: FontProps) -> PyResult<()> {
    cell.0.set_font(font.0);
    Ok(())
}

/// Sets the colour of a cell's text.
fn set_text_color(cell: CellRef, color: Color) -> PyResult<()> {
    cell.0.set_text_color(color);
    Ok(())
}

/// Aligns a cell's text `"left"`, `"center"` or `"right"`.
fn set_text_halign(cell: CellRef, halign: String) -> PyResult<()> {
    cell.0.set_text_halign(parse::<HAlign>(&halign)?);
    Ok(())
}

/// Sets the background colour of a cell.
fn set_facecolor(cell: CellRef, color: Color) -> PyResult<()> {
    cell.0.set_facecolor(color);
    Ok(())
}

/// Sets the colour of a cell's edges.
fn set_edgecolor(cell: CellRef, color: Color) -> PyResult<()> {
    cell.0.set_edgecolor(color);
    Ok(())
}

/// Sets the width of a cell's edges in points.
fn set_linewidth(cell: CellRef, width: f64) -> PyResult<()> {
    cell.0.set_linewidth(width);
    Ok(())
}

/// Sets the opacity of a cell's background and edges.
fn set_alpha(cell: CellRef, alpha: Option<f64>) -> PyResult<()> {
    cell.0.set_alpha(alpha);
    Ok(())
}

/// The edges of a cell that are drawn, by name.
fn get_visible_edges(cell: CellRef) -> PyResult<String> {
    Ok(cell.0.visible_edges().to_string())
}

/// Chooses the edges of a cell that are drawn: `"closed"`, `"open"`,
/// `"horizontal"`, `"vertical"` or letters of `BRTL`.
fn set_visible_edges(cell: CellRef, edges: String) -> PyResult<()> {
    cell.0.set_visible_edges(parse::<CellEdges>(&edges)?);
    Ok(())
}

/// The outline of a cell in its unit square (matplotlib's
/// `Cell.get_path`).
fn get_path(cell: CellRef) -> PyResult<PathArrays> {
    path_arrays(&cell.0.path(), false)
}

/// The `(width, height)` of a cell in axes fractions, and whether its
/// table set each since it was given (fitted, or stretched).
fn get_size(cell: CellRef) -> PyResult<(f64, f64, bool, bool)> {
    let fitted = (cell.0.width_fitted(), cell.0.height_fitted());
    Ok((cell.0.width(), cell.0.height(), fitted.0, fitted.1))
}

/// Sets the width of a cell in axes fractions.
fn set_width(cell: CellRef, width: f64) -> PyResult<()> {
    cell.0.set_width(width);
    Ok(())
}

/// Sets the height of a cell in axes fractions.
fn set_height(cell: CellRef, height: f64) -> PyResult<()> {
    cell.0.set_height(height);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("axes_table", axes_table)?;
    module.add("new", new)?;
    module.add("axes_tables", axes_tables)?;
    module.add("cell_keys", cell_keys)?;
    module.add("extent_for", extent_for)?;
    module.add("has_cell", has_cell)?;
    module.add("add_cell", add_cell)?;
    module.add("put_cell", put_cell)?;
    module.add("remove_cell", remove_cell)?;
    module.add("auto_set_font_size", auto_set_font_size)?;
    module.add("set_fontsize", set_fontsize)?;
    module.add("auto_set_column_width", auto_set_column_width)?;
    module.add("place_cells", place_cells)?;
    module.add("get_text", get_text)?;
    module.add("set_text", set_text)?;
    module.add("get_cell_fontsize", get_cell_fontsize)?;
    module.add("set_cell_fontsize", set_cell_fontsize)?;
    module.add("set_font", set_font)?;
    module.add("set_text_color", set_text_color)?;
    module.add("set_text_halign", set_text_halign)?;
    module.add("set_facecolor", set_facecolor)?;
    module.add("set_edgecolor", set_edgecolor)?;
    module.add("set_linewidth", set_linewidth)?;
    module.add("set_alpha", set_alpha)?;
    module.add("get_visible_edges", get_visible_edges)?;
    module.add("set_visible_edges", set_visible_edges)?;
    module.add("get_path", get_path)?;
    module.add("get_size", get_size)?;
    module.add("set_width", set_width)?;
    module.add("set_height", set_height)?;
    measure::register(module)
}

#[cfg(test)]
mod tests {
    use pltrs::artist::ArtistHandle;
    use pltrs::figure::Figure;

    use super::*;

    #[test]
    fn cells_made_outside_a_table_are_copied_into_one() {
        let ax = Figure::new().subplot();
        let holder = new(None, "bottom".into(), None).unwrap();
        add_cell(holder.clone(), 0, 0, 1.0, 2.0).unwrap();
        let made = CellRef(holder.cell(0, 0).unwrap());
        set_text(CellRef(holder.cell(0, 0).unwrap()), "x".into()).unwrap();
        set_visible_edges(CellRef(holder.cell(0, 0).unwrap()), "TB".into()).unwrap();
        let table = new(
            Some(ax.clone()),
            "center".into(),
            Some((0.1, 0.2, 0.9, 0.8)),
        )
        .unwrap();
        assert!(table.raw().is_loose());
        put_cell(table.clone(), 2, 1, made).unwrap();
        assert_eq!(cell_keys(table.clone()).unwrap(), [(2, 1)]);
        let put = || CellRef(table.cell(2, 1).unwrap());
        assert_eq!(get_text(put()).unwrap(), "x");
        assert_eq!(get_size(put()).unwrap(), (1.0, 2.0, false, false));
        assert_eq!(get_visible_edges(put()).unwrap(), "BT");
        assert!(!has_cell(table, 0, 0).unwrap());
    }

    #[test]
    fn bad_names_raise_value_errors() {
        let holder = new(None, "bottom".into(), None).unwrap();
        add_cell(holder.clone(), 0, 0, 1.0, 1.0).unwrap();
        let cell = || CellRef(holder.cell(0, 0).unwrap());
        assert!(set_visible_edges(cell(), "X".into()).is_err());
        assert!(set_text_halign(cell(), "middle".into()).is_err());
        assert!(new(None, "nowhere".into(), None).is_err());
        let ax = Figure::new().subplot();
        let empty = axes_table(
            ax,
            (Vec::new(), None, "right".into(), None),
            (None, None, "left".into()),
            (None, None, "center".into()),
            ("bottom".into(), None, "closed".into(), None),
        );
        assert!(empty.is_err());
    }
}
