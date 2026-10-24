//! The path and polygon editors as they run, taken from the canvas: their
//! patch and vertex markers are animated and drawn by their draw
//! callbacks over the background those save, then a vertex is dragged
//! with simulated mouse events, which puts the background back, draws the
//! artists over it and blits the axes.

use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use plt::events::{Event, EventKind, EventSource, MouseEvent};
use plt::figure::{BufferRegion, CanvasError};
use plt::navigation::PointerButton;
use plt::patches::{Patch, PatchShape, PathShape, Polygon};
use plt::prelude::*;
use plt::transforms::Point;

use super::editors::{editor_circle, unzipped, vertex_markers};
use crate::figures::cases::common::path_patch_curve;
use crate::figures::{CaseResult, FigureCase};

/// How close to a vertex, in pixels, a press picks it (`epsilon`).
const EPSILON: f64 = 5.0;

/// What an editor edits: the vertices of a patch, which it moves.
trait Editable: PatchShape + Clone {
    /// The vertices.
    fn points(&self) -> &[Point];

    /// Moves vertex `index` to `to`, as the editor does.
    fn move_vertex(&mut self, index: usize, to: Point);
}

impl Editable for PathShape {
    fn points(&self) -> &[Point] {
        self.0.vertices()
    }

    /// `vertices[self._ind] = event.xdata, event.ydata`.
    fn move_vertex(&mut self, index: usize, to: Point) {
        if let Some(vertex) = self.0.vertices_mut().get_mut(index) {
            *vertex = to;
        }
    }
}

impl Editable for Polygon {
    fn points(&self) -> &[Point] {
        self.vertices()
    }

    /// `poly.xy[self._ind] = x, y`, and the closing vertex with the first.
    fn move_vertex(&mut self, index: usize, to: Point) {
        let mut vertices = self.vertices().to_vec();
        let last = vertices.len().saturating_sub(1);
        for at in [
            Some(index),
            (index == 0).then_some(last),
            (index == last).then_some(0),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(vertex) = vertices.get_mut(at) {
                *vertex = to;
            }
        }
        *self = Polygon::new(vertices, self.is_closed());
    }
}

/// The editors' `PathInteractor` and `PolygonInteractor` without their
/// key bindings: draws the patch and markers from the draw callback and
/// drags vertices.
struct Interactor<S: Editable> {
    ax: Axes,
    patch: Patch<S>,
    line: Line2D,
    background: Option<BufferRegion>,
    /// The vertex being dragged.
    index: Option<usize>,
    /// The first problem met by a callback, which cannot return it.
    problem: Option<Box<dyn Error>>,
}

impl<S: Editable + 'static> Interactor<S> {
    /// Edits `patch` of `ax` through the markers `line`, both animated,
    /// with callbacks connected to the figure.
    fn connect(fig: &Figure, ax: &Axes, patch: Patch<S>, line: Line2D) -> Rc<RefCell<Self>> {
        patch.set_animated(true);
        line.set_animated(true);
        let interactor = Rc::new(RefCell::new(Self {
            ax: ax.clone(),
            patch,
            line,
            background: None,
            index: None,
            problem: None,
        }));
        for kind in [
            EventKind::Draw,
            EventKind::ButtonPress,
            EventKind::ButtonRelease,
            EventKind::Motion,
        ] {
            let weak = Rc::downgrade(&interactor);
            fig.connect(kind, move |event| {
                if let Some(interactor) = weak.upgrade() {
                    let mut interactor = interactor.borrow_mut();
                    if let Err(problem) = interactor.handle(event) {
                        interactor.problem.get_or_insert(problem);
                    }
                }
            });
        }
        interactor
    }

    fn handle(&mut self, event: &Event) -> Result<(), Box<dyn Error>> {
        match event {
            Event::Draw => self.on_draw()?,
            Event::ButtonPress(mouse) if pressed_in_axes(mouse) => {
                self.index = self.index_under(mouse)?;
            }
            Event::ButtonRelease(mouse) if mouse.button == Some(PointerButton::Left) => {
                self.index = None;
            }
            Event::Motion(mouse) if pressed_in_axes(mouse) => self.drag(mouse)?,
            _ => {}
        }
        Ok(())
    }

    /// The draw callback: saves the background of the axes and draws the
    /// patch and the markers over it.
    fn on_draw(&mut self) -> Result<(), CanvasError> {
        self.background = Some(self.ax.figure().canvas().copy_from_bbox(&self.ax.bbox())?);
        self.draw_artists()
    }

    fn draw_artists(&self) -> Result<(), CanvasError> {
        self.ax.draw_artist(&self.patch)?;
        self.ax.draw_artist(&self.line)
    }

    /// The vertex within [`EPSILON`] pixels of the pointer, the first of
    /// the closest (`get_ind_under_point`).
    fn index_under(&self, mouse: &MouseEvent) -> Result<Option<usize>, Box<dyn Error>> {
        let to_display = self.ax.figure().resolve_transform(&self.ax.trans_data())?;
        let at = Point::new(mouse.location.x, mouse.location.y);
        let distances: Vec<f64> = self
            .patch
            .shape()
            .points()
            .iter()
            .map(|vertex| {
                let shown = to_display.transform_point(*vertex);
                (shown.x - at.x).hypot(shown.y - at.y)
            })
            .collect();
        let closest = distances
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .filter(|(_, distance)| **distance < EPSILON)
            .map(|(index, _)| index);
        Ok(closest)
    }

    /// Moves the dragged vertex to the pointer, then puts the background
    /// back, draws the patch and markers over it and blits the axes.
    fn drag(&mut self, mouse: &MouseEvent) -> Result<(), Box<dyn Error>> {
        let (Some(index), Some(x), Some(y)) =
            (self.index, mouse.location.xdata, mouse.location.ydata)
        else {
            return Ok(());
        };
        self.patch
            .update_shape(|shape| shape.move_vertex(index, Point::new(x, y)));
        let (xs, ys) = unzipped(self.patch.shape().points());
        self.line.set_data(&xs, &ys);
        let canvas = self.ax.figure().canvas();
        if let Some(background) = &self.background {
            canvas.restore_region(background)?;
        }
        self.draw_artists()?;
        canvas.blit(Some(&self.ax.bbox()))?;
        Ok(())
    }
}

