//! Events and stems: `eventplot` and `stem`.

use std::ops::Deref;

use pltrs::axes::{Axes, EventplotOptions, StemOptions};
use pltrs::collections::Collection;
use pltrs::colors::Color;
use pltrs::container::Container;
use pltrs::lines::{FormatString, Line2D};

use super::super::lines::PyLineStyle;
use super::fills::orientation;
use crate::binding::prelude::*;

/// The per-row styles of an event plot: `(lineoffsets, linelengths,
/// linewidths, colors, alpha, linestyles)`.
type EventStyles = (
    Vec<f64>,
    Vec<f64>,
    Option<Vec<f64>>,
    Vec<Color>,
    Option<Vec<f64>>,
    Vec<PyLineStyle>,
);

/// One row of event lines per list of `positions` (matplotlib's
/// `eventplot`), each style one per row or one for all.
pub(super) fn eventplot(
    axes: Axes,
    positions: Vec<NdArray>,
    orient: String,
    (lineoffsets, linelengths, linewidths, colors, alpha, linestyles): EventStyles,
) -> PyResult<Vec<Collection>> {
    let positions = (positions.into_iter())
        .map(NdArray::into_vec)
        .collect::<PyResult<Vec<_>>>()?;
    let options = EventplotOptions {
        orientation: orientation(&orient),
        lineoffsets,
        linelengths,
        linewidths,
        colors,
        alpha,
        linestyles: linestyles.into_iter().map(|style| style.0).collect(),
    };
    Ok(axes
        .eventplot(&positions, options)?
        .iter()
        .map(|events| events.deref().clone())
        .collect())
}

/// The formats of a stem plot: `(linefmt, markerfmt, basefmt)`.
type StemFormats = (String, String, String);

/// The artists of a stem plot: its container, the marker line, the stem
/// lines and the baseline.
type StemArtists = (Container, Line2D, Collection, Line2D);

/// Stems from `bottom` to `heads` at `locs` (matplotlib's `stem`, whose
/// format defaults the Python side resolved).
pub(super) fn stem(
    axes: Axes,
    (locs, heads): (NdArray, NdArray),
    (linefmt, markerfmt, basefmt): StemFormats,
    bottom: f64,
    orient: String,
    label: Option<String>,
) -> PyResult<StemArtists> {
    let options = StemOptions {
        linefmt: linefmt.parse::<FormatString>()?,
        markerfmt: markerfmt.parse::<FormatString>()?,
        basefmt: Some(basefmt.parse::<FormatString>()?),
        bottom,
        orientation: orientation(&orient),
        label,
    };
    let stems = axes.stem(locs.into_vec()?, heads.into_vec()?, options)?;
    Ok((
        Container::Stem(stems.clone()),
        stems.markerline,
        stems.stemlines,
        stems.baseline,
    ))
}
