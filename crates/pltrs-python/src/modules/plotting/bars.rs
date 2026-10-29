//! Bars: `bar`, `barh` and `broken_barh`.

use pltrs::axes::{Axes, BandAlign, BarAlign, BarOptions, ErrorbarFormat, ErrorbarOptions};
use pltrs::collections::Collection;
use pltrs::colors::Color;
use pltrs::container::Container;
use pltrs::patches::{Patch, Rectangle};
use pltrs::units::{AxisDelta, AxisValue};

use super::super::lines::LineKwargs;
use super::errorbar::{BarStyle, ErrorbarArtists, Errors, error_size, errorbar_artists};
use super::values::{PyAxisDeltas, PyAxisValues};
use crate::binding::prelude::*;

/// The per-bar styles: `(color, edgecolor, linewidth, hatch)`, each
/// empty or cycled over the bars.
type BarStyles = (Vec<Color>, Vec<Color>, Vec<f64>, Vec<String>);

/// The labels: `(label, per-bar labels, tick labels, log)`.
type BarLabels = (Option<String>, Vec<String>, Vec<String>, bool);

/// The error bars of the bars: `(xerr, yerr, bar style, line kwargs)`.
type BarErrors = (Errors, Errors, BarStyle, LineKwargs);

/// The artists of a bar plot: its container, the rectangles, the error
/// bars and the bar values.
type BarArtists = (
    Container,
    Vec<Patch<Rectangle>>,
    Option<ErrorbarArtists>,
    Vec<f64>,
);

/// Vertical (or, `horizontal`, horizontal) bars at `x` with values
/// `height` (matplotlib's `bar` and `barh`, whose keyword handling the
/// Python side did): the thickness and base of each bar, the alignment
/// (`"center"` or `"edge"`), the styles, labels and error bars.
pub(super) fn bar(
    axes: Axes,
    horizontal: bool,
    (x, height): (PyAxisValues, PyAxisDeltas),
    (width, bottom): (PyAxisDeltas, Option<PyAxisValues>),
    align: String,
    (color, edgecolor, linewidth, hatch): BarStyles,
    (label, bar_labels, tick_labels, log): BarLabels,
    (xerr, yerr, style, kwargs): BarErrors,
) -> PyResult<BarArtists> {
    let defaults = BarOptions::default();
    let (ecolor, elinewidth, elinestyle, capsize, capthick, barsabove) = style;
    let options = BarOptions {
        thickness: width.0,
        base: bottom.map_or(defaults.base, |bottom| bottom.0),
        align: match align.as_str() {
            "edge" => BarAlign::Edge,
            _ => BarAlign::Center,
        },
        color,
        edgecolor,
        linewidth,
        hatch,
        label,
        bar_labels,
        tick_labels,
        log,
        errors: ErrorbarOptions {
            xerr: error_size(xerr)?,
            yerr: error_size(yerr)?,
            fmt: ErrorbarFormat::None,
            line: kwargs.0,
            ecolor,
            elinewidth,
            elinestyle: elinestyle.map(|style| style.0),
            capsize,
            capthick,
            barsabove,
            ..ErrorbarOptions::default()
        },
        ..defaults
    };
    let bars = if horizontal {
        axes.barh(x.0, height.0, options)?
    } else {
        axes.bar(x.0, height.0, options)?
    };
    let errorbar = bars.errorbar.clone().map(errorbar_artists);
    Ok((
        Container::Bar(bars.clone()),
        bars.patches,
        errorbar,
        bars.datavalues,
    ))
}

/// A horizontal band of rectangles, one per `(start, width)` of
/// `xranges`, at `position` with `height` (matplotlib's `broken_barh`);
/// `align` is `"bottom"`, `"center"` or `"top"`.
pub(super) fn broken_barh(
    axes: Axes,
    (starts, widths): (PyAxisValues, PyAxisDeltas),
    (position, height): (PyAxisValues, PyAxisDeltas),
    align: String,
) -> PyResult<Collection> {
    let ranges: Vec<(AxisValue, AxisDelta)> =
        starts.items().into_iter().zip(widths.items()).collect();
    let position = position
        .items()
        .into_iter()
        .next()
        .ok_or_else(|| PyErr::value_error("broken_barh needs a band position"))?;
    let height = height
        .items()
        .into_iter()
        .next()
        .ok_or_else(|| PyErr::value_error("broken_barh needs a band height"))?;
    let align = match align.as_str() {
        "center" => BandAlign::Center,
        "top" => BandAlign::Top,
        _ => BandAlign::Bottom,
    };
    Ok(axes.broken_barh(&ranges, (position, height), align))
}
