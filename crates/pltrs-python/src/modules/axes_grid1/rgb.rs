//! The styling of RGB axes (matplotlib's `axes_grid1.axes_rgb`): white
//! frames, and channel axes without tick labels.

use pltrs::axes::Axes;
use pltrs::axes_grid1::{hide_channel_ticklabels, whiten_frame};

use crate::binding::prelude::*;

/// Draws the spines and major tick edges of `axes` in white (`RGBAxes`).
fn whiten(axes: Axes) -> PyResult<()> {
    whiten_frame(&axes);
    Ok(())
}

/// Hides the tick labels of the channel axes `axes` (`make_rgb_axes`).
fn hide_ticklabels(axes: Axes) -> PyResult<()> {
    hide_channel_ticklabels(&axes);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("rgb_whiten_frame", whiten)?;
    module.add("rgb_hide_ticklabels", hide_ticklabels)
}
