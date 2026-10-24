//! Probe operations for the `mpl_axes` area: matplotlib's own non-image
//! tests of `Axes` (the first part of `tests/test_axes.py`; see
//! `spec/reference/probes/test_mpl_axes.py`), run in the style each probe
//! names. The ops are grouped by topic: [`axis`] (formatters, twins,
//! arrows and annotations), [`hexbin`], [`mesh`] (images and pseudocolour
//! meshes), [`bars`] (bars, grouped bars and a figure legend), [`boxes`] (box
//! and violin plots), [`errorbar`], [`stem`], [`hist`] (histograms and
//! stairs), [`scatter`] (scatter and event plots) and [`misc`]
//! (projections, pyplot's current axes and saving).

mod axis;
mod bars;
mod boxes;
mod errorbar;
mod hexbin;
mod hist;
mod mesh;
mod misc;
mod scatter;
mod stem;

use plt::axes::Axes;
use plt::colors::Color;
use plt::dates::Datetime64;
use plt::figure::Figure;
use plt_core::Rgba;

use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Op, Outcome, ProbeError, encode};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Axis.set_major_formatter", axis::set_major_formatter),
    Op::new("Axis.set_minor_formatter", axis::set_minor_formatter),
    Op::new("Axes.twin.units", axis::twin_units),
    Op::new("Axes.get_in_layout", axis::twin_in_layout),
    Op::new("Axes.arrow", axis::arrow),
    Op::new("Annotation.arrow_patch", axis::annotation_arrow),
    Op::new("Axes.hexbin", hexbin::hexbin),
    Op::new("Axes.hexbin.contains", hexbin::contains),
    Op::new("Axes.imshow", mesh::imshow),
    Op::new("Axes.pcolor", mesh::pcolor),
    Op::new("Axes.pcolormesh", mesh::pcolormesh),
    Op::new("Axes.bar", bars::bar),
    Op::new("Axes.bar.timedelta", bars::bar_timedelta),
    Op::new("Axes.broken_barh", bars::broken_barh_dates),
    Op::new("Axes.bar.datetime_start", bars::bar_datetime_start),
    Op::new("Axes.grouped_bar", bars::grouped_bar),
    Op::new("Figure.legend", bars::subfigure_legend),
    Op::new("Axes.boxplot", boxes::boxplot),
    Op::new("Axes.bxp", boxes::bxp),
    Op::new("Axes.violinplot", boxes::violinplot),
    Op::new("Axes.violinplot.set_facecolor", boxes::violin_alpha),
    Op::new("Axes.violin", boxes::violin),
    Op::new("Axes.errorbar", errorbar::errorbar),
    Op::new("ErrorbarContainer.remove", errorbar::remove),
    Op::new("Axes.errorbar.color_cycle", errorbar::color_cycle),
    Op::new("Axes.stem", stem::stem),
    Op::new("Axes.stem.dates", stem::dates),
    Op::new("Axes.stem.baseline", stem::baseline),
    Op::new("Axes.hist", hist::hist),
    Op::new("Axes.hist.dates", hist::hist_dates),
    Op::new("Axes.get_legend_handles_labels", hist::legend_labels),
    Op::new("Axes.hist.log", hist::log_barstacked),
    Op::new("Axes.hist2d", hist::hist2d),
    Op::new("Axes.stairs", hist::stairs),
    Op::new("StepPatch.set_data", hist::stairs_update),
    Op::new("Axes.scatter", scatter::scatter),
    Op::new("Axes.eventplot", scatter::eventplot),
    Op::new("MollweideAxes.transProjection", misc::mollweide),
    Op::new("pyplot.sca", misc::sca_figures),
    Op::new("Figure.savefig", misc::savefig_raw),
];

/// Runs `f` on a fresh `Figure::new().subplot()` in the probe's style
/// and rcParams.
fn on_axes(args: Args<'_>, f: impl FnOnce(&Axes) -> Outcome) -> Outcome {
    styled(args, || f(&Figure::new().subplot()))
}

/// A plt-rs result, its error as the rejection matplotlib's error mirrors.
fn made<T>(result: Result<T, impl std::fmt::Display>) -> Result<T, ProbeError> {
    result.map_err(ProbeError::rejected)
}

