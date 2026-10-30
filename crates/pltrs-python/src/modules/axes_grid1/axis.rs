//! The sides of an axes acted on together (matplotlib's
//! `axes_grid1.mpl_axes.SimpleAxisArtist`, `ax.axis["left"]`).

use pltrs::axes::Axes;
use pltrs::axes_grid1::SimpleAxisArtist;
use pltrs::axisartist::Toggle;

use super::sizes::side_named;
use crate::binding::prelude::*;

/// The `side` of `axes`.
fn simple_axis(axes: &Axes, side: &str) -> PyResult<SimpleAxisArtist> {
    Ok(SimpleAxisArtist::new(axes, side_named(side)?))
}

/// Shows or hides the major ticks, tick labels and label of `side`;
/// `all` first, the others overriding it (`toggle`).
fn toggle(
    axes: Axes,
    side: String,
    (all, ticks, ticklabels, label): (Option<bool>, Option<bool>, Option<bool>, Option<bool>),
) -> PyResult<()> {
    simple_axis(&axes, &side)?.toggle(Toggle {
        all,
        ticks,
        ticklabels,
        label,
    });
    Ok(())
}

/// Shows or hides everything on `side` (`set_visible`).
fn set_visible(axes: Axes, side: String, visible: bool) -> PyResult<()> {
    simple_axis(&axes, &side)?.set_visible(visible);
    Ok(())
}

/// Sets the label of the axis of `side` (`set_label`).
fn set_label(axes: Axes, side: String, text: String) -> PyResult<()> {
    simple_axis(&axes, &side)?.set_label(text);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("simple_axis_toggle", toggle)?;
    module.add("simple_axis_set_visible", set_visible)?;
    module.add("simple_axis_set_label", set_label)
}
