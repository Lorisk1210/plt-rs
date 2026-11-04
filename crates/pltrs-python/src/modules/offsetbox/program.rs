//! Offset boxes of the program's (matplotlib's `OffsetBox` subclasses that
//! size and draw themselves) in the tree of a box pltrs (Rust) lays out:
//! the box holding one lays it out with the extent its `get_bbox` gives
//! (through its painter, `PythonPainter::box_extent`), and Python reads
//! where its origin went before its `draw` runs (`program_box_offset`).

use pltrs::offsetbox::ProgramOffsetBox;

use crate::binding::prelude::*;

artist_handle!(ProgramOffsetBox);

/// Where the box holding the program's box `program` placed its origin,
/// in display pixels (what matplotlib's parent gives `set_offset`).
fn program_box_offset(program: ProgramOffsetBox) -> PyResult<(f64, f64)> {
    let offset = program.offset();
    Ok((offset.x, offset.y))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("program_box_offset", program_box_offset)
}
