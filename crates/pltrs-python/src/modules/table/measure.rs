//! The text of a cell: as a text artist (matplotlib's `Cell.get_text`),
//! and measured where its table draws it (`Cell.get_text_bounds`, which
//! `get_required_width` widens, and `auto_set_font_size`); the cell's look as a
//! rectangle (matplotlib's `Cell` is a `Rectangle`); the coordinates the
//! cell is drawn in (`Cell.set_transform`); and the cell drawn alone
//! (`Cell.draw`).

use pltrs::artist::{DrawStep, Drawing};
use pltrs::figure::Figure;
use pltrs::patches::{Patch, Rectangle};
use pltrs::text::{Text, TextDrawError};
use pltrs::transforms::Transform;
use pltrs_render::RasterRenderer;

use super::super::drawing::PythonRenderer;
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

/// Draws the cell alone into `drawing`, where its table places it
/// (`Cell.draw` with a renderer recording while pltrs (Rust) draws).
fn draw_cell_into(drawing: Native<Drawing>, cell: CellRef) -> PyResult<()> {
    drawing.borrow_mut().push(DrawStep::TableCell(cell.0));
    Ok(())
}

/// Draws the cell alone through the drawing methods of `renderer`, a
/// renderer of the program's (`Cell.draw` with it).
fn draw_cell_through(renderer: Owned, cell: CellRef) -> PyResult<()> {
    Ok(cell.0.draw_onto(&mut PythonRenderer::new(renderer, None))?)
}

/// A loose rectangle with the cell's look, where its table last placed
/// it (the mirror of a `Cell`).
fn cell_rectangle(cell: CellRef) -> PyResult<Patch<Rectangle>> {
    Ok(cell.0.rectangle())
}

/// The corner where the table places the cell, in the fractions of its
/// axes; `None` for a table in no axes.
fn cell_corner(cell: CellRef) -> PyResult<Option<(f64, f64)>> {
    Ok(cell.0.placed_corner().map(|p| (p.x, p.y)))
}

/// The font size the table draws the cell's text at, shrunk to fit when
/// the table sizes its fonts itself; `None` for a table in no axes.
fn cell_placed_fontsize(cell: CellRef) -> PyResult<Option<f64>> {
    Ok(cell.0.placed_fontsize())
}

/// Gives the cell the look of `rectangle` (its mirror).
fn restyle_cell(cell: CellRef, rectangle: Patch<Rectangle>) -> PyResult<()> {
    cell.0.restyle_from(&rectangle);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("text_artist", text_artist)?;
    module.add("restyle_text", restyle_text)?;
    module.add("text_bounds", text_bounds)?;
    module.add("fit_font_size", fit_font_size)?;
    module.add("set_cell_transform", set_cell_transform)?;
    module.add("draw_cell", draw_cell)?;
    module.add("draw_cell_onto_raster", draw_cell_onto_raster)?;
    module.add("draw_cell_through", draw_cell_through)?;
    module.add("draw_cell_into", draw_cell_into)?;
    module.add("cell_rectangle", cell_rectangle)?;
    module.add("cell_corner", cell_corner)?;
    module.add("cell_placed_fontsize", cell_placed_fontsize)?;
    module.add("restyle_cell", restyle_cell)
}
