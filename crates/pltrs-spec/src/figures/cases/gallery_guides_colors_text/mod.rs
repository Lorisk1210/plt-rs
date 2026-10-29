//! `figures/cases_gallery_guides_colors_text.py`: matplotlib's user guide
//! pages on colours and text, its quick start guide and its rcParams
//! guide, one case per figure. Standalone colorbars, making colormaps,
//! norms, the colormap reference and its lightness, colour
//! specifications, annotations, math text and text properties, the text
//! and quick start tutorials and rcParams each have a submodule.

mod annotation_boxes;
mod annotations;
mod colorbars;
mod colormaps;
mod colors;
mod customizing;
mod lightness;
mod manipulation;
mod norms;
mod quick_start;
mod text;
mod text_intro;

use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        colorbars::CASES,
        manipulation::CASES,
        norms::CASES,
        &colormaps::cases(),
        colors::CASES,
        annotations::CASES,
        annotation_boxes::CASES,
        text::CASES,
        text_intro::CASES,
        quick_start::CASES,
        customizing::CASES,
    ]
    .concat()
}
