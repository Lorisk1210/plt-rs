//! Artist properties given as matplotlib keyword dictionaries (the
//! `props` and `handle_props` of the widgets).

use pltrs::colors::Color;
use pltrs::lines::{LineProps, LineStyle};
use pltrs::markers::Marker;
use pltrs::patches::PatchProps;
use pltrs::rc::ColorOrAuto;

use super::{malformed, steps::unknown};
use crate::json::Json;
use crate::probes::ProbeError;

/// The entries of a keyword dictionary.
fn entries(kw: &Json) -> Result<&[(String, Json)], ProbeError> {
    kw.as_object().ok_or_else(|| malformed("properties"))
}

fn number(value: &Json, name: &str) -> Result<f64, ProbeError> {
    value.as_f64().ok_or_else(|| malformed(name))
}

fn flag(value: &Json, name: &str) -> Result<bool, ProbeError> {
    value.as_bool().ok_or_else(|| malformed(name))
}

fn color(value: &Json, name: &str) -> Result<Color, ProbeError> {
    value
        .as_str()
        .map(Color::from)
        .ok_or_else(|| malformed(name))
}

fn linestyle(value: &Json) -> Result<LineStyle, ProbeError> {
    value
        .as_str()
        .ok_or_else(|| malformed("linestyle"))?
        .parse()
        .map_err(ProbeError::rejected)
}

/// Patch properties from `{facecolor, edgecolor, alpha, fill, visible,
/// linestyle}`.
pub(super) fn patch(kw: &Json) -> Result<PatchProps, ProbeError> {
    entries(kw)?
        .iter()
        .try_fold(PatchProps::default(), |props, (name, value)| {
            Ok(match name.as_str() {
                "facecolor" => props.facecolor(color(value, name)?),
                "edgecolor" => props.edgecolor(color(value, name)?),
                "alpha" => props.alpha(number(value, name)?),
                "fill" => props.fill(flag(value, name)?),
                "visible" => props.visible(flag(value, name)?),
                "linestyle" => props.linestyle(linestyle(value)?),
                other => return Err(unknown("patch property", other)),
            })
        })
}

/// Line properties from `{color, alpha, lw, linestyle, marker,
/// markerfacecolor, markeredgecolor}`.
pub(super) fn line(kw: &Json) -> Result<LineProps, ProbeError> {
    entries(kw)?
        .iter()
        .try_fold(LineProps::default(), |props, (name, value)| {
            Ok(match name.as_str() {
                "color" => props.color(color(value, name)?),
                "alpha" => props.alpha(number(value, name)?),
                "lw" | "linewidth" => props.linewidth(number(value, name)?),
                "linestyle" => props.linestyle(linestyle(value)?),
                "marker" => props.marker(
                    value
                        .as_str()
                        .ok_or_else(|| malformed(name))?
                        .parse::<Marker>()
                        .map_err(ProbeError::rejected)?,
                ),
                "markerfacecolor" => props.markerfacecolor(ColorOrAuto::Color(color(value, name)?)),
                "markeredgecolor" => props.markeredgecolor(ColorOrAuto::Color(color(value, name)?)),
                other => return Err(unknown("line property", other)),
            })
        })
}