/// Whether `mouse` comes with the left button held over an axes.
fn pressed_in_axes(mouse: &MouseEvent) -> bool {
    mouse.location.inaxes.is_some() && mouse.button == Some(PointerButton::Left)
}

/// Presses the left button at display point `from`, moves the pointer to
/// `to` and releases the button there (`_drag`).
fn drag(fig: &Figure, from: (f64, f64), to: (f64, f64)) {
    let mut source = EventSource::new();
    source.button_press(fig, from.0, from.1, PointerButton::Left, false);
    source.motion(fig, to.0, to.1);
    source.button_release(fig, to.0, to.1, PointerButton::Left);
}

/// Draws the figure, drags a vertex from `from` to `to`, and reports the
/// first problem the editor met.
fn draw_and_drag<S: Editable + 'static>(
    fig: &Figure,
    interactor: &Rc<RefCell<Interactor<S>>>,
    from: (f64, f64),
    to: (f64, f64),
) -> CaseResult {
    fig.canvas().draw()?;
    drag(fig, from, to);
    interactor.borrow_mut().problem.take().map_or(Ok(()), Err)
}

/// `path_editor` as it runs: drawn, then its third vertex (-1.75, 2), at
/// display point (168.6, 316.8), dragged to (250, 400).
fn path_editor_blitted(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let green = PatchProps::default()
        .facecolor("green")
        .edgecolor("yellow")
        .alpha(0.5);
    let patch = ax.add_patch_with(PathShape(path_patch_curve()?), green);
    let (x, y) = unzipped(patch.shape().points());
    let line = ax.plot_with(x, y, vertex_markers()?)?;
    let interactor = Interactor::connect(fig, &ax, patch, line);
    ax.set_title("drag vertices to update path");
    ax.set_xlim(-3.0, 4.0)?.set_ylim(-3.0, 4.0)?;
    draw_and_drag(fig, &interactor, (169.0, 317.0), (250.0, 400.0))
}

/// `poly_editor` as it runs: drawn, then the vertex at angle 1, at
/// display point (428.5, 354.2), dragged to (480, 420).
fn poly_editor_blitted(fig: &Figure) -> CaseResult {
    let poly = Polygon::new(editor_circle(), true);
    let ax = fig.subplot();
    let patch = ax.add_patch(poly);
    patch.set_animated(true);
    let (x, y) = unzipped(patch.shape().points());
    let line = ax.add_line(&x, &y, vertex_markers()?)?;
    let interactor = Interactor::connect(fig, &ax, patch, line);
    ax.set_title("Click and drag a point to move it");
    ax.set_xlim(-2.0, 2.0)?.set_ylim(-2.0, 2.0)?;
    draw_and_drag(fig, &interactor, (428.0, 354.0), (480.0, 420.0))
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_tutorials/path_editor_blitted", path_editor_blitted).from_canvas(),
    FigureCase::new("gallery_tutorials/poly_editor_blitted", poly_editor_blitted).from_canvas(),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polygons_move_their_closing_vertex_with_the_first() {
        let mut square = Polygon::new([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)].map(Point::from), true);
        square.move_vertex(0, Point::new(-1.0, -1.0));
        let vertices = square.vertices();
        assert_eq!(vertices.first(), vertices.last());
        assert_eq!(vertices[0], Point::new(-1.0, -1.0));
    }
}
