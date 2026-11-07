//! The graphics contexts of the raster canvas while the program has
//! replaced the `new_gc` of its renderer (`RendererBase.new_gc = ...`,
//! returning a `GraphicsContextBase` subclass with round caps, say):
//! every artist pltrs (Rust) draws starts from one the program's method
//! makes, as matplotlib's artists start from what `renderer.new_gc()`
//! returns (see [`pltrs::artist::GcSource`]).

use std::rc::Rc;

use pltrs::artist::{GcSource, set_gc_source};
use pltrs_render::GraphicsContext;

use super::gc::PyGc;
use crate::binding::prelude::*;

/// Graphics contexts made by the program's `new_gc`: `make()` calls it
/// and describes the context it returns as [`PyGc`] takes one.
struct PythonGcSource(Callback);

impl GcSource for PythonGcSource {
    fn new_gc(&self) -> GraphicsContext {
        // An exception the program's method raises is raised from the
        // draw; the artist meanwhile starts from the default context.
        self.0
            .call::<PyGc>(())
            .map_or_else(GraphicsContext::default, |gc| gc.0)
    }
}

/// Starts the artists of the thread's draws from the graphics contexts
/// `make` describes (see [`PythonGcSource`]), or from the renderer's own
/// again with None.
fn set_program_gc(make: Option<Callback>) -> PyResult<()> {
    set_gc_source(make.map(|make| Rc::new(PythonGcSource(make)) as Rc<dyn GcSource>));
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_program_gc", set_program_gc)
}
