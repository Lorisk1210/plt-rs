//! `_native.drawing`: drawing defined in Python, done by pltrs (Rust)
//! where it draws (matplotlib's `draw` of an `Artist` subclass, the
//! `draw_path` of a path effect of the program's, and renderers of the
//! program's).
//!
//! * **Drawings.** The renderers pltrs hands Python (`RendererAgg`, and
//!   `MixedModeRenderer` around `RendererPdf`, `RendererSVG`, `RendererPS`
//!   and `RendererPgf`) draw into a `pltrs::artist::Drawing`, a native
//!   object: each of their drawing methods adds its step (`draw_path`,
//!   `draw_markers`, ..., [`steps`]), and `Artist.draw` with one of them
//!   the artist as pltrs draws it (`draw_artist`). pltrs (Rust) draws the
//!   drawing where it draws the artist, or onto a canvas
//!   (`draw_onto_canvas`, `draw_onto_raster`).
//! * **Painters.** An artist whose class takes `draw` from the program
//!   gets a painter ([`painter`]): before the figure's artists are drawn,
//!   pltrs (Rust) calls the package's `record_drawing`, which runs the
//!   artist's `draw` with the renderer of the draw recording into a
//!   drawing, and then draws the drawing at the artist's place.
//! * **Path effects** of the program's classes ([`effect`]) are called
//!   while pltrs (Rust) draws a path, with a recording renderer, and what
//!   they drew is drawn in the path's place.
//! * **Renderers of the program's** ([`foreign`]): pltrs (Rust) draws an
//!   artist, or a figure, through their methods (`draw_through`).
//! * **Transforms computing their own matrix** ([`asked`]): an artist given
//!   one is handed it again before every draw and measure.

mod asked;
mod effect;
mod foreign;
mod painter;
mod steps;

pub(super) use asked::AskingPainter;
pub(super) use effect::{PythonEffect, python_effect};
pub(super) use foreign::PythonRenderer;
pub(super) use painter::PythonPainter;

use pltrs::artist::Drawing;

use crate::binding::prelude::*;

native_value!(Drawing);

/// A new, empty drawing.
fn new() -> PyResult<Drawing> {
    Ok(Drawing::default())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new", new)?;
    steps::register(module)?;
    painter::register(module)?;
    asked::register(module)?;
    foreign::register(module)
}
