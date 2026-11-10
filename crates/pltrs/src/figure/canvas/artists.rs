//! Drawing single artists onto the canvas (matplotlib's `draw_artist`
//! and `Axes.redraw_in_frame`).

use std::rc::Rc;

use pltrs_core::{Affine2D, Bbox, Path, Point, Rgba};
use pltrs_render::{GraphicsContext, Renderer};
use pltrs_text::TextSource;

use super::{CanvasError, FigureCanvas};
use crate::artist::{ArtistHandle, ArtistId, DrawContext, Drawing, LayoutContext, RawHandle};
use crate::axes::{Axes, AxesState};
use crate::figure::{DrawErrors, Figure};
use crate::fonts::FontProperties;
use crate::legend::{LegendArtist, lay_out_legend};
use crate::text::{LineKind, typeset_line};
use crate::ticker::AxisName;

impl FigureCanvas {
    /// Draws `artist` onto the canvas, as its figure's last layout places
    /// and clips it (a legend of this canvas's figure is placed anew),
    /// through its painter with `painted`.
    fn draw_one(&self, artist: &RawHandle, painted: bool) -> Result<(), CanvasError> {
        if !self.holds(artist) {
            return self.draw_foreign(|renderer| {
                if painted {
                    artist.draw_onto(renderer)
                } else {
                    artist.draw_unpainted_onto(renderer)
                }
            });
        }
        self.draw_part(artist, painted, |ctx| {
            if painted {
                ctx.draw_child(artist.id());
            } else {
                ctx.draw_unpainted(artist.id());
            }
        })
    }

    /// Whether `artist` is in this canvas's figure.
    pub(crate) fn holds(&self, artist: &RawHandle) -> bool {
        Rc::ptr_eq(&self.figure.state, artist.figure_state())
    }

    /// Draws what `draw` draws of an artist of another figure onto the
    /// canvas, through its renderer: matplotlib draws an artist with the
    /// renderer it is given, where the artist's own figure places it.
    pub(crate) fn draw_foreign(
        &self,
        draw: impl FnOnce(&mut dyn Renderer) -> Result<(), DrawErrors>,
    ) -> Result<(), CanvasError> {
        let mut kept = self.take()?;
        let drawn = draw(&mut kept.renderer);
        self.keep(kept);
        drawn.map_err(CanvasError::Draw)
    }

    /// Draws what `draw` draws of `artist` of this canvas's figure onto
    /// the canvas, in the figure's last layout (a legend is placed anew),
    /// its painters prepared with `painted`.
    pub(crate) fn draw_part(
        &self,
        artist: &RawHandle,
        painted: bool,
        draw: impl FnOnce(&mut DrawContext<'_>),
    ) -> Result<(), CanvasError> {
        if !self.holds(artist) {
            return Err(CanvasError::ForeignArtist);
        }
        if artist.is::<LegendArtist>() {
            // matplotlib's `Legend.draw` places the legend before drawing it.
            lay_out_legend(&mut self.figure.state.borrow_mut(), artist.id())
                .map_err(|err| CanvasError::Draw(DrawErrors(vec![err])))?;
        }
        let mut kept = self.take()?;
        artist.prepare_painters(&kept.renderer, painted, true);
        let ((), errors) = self.figure.state.paint(&mut kept.renderer, draw);
        self.keep(kept);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(CanvasError::Draw(DrawErrors(errors)))
        }
    }
}

impl FigureCanvas {
    /// Draws `path`, mapped into display pixels by `transform`, onto the
    /// canvas over what it holds: stroked as `gc` says and filled with
    /// `fill` if given (matplotlib's `RendererAgg.draw_path`, through
    /// which a program draws on the canvas). Show the result with
    /// [`FigureCanvas::blit`].
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs_core::{Affine2D, Path, Point, Rgba};
    /// use pltrs_render::GraphicsContext;
    /// let fig = Figure::with_size(1.0, 1.0, 10.0);
    /// let corners = [(2.0, 2.0), (8.0, 2.0), (8.0, 8.0), (2.0, 8.0)];
    /// let square = Path::polygon(corners.map(|(x, y)| Point::new(x, y)));
    /// let canvas = fig.canvas();
    /// let gc = GraphicsContext::default();
    /// canvas.draw_path(&gc, &square, &Affine2D::IDENTITY, Some(Rgba::BLACK))?;
    /// let centre = canvas.buffer_rgba()?.pixel(5, 5);
    /// assert_eq!(centre, Some([0, 0, 0, 255]));
    /// # Ok::<(), pltrs::figure::CanvasError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the canvas cannot be made.
    pub fn draw_path(
        &self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) -> Result<(), CanvasError> {
        let mut kept = self.take()?;
        kept.renderer.draw_path(gc, path, transform, fill);
        self.keep(kept);
        Ok(())
    }
}

