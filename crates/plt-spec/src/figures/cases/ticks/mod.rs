//! `figures/cases_ticks.py`: every locator and formatter of the gallery,
//! minor ticks, the tickers of log, symlog, logit, asinh and function
//! scales, offsets and scientific notation, tick parameters and rotated or
//! multi-line tick labels.

mod gallery;
mod labels;
mod locators;
mod log;
mod scales;

use crate::figures::{CaseResult, FigureCase};

/// A case on a `width × height` in canvas at 100 dpi.
const fn sized(
    name: &'static str,
    build: fn(&plt::figure::Figure) -> CaseResult,
    width: f64,
    height: f64,
) -> FigureCase {
    FigureCase::new(name, build).sized(width, height, 100.0)
}

pub(crate) const CASES: &[FigureCase] = &[
    sized(
        "ticks/locators_gallery",
        gallery::locators_gallery,
        8.0,
        6.0,
    ),
    sized(
        "ticks/formatters_gallery",
        gallery::formatters_gallery,
        8.0,
        7.0,
    ),
    sized(
        "ticks/maxn_integer_symmetric_prune",
        locators::maxn_integer_symmetric_prune,
        7.0,
        3.0,
    ),
    sized(
        "ticks/minor_locators_and_params",
        locators::minor_locators_and_params,
        6.0,
        4.5,
    ),
    sized("ticks/minor_ndivs_rc", locators::minor_ndivs_rc, 5.0, 3.5).with_rc(&[
        ("xtick.minor.visible", "True"),
        ("ytick.minor.visible", "True"),
        ("xtick.minor.ndivs", "2"),
        ("ytick.minor.ndivs", "5"),
    ]),
    sized("ticks/eng_formatter", log::eng_formatter, 7.0, 7.0),
    sized("ticks/log_minor_labels", log::log_minor_labels, 7.0, 3.5),
    sized(
        "ticks/log_formatter_variants",
        log::log_formatter_variants,
        6.0,
        5.0,
    ),
    sized(
        "ticks/log_wide_range_decimation",
        log::log_wide_range_decimation,
        5.0,
        4.0,
    ),
    sized("ticks/log_locator_subs", log::log_locator_subs, 6.0, 3.5),
    sized("ticks/symlog_subs_minor", log::symlog_subs_minor, 6.0, 3.5),
    sized("ticks/logit_demo", scales::logit_demo, 6.4, 8.5),
    sized("ticks/asinh_demo", scales::asinh_demo, 7.0, 3.5),
    sized(
        "ticks/function_scale_mercator",
        scales::function_scale_mercator,
        5.0,
        4.0,
    ),
    sized(
        "ticks/offset_threshold_and_fixed_offset",
        labels::offset_threshold_and_fixed_offset,
        7.0,
        3.0,
    )
    .with_rc(&[("axes.formatter.offset_threshold", "2")]),
    sized(
        "ticks/mathtext_offset_and_scale",
        labels::mathtext_offset_and_scale,
        6.0,
        3.5,
    )
    .with_rc(&[
        ("axes.formatter.use_mathtext", "True"),
        ("axes.formatter.limits", "-3, 3"),
    ]),
    sized(
        "ticks/rotated_multiline_labels",
        labels::rotated_multiline_labels,
        6.0,
        4.5,
    ),
    sized(
        "ticks/rotation_mode_xtick_ytick",
        labels::rotation_mode_xtick_ytick,
        6.0,
        4.0,
    ),
    sized(
        "ticks/tick_params_grid_and_sides",
        labels::tick_params_grid_and_sides,
        5.0,
        4.0,
    ),
    sized(
        "ticks/label_positions_top_right",
        labels::label_positions_top_right,
        5.0,
        4.0,
    ),
    sized(
        "ticks/set_ticks_labels_minor_kwargs",
        labels::set_ticks_labels_minor_kwargs,
        5.0,
        4.0,
    ),
];
