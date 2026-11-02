//! Text on the raster canvas while the program has replaced the text
//! drawing of `RendererAgg` (whose own `draw_text` and `draw_tex` draw
//! through `_native.drawing`): the TeX lines of a draw go to the
//! renderer's replaced `draw_tex`, and without its own `draw_text` texts
//! are drawn as paths, as `RendererBase.draw_text` draws them.

use std::rc::Rc;

use pltrs::colors::Color;
use pltrs::fonts::FontProperties;
use pltrs::text::{DrawnPath, TexPainter, set_tex_painter};
use pltrs_core::Point;
use pltrs_render::GraphicsContext;

use super::super::font_manager::FontProps;
use super::super::path::PyPath;
use super::super::transforms::affine;
use super::gc::PyGc;
use crate::binding::prelude::*;

/// Makes the raster canvases draw text as paths while `as_paths` (while
/// `RendererAgg.draw_text` is `RendererBase.draw_text`).
fn set_texts_as_paths(as_paths: bool) -> PyResult<()> {
    pltrs_render::set_texts_as_paths(as_paths);
    Ok(())
}

/// A path the program's `draw_tex` drew, as `RendererAgg.draw_path`
/// queues it: `(gc, path, matrix, fill)`.
type QueuedPath = (PyGc, PyPath, NdArray, Option<Color>);

/// TeX lines drawn by the program's `draw_tex`: `paint((rgba,
/// antialiased, url), line, prop, (x, y, angle))` calls it and gives the
/// paths it drew.
struct PythonTexPainter(Callback);

impl PythonTexPainter {
    /// The paths `queued` describes; a malformed one ends the list.
    fn drawn(queued: Vec<QueuedPath>) -> Vec<DrawnPath> {
        queued
            .into_iter()
            .map_while(|(gc, path, matrix, fill)| {
                Some(DrawnPath {
                    gc: gc.0,
                    path: path.path,
                    transform: affine(&matrix).ok()?,
                    fill: fill.map(|color| color.resolve()).transpose().ok()?,
                })
            })
            .collect()
    }
}

impl TexPainter for PythonTexPainter {
    fn paint(
        &self,
        gc: &GraphicsContext,
        line: &str,
        font: &FontProperties,
        origin: Point,
        angle: f64,
    ) -> Vec<DrawnPath> {
        let color = gc.foreground;
        let spec = (
            (color.r, color.g, color.b, color.a),
            gc.antialiased,
            gc.url.clone(),
        );
        self.0
            .call::<Vec<QueuedPath>>((
                spec,
                line.to_owned(),
                FontProps(font.clone()),
                (origin.x, origin.y, angle),
            ))
            .map(Self::drawn)
            .unwrap_or_default()
    }
}

/// Draws the TeX lines of the thread's draws with `paint` (see
/// [`PythonTexPainter`]), or as the canvas draws them again with None.
fn set_tex_line_painter(paint: Option<Callback>) -> PyResult<()> {
    set_tex_painter(paint.map(|paint| Rc::new(PythonTexPainter(paint)) as Rc<dyn TexPainter>));
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_texts_as_paths", set_texts_as_paths)?;
    module.add("set_tex_painter", set_tex_line_painter)
}
