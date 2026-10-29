//! `_native.axes`: axes and their plotting methods (matplotlib's
//! `axes`).

use pltrs::artist::ArtistHandle;
use pltrs::axes::{Axes, BarOptions, PlotError, ScatterColors, ScatterOptions, TitleLocation};
use pltrs::collections::Collection;
use pltrs::colors::Color;
use pltrs::legend::{Legend, LegendError, LegendOptions};
use pltrs::lines::Line2D;
use pltrs::patches::{Patch, Rectangle};
use pltrs::text::Text;

use super::lines::LineKwargs;
use crate::binding::prelude::*;

artist_handle!(Axes);

impl From<PlotError> for PyErr {
    /// matplotlib's `ValueError` for data it cannot plot.
    fn from(err: PlotError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

impl From<LegendError> for PyErr {
    fn from(err: LegendError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// One line of `y` against `x` (one group of `plot`'s arguments), styled
/// by the format string and the keyword arguments.
fn plot(
    axes: Axes,
    x: NdArray,
    y: NdArray,
    fmt: Option<String>,
    kwargs: LineKwargs,
) -> PyResult<Line2D> {
    let props = kwargs.over_format(fmt.as_deref())?;
    Ok(axes.plot_with(x.into_vec()?, y.into_vec()?, props)?)
}

/// A scatter plot: marker areas in points², one colour for all markers
/// (the next of the property cycle when `None`), a legend label.
fn scatter(
    axes: Axes,
    x: NdArray,
    y: NdArray,
    sizes: Option<NdArray>,
    color: Option<Color>,
    label: Option<String>,
) -> PyResult<Collection> {
    let options = ScatterOptions {
        sizes: sizes.map(NdArray::into_vec).transpose()?,
        colors: color.map_or(ScatterColors::Cycle, ScatterColors::Single),
        ..ScatterOptions::default()
    };
    let collection = axes.scatter_with(x.into_vec()?, y.into_vec()?, options)?;
    if let Some(label) = label {
        use pltrs::artist::ArtistHandle as _;
        collection.set_label(label);
    }
    Ok(collection)
}

/// Vertical bars: their rectangles, and the bar values.
fn bar(
    axes: Axes,
    x: NdArray,
    height: NdArray,
    width: NdArray,
    bottom: Option<NdArray>,
    color: Option<Color>,
    label: Option<String>,
) -> PyResult<(Vec<Patch<Rectangle>>, Vec<f64>)> {
    let defaults = BarOptions::default();
    let options = BarOptions {
        thickness: width.into_vec()?.into(),
        base: match bottom {
            Some(bottom) => bottom.into_vec()?.into(),
            None => defaults.base.clone(),
        },
        color: color.into_iter().collect(),
        label,
        ..defaults
    };
    let bars = axes.bar(x.into_vec()?, height.into_vec()?, options)?;
    Ok((bars.patches, bars.datavalues))
}

/// Sets the title at the location `axes.titlelocation` names.
fn set_title(axes: Axes, text: String) -> PyResult<Text> {
    Ok(axes.set_title(text))
}

/// The title at `loc` (`"center"`, `"left"` or `"right"`).
fn get_title(axes: Axes, loc: String) -> PyResult<String> {
    let location = match loc.as_str() {
        "left" => TitleLocation::Left,
        "right" => TitleLocation::Right,
        "center" => TitleLocation::Center,
        _ => {
            return Err(PyErr::value_error(format!(
                "{loc:?} is not a valid value for loc; supported values are 'left', 'center', \
                 'right'"
            )));
        }
    };
    Ok(axes.title(location))
}

/// Sets the x-axis label.
fn set_xlabel(axes: Axes, text: String) -> PyResult<()> {
    axes.set_xlabel(text);
    Ok(())
}

/// The x-axis label.
fn get_xlabel(axes: Axes) -> PyResult<String> {
    Ok(axes.xaxis().label_text())
}

/// Sets the y-axis label.
fn set_ylabel(axes: Axes, text: String) -> PyResult<()> {
    axes.set_ylabel(text);
    Ok(())
}

/// The y-axis label.
fn get_ylabel(axes: Axes) -> PyResult<String> {
    Ok(axes.yaxis().label_text())
}

/// The x limits.
fn get_xlim(axes: Axes) -> PyResult<(f64, f64)> {
    Ok(axes.xlim())
}

/// Sets the x limits.
fn set_xlim(axes: Axes, left: f64, right: f64) -> PyResult<()> {
    axes.set_xlim(left, right)?;
    Ok(())
}

/// The y limits.
fn get_ylim(axes: Axes) -> PyResult<(f64, f64)> {
    Ok(axes.ylim())
}

/// Sets the y limits.
fn set_ylim(axes: Axes, bottom: f64, top: f64) -> PyResult<()> {
    axes.set_ylim(bottom, top)?;
    Ok(())
}

/// The lines of the axes (`ax.lines`).
fn get_lines(axes: Axes) -> PyResult<Vec<Line2D>> {
    Ok(axes.lines())
}

/// A legend of the labelled artists.
fn legend(axes: Axes) -> PyResult<Legend> {
    Ok(axes.legend(LegendOptions::default())?)
}

/// The legend, if one was made.
fn get_legend(axes: Axes) -> PyResult<Option<Legend>> {
    Ok(axes.current_legend())
}

/// A horizontal (`orientation` `"h"`) or vertical (`"v"`) line across the
/// axes at data position `at`, from axes fraction `start` to `end`
/// (`axhline`, `axvline`).
fn axline(
    axes: Axes,
    orientation: String,
    at: f64,
    (start, end): (f64, f64),
    kwargs: LineKwargs,
) -> PyResult<Line2D> {
    let props = kwargs.over_format(None)?;
    Ok(match orientation.as_str() {
        "h" => axes.axhline_between(at, start, end, props),
        _ => axes.axvline_between(at, start, end, props),
    })
}

/// A horizontal (`orientation` `"h"`) or vertical (`"v"`) band across the
/// axes between data positions `(low, high)`, from axes fraction `start`
/// to `end` (`axhspan`, `axvspan`).
fn axspan(
    axes: Axes,
    orientation: String,
    (low, high): (f64, f64),
    (start, end): (f64, f64),
) -> PyResult<Patch<Rectangle>> {
    Ok(match orientation.as_str() {
        "h" => axes.axhspan_between(low, high, start, end),
        _ => axes.axvspan_between(low, high, start, end),
    })
}

/// The data coordinates of the display point `(x, y)`
/// (`ax.transData.inverted().transform((x, y))`); `ValueError` when the
/// data transform cannot be inverted.
fn display_to_data(axes: Axes, x: f64, y: f64) -> PyResult<(f64, f64)> {
    let figure = axes.figure();
    let to_display = figure
        .resolve_transform(&axes.trans_data())
        .map_err(|err| PyErr::value_error(err.to_string()))?;
    let to_data = to_display
        .inverted()
        .ok_or_else(|| PyErr::value_error("the data transform of the axes is singular"))?;
    let point = to_data.transform_point(pltrs::transforms::Point::new(x, y));
    Ok((point.x, point.y))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("plot", plot)?;
    module.add("display_to_data", display_to_data)?;
    module.add("axline", axline)?;
    module.add("axspan", axspan)?;
    module.add("scatter", scatter)?;
    module.add("bar", bar)?;
    module.add("set_title", set_title)?;
    module.add("get_title", get_title)?;
    module.add("set_xlabel", set_xlabel)?;
    module.add("get_xlabel", get_xlabel)?;
    module.add("set_ylabel", set_ylabel)?;
    module.add("get_ylabel", get_ylabel)?;
    module.add("get_xlim", get_xlim)?;
    module.add("set_xlim", set_xlim)?;
    module.add("get_ylim", get_ylim)?;
    module.add("set_ylim", set_ylim)?;
    module.add("get_lines", get_lines)?;
    module.add("legend", legend)?;
    module.add("get_legend", get_legend)
}
