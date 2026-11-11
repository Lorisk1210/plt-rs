//! The painters of artists whose class takes `draw` from the program
//! (matplotlib's `Artist` subclasses overriding `draw`).
//!
//! Before the figure's artists are drawn, with the figure laid out and
//! free to change (`pltrs::artist::ArtistPainter::prepare`), pltrs (Rust)
//! calls the package's `record_drawing(artist, target)`, which runs the
//! artist's `draw` with the renderer of the draw recording into a drawing;
//! then it draws that drawing at the artist's place. An artist the draw
//! reaches without having prepared it is recorded there and then, the
//! figure being drawn meanwhile. An exception the artist's `draw` raised
//! while it was recorded beforehand raises when the draw reaches the
//! artist, and the warnings it gave are given there: the artists drawn
//! before it may fail first, as they do in matplotlib's draw. Likewise an
//! exception the class' `get_tightbbox` raised measuring the artist before
//! a layout raises where the layout takes the artist's extent, after the
//! ticks of its axes.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt;

use pltrs::artist::{
    ArtistId, ArtistPainter, DrawContext, DrawTarget, Drawing, MeasuredExtent, Painter, RawHandle,
    Renderer,
};
use pltrs::transforms::Bbox;

use crate::binding::prelude::*;
use crate::binding::{current_call, lookup, report};

/// What the renderer of a draw is, for the Python renderer recording
/// with it: its canvas size in pixels, its resolution, whether it scales
/// images (vector output), its image magnification, the raster depth and
/// whether the figure is being drawn meanwhile (it cannot be changed
/// then).
pub(super) type Target = (f64, f64, f64, bool, f64, usize, bool);

/// The [`Target`] of `renderer` at `raster_depth`.
pub(super) fn target(renderer: &dyn Renderer, raster_depth: usize, live: bool) -> Target {
    let (width, height) = renderer.canvas_size();
    (
        width,
        height,
        renderer.dpi(),
        renderer.option_scale_image(),
        renderer.image_magnification(),
        raster_depth,
        live,
    )
}

/// The drawing a native drawing object holds, taken out of it.
pub(super) fn taken(drawing: &Native<Drawing>) -> Drawing {
    std::mem::take(&mut *drawing.borrow_mut())
}

/// The painter of an artist whose class takes `draw` from the program.
pub(in crate::modules) struct PythonPainter {
    /// A weak reference to the artist's Python object, which holds the
    /// artist's handle: a strong one would make a cycle through Rust.
    artist: Owned,
    /// Whether the class' `draw` lets the artist be rasterized and
    /// filtered (matplotlib's `allow_rasterization`).
    rasterizes: bool,
    /// Whether the class measures the artist (`get_window_extent` or
    /// `get_tightbbox` of the program's).
    measures: bool,
    /// The drawing recorded for the coming draw, or the exception the
    /// artist's `draw` raised recording it.
    prepared: RefCell<Option<PyResult<Drawing>>>,
    /// The extent the class measured last, in display pixels.
    measured: Cell<MeasuredExtent>,
    /// The exception the class raised measuring the artist last, with
    /// the native call it measured in (see `current_call`).
    failure: RefCell<Option<(u64, PyErr)>>,
}

impl fmt::Debug for PythonPainter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PythonPainter")
            .field("rasterizes", &self.rasterizes)
            .field("measures", &self.measures)
            .finish_non_exhaustive()
    }
}

impl PythonPainter {
    /// The painter of the artist the weak reference `artist` refers to,
    /// whose class' `draw` lets it be rasterized as `rasterizes` says and
    /// which measures it its own way with `measures`.
    pub(in crate::modules) fn new(artist: Owned, rasterizes: bool, measures: bool) -> Self {
        Self {
            artist,
            rasterizes,
            measures,
            prepared: RefCell::new(None),
            measured: Cell::new(MeasuredExtent::Unmeasured),
            failure: RefCell::new(None),
        }
    }

    /// The artist's Python object, if it is alive.
    pub(in crate::modules) fn artist<'py>(&self, py: Python<'py>) -> Option<Bound<'py>> {
        let artist = self.artist.bind(py).call(&[], &[]).ok()?;
        (!artist.is_none()).then_some(artist)
    }

    /// Whether the class' `draw` lets the artist be rasterized, and
    /// whether the class measures it.
    pub(in crate::modules) fn flags(&self) -> (bool, bool) {
        (self.rasterizes, self.measures)
    }

    /// Runs the artist's `draw` for a draw with `target`: the drawing it
    /// drew, or the exception it raised; `None` when the artist is gone or
    /// an earlier callback of the native call drawing raised.
    fn record(&self, target: Target) -> Option<PyResult<Drawing>> {
        Python::with_gil(|py| {
            let artist = self.artist(py)?;
            let record: Callback = lookup(py, "record_drawing").ok()?.extract().ok()?;
            record
                .try_call::<Native<Drawing>>((artist, target))
                .transpose()
                .map(|drawing| drawing.map(|drawing| taken(&drawing)))
        })
    }
}

impl PythonPainter {
    /// Gives the warnings the artist's `draw` gave while it was recorded
    /// beforehand, through the package's `emit_held_warnings(artist)`:
    /// where the draw reaches it, as matplotlib's draw gives them.
    fn emit_held_warnings(&self) {
        Python::with_gil(|py| {
            let artist = self.artist(py)?;
            let emit: Callback = lookup(py, "emit_held_warnings").ok()?.extract().ok()?;
            emit.call::<Owned>((artist,))
        });
    }
}

