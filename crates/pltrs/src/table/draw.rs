//! Drawing the cells of a table: all of them as the table draws itself,
//! or one alone (matplotlib's `Cell.draw`).

use pltrs_render::Renderer;

use super::TableArtist;
use super::cell::PlacedCell;
use super::handle::TableCell;
use super::layout::AxesMaps;
use crate::artist::{ArtistHandle, ArtistId, DrawContext};
use crate::figure::{CanvasError, DrawErrors, Figure, FigureState};

impl TableArtist {
    /// The maps of the table's axes and its cells placed in the layout
    /// `ctx` draws in; `None`, the problem reported, when the table cannot
    /// be placed.
    pub(super) fn placed_for_drawing(
        &self,
        ctx: &mut DrawContext<'_>,
    ) -> Option<(AxesMaps, Vec<PlacedCell>)> {
        let layout = *ctx.layout();
        let Some(axes) = self.axes_box(layout) else {
            ctx.report(self.base.id(), "its axes no longer exist");
            return None;
        };
        let Some(maps) = AxesMaps::of(&axes) else {
            ctx.report(self.base.id(), "its axes has no area");
            return None;
        };
        match self.place(layout, &maps) {
            Ok(placed) => Some((maps, placed)),
            Err(err) => {
                ctx.report(self.base.id(), err);
                None
            }
        }
    }

    /// Draws the cell at `index`, placed at `placed` among the cells of a
    /// table in the axes `maps` maps, in the coordinates of the cell.
    pub(super) fn draw_cell(
        &self,
        ctx: &mut DrawContext<'_>,
        index: usize,
        placed: &PlacedCell,
        maps: &AxesMaps,
    ) {
        let cell = &self.cells[index].1;
        let Some(cell_maps) = cell.maps(*ctx.layout(), maps) else {
            return ctx.report(
                self.base.id(),
                "a cell's transform is not an invertible affine map",
            );
        };
        if let Err(err) = cell.draw(ctx, &self.base, &self.rc, placed, &cell_maps.to_display) {
            ctx.report(self.base.id(), err);
        }
    }
}

/// Draws the cell at `key` of the table `table` alone, where the table
/// places it.
fn draw_alone(ctx: &mut DrawContext<'_>, table: ArtistId, key: (i32, i32)) {
    let Some(artist) = ctx.layout().store().get_as::<TableArtist>(table) else {
        return;
    };
    let Some(index) = artist.cells.iter().position(|(at, _)| *at == key) else {
        return;
    };
    if let Some((maps, placed)) = artist.placed_for_drawing(ctx) {
        artist.draw_cell(ctx, index, &placed[index], &maps);
    }
}

impl TableCell {
    /// Draws the cell alone in `ctx`, where its table places it, when the
    /// table is in the figure `ctx` draws (a [`DrawStep::TableCell`] of a
    /// drawing); drawn in another figure, onto the renderer of `ctx`.
    ///
    /// [`DrawStep::TableCell`]: crate::artist::DrawStep::TableCell
    pub(crate) fn draw_in(&self, ctx: &mut DrawContext<'_>) {
        let raw = self.table().raw();
        if std::ptr::eq(
            raw.figure_state().as_ptr().cast_const(),
            ctx.layout().figure_state(),
        ) {
            draw_alone(ctx, raw.id(), self.key());
        } else if let Err(errors) = self.draw_onto(ctx.renderer()) {
            for error in errors.0 {
                ctx.report_kind(error.artist, error.kind, error.message);
            }
        }
    }

    /// Draws the cell alone onto `renderer`, its rectangle and its text
    /// where its table places it in the figure's current layout
    /// (matplotlib's `Cell.draw`); a cell of a table in no axes draws
    /// nothing.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::table::TableOptions;
    /// use pltrs_render::RasterRenderer;
    /// let fig = Figure::with_size(2.0, 2.0, 50.0);
    /// let ax = fig.subplot();
    /// let table = ax.table(TableOptions::with_text(vec![vec!["a".into()]]))?;
    /// let mut renderer = RasterRenderer::new(100.0, 100.0, 50.0)?;
    /// table.cell(0, 0).expect("a cell").draw_onto(&mut renderer)?;
    /// assert!(renderer.to_image().data().iter().any(|&byte| byte != 0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// The problems met while drawing.
    pub fn draw_onto(&self, renderer: &mut dyn Renderer) -> Result<(), DrawErrors> {
        let raw = self.table().raw();
        raw.figure_state().work_on(FigureState::prepare);
        let state = raw.figure_state().borrow();
        let mut ctx = DrawContext::new(&state, renderer);
        draw_alone(&mut ctx, raw.id(), self.key());
        ctx.finish_rasterizing();
        let errors = ctx.into_errors();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(DrawErrors(errors))
        }
    }
}

impl Figure {
    /// Draws `cell` alone onto the figure's canvas over what it holds,
    /// where its table was placed in the figure's last layout (matplotlib's
    /// `Cell.draw` with the canvas's renderer); show the result with
    /// [`FigureCanvas::blit`](crate::figure::FigureCanvas::blit).
    ///
    /// # Errors
    ///
    /// When the cell's table belongs to another figure, the canvas cannot
    /// be made, or the cell cannot be drawn.
    pub fn draw_cell(&self, cell: &TableCell) -> Result<(), CanvasError> {
        let table = cell.table().raw();
        self.canvas()
            .draw_part(table, true, |ctx| draw_alone(ctx, table.id(), cell.key()))
    }
}

#[cfg(test)]
mod tests {
    use crate::figure::{CanvasError, Figure};
    use crate::table::TableOptions;

    #[test]
    fn a_cell_draws_alone_onto_its_figure_canvas_but_not_another() {
        let fig = Figure::with_size(2.0, 2.0, 50.0);
        let table = fig
            .subplot()
            .table(TableOptions::with_text(vec![vec!["a".into(), "b".into()]]))
            .expect("a table");
        let cell = table.cell(0, 1).expect("a cell");
        fig.canvas().draw().expect("drawn");
        assert!(fig.draw_cell(&cell).is_ok());
        let other = Figure::new();
        assert!(matches!(
            other.draw_cell(&cell),
            Err(CanvasError::ForeignArtist)
        ));
    }
}
