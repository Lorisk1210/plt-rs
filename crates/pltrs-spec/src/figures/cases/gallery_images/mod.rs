//! `figures/cases_gallery_images.py`: matplotlib's gallery section
//! "Images, contours and fields", one case per figure of each example.

mod antialiasing;
mod contour_fills;
mod contour_lines;
mod data;
mod fields;
mod heatmap;
mod images;
mod meshes;
mod norms;
mod placement;
mod signals;
mod tri;

use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        antialiasing::CASES,
        contour_fills::CASES,
        contour_lines::CASES,
        fields::CASES,
        heatmap::CASES,
        images::CASES,
        meshes::CASES,
        norms::CASES,
        placement::CASES,
        signals::CASES,
        tri::CASES,
    ]
    .concat()
}
