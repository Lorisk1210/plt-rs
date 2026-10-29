//! `figures/cases_gallery_axisartist.py`: the gallery's axisartist and
//! widgets sections, one case per figure. Axis directions, axis lines
//! and parasite axes, then cursors, buttons, sliders and selectors each
//! have a submodule.

mod axislines;
mod buttons;
mod cursors;
mod directions;
mod parasites;
mod selectors;
mod sliders;

use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        directions::CASES,
        axislines::CASES,
        parasites::CASES,
        cursors::CASES,
        buttons::CASES,
        sliders::CASES,
        selectors::CASES,
    ]
    .concat()
}

/// `T = np.arange(0.0, 2.0, 0.01)`, the time of the widgets' sine waves.
fn t() -> Vec<f64> {
    pltrs::numeric::arange(0.0, 2.0, 0.01)
}

/// `np.sin(2 * np.pi * freq * t)` over `t`.
fn sine(t: &[f64], freq: f64) -> Vec<f64> {
    t.iter()
        .map(|t| (2.0 * std::f64::consts::PI * freq * t).sin())
        .collect()
}
