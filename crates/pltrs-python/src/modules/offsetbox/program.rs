//! Offset boxes of the program's (matplotlib's `OffsetBox` subclasses that
//! size and draw themselves): every Python `OffsetBox` is a
//! `ProgramBoxArtist` until it joins a figure (`new_box`), which a box of
//! the program's stays. In the tree of a box pltrs (Rust) lays out, the
//! box holding one lays it out with the extent its `get_bbox` gives
//! (through its painter, `PythonPainter::box_extent`, or
//! `set_program_extent`), and Python reads where its origin went before
//! its `draw` runs (`program_box_offset`).

use pltrs::artist::{ArtistHandle, Custom, RawHandle};
use pltrs::offsetbox::{ProgramBoxArtist, ProgramOffsetBox};
use pltrs::transforms::Bbox;

use crate::binding::prelude::*;

artist_handle!(ProgramOffsetBox);

/// The artist of a new Python `OffsetBox`, outside any figure.
fn new_box() -> PyResult<RawHandle> {
    Ok(Custom::loose(ProgramBoxArtist::default()).raw().clone())
}

/// Where the box holding the program's box `program` placed its origin,
/// in display pixels (what matplotlib's parent gives `set_offset`); None
/// when no box holds it.
fn program_box_offset(program: ProgramOffsetBox) -> PyResult<Option<(f64, f64)>> {
    Ok(program.offset().map(|offset| (offset.x, offset.y)))
}

/// Sizes the program's box `program` with the bounds `(x0, y0, width,
/// height)` its `get_bbox` gave, and lays the box holding it out again.
fn set_program_extent(program: ProgramOffsetBox, bounds: (f64, f64, f64, f64)) -> PyResult<()> {
    let (x0, y0, width, height) = bounds;
    program.set_extent(Bbox::from_bounds(x0, y0, width, height));
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new_box", new_box)?;
    module.add("program_box_offset", program_box_offset)?;
    module.add("set_program_extent", set_program_extent)
}
