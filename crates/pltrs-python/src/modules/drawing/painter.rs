//! The painters of artists whose class takes `draw` from the program
//! (matplotlib's `Artist` subclasses overriding `draw`).
//!
//! Before the figure's artists are drawn, with the figure laid out and
//! free to change (`pltrs::artist::ArtistPainter::prepare`), pltrs (Rust)
//! calls the package's `record_drawing(artist, target)`, which runs the
//! artist's `draw` with the renderer of the draw recording into a drawing;
//! then it draws that drawing at the artist's place. An artist the draw
//! reaches without having prepared it is recorded there and then, the
//! figure being drawn meanwhile.

use std::cell::{Cell, RefCell};
use std::fmt;

use pltrs::artist::{
    ArtistId, ArtistPainter, DrawContext, DrawTarget, Drawing, MeasuredExtent, Painter, RawHandle,
    Renderer,
};
use pltrs::transforms::Bbox;

use crate::binding::lookup;
use crate::binding::prelude::*;

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
    /// The drawing recorded for the coming draw.
    prepared: RefCell<Option<Drawing>>,
    /// The extent the class measured last, in display pixels.
    measured: Cell<MeasuredExtent>,
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
    /// drew; `None` when it raised (its exception raises from the native
    /// call drawing) or the artist is gone.
    fn record(&self, target: Target) -> Option<Drawing> {
        Python::with_gil(|py| {
            let artist = self.artist(py)?;
            let record: Callback = lookup(py, "record_drawing").ok()?.extract().ok()?;
            let drawing: Native<Drawing> = record.call((artist, target))?;
            Some(taken(&drawing))
        })
    }
}

impl ArtistPainter for PythonPainter {
    fn prepare(&self, _artist: &RawHandle, target: DrawTarget<'_>) {
        let drawing = self.record(self::target(target.renderer, target.raster_depth, false));
        self.prepared.replace(drawing);
    }

    fn draw(&self, ctx: &mut DrawContext<'_>, artist: ArtistId) {
        let prepared = self.prepared.take();
        let depth = ctx.raster_depth();
        let drawing = prepared.or_else(|| self.record(target(ctx.renderer(), depth, true)));
        if let Some(drawing) = drawing {
            drawing.draw(ctx, artist);
        }
    }

    fn rasterizes(&self) -> bool {
        self.rasterizes
    }

    /// The class' `get_tightbbox`, through the package's
    /// `measure_artist(artist)`: the extents of the box, None for no
    /// space; an exception raises from the native call laying out.
    fn measure(&self, _artist: &RawHandle) {
        if !self.measures {
            return;
        }
        let extents = Python::with_gil(|py| {
            let artist = self.artist(py)?;
            let measure: Callback = lookup(py, "measure_artist").ok()?.extract().ok()?;
            measure.call::<Option<(f64, f64, f64, f64)>>((artist,))
        });
        self.measured.set(match extents {
            None => MeasuredExtent::Unmeasured,
            Some(None) => MeasuredExtent::Empty,
            Some(Some((x0, y0, x1, y1))) => {
                MeasuredExtent::Extent(Bbox::from_extents(x0, y0, x1, y1))
            }
        });
    }

    fn measured_extent(&self) -> MeasuredExtent {
        self.measured.get()
    }
}

/// Has the Python object the weak reference `artist_ref` refers to draw
/// the artist `artist` (its class takes `draw` from the program), or
/// pltrs again with `None`; `rasterizes` tells whether the class' `draw`
/// lets the artist be rasterized (matplotlib's `allow_rasterization`),
/// `measures` whether the class measures the artist (layouts and tight
/// boxes then ask it).
fn set_painter(
    artist: RawHandle,
    artist_ref: Option<Owned>,
    rasterizes: bool,
    measures: bool,
) -> PyResult<()> {
    artist.set_painter(
        artist_ref
            .map(|artist_ref| Painter::new(PythonPainter::new(artist_ref, rasterizes, measures))),
    );
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_painter", set_painter)
}
