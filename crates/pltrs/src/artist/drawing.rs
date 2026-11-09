//! Drawings kept until they are drawn: what a program drew through a
//! renderer while pltrs was not drawing (matplotlib's renderer calls in
//! the `draw` of an `Artist` subclass, or in the `draw_path` of a path
//! effect of the program's), drawn where pltrs draws.
//!
//! A [`Drawing`] is a list of [`DrawStep`]s: the renderer's drawing calls
//! with what they draw, the groups, rasterizing and filtering the program
//! asked for, and artists drawn as pltrs draws them (matplotlib's
//! `artist.draw(renderer)`).

use std::rc::Rc;

use pltrs_core::{Affine2D, Path, Point, Rgba};
use pltrs_render::{GraphicsContext, PathCollectionData, QuadMeshData, Renderer, RgbaImage};
use pltrs_text::{TextRun, TextSource};

use super::{AggFilter, Artist, ArtistId, DrawContext, DrawError, DrawErrorKind, RawHandle};
use crate::figure::StateRole;
use crate::table::TableCell;

/// One drawing call of a [`Drawing`].
#[derive(Clone, Debug)]
pub enum DrawStep {
    /// [`Renderer::draw_path`].
    Path {
        /// How the path is stroked.
        gc: GraphicsContext,
        /// The path.
        path: Path,
        /// Its transform into display pixels.
        transform: Affine2D,
        /// Its fill.
        fill: Option<Rgba>,
    },
    /// [`Renderer::draw_markers`].
    Markers {
        /// How the markers are stroked.
        gc: GraphicsContext,
        /// The marker, in marker coordinates.
        marker: Path,
        /// Maps the marker to pixel offsets.
        marker_transform: Affine2D,
        /// The points marked.
        path: Path,
        /// Maps the points into display pixels.
        transform: Affine2D,
        /// The markers' fill.
        fill: Option<Rgba>,
    },
    /// [`Renderer::draw_path_collection`].
    PathCollection {
        /// The graphics context the items start from.
        gc: GraphicsContext,
        /// The collection.
        collection: Box<PathCollectionData>,
    },
    /// [`Renderer::draw_quad_mesh`].
    QuadMesh {
        /// The graphics context of the cells.
        gc: GraphicsContext,
        /// The mesh.
        mesh: Box<QuadMeshData>,
    },
    /// [`Renderer::draw_gouraud_triangles`].
    GouraudTriangles {
        /// The graphics context of the triangles.
        gc: GraphicsContext,
        /// The triangles.
        triangles: Vec<[Point; 3]>,
        /// The colours of their vertices.
        colors: Vec<[Rgba; 3]>,
        /// Maps the vertices into display pixels.
        transform: Affine2D,
    },
    /// [`Renderer::draw_image`].
    Image {
        /// The graphics context (clipping) of the image.
        gc: GraphicsContext,
        /// The bottom-left corner, in display pixels.
        origin: Point,
        /// The image.
        image: RgbaImage,
        /// Maps the image's unit square, for renderers scaling images.
        transform: Option<Affine2D>,
    },
    /// [`Renderer::draw_text`], or [`Renderer::draw_tex`] for a run of
    /// TeX text.
    Text {
        /// The graphics context (colour) of the text.
        gc: GraphicsContext,
        /// The laid-out line.
        run: Box<TextRun>,
        /// The left end of its baseline, in display pixels.
        origin: Point,
        /// Its rotation in degrees, counter-clockwise.
        angle: f64,
    },
    /// [`Renderer::open_group`].
    OpenGroup {
        /// The group's name.
        name: String,
        /// Its id.
        gid: Option<String>,
    },
    /// [`Renderer::close_group`].
    CloseGroup {
        /// The group's name.
        name: String,
    },
    /// The drawing that follows goes to a raster canvas, the program
    /// asked (matplotlib's `renderer.start_rasterizing()`).
    StartRasterizing,
    /// Ends [`DrawStep::StartRasterizing`].
    StopRasterizing,
    /// The drawing that follows goes through an image filter
    /// (matplotlib's `renderer.start_filter()`).
    StartFilter,
    /// Ends [`DrawStep::StartFilter`], filtering what was drawn since.
    StopFilter(AggFilter),
    /// An artist drawn as pltrs draws it, its painter left out, as it
    /// was then (matplotlib's `artist.draw(renderer)`).
    Artist(DrawnArtist),
    /// A cell of a table drawn alone where its table places it
    /// (matplotlib's `cell.draw(renderer)`).
    TableCell(TableCell),
}

