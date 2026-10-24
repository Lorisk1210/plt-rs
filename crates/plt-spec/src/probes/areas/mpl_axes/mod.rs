//! Probe operations for the `mpl_axes` area: matplotlib's own non-image
//! tests of `Axes` (`tests/test_axes.py`; see
//! `spec/reference/probes/test_mpl_axes.py`), run in the style each probe
//! names. The ops are grouped by topic: [`axis`] (formatters, twins,
//! arrows and annotations), [`hexbin`], [`mesh`] (images, pseudocolour
//! meshes and `pcolorfast`), [`bars`] (bars, broken bars, grouped bars and a
//! figure legend), [`bar_label`], [`boxes`] (box and violin plots),
//! [`errorbar`], [`stem`], [`hist`] (histograms and stairs), [`scatter`]
//! (scatter and event plots), [`lines`] (plot formats, infinite and
//! segment lines, fill colours), [`pie`], [`ticks`] (tick parameters,
//! gridlines, tick labels, offset texts and axis labels), [`titles`],
//! [`secondary`] (secondary axes), [`extents`] (tight boxes and spine
//! extents), [`contents`] (artist lists, spectral plots, insets and other
//! single calls) and [`misc`] (projections, pyplot's current axes and
//! saving).

mod axis;
mod bar_label;
mod bars;
mod boxes;
mod contents;
mod errorbar;
mod extents;
mod hexbin;
mod hist;
mod lines;
mod mesh;
mod misc;
mod pie;
mod scatter;
mod secondary;
mod stem;
mod ticks;
mod titles;

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
    Op::new("Axes.plot", lines::plot),
    Op::new("Axes.axline", lines::axline),
    Op::new("Axes.vlines", lines::vlines),
    Op::new("Axes.hlines", lines::hlines),
    Op::new("Axes.fill_between.cycle", lines::fill_between_cycle),
    Op::new("Axes.bar.color_cycle", bars::bar_color_cycle),
    Op::new("Axes.eventplot.segments", scatter::eventplot_segments),
    Op::new("Axes.eventplot.legend", scatter::eventplot_legend),
    Op::new("Axes.pie", pie::pie),
    Op::new("Axes.pie_label", pie::pie_label),
    Op::new("Axes.bar_label", bar_label::bar_label),
    Op::new("Axes.barh", bars::barh),
    Op::new("Axes.bar.errorbar", bars::bar_errorbar),
    Op::new("Axes.broken_barh.paths", bars::broken_barh_paths),
    Op::new("Axes.hist.bins", hist::hist_bins),
    Op::new("Axes.errorbar.caps", errorbar::caps),
    Op::new("Axes.legend.errorbar", errorbar::empty_legend),
    Op::new("Axes.tick_params", ticks::tick_params_state),
    Op::new("Axes.tick_params.rc_context", ticks::rc_context_ticks),
    Op::new("Axes.tick_params.reset", ticks::reset_grid),
    Op::new("Axes.grid", ticks::grid),
    Op::new("Axis.get_offset_text", ticks::offset_text),
    Op::new("Axis.set_ticklabels", ticks::set_ticklabels),
    Op::new("Axis.get_tick_params", ticks::get_tick_params),
    Op::new("Axis.label", ticks::label_formatting),
    Op::new("Axes.title.get_position", titles::title_positions),
    Op::new("Axes.set_title.twiny", titles::twiny_title_gap),
    Op::new("Axes.secondary_axis", secondary::secondary_axis),
    Op::new("Axes.secondary_xaxis.resize", secondary::secondary_resize),
    Op::new(
        "Axes.secondary_xaxis.minor_locator",
        secondary::secondary_minor_locator,
    ),
    Op::new(
        "Axes.secondary_xaxis.formatter",
        secondary::secondary_formatter,
    ),
    Op::new("Axes.secondary_axis.set_ticks", secondary::secondary_ticks),
    Op::new("pyplot.subplot", contents::subplot_geometry),
    Op::new("Axes.specgram", contents::specgram_extent),
    Op::new("Axes.psd", contents::psd_inverted),
    Op::new("Axes.pcolorfast", mesh::pcolorfast),
    Op::new("Axes.clim", mesh::clim),
    Op::new("FigureCanvasBase.inaxes", contents::inaxes),
    Op::new("Axes.add_patch", contents::wedge_bounds),
    Op::new("Axes.artist_sublists", contents::artist_sublists),
    Op::new("Axes.autoscale", contents::path_autoscale),
    Op::new("Axes.ecdf", contents::ecdf),
    Op::new("Axes.plot.large_offset", contents::large_offset),
    Op::new("Axes.barbs.dates", contents::vectors_at_dates),
    Op::new("Figure.savefig.pdf", contents::pdf_rasterized),
    Op::new("Axes.indicate_inset", contents::indicate_inset),
    Op::new(
        "Axes.set_box_aspect.constrained",
        contents::box_aspect_constrained,
    ),
    Op::new("Axes.get_tightbbox.decorations", extents::decoration_boxes),
    Op::new("Axes.get_tightbbox.axis_off", extents::tight_bbox_axis_off),
    Op::new(
        "Axes.get_tightbbox.long_label",
        extents::tight_bbox_long_label,
    ),
    Op::new("Axes.get_tightbbox.nan_text", extents::tight_bbox_nan_text),
    Op::new("Axes.add_collection.autolim", contents::relim_autolim_false),
    Op::new("Axes.remove.child_axes", contents::child_axes_removal),
    Op::new("Axes.inset_axes.projection", contents::inset_projection),
    Op::new("Axes.redraw_in_frame", contents::redraw_in_frame),
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
