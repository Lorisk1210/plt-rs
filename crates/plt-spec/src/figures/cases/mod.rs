//! Rust translations of the spec's figure cases, one module per group.

mod afm;
mod axes;
mod axisartist;
mod bars;
mod basic;
mod colorbar;
mod common;
mod contour;
mod fields;
mod figure;
mod gallery_images;
mod gallery_lines;
mod gallery_stats;
mod gallery_subplots;
mod gallery_text;
mod gallery_ticks;
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

/// The cases porting matplotlib's own image tests (the `mpl_*` groups).
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
        &mpl_text::cases(),
        &mpl_toolkits::cases(),
    ]
    .concat()
}

/// The cases porting matplotlib's gallery examples (the `gallery_*`
/// groups).
fn gallery_examples() -> Vec<FigureCase> {
    [
        gallery_images::cases(),
        gallery_lines::cases(),
        gallery_stats::cases(),
        gallery_subplots::cases(),
        gallery_text::cases(),
        gallery_ticks::cases(),
    ]
    .concat()
}
