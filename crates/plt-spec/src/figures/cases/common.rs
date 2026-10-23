//! Small helpers several case groups share, grouped by topic and
//! re-exported here.

mod axes;
mod data;
mod styles;

pub(super) use self::axes::{
    axis_artist, blank, decorate, example_plot, face_cmap, hide_top_right, locator_strips,
    logit_grid, mosaic, named_tick_strip, polar_box, polar_degrees, polar_grid_helper, show_label,
    spine_bounds_demo, strip_title, subplots_3d, tick_strip, tickless,
};
pub(super) use self::data::{
    Annulus, GALLERY_SEED, bivariate_normal, contour_field, cumsum, double_sine, extremes,
    framed_bivariate_normal, func3, image_field, inverse_mercator, map, markevery_sine, mercator,
    mesh_field, mgrid, mgrid_100, mgrid_count, on_mesh, path_patch_curve, scan, smooth2d, spike,
    terrain, two_bumps,
};
pub(super) use self::styles::{
    aliased, arrow, black_arrow, colors, contour_effects, cosine, count, flow_labels,
    flow_orientations, hatch_shadow_text, hatches, local_max, padded, pointing, ranges,
    shown_labels, sized, stroke, styled_box, test_annotation, ticked_stroke, values, with_stroke,
};
