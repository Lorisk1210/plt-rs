//! `figures/cases_api_gaps_b.py`: figures drawn through the matplotlib API
//! pltrs gained in its axes and artist classes (its custom legend
//! handlers draw the last legend guide figures, in `gallery_guides_axes`).

mod setters;

use crate::figures::FigureCase;

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("api_gaps_b/axes_locator", setters::axes_locator),
    FigureCase::new("api_gaps_b/collection_setters", setters::collection_setters),
    FigureCase::new("api_gaps_b/image_set_data", setters::image_set_data),
    FigureCase::new("api_gaps_b/date_axes", setters::date_axes),
];
