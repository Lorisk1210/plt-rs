//! The colours a colour mapping of the program's gives the items of a
//! collection (`Collection._mapped_colors`, the colours of a bivariate or
//! multivariate `Colorizer`), which pltrs (Rust) puts on the sides the
//! collection's mapping colours.

use pltrs::collections::Collection;
use pltrs::colors::Rgba;

use crate::binding::prelude::*;

/// Puts `colors` (an `(N, 4)` RGBA array, alpha applied) on the sides the
/// collection's mapping colours; `None` gives them back to its own
/// mapping.
fn set_program_colors(collection: Collection, colors: Option<NdArray>) -> PyResult<()> {
    let colors = colors
        .map(|colors| {
            if colors.shape.len() != 2 || colors.shape[1] != 4 {
                return Err(PyErr::value_error(format!(
                    "colors must be an (N, 4) RGBA array, not of shape {:?}",
                    colors.shape
                )));
            }
            Ok(colors
                .data
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&[r, g, b, a]| Rgba::new(r, g, b, a))
                .collect())
        })
        .transpose()?;
    collection.set_program_colors(colors);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_program_colors", set_program_colors)
}