impl ArtistPainter for PythonPainter {
    fn prepare(&self, _artist: &RawHandle, target: DrawTarget<'_>) {
        let drawing = self.record(self::target(target.renderer, target.raster_depth, false));
        if !target.rendered {
            // matplotlib's draw that renders nothing gave them, drawing
            // the artist.
            self.emit_held_warnings();
        }
        self.prepared.replace(drawing);
    }

    fn draw(&self, ctx: &mut DrawContext<'_>, artist: ArtistId) {
        let prepared = self.prepared.take();
        if prepared.is_some() {
            self.emit_held_warnings();
        }
        let depth = ctx.raster_depth();
        // `ctx.renderer()` lets the figure go: the artist's `draw` finds
        // it free.
        match prepared.or_else(|| self.record(target(ctx.renderer(), depth, true))) {
            Some(Ok(drawing)) => drawing.draw(ctx, artist),
            Some(Err(err)) => report(err),
            None => {}
        }
    }

    fn rasterizes(&self) -> bool {
        self.rasterizes
    }

    /// The class' `get_tightbbox`, through the package's
    /// `measure_artist(artist)`: the extents of the box, None for no
    /// space, or no extents when the artist only made itself ready for
    /// pltrs to measure it (an Axes class placing its titles its own
    /// way); an exception it raises is kept for the native call laying
    /// out to raise where it takes the extent (`extent_taken`). The
    /// extent measured before does not count meanwhile: the class' own
    /// asks for the tight box pltrs computes (`super().get_tightbbox`).
    fn measure(&self, _artist: &RawHandle) {
        if !self.measures {
            return;
        }
        self.measured.set(MeasuredExtent::Unmeasured);
        drop(self.failure.take());
        let extents = Python::with_gil(|py| {
            let artist = self.artist(py)?;
            let measure: Callback = lookup(py, "measure_artist").ok()?.extract().ok()?;
            measure
                .try_call::<Option<Vec<f64>>>((artist,))
                .unwrap_or_else(|err| {
                    match current_call() {
                        Some(call) => drop(self.failure.replace(Some((call, err)))),
                        None => report(err),
                    }
                    None
                })
        });
        self.measured
            .set(match extents.as_ref().map(|extents| extents.as_deref()) {
                Some(None) => MeasuredExtent::Empty,
                Some(Some(&[x0, y0, x1, y1])) => {
                    MeasuredExtent::Extent(Bbox::from_extents(x0, y0, x1, y1))
                }
                // Nothing measured, or an artist that only made itself ready
                // to be measured (no extents): pltrs measures it.
                None | Some(Some(_)) => MeasuredExtent::Unmeasured,
            });
    }

    fn measured_extent(&self) -> MeasuredExtent {
        self.measured.get()
    }

    /// Raises the exception the class raised measuring the artist, if it
    /// did so in the native call running.
    fn extent_taken(&self) {
        let failure = self.failure.take();
        if let Some((call, err)) = failure
            && current_call() == Some(call)
        {
            report(err);
        }
    }

    /// The box's `get_bbox(renderer)`, through the package's
    /// `measure_box(artist)`: its bounds; an exception raises from the
    /// native call laying out.
    fn box_extent(&self, _artist: &RawHandle) -> Option<Bbox> {
        let (x0, y0, width, height) = Python::with_gil(|py| {
            let artist = self.artist(py)?;
            let measure: Callback = lookup(py, "measure_box").ok()?.extract().ok()?;
            measure.call::<(f64, f64, f64, f64)>((artist,))
        })?;
        Some(Bbox::from_bounds(x0, y0, width, height))
    }
}

/// Has the Python object the weak reference `artist_ref` refers to draw
/// the artist `artist` (its class takes `draw` from the program), or
/// pltrs again with `None`; `rasterizes` tells whether the class' `draw`
/// lets the artist be rasterized (matplotlib's `allow_rasterization`),
/// `measures` whether the class measures the artist (layouts and tight
/// boxes then ask it). An artist that object paints so already keeps its
/// painter, and the drawing prepared for the draw running (an image whose
/// `draw` sets its array asks for its painter again).
fn set_painter(
    artist: RawHandle,
    artist_ref: Option<Owned>,
    rasterizes: bool,
    measures: bool,
) -> PyResult<()> {
    if let Some(artist_ref) = &artist_ref
        && painted_by(&artist, artist_ref, (rasterizes, measures))
    {
        return Ok(());
    }
    artist.set_painter(
        artist_ref
            .map(|artist_ref| Painter::new(PythonPainter::new(artist_ref, rasterizes, measures))),
    );
    Ok(())
}

/// Whether the object the weak reference `artist_ref` refers to paints
/// `artist` already, with the flags `flags` (see [`set_painter`]).
fn painted_by(artist: &RawHandle, artist_ref: &Owned, flags: (bool, bool)) -> bool {
    let Some(painter) = artist.painter() else {
        return false;
    };
    let Some(current) = (painter.get() as &dyn Any).downcast_ref::<PythonPainter>() else {
        return false;
    };
    current.flags() == flags
        && Python::with_gil(|py| {
            let given = artist_ref.bind(py).call(&[], &[]).ok();
            match (current.artist(py), given) {
                (Some(painting), Some(given)) => painting.as_ptr() == given.as_ptr(),
                _ => false,
            }
        })
}

/// Whether `artist` is drawn by a painter (see [`set_painter`]).
fn has_painter(artist: RawHandle) -> PyResult<bool> {
    Ok(artist.painter().is_some())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_painter", set_painter)?;
    module.add("has_painter", has_painter)
}
