//! Drawing single artists onto the canvas (matplotlib's `draw_artist`
//! and `Axes.redraw_in_frame`).

use std::rc::Rc;

use pltrs_core::Bbox;

use super::{CanvasError, FigureCanvas};
use crate::artist::{ArtistHandle, ArtistId, DrawContext, LayoutContext, RawHandle};
use crate::axes::{Axes, AxesState};
use crate::figure::{DrawErrors, Figure};
use crate::ticker::AxisName;

impl FigureCanvas {
    /// Draws `artist` of this canvas's figure onto the canvas, as the
    /// figure's last layout places and clips it.
    fn draw_one(&self, artist: &RawHandle) -> Result<(), CanvasError> {
        if !Rc::ptr_eq(&self.figure.state, artist.figure_state()) {
            return Err(CanvasError::ForeignArtist);
        }
        let mut kept = self.take()?;
        let errors = {
            let state = self.figure.state();
            let mut ctx = DrawContext::new(&state, &mut kept.renderer);
            ctx.draw_child(artist.id());
            ctx.finish_rasterizing();
            ctx.into_errors()
        };
        self.keep(kept);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(CanvasError::Draw(DrawErrors(errors)))
        }
    }
}

impl Figure {
    /// Draws `artist`, animated or not, onto the figure's canvas over what
    /// it holds, placed and clipped as the figure was last laid out
    /// (matplotlib's `Figure.draw_artist`); show the result with
    /// [`FigureCanvas::blit`]. An invisible artist draws nothing.
    ///
    /// # Errors
    ///
    /// When the artist belongs to another figure, the canvas cannot be
    /// made, or the artist cannot be drawn.
    pub fn draw_artist(&self, artist: &impl ArtistHandle) -> Result<(), CanvasError> {
        self.canvas().draw_one(artist.raw())
    }
}

impl Axes {
    /// Draws `artist`, animated or not, onto the figure's canvas over what
    /// it holds (matplotlib's `Axes.draw_artist`, the same as
    /// [`Figure::draw_artist`]).
    ///
    /// # Errors
    ///
    /// As [`Figure::draw_artist`].
    pub fn draw_artist(&self, artist: &impl ArtistHandle) -> Result<(), CanvasError> {
        self.canvas().draw_one(artist.raw())
    }

    /// Draws the axes onto the figure's canvas without their x and y axes
    /// and titles, so its background, spines and contents, its animated
    /// artists left out (matplotlib's `redraw_in_frame`).
    ///
    /// # Errors
    ///
    /// When the canvas cannot be made or an artist cannot be drawn.
    pub fn redraw_in_frame(&self) -> Result<(), CanvasError> {
        let hidden = HiddenDecorations::hide(self);
        let drawn = self.canvas().draw_one(self.raw());
        hidden.show(self);
        drawn
    }

    /// The axes' box in display pixels from the bottom-left corner of the
    /// canvas, as the figure was last laid out (matplotlib's `Axes.bbox`);
    /// an empty box for axes no longer in their figure.
    pub fn bbox(&self) -> Bbox {
        let state = self.raw().figure_state().borrow();
        LayoutContext::new(&state)
            .axes_extent(self.id())
            .unwrap_or(Bbox::from_extents(0.0, 0.0, 0.0, 0.0))
    }

    /// The canvas of the axes' figure.
    fn canvas(&self) -> FigureCanvas {
        Figure::root_of(self.raw().figure_state()).canvas()
    }
}

/// What [`Axes::redraw_in_frame`] hides while it draws, with whether it
/// was visible.
struct HiddenDecorations {
    axes: [(AxisName, bool); 2],
    titles: Vec<(ArtistId, bool)>,
}

impl HiddenDecorations {
    /// Hides the x and y axes and the titles of `ax`, without the figure
    /// counting as changed (matplotlib's `_cm_set(visible=False)`).
    fn hide(ax: &Axes) -> Self {
        let mut state = ax.raw().figure_state().borrow_mut();
        let mut hidden = Self {
            axes: [(AxisName::X, false), (AxisName::Y, false)],
            titles: Vec::new(),
        };
        let Some(axes) = state.store.get_as_mut::<AxesState>(ax.id()) else {
            return hidden;
        };
        for (name, visible) in &mut hidden.axes {
            let axis = axes.axis_mut(*name);
            *visible = std::mem::replace(&mut axis.visible, false);
        }
        let titles = axes.titles.clone();
        for title in titles {
            if let Some(artist) = state.store.get_mut(title) {
                let visible = std::mem::replace(&mut artist.base_mut().visible, false);
                hidden.titles.push((title, visible));
            }
        }
        hidden
    }

    /// Shows again what [`HiddenDecorations::hide`] hid.
    fn show(self, ax: &Axes) {
        let mut state = ax.raw().figure_state().borrow_mut();
        if let Some(axes) = state.store.get_as_mut::<AxesState>(ax.id()) {
            for (name, visible) in self.axes {
                axes.axis_mut(name).visible = visible;
            }
        }
        for (title, visible) in self.titles {
            if let Some(artist) = state.store.get_mut(title) {
                artist.base_mut().visible = visible;
            }
        }
    }
}
