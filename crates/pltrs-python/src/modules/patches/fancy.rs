//! The styles and mutation of fancy patches (matplotlib's
//! `FancyBboxPatch.set_boxstyle`, `FancyArrowPatch.set_arrowstyle`,
//! `set_connectionstyle`, `set_mutation_scale`, `set_mutation_aspect` and
//! their getters).
//!
//! A style crosses as matplotlib's description string (`"round,pad=0.3"`),
//! which the Python side turns into its style object; an arrow is a
//! `FancyArrowPatch` or a `ConnectionPatch`.

use pltrs::artist::{RawHandle, TypedHandle};
use pltrs::patches::{
    ArrowSpec, ArrowStyle, BoxStyle, ConnectionPatch, ConnectionStyle, FancyArrowPatch,
    FancyBboxPatch, Patch,
};

use super::super::patch_styles::style_error;
use crate::binding::prelude::*;

/// The error for an artist that is no fancy patch.
fn not_fancy() -> PyErr {
    PyErr::type_error("the artist is no FancyBboxPatch or FancyArrowPatch")
}

/// How the arrow `patch` is built.
fn arrow_spec(patch: &RawHandle) -> PyResult<ArrowSpec> {
    if let Some(arrow) = Patch::<FancyArrowPatch>::from_raw_handle(patch) {
        Ok(arrow.shape().spec)
    } else if let Some(link) = Patch::<ConnectionPatch>::from_raw_handle(patch) {
        Ok(link.shape().spec)
    } else {
        Err(not_fancy())
    }
}

/// Changes how the arrow `patch` is built.
fn update_arrow_spec(patch: &RawHandle, f: impl FnOnce(&mut ArrowSpec)) -> PyResult<()> {
    if let Some(arrow) = Patch::<FancyArrowPatch>::from_raw_handle(patch) {
        arrow.update_shape(|shape| f(&mut shape.spec));
    } else if let Some(link) = Patch::<ConnectionPatch>::from_raw_handle(patch) {
        link.update_shape(|shape| f(&mut shape.spec));
    } else {
        return Err(not_fancy());
    }
    Ok(())
}

/// The box style of a fancy box, described.
fn get_boxstyle(patch: Patch<FancyBboxPatch>) -> PyResult<String> {
    Ok(patch.shape().boxstyle.to_string())
}

/// Sets the box style of a fancy box from its description.
fn set_boxstyle(patch: Patch<FancyBboxPatch>, spec: String) -> PyResult<()> {
    let style = spec.parse::<BoxStyle>().map_err(style_error)?;
    patch.update_shape(|shape| shape.boxstyle = style);
    Ok(())
}

/// The arrow style of an arrow, described.
fn get_arrowstyle(patch: RawHandle) -> PyResult<String> {
    Ok(arrow_spec(&patch)?.arrowstyle.to_string())
}

/// Sets the arrow style of an arrow from its description.
fn set_arrowstyle(patch: RawHandle, spec: String) -> PyResult<()> {
    let style = spec.parse::<ArrowStyle>().map_err(style_error)?;
    update_arrow_spec(&patch, |arrow| arrow.arrowstyle = style)
}

/// The connection style of an arrow, described.
fn get_connectionstyle(patch: RawHandle) -> PyResult<String> {
    Ok(arrow_spec(&patch)?.connectionstyle.to_string())
}

/// Sets the connection style of an arrow from its description.
fn set_connectionstyle(patch: RawHandle, spec: String) -> PyResult<()> {
    let style = spec.parse::<ConnectionStyle>().map_err(style_error)?;
    update_arrow_spec(&patch, |arrow| arrow.connectionstyle = style)
}

/// The mutation scale and aspect of a fancy box or an arrow.
fn get_mutation(patch: RawHandle) -> PyResult<(f64, f64)> {
    if let Some(fancy) = Patch::<FancyBboxPatch>::from_raw_handle(&patch) {
        let shape = fancy.shape();
        return Ok((shape.mutation_scale, shape.mutation_aspect));
    }
    let spec = arrow_spec(&patch)?;
    Ok((spec.mutation_scale, spec.mutation_aspect))
}

/// Sets the mutation scale and aspect of a fancy box or an arrow, each
/// kept when `None`.
fn set_mutation(patch: RawHandle, scale: Option<f64>, aspect: Option<f64>) -> PyResult<()> {
    if let Some(fancy) = Patch::<FancyBboxPatch>::from_raw_handle(&patch) {
        fancy.update_shape(|shape| {
            shape.mutation_scale = scale.unwrap_or(shape.mutation_scale);
            shape.mutation_aspect = aspect.unwrap_or(shape.mutation_aspect);
        });
        return Ok(());
    }
    update_arrow_spec(&patch, |spec| {
        spec.mutation_scale = scale.unwrap_or(spec.mutation_scale);
        spec.mutation_aspect = aspect.unwrap_or(spec.mutation_aspect);
    })
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_boxstyle", get_boxstyle)?;
    module.add("set_boxstyle", set_boxstyle)?;
    module.add("get_arrowstyle", get_arrowstyle)?;
    module.add("set_arrowstyle", set_arrowstyle)?;
    module.add("get_connectionstyle", get_connectionstyle)?;
    module.add("set_connectionstyle", set_connectionstyle)?;
    module.add("get_mutation", get_mutation)?;
    module.add("set_mutation", set_mutation)
}
