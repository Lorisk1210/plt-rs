//! `figures/cases_toolkits.py`: `axes_grid1` image grids and dividers in
//! one submodule, inset locators, RGB axes and `axisartist` in another,
//! bbox connectors and host and parasite axes in a third.

mod grids;
mod insets;
mod parasites;

use plt::numeric::{Grid, linspace};

use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("toolkits/imagegrid_cbar_edge", grids::cbar_edge).sized(7.0, 4.0, 100.0),
    FigureCase::new(
        "toolkits/imagegrid_direction_label_modes",
        grids::label_modes,
    )
    .sized(7.0, 4.0, 100.0),
    FigureCase::new("toolkits/imagegrid_each_cbar_left", grids::each_cbar_left)
        .sized(7.0, 3.0, 100.0),
    FigureCase::new("toolkits/divider_colorbars_sides", grids::divider_sides)
        .sized(6.0, 5.0, 100.0),
    FigureCase::new(
        "toolkits/divider_marginal_histograms",
        grids::marginal_histograms,
    )
    .sized(5.0, 5.0, 100.0),
    FigureCase::new("toolkits/inset_locator_inset_axes", insets::inset_locator)
        .sized(6.0, 4.0, 100.0),
    FigureCase::new("toolkits/zoomed_inset_mark_inset", insets::zoomed_marked)
        .sized(5.0, 4.0, 100.0),
    FigureCase::new("toolkits/rgb_axes", insets::rgb).sized(5.0, 4.0, 100.0),
    FigureCase::new(
        "toolkits/bbox_connector_patch",
        parasites::bbox_connector_patch,
    )
    .sized(5.0, 4.0, 100.0),
    FigureCase::new(
        "toolkits/host_twinx_twiny_legend",
        parasites::twinx_twiny_legend,
    ),
    FigureCase::new("toolkits/host_twin_aux_tight", parasites::twin_aux_tight)
        .sized(7.0, 3.5, 100.0),
    FigureCase::new(
        "toolkits/host_aux_axes_best_legend",
        parasites::aux_axes_best_legend,
    )
    .sized(5.0, 3.5, 100.0),
    FigureCase::new(
        "toolkits/axisartist_axisline_styles",
        insets::axisline_styles,
    )
    .sized(4.0, 4.0, 100.0),
];

/// `_field(n, phase)`: `sin(x + phase) * cos(y)` on an `n × n` grid over
/// `[-3, 3]²`.
fn field(n: usize, phase: f64) -> Grid<f64> {
    let x = linspace(-3.0, 3.0, n);
    Grid::from_fn(n, n, |r, c| (x[c] + phase).sin() * x[r].cos())
}