/// The artist of a [`DrawStep::Artist`]: its handle, and the artist as it
/// was when the step was recorded. matplotlib's `artist.draw(renderer)`
/// draws the artist as it is then, and a program's `draw` may change it
/// and draw it again (astropy's tick labels draw one text once per
/// label); the figure copies an artist a step keeps before it changes it,
/// so each step draws what was drawn.
#[derive(Clone, Debug)]
pub struct DrawnArtist {
    /// The artist's handle.
    handle: RawHandle,
    /// The artist as it was, shared with its figure; `None` for an artist
    /// its figure no longer holds, or whose figure was busy.
    as_drawn: Option<Rc<dyn Artist>>,
    /// Whether the artist's own image filter applies to it (not when the
    /// drawing around it filters it already).
    own_filter: bool,
}

impl DrawnArtist {
    /// The artist of `handle` as it is now.
    pub fn new(handle: RawHandle) -> Self {
        let as_drawn =
            (handle.figure_state().try_borrow()).and_then(|state| state.shared_artist(handle.id()));
        Self {
            handle,
            as_drawn,
            own_filter: true,
        }
    }

    /// The artist drawn without its own image filter, which the drawing
    /// around it applies (matplotlib's `allow_rasterization` filtering
    /// what the artist's `draw` drew).
    #[must_use]
    pub fn filtered_around(self) -> Self {
        Self {
            own_filter: false,
            ..self
        }
    }

    /// The artist's handle.
    pub fn handle(&self) -> &RawHandle {
        &self.handle
    }
}

/// What a program drew through a renderer while pltrs was not drawing,
/// in order: its [`DrawStep`]s, drawn where pltrs draws (by an artist's
/// [`ArtistPainter`](super::ArtistPainter), or a custom path effect).
///
/// ```
/// use pltrs::artist::{DrawStep, Drawing};
/// use pltrs::path::Path;
/// use pltrs::transforms::{Affine2D, Point};
/// use pltrs_render::{GraphicsContext, RasterRenderer};
/// let mut drawing = Drawing::default();
/// drawing.push(DrawStep::Path {
///     gc: GraphicsContext::default(),
///     path: Path::polyline(vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)]),
///     transform: Affine2D::IDENTITY,
///     fill: None,
/// });
/// let mut canvas = RasterRenderer::new(10.0, 10.0, 72.0)?;
/// assert!(drawing.draw_onto(&mut canvas).is_empty());
/// assert!(canvas.to_image().pixel(5, 4).is_some_and(|p| p[3] > 0), "the line");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct Drawing {
    steps: Vec<DrawStep>,
}

impl Drawing {
    /// Adds `step` at the end.
    pub fn push(&mut self, step: DrawStep) {
        self.steps.push(step);
    }

    /// The steps, in order.
    pub fn steps(&self) -> &[DrawStep] {
        &self.steps
    }

    /// Whether nothing was drawn.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Draws the steps where `ctx` draws, as artist `artist` drew them:
    /// its problems are reported as `artist`'s.
    pub fn draw(&self, ctx: &mut DrawContext<'_>, artist: ArtistId) {
        for step in &self.steps {
            match step {
                DrawStep::StartRasterizing => ctx.start_program_rasterizing(),
                DrawStep::StopRasterizing => ctx.stop_program_rasterizing(),
                DrawStep::Artist(drawn) => ctx.draw_as_drawn(drawn),
                DrawStep::TableCell(cell) => cell.draw_in(ctx),
                other => {
                    if let Some(problem) = render(ctx.renderer(), other) {
                        ctx.report_kind(artist, DrawErrorKind::InvalidValue, problem);
                    }
                }
            }
        }
    }

