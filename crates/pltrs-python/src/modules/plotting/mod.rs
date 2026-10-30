//! `_native.plotting`: the plotting methods of matplotlib's `Axes`
//! (`axes._axes`), one file per family, which the mixins of
//! `pltrs/axes/_plotting/` call.
//!
//! The Python side turns matplotlib's signatures (positional forms,
//! aliases, defaults, input checks raising matplotlib's messages) into
//! explicit arguments; these functions hand them to pltrs (Rust)'s
//! plotting methods and return the artists made, with the native
//! `Container` of those that make one.

#![allow(
    clippy::too_many_arguments,
    reason = "native functions take a plotting method's arguments in matplotlib's order, \
              grouped in tuples where they belong together"
)]

mod bars;
mod bins2d;
mod ecdf;
mod errorbar;
mod events;
mod fields;
pub(super) mod fills;
mod hist;
mod images;
mod labels;
mod lines;
pub(super) mod mapping;
mod mesh;
mod scatter;
mod spectral;
mod stats;
mod tripcolor;
mod values;

use pltrs::artist::RawHandle;
use pltrs::axes::Axes;
use pltrs::collections::Collection;
use pltrs::container::Container;

use crate::binding::prelude::*;

native_value!(Container);

/// Removes a container's artists and its record in its axes
/// (matplotlib's `Container.remove`).
fn remove_container(container: Container) -> PyResult<()> {
    container.remove();
    Ok(())
}

/// The collections of the axes (`ax.collections`).
fn get_collections(axes: Axes) -> PyResult<Vec<Collection>> {
    Ok(axes.collections())
}

/// The patches of the axes, whatever their shape (`ax.patches`).
fn get_patches(axes: Axes) -> PyResult<Vec<RawHandle>> {
    Ok(axes.patches())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("remove_container", remove_container)?;
    module.add("get_collections", get_collections)?;
    module.add("get_patches", get_patches)?;
    module.add("errorbar", errorbar::errorbar)?;
    module.add("plot", lines::plot)?;
    module.add("format_parts", lines::format_parts)?;
    module.add("axline", lines::axline)?;
    module.add("segments", lines::segments)?;
    module.add("next_patch_color", scatter::next_patch_color)?;
    module.add("scatter", scatter::scatter)?;
    module.add("bar", bars::bar)?;
    module.add("broken_barh", bars::broken_barh)?;
    module.add("grouped_bar", bars::grouped_bar)?;
    module.add("bar_label", labels::bar_label)?;
    module.add("pie_label", labels::pie_label)?;
    module.add("pcolorfast", images::pcolorfast)?;
    module.add("spy", images::spy)?;
    module.add("style_matrix_axes", images::style_matrix_axes)?;
    module.add("hexbin", bins2d::hexbin)?;
    module.add("hist2d", bins2d::hist2d)?;
    module.add("psd", spectral::psd)?;
    module.add("csd", spectral::csd)?;
    module.add("cohere", spectral::cohere)?;
    module.add("single_spectrum", spectral::single_spectrum)?;
    module.add("specgram", spectral::specgram)?;
    module.add("xcorr", spectral::xcorr)?;
    module.add("ecdf", ecdf::ecdf)?;
    module.add("tripcolor", tripcolor::tripcolor)?;
    module.add("hist", hist::hist)?;
    module.add("fill", fills::fill)?;
    module.add("fill_between", fills::fill_between)?;
    module.add("stairs", fills::stairs)?;
    module.add("stackplot", fills::stackplot)?;
    module.add("eventplot", events::eventplot)?;
    module.add("stem", events::stem)?;
    module.add("bxp", stats::bxp)?;
    module.add("violin", stats::violin)?;
    module.add("pie", stats::pie)?;
    module.add("pcolor", mesh::pcolor)?;
    module.add("pcolormesh_rgba", mesh::pcolormesh_rgba)?;
    module.add("convert_units", values::convert_units)?;
    module.add("quiver", fields::quiver)?;
    module.add("quiver_set_uvc", fields::quiver_set_uvc)?;
    module.add("quiverkey", fields::quiverkey)?;
    module.add("quiverkey_text", fields::quiverkey_text)?;
    module.add("quiverkey_paths", fields::quiverkey_paths)?;
    module.add("barbs", fields::barbs)?;
    module.add("streamplot", fields::streamplot)
}
