//! The text of a cell: as a text artist (matplotlib's `Cell.get_text`),
//! and measured where its table draws it (`Cell.get_text_bounds`,
//! `get_required_width` and `auto_set_font_size`); the coordinates the
//! cell is drawn in (`Cell.set_transform`); and the cell drawn alone
//! (`Cell.draw`).

use pltrs::figure::Figure;
use pltrs::text::{Text, TextDrawError};
use pltrs::transforms::Transform;
use pltrs_render::RasterRenderer;

use super::CellRef;
use crate::binding::prelude::*;

/// `ValueError` for a text that cannot be laid out (malformed mathtext).
fn text_error(err: TextDrawError) -> PyErr {
    PyErr::value_error(err.to_string())
}

/// The `(x, y, width, height)` of the cell's text in the cell's
/// coordinates; `None` for a table in no axes.
fn text_bounds(cell: CellRef) -> PyResult<Option<(f64, f64, f64, f64)>> {
    Ok(cell
        .0
        .text_bounds()
        .map_err(text_error)?
        .map(|b| (b.x0(), b.y0(), b.width(), b.height())))
}

/// The width the cell needs for its text; `None` for a table in no axes.
fn required_width(cell: CellRef) -> PyResult<Option<f64>> {
    cell.0.required_width().map_err(text_error)
}

/// Shrinks the cell's font until its text fits; the size.
fn fit_font_size(cell: CellRef) -> PyResult<f64> {
    cell.0.auto_set_font_size().map_err(text_error)
}

/// A loose text with the look of the cell's text.
fn text_artist(cell: CellRef) -> PyResult<Text> {
    Ok(cell.0.text_artist())
}

/// Gives the cell's text the look of `text`.
fn restyle_text(cell: CellRef, text: Text) -> PyResult<()> {
    cell.0.restyle_text_from(&text);
    Ok(())
}

/// Draws the cell in the coordinates `transform` (`None`: the fractions
/// of the table's axes).
fn set_cell_transform(cell: CellRef, transform: Option<Transform>) -> PyResult<()> {
    cell.0.set_transform(transform);
    Ok(())
}

/// Draws the cell alone onto its figure's canvas, where its table was
/// last placed (`Cell.draw` with the canvas's renderer).
fn draw_cell(figure: Figure, cell: CellRef) -> PyResult<()> {
    Ok(figure.draw_cell(&cell.0)?)
}

/// Draws the cell alone onto the raster canvas `raster` (`Cell.draw` with
/// a `RendererAgg` the program made).
fn draw_cell_onto_raster(raster: Native<RasterRenderer>, cell: CellRef) -> PyResult<()> {
    Ok(cell.0.draw_onto(&mut *raster.borrow_mut())?)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("text_artist", text_artist)?;
    module.add("restyle_text", restyle_text)?;
    module.add("text_bounds", text_bounds)?;
    module.add("required_width", required_width)?;
    module.add("fit_font_size", fit_font_size)?;
    module.add("set_cell_transform", set_cell_transform)?;
    module.add("draw_cell", draw_cell)?;
    module.add("draw_cell_onto_raster", draw_cell_onto_raster)
}
