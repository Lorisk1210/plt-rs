//! Rust translations of the spec's figure cases, one module per group.

mod afm;
mod api_gaps_a;
mod api_gaps_b;
mod axes;
mod axisartist;
mod bars;
mod basic;
mod blitting;
mod colorbar;
mod common;
mod contour;
mod fields;
mod figure;
mod gallery_3d;
mod gallery_axisartist;
mod gallery_guides_axes;
mod gallery_guides_colors_text;
mod gallery_images;
mod gallery_lines;
mod gallery_misc;
mod gallery_plot_types;
mod gallery_shapes;
mod gallery_stats;
mod gallery_subplots;
mod gallery_text;
mod gallery_ticks;
mod gallery_tutorials;
mod geo;
mod hinting;
mod hist;
mod images;
mod layout;
mod legend;
mod lines;
mod mathtext;
mod misc;
mod modern;
mod mpl_3d_a;
mod mpl_3d_b;
mod mpl_axes_a;
mod mpl_axes_b;
mod mpl_axes_c;
mod mpl_axes_d;
mod mpl_backends;
mod mpl_collections;
mod mpl_figure;
mod mpl_image;
mod mpl_lines;
mod mpl_misc;
mod mpl_polar;
mod mpl_rendering;
mod mpl_rest;
mod mpl_small;
mod mpl_tests;
mod mpl_tests_agg;
mod mpl_tests_axes;
mod mpl_tests_colorbar;
mod mpl_tests_constrainedlayout;
mod mpl_tests_datetime;
mod mpl_tests_figure;
mod mpl_tests_ft2font;
mod mpl_tests_image;
mod mpl_tests_legend;
mod mpl_tests_lines;
mod mpl_tests_marker;
mod mpl_tests_offsetbox;
mod mpl_tests_patches;
mod mpl_tests_quiver;
mod mpl_tests_table;
mod mpl_tests_text;
mod mpl_tests_tightlayout;
mod mpl_tests_units;
mod mpl_tests_widgets;
mod mpl_text;
mod mpl_toolkits;
mod mplot3d;
mod offsetbox;
mod patches;
mod polar;
mod pyplot;
mod rc;
mod scatter;
mod selectors;
mod shaping;
mod spines;
mod stats;
mod styles;
mod text;
mod ticks;
mod toolkits;
mod vectors;
mod widgets;

use super::FigureCase;

/// Every translated case.
pub(crate) fn all() -> Vec<FigureCase> {
    [
        afm::CASES,
        api_gaps_a::CASES,
        api_gaps_b::CASES,
        axes::FORMATTING,
        axes::GRIDS,
        axes::LABELS,
        axes::LIMITS,
        axes::SCALES,
        axes::SPINES,
        axes::TICKS,
        axes::TWINS,
        axisartist::CASES,
        bars::CASES,
        basic::CASES,
        &blitting::cases(),
        colorbar::CASES,
        contour::CASES,
        fields::BARBS,
        fields::QUIVER,
        fields::STREAMPLOT,
        figure::CASES,
        geo::ARTISTS,
        geo::CASES,
        geo::GRIDS,
        hinting::CASES,
        hist::CASES,
        images::CASES,
        layout::CONSTRAINED,
        layout::GRIDS,
        layout::INSETS,
        layout::TIGHT,
        legend::CASES,
        lines::CASES,
        mathtext::DELIMITER_CASES,
        mathtext::FONT_CASES,
        mathtext::LIGHTWEIGHT_CASES,
        mathtext::TESTS_A_CASES,
        mathtext::TESTS_B_CASES,
        misc::ARTISTS,
        misc::AXES_GRID,
        misc::AXISARTIST,
        misc::DATES,
        misc::INSETS,
        misc::SANKEY,
        misc::SPECTRAL,
        misc::UNITS,
        misc::WIDGETS,
        &matplotlib_tests(),
        &gallery_examples(),
        modern::CASES,
        mplot3d::API,
        mplot3d::API_PLOTS,
        mplot3d::AXLIM,
        mplot3d::CONTOURS,
        mplot3d::OBJECTS,
        mplot3d::SCATTER,
        mplot3d::SURFACES,
        mplot3d::VIEW,
        offsetbox::CASES,
        patches::COLLECTIONS,
        patches::FANCY,
        patches::GALLERY,
        patches::HATCHING,
        patches::SHAPES,
        patches::STYLES,
        polar::CASES,
        polar::RADIAL,
        polar::SECTORS,
        pyplot::CASES,
        &rc::cases(),
        scatter::CASES,
        scatter::LEGENDS,
        scatter::VARIANTS,
        selectors::CASES,
        spines::CASES,
        shaping::CASES,
        stats::BOXPLOT,
        stats::ERRORBAR,
        stats::EVENTS,
        stats::FILL_BETWEEN,
        stats::PIE,
        stats::VIOLIN,
        styles::CASES,
        styles::SHEETS,
        text::ANNOTATE,
        text::BOXES,
        text::EFFECTS,
        text::GALLERY,
        text::LAYOUT,
        text::MATH,
        text::TABLES,
        text::TABLE_CELLS,
        ticks::CASES,
        toolkits::CASES,
        vectors::CASES,
        widgets::CASES,
    ]
    .concat()
}

/// The cases porting matplotlib's own tests: its image tests (the `mpl_*`
/// groups) and the drawing of its other tests (the `mpl_tests_*` groups).
fn matplotlib_tests() -> Vec<FigureCase> {
    [
        &mpl_3d_a::cases(),
        &mpl_3d_b::cases(),
        mpl_axes_a::CASES,
        &mpl_axes_b::cases(),
        &mpl_axes_c::cases(),
        &mpl_axes_d::cases(),
        &mpl_backends::cases(),
        &mpl_collections::cases(),
        &mpl_figure::cases(),
        &mpl_image::cases(),
        &mpl_lines::cases(),
        &mpl_misc::cases(),
        &mpl_polar::cases(),
        &mpl_rendering::cases(),
        &mpl_rest::cases(),
        &mpl_small::cases(),
        &mpl_tests_agg::cases(),
        &mpl_tests_axes::cases(),
        &mpl_tests_datetime::cases(),
        &mpl_tests_ft2font::cases(),
        &mpl_tests_image::cases(),
        &mpl_tests_units::cases(),
        &mpl_tests_widgets::cases(),
        &mpl_text::cases(),
        &mpl_toolkits::cases(),
        &mpl_tests_figure::cases(),
        &mpl_tests_colorbar::cases(),
        &mpl_tests_legend::cases(),
        &mpl_tests_constrainedlayout::cases(),
        &mpl_tests_tightlayout::cases(),
        &mpl_tests_patches::cases(),
        &mpl_tests_text::cases(),
        &mpl_tests_lines::cases(),
        &mpl_tests_quiver::cases(),
        &mpl_tests_table::cases(),
        &mpl_tests_offsetbox::cases(),
        &mpl_tests_marker::cases(),
    ]
    .concat()
}

/// The cases porting matplotlib's gallery examples (the `gallery_*`
/// groups).
fn gallery_examples() -> Vec<FigureCase> {
    [
        gallery_3d::cases(),
        gallery_axisartist::cases(),
        gallery_guides_axes::cases(),
        gallery_guides_colors_text::cases(),
        gallery_images::cases(),
        gallery_lines::cases(),
        gallery_misc::cases(),
        gallery_plot_types::cases(),
        gallery_shapes::cases(),
        gallery_stats::cases(),
        gallery_subplots::cases(),
        gallery_text::cases(),
        gallery_ticks::cases(),
        gallery_tutorials::cases(),
    ]
    .concat()
}
