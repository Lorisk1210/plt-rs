//! Scatter plots (colour, edge and width arguments) and event plots.

use plt::artist::ArtistHandle;
use plt::axes::{EventplotOptions, MappedColors, ScatterColors, ScatterOptions};
use plt::collections::EdgeColors;
use plt::colors::Normalize;
use plt::markers::{FillStyle, Marker, MarkerStyle};

use super::{colors, is_color_list, made, number_list, numbers, on_axes, optional, rgba_list};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// One colour, or colours.
fn color_spec(value: &Json) -> Result<ScatterColors, ProbeError> {
    if is_color_list(value) {
        Ok(ScatterColors::PerPoint(colors(value)?))
    } else {
        Ok(ScatterColors::Single(super::color(value)?))
    }
}

/// `c` (colours, or numbers to map through the probe's `norm`, `vmin` and
/// `vmax`) or `color` (colours).
fn marker_colors(args: Args<'_>) -> Result<ScatterColors, ProbeError> {
    let Some(value) = args.get("c") else {
        return args
            .get("color")
            .map_or(Ok(ScatterColors::Cycle), color_spec);
    };
    let Some(values) = value.as_f64_vec() else {
        return color_spec(value);
    };
    let mut mapped = MappedColors::new(values);
    if let Some(norm) = optional(args, "norm", number_list)? {
        let [vmin, vmax] = norm[..] else {
            return Err(ProbeError::BadArgs("norm is not [vmin, vmax]".into()));
        };
        mapped = mapped.with_norm(Normalize::new(vmin, vmax));
    }
    let (vmin, vmax) = (args.opt_f64("vmin")?, args.opt_f64("vmax")?);
    if vmin.is_some() || vmax.is_some() {
        mapped = mapped.with_limits(vmin, vmax);
    }
    Ok(mapped.into())
}

/// `edgecolors` (or `edgecolor`): `"face"`, a colour or colours; without
/// them, the `color` keyword colours the edges too, as in matplotlib.
fn edge_colors(args: Args<'_>) -> Result<Option<EdgeColors>, ProbeError> {
    let Some(value) = ["edgecolors", "edgecolor", "color"]
        .into_iter()
        .find_map(|key| args.get(key))
    else {
        return Ok(None);
    };
    Ok(Some(match value {
        Json::String(face) if face == "face" => EdgeColors::Face,
        other => EdgeColors::Colors(colors(other)?),
    }))
}

/// `marker` with `fillstyle`.
fn marker_style(args: Args<'_>) -> Result<Option<MarkerStyle>, ProbeError> {
    let marker = args.opt_str("marker")?;
    let fill = args.opt_str("fillstyle")?;
    if marker.is_none() && fill.is_none() {
        return Ok(None);
    }
    let marker: Marker = made(marker.unwrap_or("o").parse())?;
    let fill: FillStyle = made(fill.unwrap_or("full").parse())?;
    Ok(Some(made(MarkerStyle::new(marker, fill))?))
}

pub(super) fn scatter(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = ScatterOptions {
            sizes: optional(args, "s", number_list)?,
            colors: marker_colors(args)?,
            marker: marker_style(args)?,
            edgecolors: edge_colors(args)?,
            linewidths: optional(args, "linewidths", number_list)?,
            ..ScatterOptions::default()
        };
        let points = made(ax.scatter_with(args.f64_vec("x")?, args.f64_vec("y")?, options))?;
        let colors = made(points.colors())?;
        Ok(encode::object([
            ("faces", rgba_list(&colors.faces)),
            ("edges", rgba_list(&colors.edges)),
            ("linewidths", numbers(&points.linewidths())),
        ]))
    })
}

pub(super) fn eventplot(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = EventplotOptions {
            colors: optional(args, "colors", colors)?.unwrap_or_default(),
            alpha: optional(args, "alpha", number_list)?.unwrap_or_default(),
            ..EventplotOptions::default()
        };
        let rows = made(ax.eventplot(&super::datasets(args.json("positions")?)?, options))?;
        let colors = rows
            .iter()
            .map(|row| Ok(rgba_list(&made(row.colors())?.edges)))
            .collect::<Result<Vec<_>, ProbeError>>()?;
        Ok(encode::object([
            ("colors", Json::Array(colors)),
            (
                "alphas",
                Json::Array(rows.iter().map(|row| Json::from(row.alpha())).collect()),
            ),
        ]))
    })
}
