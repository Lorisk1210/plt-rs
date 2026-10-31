//! `_native.axisartist`: the `axisartist` toolkit (matplotlib's
//! `mpl_toolkits.axisartist`): axes drawn by axis artists and their parts,
//! curvilinear grids and floating axes, and the numeric helpers of the
//! grids (angular grid steps, angle labels and where grid lines cross a
//! box).

mod artists;
mod axis_texts;
mod curvilinear;
mod helpers;

use crate::binding::prelude::*;

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    artists::register(module)?;
    axis_texts::register(module)?;
    curvilinear::register(module)?;
    helpers::register(module)
}
