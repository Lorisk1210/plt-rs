//! `figures/cases_gallery_text.py`: the gallery's text, labels and
//! annotations section, one case per figure. Text placement, labels and
//! titles, fonts, math text, annotations and their arrows, angle
//! annotations, arrow graphs, annotation boxes and legends each have a
//! submodule.

mod angles;
mod annotations;
mod arrows;
mod boxes;
mod fonts;
mod labels;
mod legends;
mod math;
mod placement;

use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        placement::CASES,
        labels::CASES,
        math::CASES,
        fonts::CASES,
        annotations::CASES,
        legends::CASES,
        boxes::CASES,
        arrows::CASES,
        angles::CASES,
    ]
    .concat()
}
