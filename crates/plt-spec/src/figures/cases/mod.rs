//! Rust translations of the spec's figure cases, one module per group.

mod axes;
mod bars;
mod basic;
mod colorbar;
mod contour;
mod fields;
mod geo;
mod hist;
mod images;
mod layout;
mod legend;
mod lines;
mod misc;
mod mplot3d;
mod offsetbox;
mod patches;
mod polar;
mod pyplot;
mod scatter;
mod selectors;
mod stats;
mod styles;
mod text;
mod vectors;

use super::FigureCase;

/// Every translated case.
pub(crate) fn all() -> Vec<FigureCase> {
    [
        axes::FORMATTING,
        axes::GRIDS,
        axes::LABELS,
        axes::LIMITS,
        axes::SCALES,
        axes::SPINES,
        axes::TICKS,
        axes::TWINS,
        bars::CASES,
        basic::CASES,
        colorbar::CASES,
        contour::CASES,
        fields::BARBS,
        fields::QUIVER,
        fields::STREAMPLOT,
        geo::CASES,
        hist::CASES,
        images::CASES,
        layout::CONSTRAINED,
        layout::GRIDS,
        layout::INSETS,
        layout::TIGHT,
        legend::CASES,
        lines::CASES,
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
        mplot3d::CONTOURS,
        mplot3d::OBJECTS,
        mplot3d::SCATTER,
        mplot3d::SURFACES,
        mplot3d::VIEW,
        offsetbox::CASES,
        patches::COLLECTIONS,
        patches::FANCY,
        patches::HATCHING,
        patches::SHAPES,
        patches::STYLES,
        polar::CASES,
        pyplot::CASES,
        scatter::CASES,
        selectors::CASES,
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
        text::LAYOUT,
        text::MATH,
        text::TABLES,
        vectors::CASES,
    ]
    .concat()
}