impl FigureCanvas {
    /// Draws `drawing` onto the canvas over what it holds (what a program
    /// drew through the canvas's renderer, see [`Drawing`]); show the
    /// result with [`FigureCanvas::blit`].
    ///
    /// # Errors
    ///
    /// When the canvas cannot be made, or a part of the drawing cannot be
    /// drawn.
    pub fn draw_drawing(&self, drawing: &Drawing) -> Result<(), CanvasError> {
        let mut kept = self.take()?;
        let problems = drawing.draw_onto(&mut kept.renderer);
        self.keep(kept);
        if problems.is_empty() {
            Ok(())
        } else {
            Err(CanvasError::Draw(DrawErrors(problems)))
        }
    }
}

impl FigureCanvas {
    /// Draws the line `text`, typeset as `kind` in `font` at the canvas's
    /// resolution, onto the canvas over what it holds, with the left end
    /// of its baseline at `origin` (display pixels) and rotated by `angle`
    /// degrees counter-clockwise (matplotlib's `RendererAgg.draw_text`
    /// and `draw_tex`, through which a program draws text on the canvas).
    ///
    /// # Errors
    ///
    /// When the canvas cannot be made, or the line cannot be typeset or
    /// its TeX fonts cannot be drawn.
    pub fn draw_text(
        &self,
        gc: &GraphicsContext,
        text: &str,
        font: &FontProperties,
        kind: LineKind,
        origin: Point,
        angle: f64,
    ) -> Result<(), CanvasError> {
        let dpi = self.figure.dpi();
        let run =
            typeset_line(text, font, kind, dpi, &crate::rc::params()).map_err(CanvasError::Text)?;
        let mut kept = self.take()?;
        let drawn = if let Some(TextSource::Tex(tex)) = &run.source {
            kept.renderer
                .draw_tex(gc, &run, tex, origin, angle)
                .map_err(|err| CanvasError::Text(err.into()))
        } else {
            kept.renderer.draw_text(gc, &run, origin, angle);
            Ok(())
        };
        self.keep(kept);
        drawn
    }
}

impl Figure {
    /// Draws `artist`, animated or not, onto the figure's canvas over what
    /// it holds, placed and clipped as its figure was last laid out, but
    /// for a legend of this figure, placed anew as matplotlib's
    /// `Legend.draw` places it (matplotlib's `Figure.draw_artist`); an
    /// artist of another figure is drawn where that figure places it, as
    /// matplotlib draws an artist with the renderer it is given. Show the
    /// result with [`FigureCanvas::blit`]. An invisible artist draws
    /// nothing.
    ///
    /// # Errors
    ///
    /// When the canvas cannot be made, or the artist cannot be drawn.
    pub fn draw_artist(&self, artist: &impl ArtistHandle) -> Result<(), CanvasError> {
        self.canvas().draw_one(artist.raw(), true)
    }

    /// [`Figure::draw_artist`] with the artist drawn as pltrs draws it, its
    /// painter left out (see [`ArtistPainter`](crate::artist::ArtistPainter)):
    /// what the drawing of a painter of the program's draws its artist with
    /// onto the canvas (matplotlib's `super().draw(renderer)`). The artist
    /// may have been removed from the figure.
    ///
    /// # Errors
    ///
    /// As [`Figure::draw_artist`].
    pub fn draw_artist_unpainted(&self, artist: &impl ArtistHandle) -> Result<(), CanvasError> {
        self.canvas().draw_one(artist.raw(), false)
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
        self.canvas().draw_one(artist.raw(), true)
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
        let drawn = self.canvas().draw_one(self.raw(), true);
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
