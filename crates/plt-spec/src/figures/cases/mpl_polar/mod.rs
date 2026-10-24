//! `figures/cases_mpl_polar.py`: matplotlib's own image tests from
//! `tests/test_polar.py`, `tests/test_contour.py` and
//! `tests/test_colorbar.py` — every `@image_comparison` test and the
//! `fig_test` side of every `@check_figures_equal` test, one submodule per
//! test module.
//!
//! Cases keep the test's style: the test suite's default `classic` plus
//! `_classic_test_patch` (8 × 6 in figures saved at 100 dpi, or at the dpi
//! the test saves at), or the reference configuration for tests asking for
//! `mpl20`/`default`. `remove_text=True` becomes
//! [`remove_ticks_and_titles`] at the end of the build.

mod colorbars;
mod contours;
mod polar;
mod positioning;

use plt::colors::Color;
use plt::contour::{ContourOptions, Levels, QuadField};
use plt::numeric::Grid;

use super::mpl_tests::classic;
pub(super) use super::mpl_tests::remove_ticks_and_titles;
use crate::figures::FigureCase;

/// Colours by name.
fn colors(names: &[&str]) -> Vec<Color> {
    names.iter().map(|&name| Color::from(name)).collect()
}

/// The levels most colorbar tests contour [`ramp`] at.
const RAMP_LEVELS: [f64; 7] = [0.0, 200.0, 400.0, 600.0, 800.0, 1000.0, 1200.0];

/// `np.arange(1200).reshape(30, 40)`, which most colorbar tests contour.
fn ramp() -> QuadField {
    QuadField::new(Grid::from_fn(30, 40, |r, c| (r * 40 + c) as f64))
}

/// Contours at [`RAMP_LEVELS`].
fn ramp_levels() -> ContourOptions {
    ContourOptions {
        levels: Levels::Values(RAMP_LEVELS.to_vec()),
        ..ContourOptions::default()
    }
}

/// The tests of `test_polar.py`.
const POLAR: &[FigureCase] = &[
    FigureCase::new("mpl_polar/polar_annotations", polar::polar_annotations),
    FigureCase::new(
        "mpl_polar/polar_coord_annotations",
        polar::polar_coord_annotations,
    ),
    FigureCase::new("mpl_polar/polar_alignment", polar::polar_alignment),
    classic("mpl_polar/polar_wrap", polar::polar_wrap),
    classic("mpl_polar/polar_units_1_ref", polar::polar_units_1_ref),
    classic("mpl_polar/polar_units_2_ref", polar::polar_units_2_ref),
    FigureCase::new("mpl_polar/polar_rmin", polar::polar_rmin),
    FigureCase::new("mpl_polar/polar_negative_rmin", polar::polar_negative_rmin),
    FigureCase::new("mpl_polar/polar_rorigin", polar::polar_rorigin),
    FigureCase::new("mpl_polar/polar_invertedylim", polar::polar_invertedylim),
    FigureCase::new(
        "mpl_polar/polar_invertedylim_rorigin",
        polar::polar_invertedylim_rorigin,
    ),
    FigureCase::new(
        "mpl_polar/polar_theta_position",
        polar::polar_theta_position,
    ),
    FigureCase::new(
        "mpl_polar/polar_rlabel_position",
        polar::polar_rlabel_position,
    ),
    FigureCase::new(
        "mpl_polar/polar_title_position",
        polar::polar_title_position,
    ),
    FigureCase::new("mpl_polar/polar_theta_limits", polar::polar_theta_limits)
        .sized(8.0, 6.0, 100.0),
    classic("mpl_polar/polar_rlim", polar::polar_rlim),
    classic("mpl_polar/polar_rlim_bottom", polar::polar_rlim_bottom),
    classic(
        "mpl_polar/polar_interpolation_steps_constant_r",
        polar::polar_interpolation_steps_constant_r,
    ),
    classic(
        "mpl_polar/polar_interpolation_steps_variable_r",
        polar::polar_interpolation_steps_variable_r,
    ),
    classic("mpl_polar/remove_shared_polar", polar::remove_shared_polar),
    classic(
        "mpl_polar/remove_shared_polar_ref",
        polar::remove_shared_polar_ref,
    ),
    FigureCase::new("mpl_polar/polar_log", polar::polar_log),
    classic("mpl_polar/polar_log_rorigin", polar::polar_log_rorigin),
    FigureCase::new(
        "mpl_polar/polar_errorbar_before",
        polar::polar_errorbar_before,
    )
    .sized(5.0, 5.0, 100.0),
    FigureCase::new(
        "mpl_polar/polar_errorbar_after",
        polar::polar_errorbar_after,
    )
    .sized(5.0, 5.0, 100.0),
];