    /// Draws the steps onto `renderer`, the artists among them in their
    /// figures (see [`RawHandle::draw_unpainted_onto`]); the problems met.
    pub fn draw_onto(&self, renderer: &mut dyn Renderer) -> Vec<DrawError> {
        let mut problems = Vec::new();
        for step in &self.steps {
            match step {
                DrawStep::StartRasterizing => renderer.start_rasterizing(),
                DrawStep::StopRasterizing => renderer.stop_rasterizing(),
                DrawStep::Artist(drawn) => {
                    if let Err(errors) = drawn.handle.draw_unpainted_onto(renderer) {
                        problems.extend(errors.0);
                    }
                }
                DrawStep::TableCell(cell) => {
                    if let Err(errors) = cell.draw_onto(renderer) {
                        problems.extend(errors.0);
                    }
                }
                other => {
                    if let Some(problem) = render(renderer, other) {
                        problems.push(DrawError {
                            artist: ArtistId::from_index(0),
                            message: problem,
                            kind: DrawErrorKind::InvalidValue,
                        });
                    }
                }
            }
        }
        problems
    }
}

impl Extend<DrawStep> for Drawing {
    fn extend<I: IntoIterator<Item = DrawStep>>(&mut self, steps: I) {
        self.steps.extend(steps);
    }
}

impl IntoIterator for Drawing {
    type Item = DrawStep;
    type IntoIter = std::vec::IntoIter<DrawStep>;

    fn into_iter(self) -> Self::IntoIter {
        self.steps.into_iter()
    }
}

/// Draws `step`, a call of the renderer itself, onto `renderer`; the
/// problem met, if any (TeX that cannot be drawn).
fn render(renderer: &mut dyn Renderer, step: &DrawStep) -> Option<String> {
    match step {
        DrawStep::Path {
            gc,
            path,
            transform,
            fill,
        } => renderer.draw_path(gc, path, transform, *fill),
        DrawStep::Markers {
            gc,
            marker,
            marker_transform,
            path,
            transform,
            fill,
        } => renderer.draw_markers(gc, marker, marker_transform, path, transform, *fill),
        DrawStep::PathCollection { gc, collection } => {
            renderer.draw_path_collection(gc, &collection.view());
        }
        DrawStep::QuadMesh { gc, mesh } => renderer.draw_quad_mesh(gc, &mesh.view()),
        DrawStep::GouraudTriangles {
            gc,
            triangles,
            colors,
            transform,
        } => renderer.draw_gouraud_triangles(gc, triangles, colors, transform),
        DrawStep::Image {
            gc,
            origin,
            image,
            transform,
        } => renderer.draw_image(gc, *origin, image, transform.as_ref()),
        DrawStep::Text {
            gc,
            run,
            origin,
            angle,
        } => {
            if let Some(TextSource::Tex(tex)) = &run.source {
                return renderer
                    .draw_tex(gc, run, tex, *origin, *angle)
                    .err()
                    .map(|err| err.to_string());
            }
            renderer.draw_text(gc, run, *origin, *angle);
        }
        DrawStep::OpenGroup { name, gid } => renderer.open_group(name, gid.as_deref()),
        DrawStep::CloseGroup { name } => renderer.close_group(name),
        DrawStep::StartFilter => renderer.start_filter(),
        DrawStep::StopFilter(filter) => renderer.stop_shared_filter(filter.shared()),
        DrawStep::StartRasterizing
        | DrawStep::StopRasterizing
        | DrawStep::Artist(_)
        | DrawStep::TableCell(_) => {}
    }
    None
}

