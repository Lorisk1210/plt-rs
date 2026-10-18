//! `figures/cases_colorbar.py`: colorbar placement (orientation,
//! location, shrink/aspect/pad/fraction, `cax`, several parents),
//! extensions, ticks/format/label, discrete and non-linear norms, edges,
//! contour lines, constrained layout and following the mappable; and
//! `figures/cases_colorbar_ext.py` (see [`suite`]).

mod constrained;
mod contours;
pub(super) mod data;
mod extensions;
mod follow;
mod norms;
mod placement;
mod suite;
mod ticks;

use crate::figures::{CaseResult, FigureCase};

/// A case on a 5 × 3.5 in canvas.
const fn wide(name: &'static str, build: fn(&plt::figure::Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build).sized(5.0, 3.5, 100.0)
}

/// A case on a 4 × 4 in canvas.
const fn square(name: &'static str, build: fn(&plt::figure::Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build).sized(4.0, 4.0, 100.0)
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("colorbar/image_default", placement::image_default),
    square("colorbar/horizontal", placement::horizontal),
    square("colorbar/location_top", placement::location_top),
    wide("colorbar/location_left", placement::location_left),
    wide("colorbar/shrink_aspect_pad", placement::shrink_aspect_pad),
    wide("colorbar/fraction", placement::fraction),
    wide(
        "colorbar/extend_both_contourf",
        contours::extend_both_contourf,
    ),
    wide(
        "colorbar/extend_min_extendfrac",
        extensions::extend_min_extendfrac,
    ),
    wide(
        "colorbar/extend_max_extendrect",
        extensions::extend_max_extendrect,
    ),
    square(
        "colorbar/extend_both_horizontal_auto",
        extensions::extend_both_horizontal_auto,
    ),
    wide("colorbar/ticks_format_label", ticks::ticks_format_label),
    wide("colorbar/formatter_minorticks", ticks::formatter_minorticks),
    wide("colorbar/boundarynorm_uniform", norms::boundarynorm_uniform),
    wide(
        "colorbar/boundarynorm_proportional_extend",
        norms::boundarynorm_proportional_extend,
    ),
    wide("colorbar/lognorm", norms::lognorm),
    wide("colorbar/twoslopenorm", norms::twoslopenorm),
    wide("colorbar/drawedges", norms::drawedges),
    wide("colorbar/scalarmappable_only", norms::scalarmappable_only),
    wide("colorbar/scatter", norms::scatter),
    wide("colorbar/contour_lines", contours::contour_lines),
    wide("colorbar/contourf_add_lines", contours::contourf_add_lines),
    FigureCase::new("colorbar/multiple_axes", placement::multiple_axes).sized(5.0, 5.0, 100.0),
    wide("colorbar/cax_explicit", placement::cax_explicit),
    square(
        "colorbar/inset_cax_horizontal",
        placement::inset_cax_horizontal,
    ),
    FigureCase::new(
        "colorbar/constrained_layout",
        constrained::constrained_layout,
    )
    .sized(6.0, 4.0, 100.0),
    FigureCase::new(
        "colorbar/constrained_shared",
        constrained::constrained_shared,
    )
    .sized(6.0, 4.0, 100.0),
    wide("colorbar/follows_set_clim", follow::follows_set_clim),
    wide("colorbar/follows_norm_change", follow::follows_norm_change),
    FigureCase::new(
        "colorbar/extension_length_uniform",
        suite::extension_length_uniform,
    ),
    FigureCase::new(
        "colorbar/extension_length_proportional",
        suite::extension_length_proportional,
    ),
    FigureCase::new("colorbar/extension_shape_rect", suite::extension_shape_rect),
    FigureCase::new(
        "colorbar/drawedges_with_extensions",
        suite::drawedges_with_extensions,
    )
    .sized(5.0, 4.0, 100.0),
    FigureCase::new(
        "colorbar/boundaries_values_irregular",
        suite::boundaries_values_irregular,
    )
    .sized(6.0, 3.5, 100.0),
    FigureCase::new(
        "colorbar/lognorm_extend_minor_custom_ticks",
        suite::lognorm_extend_minor_custom_ticks,
    )
    .sized(6.0, 3.5, 100.0),
    FigureCase::new(
        "colorbar/power_symlog_centered_norms",
        suite::power_symlog_centered_norms,
    )
    .sized(8.0, 3.0, 100.0),
    FigureCase::new(
        "colorbar/anchor_panchor_ticklocation",
        suite::anchor_panchor_ticklocation,
    )
    .sized(6.0, 4.0, 100.0),
];
