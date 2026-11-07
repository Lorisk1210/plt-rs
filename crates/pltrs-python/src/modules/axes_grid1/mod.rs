//! `_native.axes_grid1`: axes placed by pltrs (Rust)'s
//! [`pltrs::axes_grid1`] (matplotlib's `mpl_toolkits.axes_grid1`):
//! dividers and their sizes, the sides of an axes acted on together, host
//! axes and their parasites, inset axes anchored in a box of their parent,
//! the patches outlining and joining boxes, and the styling of RGB axes.

mod axis;
mod dividers;
mod inset_locator;
mod parasites;
mod patches;
mod rgb;
mod sizes;

pub(super) use sizes::side_named;

use crate::binding::prelude::*;

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    axis::register(module)?;
    dividers::register(module)?;
    inset_locator::register(module)?;
    parasites::register(module)?;
    patches::register(module)?;
    rgb::register(module)
}
