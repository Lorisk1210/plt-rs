//! Painting a figure: drawing its artists with its state in its cell,
//! where the program's code the draw runs finds it free.
//!
//! A draw runs the program's code (a path effect, a norm, a transform),
//! which matplotlib lets read and change the figure at once, the rest of
//! the draw seeing the change. So painting keeps the figure's state in its
//! cell and borrows it only for the moments the drawing reads it (see
//! [`DrawContext`]): the program's code the drawing calls meanwhile finds
//! the figure free on the painting thread. For other threads (a host's
//! thread switching in the middle of that code) the figure is in use until
//! the painting ends, as a figure a layout lends its state from.

use pltrs_render::Renderer;

use super::FigureCell;
use super::lend::{Lending, current_thread};
use crate::artist::{DrawContext, DrawError};

/// The figure lent to the painting thread for the length of a painting,
/// given back to where it was when dropped (also when the painting
/// unwinds).
struct LentForPainting<'a> {
    cell: &'a FigureCell,
    before: Lending,
}

impl<'a> LentForPainting<'a> {
    /// Lends the figure of `cell` to the current thread, unless code
    /// further up the call stack lends it already.
    fn new(cell: &'a FigureCell) -> Self {
        let before = cell.lending();
        if before == Lending::Home {
            cell.set_lending(Lending::Lent(current_thread()));
        }
        cell.count_painting(true);
        Self { cell, before }
    }
}

impl Drop for LentForPainting<'_> {
    fn drop(&mut self) {
        self.cell.count_painting(false);
        self.cell.set_lending(self.before);
    }
}

impl FigureCell {
    /// Runs `paint` with a context drawing the figure through `renderer`,
    /// then ends what it rasterized; returns what `paint` returned and the
    /// problems met. Meanwhile the figure is free for the program's code
    /// the drawing runs on this thread, wherever the context lets it go,
    /// and in use for other threads.
    pub(crate) fn paint<R>(
        &self,
        renderer: &mut dyn Renderer,
        paint: impl FnOnce(&mut DrawContext<'_>) -> R,
    ) -> (R, Vec<DrawError>) {
        let _lent = LentForPainting::new(self);
        let mut ctx = DrawContext::new(self, renderer);
        let painted = paint(&mut ctx);
        ctx.finish_rasterizing();
        (painted, ctx.into_errors())
    }

    /// Draws the whole figure through `renderer` (see
    /// [`paint`](Self::paint)), returning the problems met: those of the
    /// figure's layout, then those of its drawing.
    pub(crate) fn draw(&self, renderer: &mut dyn Renderer) -> Vec<DrawError> {
        let root = self.borrow().root;
        let ((), drawn) = self.paint(renderer, |ctx| ctx.draw_child(root));
        let mut errors = self.borrow().draw_problems();
        errors.extend(drawn);
        errors
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use pltrs_core::{Affine2D, Path, Rgba};
    use pltrs_render::{GraphicsContext, Renderer};

    use crate::artist::ArtistHandle;
    use crate::axes::{Axes, TitleLocation};
    use crate::colors::Color;
    use crate::figure::Figure;
    use crate::lines::Line2D;
    use crate::patheffects::{CustomEffect, CustomPathEffect, PathEffect};

    /// Changes the figure while it draws the path of `line`: its colour,
    /// the title, the width of `other`; records what it reads back.
    #[derive(Debug)]
    struct Changing {
        line: Line2D,
        other: Line2D,
        ax: Axes,
        seen: Rc<RefCell<Vec<String>>>,
    }

    impl CustomPathEffect for Changing {
        fn draw_path(
            &self,
            renderer: &mut dyn Renderer,
            gc: &mut GraphicsContext,
            path: &Path,
            affine: &Affine2D,
            fill: Option<Rgba>,
        ) {
            self.line.set_color("red");
            let title = self.ax.set_title("made while drawing");
            self.other.set_linewidth(4.0);
            self.seen.borrow_mut().extend([
                format!("{:?}", self.line.color()),
                title.text(),
                self.other.linewidth().to_string(),
            ]);
            renderer.draw_path(gc, path, affine, fill);
        }
    }

    /// A figure of two lines, the second drawn after the first.
    fn two_lines() -> (Figure, Axes, Line2D, Line2D) {
        let fig = Figure::with_size(3.0, 2.0, 50.0);
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("a line");
        let other = ax.plot([0.0, 1.0], [0.0, 1.0]).expect("a line");
        line.set_zorder(1.0);
        other.set_zorder(2.0);
        (fig, ax, line, other)
    }

    #[test]
    fn code_the_drawing_runs_changes_the_figure_at_once() {
        let (fig, ax, line, other) = two_lines();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let effect = Changing {
            line: line.clone(),
            other: other.clone(),
            ax: ax.clone(),
            seen: Rc::clone(&seen),
        };
        line.set_path_effects(vec![PathEffect::custom(CustomEffect::new(effect))]);
        let drawn = fig.to_png().expect("drawn");
        assert_eq!(
            *seen.borrow(),
            [
                format!("{:?}", Color::from("red")),
                "made while drawing".to_owned(),
                "4".to_owned()
            ],
            "the changes are read back at once"
        );
        assert!(!fig.raw().figure_in_use());
        assert!(!line.stale() && !other.stale(), "changed while drawn");
        // The rest of the draw saw the changes: the other line and the
        // title, drawn after, are drawn as a figure made with them draws
        // them.
        let (made, made_ax, _, made_other) = two_lines();
        made_other.set_linewidth(4.0);
        made_ax.set_title("made while drawing");
        assert_eq!(drawn, made.to_png().expect("drawn"));
        assert_eq!(ax.title(TitleLocation::Center), "made while drawing");
    }
}
