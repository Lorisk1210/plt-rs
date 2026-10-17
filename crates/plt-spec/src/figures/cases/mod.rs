//! Rust translations of the spec's figure cases, one module per group.

mod axes;
mod bars;
mod basic;
mod colorbar;
mod contour;
mod fields;
mod hist;
mod images;
mod layout;
mod legend;
mod lines;
mod misc;
mod mplot3d;
mod patches;
mod polar;
mod scatter;
mod stats;
mod styles;
mod text;

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
        hist::CASES,
        images::CASES,
        layout::CASES,
        legend::CASES,
        lines::CASES,
        misc::ARTISTS,
        misc::DATES,
        misc::SANKEY,
        misc::SPECTRAL,
        misc::UNITS,
        misc::WIDGETS,
        mplot3d::CONTOURS,
        mplot3d::OBJECTS,
        mplot3d::SCATTER,
        mplot3d::SURFACES,
        mplot3d::VIEW,
        patches::COLLECTIONS,
        patches::FANCY,
        patches::HATCHING,
        patches::SHAPES,
        patches::STYLES,
        polar::CASES,
        scatter::CASES,
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
    ]
    .concat()
}
