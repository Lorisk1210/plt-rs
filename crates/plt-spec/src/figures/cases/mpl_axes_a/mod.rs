//! `figures/cases_mpl_axes_a.py`: matplotlib's own image tests from
//! `tests/test_axes.py` (up to line 2069) — every `@image_comparison` test
//! and the `fig_test` side of every `@check_figures_equal` test (with the
//! `fig_ref` side as `<test>_ref` where it draws something else).
//!
//! Cases keep the test's style: the test suite's default `classic` plus
//! `_classic_test_patch` (8 × 6 in figures saved at 100 dpi), or the
//! reference configuration for tests asking for `mpl20`/`default`.
//! `remove_text=True` becomes [`remove_ticks_and_titles`] at the end of
//! the build.

mod artists;
mod fills;
mod hexbin;
mod images;
mod jpl;
mod labels;
mod limits;
mod markevery;
mod meshes;
mod signals;
mod twins;

use plt::rc::RcParams;

use super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

/// `image.interpolation: nearest`, set by the imshow tests "to match the
/// existing baseline image".
const NEAREST: &[(&str, &str)] = &[("image.interpolation", "nearest")];

/// `lines.markeredgewidth: 30`, which `test_formatter_ticker` sets to
/// check that it leaves the ticks alone.
const MARKEREDGEWIDTH_30: &[(&str, &str)] = &[("lines.markeredgewidth", "30")];

