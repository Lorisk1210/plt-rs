//! `figures/cases_gallery_tutorials.py`: the gallery's tutorials (one case
//! per figure), the first frame of its animations and the figures its
//! event handling examples show before any interaction. The introductory
//! tutorials (artists, coding shortcuts and pyplot), the image tutorial,
//! the lifecycle of a plot, the animations, the event handling examples
//! and their pickers and editors each have a submodule, and so do the
//! editors run as written, with blitting.

mod animation;
mod basics;
mod blitted_editors;
mod editors;
mod events;
mod images;
mod lifecycle;
mod pyplot;

use pltrs::numeric::Grid;

use crate::figures::FigureCase;
use crate::rng::SpecRng;

pub(super) use crate::figures::cases::common::GALLERY_SEED as SEED;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        basics::CASES,
        images::CASES,
        lifecycle::CASES,
        pyplot::CASES,
        animation::CASES,
        events::CASES,
        editors::CASES,
        blitted_editors::CASES,
    ]
    .concat()
}

/// The next `rows × cols` uniform draws of `g` in `[0, 1)`, row by row
/// (`np.random.rand(rows, cols)`, `_rows`).
fn uniform_rows(g: &mut SpecRng, rows: usize, cols: usize) -> Grid<f64> {
    Grid::from_vec(rows, cols, g.uniform(rows * cols, 0.0, 1.0)).expect("as many draws as cells")
}
