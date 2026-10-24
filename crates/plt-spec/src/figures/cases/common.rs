//! Small helpers several case groups share, grouped by topic and
//! re-exported here.

mod axes;
mod data;
mod stats;
mod styles;

pub(super) use self::axes::{
    MarkeveryAxes, axis_artist, blank, constrained, decorate, example_plot, face_cmap,
    hide_top_right, locator_strips, logit_grid, markevery_panel, mosaic, named_tick_strip,
    polar_box, polar_degrees, polar_grid_helper, show_label, spine_bounds_demo, stacked_pair,
    strip_title, subplots_3d, tick_strip, tickless,
};
pub(super) use self::data::{
    Annulus, GALLERY_SEED, arange_grid, bivariate_normal, contour_field, cumsum, datetime64,
    double_sine, extremes, framed_bivariate_normal, func3, image_field, indices, inverse_mercator,
    map, markevery_sine, mercator, mesh_field, mgrid, mgrid_100, mgrid_count, on_mesh,
    path_patch_curve, points, radians, scan, segment, smooth2d, spike, strings, terrain, trimmed,
    two_bumps,
};
pub(super) use self::stats::{bins, drawn_boxes, drawn_violins, laid_out, with_sym};
pub(super) use self::styles::{
    aliased, arrow, autoscale_unset, black_arrow, colors, contour_effects, cosine, count, ellipse,
    flow_labels, flow_orientations, hatch_shadow_text, hatches, local_max, padded, percent_label,
    pointing, ranges, rgba, shown_labels, sized, stroke, styled_box, test_annotation,
    ticked_stroke, values, with_stroke, xkcd,
};