/// `pcolormesh.snap: False`, set by the pcolormesh tests until their
/// baseline images are regenerated.
fn unsnapped(rc: &mut RcParams) -> CaseResult {
    rc.set("pcolormesh.snap", "False")?;
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    classic("mpl_axes_a/invisible_axes", labels::invisible_axes),
    classic("mpl_axes_a/label_loc_vertical", labels::label_loc_vertical),
    classic(
        "mpl_axes_a/label_loc_horizontal",
        labels::label_loc_horizontal,
    ),
    classic("mpl_axes_a/label_loc_rc", labels::label_loc_rc),
    classic("mpl_axes_a/acorr", signals::acorr),
    classic("mpl_axes_a/acorr_integers", signals::acorr_integers),
    classic("mpl_axes_a/spy", signals::spy),
    FigureCase::new("mpl_axes_a/matshow", signals::matshow).sized(8.0, 6.0, 80.0),
    FigureCase::new("mpl_axes_a/formatter_ticker_001", jpl::formatter_ticker_001)
        .with_rc(MARKEREDGEWIDTH_30),
    FigureCase::new("mpl_axes_a/formatter_ticker_002", jpl::formatter_ticker_002)
        .with_rc(MARKEREDGEWIDTH_30),
    FigureCase::new("mpl_axes_a/formatter_ticker_003", jpl::formatter_ticker_003)
        .with_rc(MARKEREDGEWIDTH_30),
    FigureCase::new("mpl_axes_a/formatter_ticker_004", jpl::formatter_ticker_004)
        .with_rc(MARKEREDGEWIDTH_30),
    FigureCase::new("mpl_axes_a/formatter_ticker_005", jpl::formatter_ticker_005)
        .with_rc(MARKEREDGEWIDTH_30),
    FigureCase::new(
        "mpl_axes_a/twin_axis_locators_formatters",
        twins::twin_axis_locators_formatters,
    ),
    classic("mpl_axes_a/twin_logscale_x", twins::twin_logscale_x),
    classic("mpl_axes_a/twin_logscale_y", twins::twin_logscale_y),
    classic("mpl_axes_a/twinx_axis_scales", twins::twinx_axis_scales),
    classic(
        "mpl_axes_a/minorticks_on_rcParams_both",
        twins::minorticks_on_rc_params_both,
    ),
    classic(
        "mpl_axes_a/autoscale_tiny_range",
        limits::autoscale_tiny_range,
    ),
    classic("mpl_axes_a/sticky_shared_axes", limits::sticky_shared_axes),
    classic(
        "mpl_axes_a/sticky_shared_axes_ref",
        limits::sticky_shared_axes_ref,
    ),
    FigureCase::new("mpl_axes_a/sticky_tolerance", limits::sticky_tolerance),
    FigureCase::new(
        "mpl_axes_a/sticky_tolerance_contourf",
        limits::sticky_tolerance_contourf,
    ),
    FigureCase::new("mpl_axes_a/nonfinite_limits", limits::nonfinite_limits),
    FigureCase::new(
        "mpl_axes_a/limits_empty_data_scatter",
        limits::limits_empty_data_scatter,
    ),
    FigureCase::new(
        "mpl_axes_a/limits_empty_data_plot",
        limits::limits_empty_data_plot,
    ),
    FigureCase::new(
        "mpl_axes_a/limits_empty_data_fill_between",
        limits::limits_empty_data_fill_between,
    ),
    FigureCase::new("mpl_axes_a/single_point", limits::single_point)
        .with_rc(&[("lines.marker", "o"), ("axes.grid", "True")]),
    classic("mpl_axes_a/shaped_data", limits::shaped_data),
    FigureCase::new("mpl_axes_a/canonical", limits::canonical),
    FigureCase::new(
        "mpl_axes_a/bar_tick_label_single",
        limits::bar_tick_label_single,
    ),
    classic("mpl_axes_a/hexbin_extent", hexbin::hexbin_extent),
    classic("mpl_axes_a/hexbin_empty", hexbin::hexbin_empty),
    FigureCase::new("mpl_axes_a/hexbin_log", hexbin::hexbin_log),
    FigureCase::new("mpl_axes_a/hexbin_linear", hexbin::hexbin_linear),
    classic(
        "mpl_axes_a/hexbin_mincnt_behavior_upon_C_parameter",
        hexbin::hexbin_mincnt_behavior_upon_c_parameter,
    ),
    classic(
        "mpl_axes_a/hexbin_mincnt_behavior_upon_C_parameter_ref",
        hexbin::hexbin_mincnt_behavior_upon_c_parameter_ref,
    ),
    FigureCase::new("mpl_axes_a/basic_annotate", artists::basic_annotate),
    classic("mpl_axes_a/arrow_simple", artists::arrow_simple),
    classic("mpl_axes_a/errorbar_dashes", artists::errorbar_dashes),
    FigureCase::new("mpl_axes_a/aitoff_proj", artists::aitoff_proj).sized(8.0, 4.2, 100.0),
    classic(
        "mpl_axes_a/polycollection_joinstyle",
        artists::polycollection_joinstyle,
    ),
    FigureCase::new("mpl_axes_a/arc_angles", artists::arc_angles),
    classic("mpl_axes_a/arc_ellipse", artists::arc_ellipse),
    FigureCase::new("mpl_axes_a/imshow", images::imshow).with_rc(NEAREST),
    FigureCase::new("mpl_axes_a/imshow_clip", images::imshow_clip).with_rc(NEAREST),
    FigureCase::new("mpl_axes_a/fill_units", jpl::fill_units).sized(6.4, 4.8, 60.0),
    FigureCase::new("mpl_axes_a/axvspan_epoch", jpl::axvspan_epoch),
    FigureCase::new("mpl_axes_a/axhspan_epoch", jpl::axhspan_epoch),
    classic(
        "mpl_axes_a/fill_between_interpolate",
        fills::fill_between_interpolate,
    ),
    FigureCase::new(
        "mpl_axes_a/fill_between_interpolate_decreasing",
        fills::fill_between_interpolate_decreasing,
    )
    .sized(9.0, 9.0, 100.0),
    classic(
        "mpl_axes_a/fill_between_interpolate_nan",
        fills::fill_between_interpolate_nan,
    ),
    FigureCase::new("mpl_axes_a/symlog", fills::symlog),
    classic("mpl_axes_a/symlog2", fills::symlog2),
    classic("mpl_axes_a/pcolormesh", meshes::pcolormesh).with_rc_setup(unsnapped),
    classic("mpl_axes_a/pcolormesh_small", meshes::pcolormesh_small),
    classic("mpl_axes_a/pcolormesh_alpha", meshes::pcolormesh_alpha).with_rc_setup(unsnapped),
    classic("mpl_axes_a/pcolormesh_rgba_3", meshes::pcolormesh_rgba_3),
    classic("mpl_axes_a/pcolormesh_rgba_4", meshes::pcolormesh_rgba_4),
    classic(
        "mpl_axes_a/pcolormesh_nearest_noargs",
        meshes::pcolormesh_nearest_noargs,
    ),
    classic(
        "mpl_axes_a/pcolormesh_log_scale",
        meshes::pcolormesh_log_scale,
    ),
    classic("mpl_axes_a/pcolor_log_scale", meshes::pcolor_log_scale),
    FigureCase::new(
        "mpl_axes_a/pcolormesh_datetime_axis",
        meshes::pcolormesh_datetime_axis,
    ),
    FigureCase::new(
        "mpl_axes_a/pcolor_datetime_axis",
        meshes::pcolor_datetime_axis,
    ),
    classic("mpl_axes_a/pcolornearest", meshes::pcolornearest),
    classic("mpl_axes_a/pcolornearestunits", meshes::pcolornearestunits),
    classic(
        "mpl_axes_a/pcolorauto_snap_false",
        meshes::pcolorauto_snap_false,
    ),
    classic(
        "mpl_axes_a/pcolorauto_snap_true",
        meshes::pcolorauto_snap_true,
    ),
    FigureCase::new("mpl_axes_a/markevery", markevery::markevery),
    FigureCase::new("mpl_axes_a/markevery_line", markevery::markevery_line),
    classic(
        "mpl_axes_a/markevery_linear_scales",
        markevery::markevery_linear_scales,
    ),
    classic(
        "mpl_axes_a/markevery_linear_scales_zoomed",
        markevery::markevery_linear_scales_zoomed,
    ),
    classic(
        "mpl_axes_a/markevery_log_scales",
        markevery::markevery_log_scales,
    ),
    FigureCase::new("mpl_axes_a/markevery_polar", markevery::markevery_polar),
    classic(
        "mpl_axes_a/markevery_linear_scales_nans",
        markevery::markevery_linear_scales_nans,
    ),
    classic("mpl_axes_a/marker_edges", markevery::marker_edges),
];
