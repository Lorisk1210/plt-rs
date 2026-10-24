//! The `path.*` and `agg.path.chunksize` cases.

use plt::colors::Color;
use plt::patheffects::{GcUpdate, PathEffect, Stroke};
use plt::prelude::*;
use plt::rc::RcParams;

use super::{SMALL, curves, rc_cases};
use crate::figures::{CaseResult, FigureCase};

/// `_sawtooth`: a curve with a sawtooth half a pixel high; simplification
/// at a threshold of a pixel flattens it.
fn sawtooth(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 10.0, 3000);
    let y: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, v)| v.sin() + if i % 2 == 0 { 0.008 } else { -0.008 })
        .collect();
    fig.subplot().plot(x, y)?;
    Ok(())
}

/// `_lissajous`: a dense, thick, translucent Lissajous curve; where
/// separately drawn chunks cross, they blend twice.
fn lissajous(fig: &Figure) -> CaseResult {
    let t = linspace(0.0, std::f64::consts::TAU, 3000);
    let x: Vec<f64> = t.iter().map(|v| (3.0 * v).sin()).collect();
    let y: Vec<f64> = t.iter().map(|v| (4.0 * v).sin()).collect();
    let props = LineProps::default().linewidth(5.0).alpha(0.5);
    fig.subplot().plot_with(x, y, props)?;
    Ok(())
}

/// `rc={"path.effects": [withStroke(linewidth=4, foreground="0.8")]}`.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every rc setup shares the fallible signature"
)]
fn stroke_effect(rc: &mut RcParams) -> CaseResult {
    let outline = Stroke::new(GcUpdate {
        linewidth: Some(4.0),
        foreground: Some(Color::from("0.8")),
        ..GcUpdate::default()
    });
    rc.path_effects = vec![PathEffect::with_stroke(outline)];
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = rc_cases! {
    sawtooth, SMALL => {
        "path_simplify_threshold": [("path.simplify_threshold", "1.0")],
        "path_simplify": [("path.simplify", "False"), ("path.simplify_threshold", "1.0")],
    }
    lissajous, SMALL => {
        "agg_path_chunksize": [("agg.path.chunksize", "300")],
    }
    curves, SMALL => {
        "path_snap": [("path.snap", "False")],
        "path_sketch": [("path.sketch", "3, 30, 2")],
    }
};

/// `path.effects`, which has no rc-file text.
pub(crate) const EFFECTS: &[FigureCase] = &[FigureCase::new("rc/path_effects", curves)
    .sized(SMALL.0, SMALL.1, 100.0)
    .with_rc_setup(stroke_effect)];