/// The tests of `test_contour.py`.
const CONTOUR: &[FigureCase] = &[
    classic("mpl_polar/contour_set_paths", contours::contour_set_paths),
    FigureCase::new(
        "mpl_polar/contour_manual_labels",
        contours::contour_manual_labels,
    )
    .sized(6.0, 2.0, 200.0),
    FigureCase::new(
        "mpl_polar/contour_label_with_disconnected_segments",
        contours::contour_label_with_disconnected_segments,
    ),
    classic(
        "mpl_polar/given_colors_levels_and_extends",
        contours::given_colors_levels_and_extends,
    )
    .with_rc(CLASSIC_NO_SNAP),
    FigureCase::new("mpl_polar/hatch_colors", contours::hatch_colors),
    FigureCase::new("mpl_polar/log_locator_levels", contours::log_locator_levels),
    FigureCase::new(
        "mpl_polar/contour_datetime_axis",
        contours::contour_datetime_axis,
    ),
    FigureCase::new("mpl_polar/labels", contours::labels),
    classic("mpl_polar/corner_mask_1", contours::corner_mask_1),
    classic("mpl_polar/corner_mask_2", contours::corner_mask_2),
    FigureCase::new(
        "mpl_polar/contourf_log_extension",
        contours::contourf_log_extension,
    )
    .sized(10.0, 5.0, 100.0)
    .with_rc(NO_SNAP),
    FigureCase::new("mpl_polar/contour_addlines", contours::contour_addlines).with_rc(NO_SNAP),
    FigureCase::new("mpl_polar/contour_uneven", contours::contour_uneven).with_rc(NO_SNAP),
    classic(
        "mpl_polar/contour_closed_line_loop",
        contours::contour_closed_line_loop,
    )
    .sized(2.0, 2.0, 100.0),
    classic("mpl_polar/contour_manual", contours::contour_manual).sized(4.0, 4.0, 100.0),
    classic(
        "mpl_polar/contour_line_start_on_corner_edge",
        contours::contour_line_start_on_corner_edge,
    )
    .sized(6.0, 5.0, 100.0),
    FigureCase::new("mpl_polar/contourf_rasterize", contours::contourf_rasterize)
        .sized(6.4, 4.8, 25.0),
    classic("mpl_polar/all_algorithms", contours::all_algorithms),
    classic("mpl_polar/contour_aliases", contours::contour_aliases),
];

/// The tests of `test_colorbar.py` (the first two are covered by
/// `colorbar/extension_*`).
const COLORBAR: &[FigureCase] = &[
    FigureCase::new(
        "mpl_polar/colorbar_positioning_gridspec_1",
        positioning::colorbar_positioning_gridspec_1,
    )
    .sized(6.4, 4.8, 40.0),
    FigureCase::new(
        "mpl_polar/colorbar_positioning_gridspec_2",
        positioning::colorbar_positioning_gridspec_2,
    )
    .sized(6.4, 4.8, 40.0),
    FigureCase::new(
        "mpl_polar/colorbar_positioning_gridspec_3",
        positioning::colorbar_positioning_gridspec_3,
    )
    .sized(6.4, 4.8, 40.0),
    FigureCase::new(
        "mpl_polar/colorbar_positioning_1",
        positioning::colorbar_positioning_1,
    )
    .sized(6.4, 4.8, 40.0),
    FigureCase::new(
        "mpl_polar/colorbar_positioning_2",
        positioning::colorbar_positioning_2,
    )
    .sized(6.4, 4.8, 40.0),
    FigureCase::new(
        "mpl_polar/colorbar_positioning_3",
        positioning::colorbar_positioning_3,
    )
    .sized(6.4, 4.8, 40.0),
    FigureCase::new(
        "mpl_polar/colorbar_positioning_4",
        positioning::colorbar_positioning_4,
    )
    .sized(6.4, 4.8, 40.0),
    classic("mpl_polar/contour_colorbar", colorbars::contour_colorbar).sized(4.0, 2.0, 100.0),
    classic(
        "mpl_polar/gridspec_make_colorbar",
        colorbars::gridspec_make_colorbar,
    )
    .sized(8.0, 6.0, 40.0),
    classic(
        "mpl_polar/colorbar_single_scatter",
        colorbars::colorbar_single_scatter,
    )
    .sized(8.0, 6.0, 40.0),
    classic(
        "mpl_polar/colorbar_closed_patch",
        colorbars::colorbar_closed_patch,
    )
    .with_rc(CLASSIC_NO_SNAP),
    FigureCase::new("mpl_polar/keeping_xlabel", colorbars::keeping_xlabel),
    FigureCase::new(
        "mpl_polar/colorbar_change_lim_scale",
        colorbars::colorbar_change_lim_scale,
    ),
    classic(
        "mpl_polar/axes_handles_same_functions",
        colorbars::axes_handles_same_functions,
    ),
    FigureCase::new("mpl_polar/twoslope_colorbar", colorbars::twoslope_colorbar),
    FigureCase::new(
        "mpl_polar/proportional_colorbars",
        colorbars::proportional_colorbars,
    ),
    FigureCase::new(
        "mpl_polar/colorbar_extend_drawedges",
        colorbars::colorbar_extend_drawedges,
    )
    .sized(10.0, 4.0, 100.0)
    .with_rc(THICK_AXES),
    FigureCase::new(
        "mpl_polar/colorbar_contourf_extend_patches",
        colorbars::colorbar_contourf_extend_patches,
    )
    .sized(10.0, 4.0, 100.0)
    .with_rc(THICK_AXES),
    FigureCase::new("mpl_polar/nonorm", colorbars::nonorm).sized(6.0, 1.0, 100.0),
    FigureCase::new("mpl_polar/boundaries", colorbars::boundaries).sized(2.0, 2.0, 100.0),
    classic(
        "mpl_polar/colorbar_extend_alpha",
        colorbars::colorbar_extend_alpha,
    )
    .sized(8.0, 6.0, 40.0),
    classic("mpl_polar/passing_location", colorbars::passing_location),
];

/// The thicker axes lines of the `extend` colorbar tests.
const THICK_AXES: &[(&str, &str)] = &[("axes.linewidth", "2")];

/// What the tests written before `pcolormesh.snap` became the default
/// keep.
const NO_SNAP: &[(&str, &str)] = &[("pcolormesh.snap", "False")];

/// [`NO_SNAP`] on top of the test suite's patch of the classic style.
const CLASSIC_NO_SNAP: &[(&str, &str)] = &[
    ("ytick.alignment", "center_baseline"),
    ("hatch.color", "edge"),
    ("text.hinting", "default"),
    ("pcolormesh.snap", "False"),
];

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [POLAR, CONTOUR, COLORBAR].concat()
}
