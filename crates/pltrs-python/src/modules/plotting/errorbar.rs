//! `errorbar`, and the error bars other plotting methods draw.

use pltrs::axes::{Axes, ErrorSize, ErrorbarContainer, ErrorbarFormat, ErrorbarOptions};
use pltrs::collections::Collection;
use pltrs::colors::Color;
use pltrs::container::Container;
use pltrs::lines::{Line2D, MarkEvery};

use super::super::lines::{LineKwargs, PyLineStyle};
use super::values::PyAxisValues;
use crate::binding::prelude::*;

/// Error sizes below and above each point, already broadcast by the
/// Python side.
pub(super) type Errors = Option<(NdArray, NdArray)>;

/// The styles of the bars and caps: `(ecolor, elinewidth, elinestyle,
/// capsize, capthick, barsabove)`.
pub(super) type BarStyle = (
    Option<Color>,
    Option<f64>,
    Option<PyLineStyle>,
    Option<f64>,
    Option<f64>,
    bool,
);

/// The limit flags `(lolims, uplims, xlolims, xuplims)`, each one per
/// point.
type Limits = (Vec<bool>, Vec<bool>, Vec<bool>, Vec<bool>);

/// The artists of an error bar plot: its container, the data line (none
/// for `fmt="none"`), the caps and the bar collections.
pub(super) type ErrorbarArtists = (Container, Option<Line2D>, Vec<Line2D>, Vec<Collection>);

/// Asymmetric error sizes from the lower and upper arrays.
pub(super) fn error_size(errors: Errors) -> PyResult<Option<ErrorSize>> {
    errors
        .map(|(lower, upper)| {
            Ok(ErrorSize::Asymmetric {
                lower: lower.into_vec()?.into(),
                upper: upper.into_vec()?.into(),
            })
        })
        .transpose()
}

/// The artists of an error bar container, for Python.
pub(super) fn errorbar_artists(container: ErrorbarContainer) -> ErrorbarArtists {
    (
        Container::Errorbar(container.clone()),
        container.data_line,
        container.caplines,
        container.barlinecols,
    )
}

/// Error bars around `(x, y)` (matplotlib's `errorbar`, whose keyword
/// handling and input checks the Python side did).
pub(super) fn errorbar(
    axes: Axes,
    (x, y): (PyAxisValues, PyAxisValues),
    (xerr, yerr): (Errors, Errors),
    fmt: String,
    kwargs: LineKwargs,
    (ecolor, elinewidth, elinestyle, capsize, capthick, barsabove): BarStyle,
    (lolims, uplims, xlolims, xuplims): Limits,
    every: Option<Vec<bool>>,
) -> PyResult<ErrorbarArtists> {
    let options = ErrorbarOptions {
        xerr: error_size(xerr)?,
        yerr: error_size(yerr)?,
        fmt: fmt.parse::<ErrorbarFormat>()?,
        line: kwargs.0,
        ecolor,
        elinewidth,
        elinestyle: elinestyle.map(|style| style.0),
        capsize,
        capthick,
        barsabove,
        lolims,
        uplims,
        xlolims,
        xuplims,
        errorevery: every.map(MarkEvery::Mask),
    };
    Ok(errorbar_artists(axes.errorbar(x.0, y.0, options)?))
}
