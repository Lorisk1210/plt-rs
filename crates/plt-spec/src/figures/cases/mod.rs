//! Rust translations of the spec's figure cases, one module per group.

mod afm;
mod axes;
mod axisartist;
mod bars;
mod basic;
mod colorbar;
mod contour;
mod fields;
mod figure;
mod geo;
mod hinting;
mod hist;
mod images;
mod layout;
mod legend;
mod lines;
mod mathtext;
mod misc;
mod mplot3d;
mod offsetbox;
mod patches;
mod polar;
mod pyplot;
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
