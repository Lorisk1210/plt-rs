//! Bars: `bar`, `barh`, `broken_barh` and `grouped_bar`.

use pltrs::axes::{
    Axes, BandAlign, BarAlign, BarOptions, ErrorbarFormat, ErrorbarOptions, GroupedBarData,
    GroupedBarError, GroupedBarOptions,
};
use pltrs::collections::Collection;
use pltrs::colors::Color;
use pltrs::container::Container;
use pltrs::orientation::Orientation;
use pltrs::patches::{Patch, Rectangle};

use super::super::lines::{LineKwargs, PyLineStyle};
use super::errorbar::{BarStyle, ErrorbarArtists, Errors, error_size, errorbar_artists};
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
/// `height` (matplotlib's `bar` and `barh`, whose keyword handling and
/// unit conversion the Python side did): the thickness and base of each
/// bar, the alignment (`"center"` or `"edge"`), the styles, labels and
/// error bars.
pub(super) fn bar(
    axes: Axes,
    horizontal: bool,
    (x, height): (NdArray, NdArray),
    (width, bottom): (NdArray, Option<NdArray>),
    align: String,
    (color, edgecolor, linewidth, hatch): BarStyles,
    (label, bar_labels, tick_labels, log): BarLabels,
    (xerr, yerr, style, kwargs): BarErrors,
) -> PyResult<BarArtists> {
    let defaults = BarOptions::default();
    let (ecolor, elinewidth, elinestyle, capsize, capthick, barsabove) = style;
    let options = BarOptions {
        thickness: width.into_vec()?.into(),
        base: match bottom {
            Some(bottom) => bottom.into_vec()?.into(),
            None => defaults.base,
        },
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
    let (x, height) = (x.into_vec()?, height.into_vec()?);
    let bars = if horizontal {
        axes.barh(x, height, options)?
    } else {
        axes.bar(x, height, options)?
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
/// `xranges`, at `position` with `height` (matplotlib's `broken_barh`,
/// whose unit conversion the Python side did); `align` is `"bottom"`,
/// `"center"` or `"top"`.
pub(super) fn broken_barh(
    axes: Axes,
    (starts, widths): (NdArray, NdArray),
    (position, height): (f64, f64),
    align: String,
) -> PyResult<Collection> {
    let ranges: Vec<(f64, f64)> = (starts.data.iter().copied())
        .zip(widths.data.iter().copied())
        .collect();
    let align = match align.as_str() {
        "center" => BandAlign::Center,
        "top" => BandAlign::Top,
        _ => BandAlign::Bottom,
    };
    Ok(axes.broken_barh(&ranges, (position, height), align))
}

impl From<GroupedBarError> for PyErr {
    /// matplotlib's `ValueError` for datasets, positions and labels that
    /// do not fit together.
    fn from(err: GroupedBarError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// The per-dataset styles of grouped bars: `(colors, facecolor,
/// edgecolor, linewidth, linestyle, hatch)`, each empty or cycled over
/// the datasets.
type DatasetStyles = (
    Vec<Color>,
    Vec<Color>,
    Vec<Color>,
    Vec<f64>,
    Vec<PyLineStyle>,
    Vec<String>,
);

/// The bars of one dataset of a grouped bar chart: its container, the
/// rectangles and the bar values.
type DatasetBars = (Container, Vec<Patch<Rectangle>>, Vec<f64>);

/// One group of bars per category and one bar per dataset in each group
/// (matplotlib's `grouped_bar`, whose input forms the Python side reduced
/// to a list of datasets): at `positions` (default 0, 1, ...), spaced
/// `(group_spacing, bar_spacing)`, with the category `tick_labels` and
/// the dataset `labels`, vertical or (`horizontal`) horizontal.
pub(super) fn grouped_bar(
    axes: Axes,
    datasets: Vec<Vec<f64>>,
    positions: Option<Vec<f64>>,
    (group_spacing, bar_spacing): (f64, f64),
    (tick_labels, labels): (Vec<String>, Vec<String>),
    horizontal: bool,
    (colors, facecolor, edgecolor, linewidth, linestyle, hatch): DatasetStyles,
) -> PyResult<Vec<DatasetBars>> {
    let options = GroupedBarOptions {
        positions,
        group_spacing,
        bar_spacing,
        tick_labels,
        labels,
        orientation: if horizontal {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        },
        colors,
        facecolor,
        edgecolor,
        linewidth,
        linestyle: linestyle.into_iter().map(|style| style.0).collect(),
        hatch,
        ..GroupedBarOptions::default()
    };
    Ok(axes
        .grouped_bar(GroupedBarData::Datasets(datasets), options)?
        .into_iter()
        .map(|bars| {
            let patches = bars.patches.clone();
            let values = bars.datavalues.clone();
            (Container::Bar(bars), patches, values)
        })
        .collect())
}