/// A list of RGBA colours as JSON.
fn rgba_list(colors: &[Rgba]) -> Json {
    Json::Array(colors.iter().map(|&c| encode::rgba(c)).collect())
}

/// A list of numbers as JSON.
fn numbers(values: &[f64]) -> Json {
    Json::Array(values.iter().map(|&v| Json::from(v)).collect())
}

/// A pair of numbers (limits) as JSON.
fn pair((low, high): (f64, f64)) -> Json {
    numbers(&[low, high])
}

/// A count as JSON.
fn count(n: usize) -> Json {
    Json::from(n as f64)
}

/// A legend label as the spec records it: `null` when hidden.
fn legend_label(label: Option<String>) -> Json {
    label
        .filter(|label| !label.is_empty() && !label.starts_with('_'))
        .map_or(Json::Null, Json::from)
}

/// A colour argument: a name, an RGB(A) list or `[name, alpha]`.
fn color(value: &Json) -> Result<Color, ProbeError> {
    match value {
        Json::String(name) => Ok(Color::from(name.as_str())),
        Json::Array(items) => match items.as_slice() {
            [Json::String(name), alpha] => {
                let alpha = alpha
                    .as_f64()
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad alpha in {value}")))?;
                let rgba = made(Color::from(name.as_str()).resolve())?;
                Ok(Color::from(Rgba::new(rgba.r, rgba.g, rgba.b, alpha)))
            }
            _ => match value.as_f64_vec().as_deref() {
                Some(&[r, g, b]) => Ok(Color::from([r, g, b])),
                Some(&[r, g, b, a]) => Ok(Color::from([r, g, b, a])),
                _ => Err(ProbeError::BadArgs(format!("not a colour: {value}"))),
            },
        },
        other => Err(ProbeError::BadArgs(format!("not a colour: {other}"))),
    }
}

/// Whether `value` is a list of colours rather than one colour.
fn is_color_list(value: &Json) -> bool {
    value.as_array().is_some_and(|items| {
        items
            .iter()
            .all(|item| matches!(item, Json::String(_) | Json::Array(_)))
    })
}

/// One colour, or a list of colours, as a list.
fn colors(value: &Json) -> Result<Vec<Color>, ProbeError> {
    if is_color_list(value) {
        value
            .as_array()
            .unwrap_or_default()
            .iter()
            .map(color)
            .collect()
    } else {
        Ok(vec![color(value)?])
    }
}

/// Optional argument `key` converted by `f`.
fn optional<T>(
    args: Args<'_>,
    key: &str,
    f: impl FnOnce(&Json) -> Result<T, ProbeError>,
) -> Result<Option<T>, ProbeError> {
    args.get(key).filter(|v| !v.is_null()).map(f).transpose()
}

/// A list of strings.
fn strings(value: &Json) -> Result<Vec<String>, ProbeError> {
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .ok_or_else(|| ProbeError::BadArgs(format!("not a list of strings: {value}")))
}

/// A number or a list of numbers as a list.
fn number_list(value: &Json) -> Result<Vec<f64>, ProbeError> {
    value
        .as_f64()
        .map(|v| vec![v])
        .or_else(|| value.as_f64_vec())
        .ok_or_else(|| ProbeError::BadArgs(format!("not numbers: {value}")))
}

/// An ISO date such as `"2013-09-28T11:00:00"`.
fn iso_date(text: &str) -> Result<Datetime64, ProbeError> {
    made(Datetime64::parse(text))?.ok_or_else(|| ProbeError::BadArgs(format!("{text} is NaT")))
}

/// A list of ISO dates.
fn iso_dates(value: &Json) -> Result<Vec<Datetime64>, ProbeError> {
    value
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("not a list of dates: {value}")))?
        .iter()
        .map(|day| {
            iso_date(
                day.as_str()
                    .ok_or_else(|| ProbeError::BadArgs(format!("not a date: {day}")))?,
            )
        })
        .collect()
}

/// A list of lists of numbers.
fn datasets(value: &Json) -> Result<Vec<Vec<f64>>, ProbeError> {
    value
        .as_array()
        .and_then(|sets| sets.iter().map(Json::as_f64_vec).collect())
        .ok_or_else(|| ProbeError::BadArgs(format!("not a list of datasets: {value}")))
}
