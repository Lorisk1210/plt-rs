//! Lines: `plot`, `axline`, `hlines` and `vlines`.

use pltrs::axes::Axes;
use pltrs::collections::Collection;
use pltrs::colors::Color;
use pltrs::lines::{AxLineThrough, FormatString, Line2D, LineProps};
use pltrs::transforms::{Point, Transform};

use super::super::lines::LineKwargs;
use crate::binding::prelude::*;

/// The single-letter colours a format string keeps as given; matplotlib
/// turns the others (`"C3"`, a whole-string colour) into RGBA tuples.
const LETTER_COLORS: [&str; 8] = ["b", "g", "r", "c", "m", "y", "k", "w"];

/// The properties of the format string `fmt`, its colour as matplotlib's
/// `_process_plot_format` returns it.
fn format_props(fmt: &str) -> PyResult<LineProps> {
    let mut props = LineProps::fmt(fmt)?;
    if let Some(Color::Spec(spec)) = &props.color
        && !LETTER_COLORS.contains(&spec.as_str())
    {
        props.color = Some(Color::Rgba(Color::from(spec.as_str()).resolve()?));
    }
    Ok(props)
}

/// The parts of a format string: `(linestyle, marker, color)`, `"None"`
/// for a part it turns off and `None` for the colour it leaves to the
/// property cycle (matplotlib's `_process_plot_format`).
pub(super) fn format_parts(fmt: String) -> PyResult<(String, Option<String>, Option<Color>)> {
    let parsed: FormatString = fmt.parse()?;
    let props = format_props(&fmt)?;
    Ok((
        parsed
            .linestyle
            .map_or_else(|| "None".to_owned(), |style| style.to_string()),
        parsed.marker.map(|marker| match marker {
            pltrs::markers::Marker::Symbol(symbol) => pltrs::markers::MarkerSymbol::all()
                .find(|(_, s)| *s == symbol)
                .map_or_else(
                    || "None".to_owned(),
                    |(key, _)| match key {
                        pltrs::markers::MarkerKey::Char(c) => c.to_owned(),
                        pltrs::markers::MarkerKey::Code(code) => code.to_string(),
                    },
                ),
            _ => "None".to_owned(),
        }),
        props.color,
    ))
}

/// One line of `y` against `x` (one dataset of `plot`), styled by the
/// format string and the keyword arguments on top, rescaling the view
/// along the axes `rescale` (`(scalex, scaley)`) asks for.
pub(super) fn plot(
    axes: Axes,
    x: NdArray,
    y: NdArray,
    fmt: Option<String>,
    kwargs: LineKwargs,
    rescale: (bool, bool),
) -> PyResult<Line2D> {
    let base = match fmt {
        Some(fmt) => format_props(&fmt)?,
        None => LineProps::default(),
    };
    Ok(axes.plot_scaled(
        x.into_vec()?,
        y.into_vec()?,
        base.overlay(kwargs.0),
        rescale,
    )?)
}

/// An infinite line through `xy1` and `xy2`, or through `xy1` with
/// `slope` (matplotlib's `axline`).
pub(super) fn axline(
    axes: Axes,
    (x1, y1): (f64, f64),
    xy2: Option<(f64, f64)>,
    slope: Option<f64>,
    kwargs: LineKwargs,
) -> PyResult<Line2D> {
    let through = match (xy2, slope) {
        (Some((x2, y2)), _) => AxLineThrough::Point(Point::new(x2, y2)),
        (None, Some(slope)) => AxLineThrough::Slope(slope),
        (None, None) => {
            return Err(PyErr::type_error(
                "Exactly one of 'xy2' and 'slope' must be given",
            ));
        }
    };
    Ok(axes.axline(Point::new(x1, y1), through, kwargs.0)?)
}

/// Horizontal (`vertical` false) or vertical segments at `positions`
/// from `low` to `high` (matplotlib's `hlines` and `vlines`), in
/// `transform` coordinates (`None`: the data coordinates).
pub(super) fn segments(
    axes: Axes,
    vertical: bool,
    (positions, low, high): (NdArray, NdArray, NdArray),
    transform: Option<Transform>,
) -> PyResult<Collection> {
    let transform = transform.unwrap_or_else(|| axes.trans_data());
    Ok(if vertical {
        axes.vlines_in(
            positions.into_vec()?,
            low.into_vec()?,
            high.into_vec()?,
            transform,
        )?
    } else {
        axes.hlines_in(
            positions.into_vec()?,
            low.into_vec()?,
            high.into_vec()?,
            transform,
        )?
    })
}
