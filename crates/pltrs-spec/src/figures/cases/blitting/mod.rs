//! `figures/cases_blitting.py`: figures drawn on their canvas, then
//! changed by putting saved regions back and drawing single artists over
//! them. Every case is taken from the canvas ([`Capture::Canvas`]): the
//! figures of the blitting tutorial, regions restored in part and
//! elsewhere, artists of every kind drawn one by one, `redraw_in_frame`,
//! and blitted animations.
//!
//! [`Capture::Canvas`]: crate::figures::Capture::Canvas

mod animation;
mod artists;
mod tutorial;

use std::f64::consts::PI;

use pltrs::numeric::linspace;

use crate::figures::{CaseResult, FigureCase};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [tutorial::CASES, artists::CASES, animation::CASES].concat()
}

/// A canvas case of this group.
const fn blitting(
    name: &'static str,
    build: fn(&pltrs::figure::Figure) -> CaseResult,
) -> FigureCase {
    FigureCase::new(name, build).from_canvas()
}

/// The 100 points from 0 to 2π the waves are drawn at (`X`).
fn wave_x() -> Vec<f64> {
    linspace(0.0, 2.0 * PI, 100)
}

/// `f(x + shift)` at every `x`.
fn shifted(x: &[f64], shift: f64, f: fn(f64) -> f64) -> Vec<f64> {
    x.iter().map(|x| f(x + shift)).collect()
}
