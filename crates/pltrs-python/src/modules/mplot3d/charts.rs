//! The 3-D plotting methods drawing error bars, stems and areas between
//! curves (matplotlib's `Axes3D.errorbar`, `stem` and `fill_between`),
//! whose arguments the Python side resolved as matplotlib does.

use pltrs::axes::ErrorbarFormat;
use pltrs::collections::{Collection, CollectionProps, EdgeColors};
use pltrs::colors::Color;
use pltrs::lines::{Line2D, MarkEvery};
use pltrs::mplot3d::{Axes3D, Errorbar3DOptions, FillBetween3DOptions, FillMode, Stem3DOptions};

use super::super::collections::rgba_rows;
use super::super::lines::LineKwargs;
use super::super::plotting::errorbar::{Errors, error_size};
use super::super::plotting::values::PyAxisValues;
use super::axis3;
use super::plotting::{Shading, light};
use crate::binding::prelude::*;

/// Three coordinates as plotting values.
type Coordinates = (PyAxisValues, PyAxisValues, PyAxisValues);

/// The bars and caps: `(ecolor, elinewidth, capsize, capthick,
/// barsabove)`.
type CapStyle = (Option<Color>, Option<f64>, Option<f64>, Option<f64>, bool);

/// The limit flags along x, y and z, each one per point: `(lolims,
/// uplims)`.
type Limits = (
    (Vec<bool>, Vec<bool>, Vec<bool>),
    (Vec<bool>, Vec<bool>, Vec<bool>),
);

/// Error bars around the points `(x, y, z)` (matplotlib's
/// `Axes3D.errorbar`): the errors below and above each point along each
/// axis (`None`: no bars along it), the format and the line properties,
/// the style of the bars and caps, the limits, the points that get bars
/// (`None`: all) and `axlim_clip`. Returns the data line (none for
/// `fmt="none"`), the bars and the caps.
fn errorbar(
    axes: Axes3D,
    (x, y, z): Coordinates,
    (xerr, yerr, zerr): (Errors, Errors, Errors),
    (fmt, kwargs): (String, LineKwargs),
    (ecolor, elinewidth, capsize, capthick, barsabove): CapStyle,
    ((xlo, ylo, zlo), (xup, yup, zup)): Limits,
    (every, axlim_clip): (Option<Vec<bool>>, bool),
) -> PyResult<(Option<Line2D>, Vec<Collection>, Vec<Line2D>)> {
    let options = Errorbar3DOptions {
        errors: [error_size(xerr)?, error_size(yerr)?, error_size(zerr)?],
        fmt: fmt.parse::<ErrorbarFormat>()?,
        line: kwargs.0,
        ecolor,
        elinewidth,
        capsize,
        capthick,
        barsabove,
        lolims: [xlo, ylo, zlo],
        uplims: [xup, yup, zup],
        errorevery: every.map(MarkEvery::Mask),
        axlim_clip,
    };
    let bars = axes.errorbar(x.0, y.0, z.0, options)?;
    Ok((bars.data_line, bars.errlines, bars.caplines))
}

/// Stems from `bottom` along `orientation` to the heads `(x, y, z)`
/// (matplotlib's `Axes3D.stem`), styled by the format strings of the
/// stems, the markers and the baseline. Returns the markers, the stems
/// and the baseline.
fn stem(
    axes: Axes3D,
    (x, y, z): Coordinates,
    (linefmt, markerfmt, basefmt): (String, String, String),
    bottom: f64,
    orientation: String,
    axlim_clip: bool,
) -> PyResult<(Line2D, Collection, Line2D)> {
    let options = Stem3DOptions {
        linefmt: linefmt.parse()?,
        markerfmt: markerfmt.parse()?,
        basefmt: basefmt.parse()?,
        bottom,
        orientation: axis3(&orientation)?,
        axlim_clip,
    };
    let stems = axes.stem(x.0, y.0, z.0, options)?;
    Ok((stems.markerline, stems.stemlines, stems.baseline))
}

/// The cut of the area between two curves into polygons: `"auto"`,
/// `"quad"` or `"polygon"`.
fn fill_mode(name: &str) -> PyResult<FillMode> {
    match name {
        "auto" => Ok(FillMode::Auto),
        "quad" => Ok(FillMode::Quad),
        "polygon" => Ok(FillMode::Polygon),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for mode; supported values are 'auto', 'quad', \
             'polygon'"
        ))),
    }
}

/// The area between the curves `first` and `second` (matplotlib's
/// `Axes3D.fill_between`): where to fill (`None`: everywhere), the cut,
/// the face colours (`None`: the next patch colour) and the edge colours
/// shaded with them (RGBA rows), the shading and `axlim_clip`.
fn fill_between(
    axes: Axes3D,
    (first, second): (Coordinates, Coordinates),
    (mask, mode): (Option<Vec<bool>>, String),
    (facecolors, edgecolors): (Option<NdArray>, Option<NdArray>),
    (shade, lightsource): Shading,
    axlim_clip: bool,
) -> PyResult<Collection> {
    let props = CollectionProps {
        facecolors: facecolors.as_ref().map(rgba_rows).transpose()?,
        edgecolors: edgecolors
            .as_ref()
            .map(rgba_rows)
            .transpose()?
            .map(EdgeColors::Colors),
        ..CollectionProps::default()
    };
    let options = FillBetween3DOptions {
        mask,
        mode: fill_mode(&mode)?,
        shade,
        lightsource: lightsource.map(light),
        props,
        axlim_clip,
    };
    let curve = |(x, y, z): Coordinates| (x.0, y.0, z.0);
    Ok(axes.fill_between(curve(first), curve(second), options)?)
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("errorbar", errorbar)?;
    module.add("stem", stem)?;
    module.add("fill_between", fill_between)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_modes_are_named() {
        assert_eq!(fill_mode("quad").unwrap(), FillMode::Quad);
        assert!(fill_mode("triangle").is_err());
    }
}
