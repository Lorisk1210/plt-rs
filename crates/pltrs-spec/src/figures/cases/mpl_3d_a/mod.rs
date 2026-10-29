//! `figures/cases_mpl_3d_a.py`: matplotlib's own image tests from the
//! first part of `mpl_toolkits/mplot3d/tests/test_axes3d.py` — every
//! `@mpl3d_image_comparison` and `@image_comparison` test and the
//! `fig_test` side of every `@check_figures_equal` test, by topic.
//!
//! `mpl3d_image_comparison` tests ask for `mpl20` and run in the reference
//! configuration, without titles and tick labels (`remove_text=True`
//! becomes [`remove_ticks_and_titles`] at the end of the build) unless
//! they say otherwise; `check_figures_equal` tests run under the test
//! suite's default `classic` plus `_classic_test_patch` style.

mod axes;
mod contours;
mod plots;
mod scatter;
mod surfaces;

use std::f64::consts::PI;

use pltrs::figure::Figure;
use pltrs::mplot3d::{Axes3D, Axes3DError, Axes3DOptions};
use pltrs::numeric::{Grid, arange, linspace, linspace_with, meshgrid};

use super::common::aliased;
use super::mpl_tests::classic;
pub(super) use super::mpl_tests::remove_ticks_and_titles;
use crate::figures::FigureCase;

/// `rc` of the tests written before matplotlib 3.9 changed the 3-D
/// margins.
const AUTOMARGIN: &[(&str, &str)] = &[("axes3d.automargin", "True")];

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [AXES, PLOTS, CONTOURS, SCATTER, SURFACES].concat()
}

/// Axes, ticks, labels, aspects, views and texts.
const AXES: &[FigureCase] = &[
    classic("mpl_3d_a/invisible_axes", |fig| {
        axes::invisible_axes(fig);
        Ok(())
    }),
    FigureCase::new("mpl_3d_a/grid_off", |fig| {
        axes::grid_off(fig);
        Ok(())
    }),
    FigureCase::new("mpl_3d_a/invisible_ticks_axis", |fig| {
        axes::invisible_ticks_axis(fig);
        Ok(())
    }),
    FigureCase::new("mpl_3d_a/axis_positions", axes::axis_positions),
    FigureCase::new(
        "mpl_3d_a/axis_positions_inverted",
        axes::axis_positions_inverted,
    )
    .sized(10.0, 6.0, 100.0),
    FigureCase::new("mpl_3d_a/aspects", axes::aspects),
    FigureCase::new("mpl_3d_a/aspects_adjust_box", axes::aspects_adjust_box)
        .sized(11.0, 3.0, 100.0),
    FigureCase::new("mpl_3d_a/axes3d_primary_views", axes::primary_views),
    FigureCase::new("mpl_3d_a/text3d", axes::text3d).with_rc(AUTOMARGIN),
    classic("mpl_3d_a/text3d_modification", axes::text3d_modification),
    classic("mpl_3d_a/tight_layout_text", axes::tight_layout_text),
];

/// Bars, lines and `fill_between`.
const PLOTS: &[FigureCase] = &[
    FigureCase::new("mpl_3d_a/bar3d", plots::bar3d),
    FigureCase::new("mpl_3d_a/bar3d_shaded", plots::bar3d_shaded).sized(16.0, 4.0, 100.0),
    FigureCase::new("mpl_3d_a/bar3d_notshaded", plots::bar3d_notshaded),
    FigureCase::new("mpl_3d_a/lines3d", plots::lines3d),
    classic("mpl_3d_a/plot_scalar", plots::plot_scalar),
    FigureCase::new("mpl_3d_a/mixedsubplot", plots::mixedsubplot)
        .sized(4.0, 8.0, 100.0)
        .with_rc(AUTOMARGIN),
    FigureCase::new("mpl_3d_a/plot_3d_from_2d", plots::plot_3d_from_2d),
    FigureCase::new("mpl_3d_a/fill_between_quad", plots::fill_between_quad),
    FigureCase::new("mpl_3d_a/fill_between_polygon", plots::fill_between_polygon),
];

/// Contours.
const CONTOURS: &[FigureCase] = &[
    FigureCase::new("mpl_3d_a/contour3d", contours::contour3d).with_rc(AUTOMARGIN),
    FigureCase::new("mpl_3d_a/contour3d_extend3d", contours::contour3d_extend3d)
        .with_rc(AUTOMARGIN),
    FigureCase::new("mpl_3d_a/contourf3d", contours::contourf3d).with_rc(AUTOMARGIN),
    FigureCase::new("mpl_3d_a/contourf3d_fill", contours::contourf3d_fill).with_rc(AUTOMARGIN),
    classic(
        "mpl_3d_a/contourf3d_extend_both",
        contours::contourf3d_extend_both,
    ),
    classic(
        "mpl_3d_a/contourf3d_extend_min",
        contours::contourf3d_extend_min,
    ),
    classic(
        "mpl_3d_a/contourf3d_extend_max",
        contours::contourf3d_extend_max,
    ),
    FigureCase::new("mpl_3d_a/tricontour", contours::tricontour).with_rc(AUTOMARGIN),
];

