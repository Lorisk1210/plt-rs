//! `figures/cases_mpl_collections.py`: matplotlib's own image tests from
//! `tests/test_collections.py` and `tests/test_patches.py` — every
//! `@image_comparison` test and the `fig_test` side of every
//! `@check_figures_equal` test.
//!
//! Cases keep the test's style: the test suite's default `classic` plus
//! `_classic_test_patch` (8 × 6 in figures saved at 100 dpi, or at
//! classic's 80 dpi figure resolution for the tests that save at it), or
//! the reference configuration for tests asking for `mpl20`/`default`.
//! `remove_text=True` becomes [`remove_ticks_and_titles`] at the end of
//! the build.

mod arcs;
mod collections;
mod events;
mod patches;

pub(super) use patches::cut_stars;

use super::mpl_tests::classic;
pub(super) use super::mpl_tests::remove_ticks_and_titles;
use crate::figures::FigureCase;

/// The `EventCollection` tests (all in the `mpl20` style).
const EVENTS: &[FigureCase] = &[
    FigureCase::new(
        "mpl_collections/EventCollection__get_props",
        events::get_props,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__set_positions",
        events::set_positions,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__add_positions",
        events::add_positions,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__append_positions",
        events::append_positions,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__extend_positions",
        events::extend_positions,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__switch_orientation",
        events::switch_orientation,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__switch_orientation_2x",
        events::switch_orientation_2x,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__set_orientation",
        events::set_orientation,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__set_linelength",
        events::set_linelength,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__set_lineoffset",
        events::set_lineoffset,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__set_prop_1",
        events::set_prop_1,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__set_prop_2",
        events::set_prop_2,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__set_prop_3",
        events::set_prop_3,
    ),
    FigureCase::new(
        "mpl_collections/EventCollection__set_color",
        events::set_color,
    ),
];

/// The other tests of `test_collections.py`.
const COLLECTIONS: &[FigureCase] = &[
    FigureCase::new(
        "mpl_collections/collection_log_datalim",
        collections::collection_log_datalim,
    ),
    classic(
        "mpl_collections/EllipseCollection",
        collections::ellipse_collection,
    ),
    FigureCase::new(
        "mpl_collections/polycollection_close",
        collections::polycollection_close,
    )
    .with_rc(&[("axes3d.automargin", "True")]),
    classic(
        "mpl_collections/scalarmap_change_cmap",
        collections::scalarmap_change_cmap,
    ),
    classic(
        "mpl_collections/regularpolycollection_rotate",
        collections::regularpolycollection_rotate,
    ),
    classic(
        "mpl_collections/regularpolycollection_scale",
        collections::regularpolycollection_scale,
    ),
    classic("mpl_collections/size_in_xy", collections::size_in_xy),
    FigureCase::new(
        "mpl_collections/cap_and_joinstyle_image",
        collections::cap_and_joinstyle_image,
    ),
    FigureCase::new(
        "mpl_collections/scatter_post_alpha",
        collections::scatter_post_alpha,
    ),
    classic(
        "mpl_collections/fill_between_poly_collection_set_data",
        collections::fill_between_poly_collection_set_data,
    ),
    classic(
        "mpl_collections/fill_between_poly_collection_set_data_step_pre",
        collections::fill_between_poly_collection_set_data_step_pre,
    ),
    FigureCase::new(
        "mpl_collections/check_masked_offsets",
        collections::check_masked_offsets,
    ),
    classic(
        "mpl_collections/masked_set_offsets",
        collections::masked_set_offsets,
    ),
    classic(
        "mpl_collections/striped_lines_orange",
        collections::striped_lines_orange,
    ),
    classic(
        "mpl_collections/striped_lines_r_k",
        collections::striped_lines_r_k,
    ),
    classic(
        "mpl_collections/hatch_linewidth",
        collections::hatch_linewidth,
    ),
];

/// The tests of `test_patches.py`.
const PATCHES: &[FigureCase] = &[
    classic(
        "mpl_collections/rotate_rect_draw",
        patches::rotate_rect_draw,
    ),
    FigureCase::new("mpl_collections/units_rectangle", patches::units_rectangle),
    classic(
        "mpl_collections/dash_offset_patch_draw",
        patches::dash_offset_patch_draw,
    ),
    FigureCase::new("mpl_collections/clip_to_bbox", patches::clip_to_bbox),
    classic(
        "mpl_collections/patch_alpha_coloring",
        patches::patch_alpha_coloring,
    ),
    classic(
        "mpl_collections/patch_alpha_override",
        patches::patch_alpha_override,
    ),
    classic(
        "mpl_collections/patch_custom_linestyle",
        patches::patch_custom_linestyle,
    ),
    classic(
        "mpl_collections/patch_linestyle_none",
        patches::patch_linestyle_none,
    ),
    classic("mpl_collections/wedge_range", patches::wedge_range),
    FigureCase::new(
        "mpl_collections/multi_color_hatch",
        patches::multi_color_hatch,
    ),
    FigureCase::new(
        "mpl_collections/connection_patch",
        patches::connection_patch,
    ),
    classic(
        "mpl_collections/connection_patch_fig",
        patches::connection_patch_fig,
    )
    .sized(8.0, 6.0, 80.0),
    classic(
        "mpl_collections/connection_patch_pixel_points",
        patches::connection_patch_pixel_points,
    )
    .sized(8.0, 6.0, 80.0),
    classic("mpl_collections/shadow", patches::shadow).sized(8.0, 6.0, 80.0),
    classic(
        "mpl_collections/set_and_get_hatch_linewidth",
        patches::set_and_get_hatch_linewidth,
    ),
    FigureCase::new(
        "mpl_collections/patch_edgegapcolor_visual",
        patches::patch_edgegapcolor_visual,
    ),
];

/// The arc and annulus tests of `test_patches.py`.
const ARCS: &[FigureCase] = &[
    FigureCase::new("mpl_collections/large_arc", arcs::large_arc),
    FigureCase::new("mpl_collections/rotated_arcs", arcs::rotated_arcs).sized(10.0, 10.0, 100.0),
    FigureCase::new("mpl_collections/annulus", arcs::annulus),
    FigureCase::new("mpl_collections/annulus_setters", arcs::annulus_setters),
    FigureCase::new("mpl_collections/annulus_setters2", arcs::annulus_setters2),
    FigureCase::new("mpl_collections/autoscale_arc", arcs::autoscale_arc).sized(4.0, 1.0, 100.0),
    classic("mpl_collections/arc_in_collection", arcs::arc_in_collection),
    classic("mpl_collections/modifying_arc", arcs::modifying_arc),
];

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [EVENTS, COLLECTIONS, PATCHES, ARCS].concat()
}
