//! Measuring and drawing an artist apart from its figure's draws:
//! matplotlib's `get_window_extent` and `draw(renderer)` work on any
//! artist that knows its figure, whether the figure draws it or not (an
//! artist removed from it, or only given it with `set_figure`).

use pltrs_core::Bbox;

use super::{ArtistHandle, ArtistLookup, LayoutContext, RawHandle, Renderer};
use crate::figure::DrawErrors;

impl RawHandle {
    /// The artist's extent in display coordinates in its figure's current
    /// layout (laid out first), whether the figure draws the artist or not
    /// (matplotlib's `get_window_extent`); `None` when it occupies no
    /// space. Unlike [`ArtistHandle::window_extent`](super::ArtistHandle::window_extent),
    /// an artist removed from its figure, or made outside any, is measured
    /// too: where it was, with the axes and the figure it was removed
    /// with (a spine of a removed axes with that axes' ticks).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let label = ax.text(0.5, 0.5, "gone");
    /// let extent = label.window_extent();
    /// label.remove();
    /// assert_eq!(label.window_extent(), None);
    /// assert_eq!(label.raw().measured_extent(), extent);
    /// ```
    pub fn measured_extent(&self) -> Option<Bbox> {
        let _standing = self.standing_in();
        let state = self.figure_state().prepared();
        let ctx = LayoutContext::new(&state);
        state.artist(self.id())?.window_extent(&ctx)
    }

    /// [`measured_extent`](Self::measured_extent) in the figure's state of
    /// now, without laying the figure out first: what matplotlib's
    /// `Text.get_window_extent` gives, which reads the text's transform as
    /// it is (before a draw, with the view limits autoscaling has not
    /// computed yet and the positions a layout engine has not set yet).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::prelude::*;
    /// let fig = Figure::with_size(4.0, 4.0, 100.0);
    /// let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    /// let label = ax.text(1.0, 1.0, "at the corner");
    /// ax.plot([0.0, 4.0], [0.0, 4.0])?;
    /// let before = label.raw().extent_as_is().expect("measured");
    /// assert_eq!(before.x0(), 400.0, "at x = 1 of the unit view");
    /// let laid_out = label.raw().measured_extent().expect("measured");
    /// assert!(laid_out.x0() < 400.0, "autoscaled to the line");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn extent_as_is(&self) -> Option<Bbox> {
        let _standing = self.standing_in();
        let state = self.figure_state().borrow();
        let ctx = LayoutContext::new(&state);
        state.artist(self.id())?.window_extent(&ctx)
    }

    /// Draws the artist through `renderer` in its figure's current layout
    /// (matplotlib's `artist.draw(renderer)`), whether the figure draws
    /// it or not: through its painter if it has one, else as
    /// [`RawHandle::draw_unpainted_onto`] draws it.
    ///
    /// # Errors
    ///
    /// The problems met while drawing.
    pub fn draw_onto(&self, renderer: &mut dyn Renderer) -> Result<(), DrawErrors> {
        self.draw_apart(renderer, true)
    }

    /// Draws the artist through `renderer` in its figure's current layout,
    /// whether the figure draws it or not, as pltrs draws it, its painter
    /// left out (what a painter of the program's draws its artist with,
    /// matplotlib's `super().draw(renderer)`): through its image filter
    /// if it has one; an invisible artist draws nothing. The artist then
    /// counts as drawn at the renderer's resolution.
    ///
    /// # Errors
    ///
    /// The problems met while drawing.
    pub fn draw_unpainted_onto(&self, renderer: &mut dyn Renderer) -> Result<(), DrawErrors> {
        self.draw_apart(renderer, false)
    }

    /// Draws the artist through `renderer`, through its painter with
    /// `painted`; a root figure is laid out and drawn whole (matplotlib's
    /// `figure.draw(renderer)`).
    fn draw_apart(&self, renderer: &mut dyn Renderer, painted: bool) -> Result<(), DrawErrors> {
        let figure = self.figure();
        if figure.id() == self.id() {
            return figure.draw(renderer);
        }
        self.prepare_painters(renderer, painted, true);
        let Some(artist) = self.figure_state().borrow().shared_artist(self.id()) else {
            return Ok(());
        };
        let ((), errors) = self.figure_state().paint(renderer, |ctx| {
            match artist.base().painter.as_ref().filter(|_| painted) {
                Some(painter) => ctx.draw_painted(self.id(), &*artist, painter),
                None => ctx.draw_unpainted(self.id()),
            }
        });
        if errors.is_empty() {
            Ok(())
        } else {
            Err(DrawErrors(errors))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::text::{AnnotateOptions, Annotation, ArrowProps, LegacyArrowProps};

    #[test]
    fn an_artist_given_only_its_figure_measures_and_draws_there() {
        let fig = Figure::with_size(2.0, 2.0, 100.0);
        let options = AnnotateOptions {
            xytext: Some((50.0, 50.0).into()),
            xycoords: "figure pixels".parse().expect("a coordinate name"),
            arrowprops: Some(ArrowProps::Legacy(LegacyArrowProps {
                width: 8.0,
                headwidth: 10.0,
                shrink: 0.0,
                ..LegacyArrowProps::default()
            })),
            ..AnnotateOptions::default()
        };
        let note = Annotation::loose("", (0.0, 50.0), options).expect("not an offset");
        let note = fig.take_artist(note.raw()).expect("a loose artist");
        assert!(!note.is_attached());
        let mut canvas =
            pltrs_render::RasterRenderer::new(200.0, 200.0, 100.0).expect("a small canvas");
        note.draw_onto(&mut canvas).expect("drawn");
        let extent = note.measured_extent().expect("the arrow occupies space");
        // The arrow is measured as drawn, at 100/72 pixels per point.
        assert!((extent.width() - 50.0).abs() < 1e-9, "{extent:?}");
        assert!((extent.height() - 1000.0 / 72.0).abs() < 1e-9, "{extent:?}");
        assert_eq!(note.window_extent(), None, "the figure does not draw it");
    }
}
