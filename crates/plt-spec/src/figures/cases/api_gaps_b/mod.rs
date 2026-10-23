//! `figures/cases_api_gaps_b.py`: figures drawn through the matplotlib API
//! plt-rs gained in its axes and artist classes, custom legend handlers
//! first.

mod legends;
mod setters;

use crate::figures::FigureCase;

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("api_gaps_b/legend_guide_11", legends::legend_guide_11),
    FigureCase::new("api_gaps_b/legend_guide_12", legends::legend_guide_12),
    FigureCase::new("api_gaps_b/axes_locator", setters::axes_locator),
    FigureCase::new("api_gaps_b/collection_setters", setters::collection_setters),
    FigureCase::new("api_gaps_b/image_set_data", setters::image_set_data),
    FigureCase::new("api_gaps_b/date_axes", setters::date_axes),
];