/// Scatter plots.
const SCATTER: &[FigureCase] = &[
    FigureCase::new("mpl_3d_a/scatter3d", scatter::scatter3d),
    FigureCase::new("mpl_3d_a/scatter3d_color", scatter::scatter3d_color),
    FigureCase::new("mpl_3d_a/scatter3d_linewidth", scatter::scatter3d_linewidth),
    classic("mpl_3d_a/scatter3d_cmap_alpha", scatter::cmap_alpha),
    classic(
        "mpl_3d_a/scatter3d_linewidth_modification",
        scatter::linewidth_modification,
    ),
    classic("mpl_3d_a/scatter3d_modification", scatter::modification),
    classic("mpl_3d_a/scatter3d_sorting", scatter::sorting),
    classic(
        "mpl_3d_a/marker_draw_order_data_reversed_m50",
        scatter::draw_order_m50,
    ),
    classic(
        "mpl_3d_a/marker_draw_order_data_reversed_130",
        scatter::draw_order_130,
    ),
    classic(
        "mpl_3d_a/marker_draw_order_view_rotated",
        scatter::draw_order_view_rotated,
    ),
    classic("mpl_3d_a/plot_scatter_masks", scatter::plot_scatter_masks),
];

/// Surfaces, triangulated surfaces and wireframes.
const SURFACES: &[FigureCase] = &[
    FigureCase::new("mpl_3d_a/surface3d", surfaces::surface3d)
        .with_rc(&[("pcolormesh.snap", "False"), ("axes3d.automargin", "True")]),
    FigureCase::new(
        "mpl_3d_a/surface3d_label_offset_tick_position",
        surfaces::label_offset_tick_position,
    ),
    FigureCase::new("mpl_3d_a/surface3d_shaded", surfaces::surface3d_shaded).with_rc(AUTOMARGIN),
    FigureCase::new("mpl_3d_a/surface3d_masked", surfaces::surface3d_masked),
    classic("mpl_3d_a/plot_surface_None_arg", surfaces::none_arg),
    FigureCase::new(
        "mpl_3d_a/surface3d_masked_strides",
        surfaces::masked_strides,
    ),
    FigureCase::new("mpl_3d_a/trisurf3d", surfaces::trisurf3d),
    FigureCase::new("mpl_3d_a/trisurf3d_shaded", surfaces::trisurf3d_shaded),
    FigureCase::new("mpl_3d_a/wireframe3d", surfaces::wireframe3d),
    FigureCase::new(
        "mpl_3d_a/wireframe3dasymmetric",
        surfaces::wireframe3dasymmetric,
    ),
    FigureCase::new(
        "mpl_3d_a/wireframe3dzerocstride",
        surfaces::wireframe3dzerocstride,
    ),
    FigureCase::new(
        "mpl_3d_a/wireframe3dzerorstride",
        surfaces::wireframe3dzerorstride,
    ),
];

/// `fig.subplots(rows, cols, subplot_kw={"projection": "3d"})`, row by
/// row.
fn subplots_3d(fig: &Figure, rows: usize, cols: usize) -> Result<Vec<Axes3D>, Axes3DError> {
    (1..=rows * cols)
        .map(|index| fig.add_subplot_3d(rows, cols, index, Axes3DOptions::default()))
        .collect()
}

/// `np.arange(n)` as floats.
fn range(n: u32) -> Vec<f64> {
    (0..n).map(f64::from).collect()
}

/// `sin(hypot(x, y))` on `np.arange(-5, 5, 0.25)` squared.
fn sinc_surface() -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let axis = arange(-5.0, 5.0, 0.25);
    let (x, y) = meshgrid(&axis, &axis);
    let z = Grid::from_fn(x.rows(), x.cols(), |i, j| x[(i, j)].hypot(y[(i, j)]).sin());
    (x, y, z)
}

/// The points of the `trisurf3d` tests: rings of a disc (alternate rings
/// turned by half a step) and the saddle `sin(-x y)` over them.
fn disc_points() -> [Vec<f64>; 3] {
    const ANGLES: u32 = 36;
    let radii = linspace(0.125, 1.0, 8);
    let angles = linspace_with(0.0, 2.0 * PI, ANGLES as usize, false);
    let mut x = vec![0.0];
    let mut y = vec![0.0];
    for angle in &angles {
        for (ring, radius) in radii.iter().enumerate() {
            let turned = if ring % 2 == 1 {
                angle + PI / f64::from(ANGLES)
            } else {
                *angle
            };
            x.push(radius * turned.cos());
            y.push(radius * turned.sin());
        }
    }
    let z = x.iter().zip(&y).map(|(x, y)| (-x * y).sin()).collect();
    [x, y, z]
}