impl DrawContext<'_> {
    /// Draws the artist `drawn` keeps as it was when it was drawn, if it
    /// is an artist of the figure drawn, else as
    /// [`draw_handle`](Self::draw_handle) draws it.
    pub fn draw_as_drawn(&mut self, drawn: &DrawnArtist) {
        match &drawn.as_drawn {
            Some(artist) if self.draws(drawn.handle.figure_state()) => {
                self.draw_own_filtered(&**artist, Some(drawn.handle.id()), drawn.own_filter);
            }
            _ => self.draw_handle(&drawn.handle),
        }
    }

    /// Draws `artist` as pltrs draws it, its painter left out, where this
    /// context draws (matplotlib's `artist.draw(renderer)` called while a
    /// figure draws): an artist of the figure drawn, in it or removed
    /// from it, as [`DrawContext::draw_unpainted`] draws it; an artist of
    /// none (a loose artist) in the figure drawn, whose transforms its
    /// transforms resolve in, as matplotlib evaluates them when it draws;
    /// an artist of another figure in that figure's layout, at this
    /// context's resolution, its problems reported as this context's.
    pub fn draw_handle(&mut self, artist: &RawHandle) {
        if self.draws(artist.figure_state()) {
            self.draw_unpainted(artist.id());
            return;
        }
        let (drawn, role) = {
            let state = artist.figure_state().borrow();
            (state.shared_artist(artist.id()), state.role)
        };
        let Some(drawn) = drawn else {
            return;
        };
        if role == StateRole::LooseHolder {
            self.draw_own(&*drawn, None);
            return;
        }
        let ((), errors) = artist
            .figure_state()
            .paint(self.renderer(), |inner| inner.draw_unpainted(artist.id()));
        for error in errors {
            self.report_kind(error.artist, error.kind, error.message);
        }
    }
}

#[cfg(test)]
mod tests {
    use pltrs_render::RasterRenderer;

    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::lines::{Line2D, LineProps};

    fn line_step() -> DrawStep {
        DrawStep::Path {
            gc: GraphicsContext {
                linewidth: 2.0,
                ..GraphicsContext::default()
            },
            path: Path::polyline(vec![Point::new(0.0, 5.0), Point::new(10.0, 5.0)]),
            transform: Affine2D::IDENTITY,
            fill: None,
        }
    }

    #[test]
    fn steps_draw_in_order_onto_a_renderer() {
        let mut drawing = Drawing::default();
        assert!(drawing.is_empty());
        drawing.push(DrawStep::OpenGroup {
            name: "mine".into(),
            gid: None,
        });
        drawing.push(line_step());
        drawing.push(DrawStep::CloseGroup {
            name: "mine".into(),
        });
        assert_eq!(drawing.steps().len(), 3);
        let mut twice = drawing.clone();
        twice.extend(drawing.clone());
        assert_eq!(twice.into_iter().count(), 6);
        let mut canvas = RasterRenderer::new(10.0, 10.0, 72.0).unwrap();
        assert_eq!(drawing.draw_onto(&mut canvas), Vec::new());
        let image = canvas.to_image();
        assert_eq!(image.pixel(5, 4).map(|p| p[3]), Some(255));
        assert_eq!(image.pixel(5, 0).map(|p| p[3]), Some(0));
    }

    #[test]
    fn artists_of_other_figures_draw_in_their_own_layout() {
        let loose = Line2D::loose(vec![0.0, 10.0], vec![5.0, 5.0], LineProps::default());
        loose.set_transform(crate::transforms::Transform::Display);
        let mut drawing = Drawing::default();
        drawing.push(DrawStep::Artist(DrawnArtist::new(loose.raw().clone())));
        let mut canvas = RasterRenderer::new(10.0, 10.0, 72.0).unwrap();
        assert_eq!(drawing.draw_onto(&mut canvas), Vec::new());
        assert!(canvas.to_image().pixel(5, 4).is_some_and(|p| p[3] > 0));
        // Drawn where a figure draws, through a context.
        let fig = Figure::with_size(1.0, 1.0, 10.0);
        let mut canvas = RasterRenderer::new(10.0, 10.0, 10.0).unwrap();
        let ((), errors) = fig
            .state_cell()
            .paint(&mut canvas, |ctx| drawing.draw(ctx, fig.id()));
        assert_eq!(errors, Vec::new());
    }
}
